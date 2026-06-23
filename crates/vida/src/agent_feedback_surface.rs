use std::process::ExitCode;

const MAX_FEEDBACK_NOTES_BYTES: usize = 2048;

fn canonical_feedback_outcome(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "success" => Some("success"),
        "failure" => Some("failure"),
        "neutral" => Some("neutral"),
        _ => None,
    }
}

pub(crate) async fn run_agent_feedback(args: super::AgentFeedbackArgs) -> ExitCode {
    let project_root = match super::resolve_runtime_project_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let outcome_input = args.outcome.as_deref().unwrap_or("success");
    let outcome = match canonical_feedback_outcome(outcome_input) {
        Some(canonical) => canonical,
        None => {
            eprintln!(
                "Unsupported feedback outcome `{outcome_input}`. Allowed values: success, failure, neutral."
            );
            return ExitCode::from(2);
        }
    };
    let task_class = args.task_class.as_deref().unwrap_or("unspecified");
    let input = super::HostAgentFeedbackInput {
        agent_id: &args.agent_id,
        score: args.score,
        outcome,
        task_class,
        notes: args.notes.as_deref(),
        source: "vida agent-feedback",
        task_id: None,
        task_display_id: None,
        task_title: None,
        runtime_role: None,
        selected_tier: Some(&args.agent_id),
        estimated_task_price_units: None,
        lifecycle_state: None,
        effective_score: None,
        reason: None,
    };
    match append_host_agent_feedback(&project_root, &input) {
        Ok(view) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&view).expect("agent feedback json should render")
                );
            } else {
                super::print_surface_header(super::RenderMode::Plain, "vida agent-feedback");
                println!(
                    "host cli system: {}",
                    view["host_cli_system"].as_str().unwrap_or("")
                );
                println!("agent id: {}", view["agent_id"].as_str().unwrap_or(""));
                println!(
                    "recorded score: {}",
                    view["recorded_score"].as_u64().unwrap_or_default()
                );
                println!(
                    "outcome: {}",
                    view["recorded_outcome"].as_str().unwrap_or("")
                );
                println!(
                    "task class: {}",
                    view["recorded_task_class"].as_str().unwrap_or("")
                );
                if let Some(notes) = view["recorded_notes"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                {
                    println!("notes: {notes}");
                }
                println!(
                    "effective score: {}",
                    view["strategy_row"]["effective_score"]
                        .as_u64()
                        .unwrap_or_default()
                );
                println!(
                    "lifecycle state: {}",
                    view["strategy_row"]["lifecycle_state"]
                        .as_str()
                        .unwrap_or("")
                );
                println!(
                    "scorecards store: {}",
                    view["scorecards_store"].as_str().unwrap_or("")
                );
                println!(
                    "strategy store: {}",
                    view["strategy_store"].as_str().unwrap_or("")
                );
                println!(
                    "observability store: {}",
                    view["observability_store"].as_str().unwrap_or("")
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn infer_feedback_outcome_from_close_reason(reason: &str) -> &'static str {
    let normalized = normalized_close_reason_for_feedback(reason);
    let inferred = if !feedback_failure_markers(&normalized).is_empty() {
        "failure"
    } else if !super::contains_keywords(
        &normalized,
        &[
            "neutral".to_string(),
            "partial".to_string(),
            "handoff".to_string(),
            "handoff pending".to_string(),
        ],
    )
    .is_empty()
    {
        "neutral"
    } else {
        "success"
    };
    canonical_feedback_outcome(inferred).expect("inferred feedback outcome must be canonical")
}

fn normalized_close_reason_for_feedback(reason: &str) -> String {
    let mut normalized = reason.to_ascii_lowercase();
    let mut ignored_phrases = ignored_feedback_contract_language(reason)
        .into_iter()
        .chain(ignored_canonical_close_meta_language(reason))
        .chain(ignored_feedback_meta_language(reason))
        .collect::<Vec<_>>();
    ignored_phrases.sort_by_key(|phrase| std::cmp::Reverse(phrase.len()));
    ignored_phrases.dedup();
    for phrase in ignored_phrases {
        normalized = normalized.replace(&phrase, " feedback_context_language ");
    }
    normalized
}

fn feedback_failure_markers(normalized_reason: &str) -> Vec<String> {
    super::contains_keywords(
        normalized_reason,
        &[
            "fail".to_string(),
            "failed".to_string(),
            "blocked".to_string(),
            "abort".to_string(),
            "abandon".to_string(),
            "rejected".to_string(),
            "rollback".to_string(),
        ],
    )
}

fn feedback_success_markers(normalized_reason: &str) -> Vec<String> {
    super::contains_keywords(
        normalized_reason,
        &[
            "tests passed".to_string(),
            "test passed".to_string(),
            "proof commands passed".to_string(),
            "proof passed".to_string(),
            "proofs passed".to_string(),
            "green".to_string(),
        ],
    )
}

fn close_feedback_outcome_inference(reason: &str, outcome: &str, score: u64) -> serde_json::Value {
    let normalized = normalized_close_reason_for_feedback(reason);
    let ignored_meta_language: Vec<String> = ignored_canonical_close_meta_language(reason)
        .into_iter()
        .chain(ignored_feedback_meta_language(reason))
        .collect();
    serde_json::json!({
        "outcome": outcome,
        "score": score,
        "failure_markers": feedback_failure_markers(&normalized),
        "success_markers": feedback_success_markers(&normalized),
        "ignored_contract_language": ignored_feedback_contract_language(reason),
        "ignored_meta_language": ignored_meta_language,
        "rule": "contract and marker-explanation language is not failure evidence; concrete failed outcomes still score as failure",
    })
}

fn ignored_feedback_contract_language(reason: &str) -> Vec<String> {
    ignored_feedback_phrases(
        reason,
        &["fail-closed", "fail closed", "fail_closed", "fail-closes"],
    )
}

fn ignored_feedback_meta_language(reason: &str) -> Vec<String> {
    let mut ignored = ignored_feedback_phrases(
        reason,
        &[
            "explicit failed markers still fail",
            "explicit failure markers still fail",
            "explicit failure markers still score as failure",
            "failure markers still fail",
            "failed markers still fail",
            "failure marker still fails",
            "failed marker still fails",
            "failure markers",
            "failed markers",
            "failure marker",
            "failed marker",
            "failure keywords",
            "failed keywords",
            "failure keyword",
            "failed keyword",
            "blocker list empty",
            "blocker list is empty",
            "empty blocker list",
            "blocker entries empty",
            "blocker entries are empty",
            "no blocker entries",
            "zero blocker entries",
            "blocker codes empty",
            "blocker codes are empty",
            "blocker details",
            "blocker detail",
            "blocker fields",
            "blocker field",
            "blocker flags",
            "blocker flag",
            "blocked flag",
            "blocked field",
            "continuation blockers remain separate",
            "continuation blockers remain",
            "continuation blocker remains",
            "continuation blocker",
            "continuation_blocked flag",
            "continuation_blocked",
            "failed/tampered parent adapter results",
            "failed/tampered parent-adapter results",
            "failed or tampered parent adapter results",
            "failed or tampered parent-adapter results",
            "contextual failed-result defect descriptions",
            "failed-result defect descriptions",
            "failed-result defect description",
            "failed-result wording",
            "failed result wording",
            "records failure",
            "recorded failure",
            "recording failure",
            "failure-case coverage",
            "failure case coverage",
            "failure-path coverage",
            "failure path coverage",
            "failure scenario coverage",
            "failure scenarios covered",
            "failure cases covered",
            "failure coverage",
            "failure-case",
            "failure case",
            "rejected wording coverage",
            "rejected wording",
            "rejected patch wording",
            "concrete rejected patch wording",
            "rejected outcome coverage",
            "rejection coverage",
            "rejected parent closure while child remains open",
            "rejected close invariant",
            "rejected parent closure path proved",
            "rejected alternatives",
            "rejected alternative",
            "rejected candidates",
            "rejected candidate",
            "rejected options",
            "rejected option",
            "rejected routes",
            "rejected route",
            "rejected profiles",
            "rejected profile",
            "rejected model profiles",
            "rejected model profile",
            "did not fail",
            "didn't fail",
            "does not fail",
            "do not fail",
            "not failed",
            "not a failure",
            "no failure",
            "without failure",
            "does not count as failure",
            "do not count as failure",
        ],
    );
    let normalized = reason.to_ascii_lowercase();
    if normalized.contains("diagnostics for failed subprocess")
        || normalized.contains("helper diagnostics for failed subprocess")
        || normalized.contains("failed subprocess status/stdout/stderr")
    {
        ignored.push("failed subprocess status".to_string());
        ignored.push("failed subprocess status/stdout/stderr".to_string());
    }
    ignored.sort();
    ignored.dedup();
    ignored
}

fn ignored_feedback_phrases(reason: &str, phrases: &[&str]) -> Vec<String> {
    let normalized = reason.to_ascii_lowercase();
    phrases
        .iter()
        .filter(|phrase| normalized.contains(**phrase))
        .map(|phrase| (*phrase).to_string())
        .collect()
}

fn default_feedback_score(outcome: &str, task_class: &str) -> u64 {
    match outcome {
        "failure" => 35,
        "neutral" => 60,
        _ => match task_class {
            "architecture" => 90,
            "verification" => 88,
            "specification" => 84,
            _ => 82,
        },
    }
}

fn ignored_canonical_close_meta_language(reason: &str) -> Vec<String> {
    let mut ignored = ignored_feedback_phrases(
        reason,
        &[
            "close feedback derivation",
            "blocked feedback derivation",
            "canonical close blocked feedback derivation",
            "blocker keyword matching",
            "blocked reason detection",
            "failure evidence",
            "concrete blocked task outcomes",
            "concrete blocked reasons",
            "top-level blocked/actionable",
            "top level blocked/actionable",
            "actionable blocked output",
            "actionable blockers",
            "fail closed with actionable blockers",
            "genuinely blocked",
            "readiness blockers",
            "readiness blocker",
            "verifier blocker closure regression fix",
            "blocker list empty",
            "blocker list is empty",
            "empty blocker list",
            "blocker entries empty",
            "blocker entries are empty",
            "no blocker entries",
            "zero blocker entries",
            "blocker codes empty",
            "blocker codes are empty",
            "no blocker codes",
            "zero blocker codes",
            "blocker details",
            "blocker detail",
            "blocker fields",
            "blocker field",
            "blocker flags",
            "blocker flag",
            "blocked flag",
            "blocked field",
            "continuation blockers remain separate",
            "continuation blockers remain",
            "continuation blocker remains",
            "continuation blocker",
            "continuation_blocked flag",
            "continuation_blocked",
            "blocker coverage",
            "spawn-blocker ordering",
            "blocked task projections",
            "blocked task projection",
            "blocked projections",
            "blocked projection",
            "structured blocked json",
            "stderr-only failure",
            "stderr-only exit",
            "blocked coverage",
            "blocked path coverage",
            "blocked-path coverage",
            "blocked scenario coverage",
            "blocked scenarios covered",
            "blocked routes",
            "blocked route",
            "blocked alternatives",
            "blocked alternative",
            "blocked candidates",
            "blocked candidate",
            "approval coverage",
            "awaiting approval coverage",
            "approval_wait coverage",
            "approval required coverage",
            "pending approval coverage",
            "ready/blocked/progress/list/tree",
        ],
    );
    ignored.extend(ignored_historical_blocker_meta_phrases(reason));
    ignored.extend(ignored_historical_failure_state_segments(reason));
    ignored.extend(ignored_canonical_close_meta_segments(reason));
    ignored.sort();
    ignored.dedup();
    ignored
}

fn ignored_historical_blocker_meta_phrases(reason: &str) -> Vec<String> {
    let normalized = reason.to_ascii_lowercase();
    let has_historical_context = [
        "closure_ready=false",
        "verdict text",
        "no longer bridges",
        "regression fix",
        "committed and pushed",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));

    if !has_historical_context {
        return Vec::new();
    }

    [
        "blocker/rework",
        "blocker_code/blockers",
        "blocked verification lane",
        "git/remote blocker",
    ]
    .into_iter()
    .filter(|phrase| normalized.contains(*phrase))
    .map(ToString::to_string)
    .collect()
}

fn ignored_canonical_close_historical_context(reason: &str) -> Vec<String> {
    let mut ignored = ignored_historical_blocker_meta_phrases(reason);
    ignored.extend(ignored_historical_failure_state_segments(reason));
    ignored.sort();
    ignored.dedup();
    ignored
}

fn ignored_historical_failure_state_segments(reason: &str) -> Vec<String> {
    let full_reason_normalized = reason.to_ascii_lowercase();
    reason
        .split(['.', ';', '\n'])
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = trimmed.to_ascii_lowercase();
            if !has_failure_state_language(&normalized)
                || !(has_historical_or_success_evidence_context(&normalized)
                    || has_resolved_failure_artifact_action_context(
                        &normalized,
                        &full_reason_normalized,
                    ))
                || has_current_failure_outcome_language(&normalized)
            {
                return None;
            }
            Some(normalized)
        })
        .collect()
}

fn has_failure_state_language(normalized: &str) -> bool {
    let keyword_markers = [
        "blocked".to_string(),
        "blocker".to_string(),
        "failed".to_string(),
        "failure".to_string(),
        "rejected".to_string(),
    ];
    if !super::contains_keywords(normalized, &keyword_markers).is_empty() {
        return true;
    }

    [
        "failure-state",
        "failure state",
        "canonical_gate_blocked",
        "canonical_status_blocked",
        "close_feedback_canonical_status_blocked",
        "status=blocked",
        "status: blocked",
        "blocker details",
        "blocker code",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn has_resolved_failure_artifact_action_context(
    segment_normalized: &str,
    full_reason_normalized: &str,
) -> bool {
    if !has_proof_or_success_context(full_reason_normalized)
        || has_unresolved_failure_artifact_context(segment_normalized)
    {
        return false;
    }

    let describes_resolution_action = [
        "rejected ",
        "rejects ",
        "rejecting ",
        "reject ",
        "guarded ",
        "guards ",
        "prevented ",
        "prevents ",
        "denied ",
        "denies ",
        "disallowed ",
        "disallows ",
    ]
    .iter()
    .any(|phrase| {
        segment_normalized.starts_with(phrase) || segment_normalized.contains(&format!(" {phrase}"))
    });
    let names_runtime_artifact = [
        "receipt",
        "receipts",
        "task-ensure",
        "task ensure",
        "final-snapshot",
        "final snapshot",
        "terminal closure",
        "resume path",
        "resume paths",
        "materialization-only",
        "dispatch packet",
        "run-graph",
    ]
    .iter()
    .any(|phrase| segment_normalized.contains(phrase));
    let scopes_completed_policy = [
        " before ",
        " after ",
        " instead of ",
        " no longer ",
        " now ",
        " coverage",
        " regression",
        " path",
        " paths",
    ]
    .iter()
    .any(|phrase| segment_normalized.contains(phrase));

    describes_resolution_action && names_runtime_artifact && scopes_completed_policy
}

fn has_unresolved_failure_artifact_context(normalized: &str) -> bool {
    [
        "lack", "lacked", "lacking", "missing", "without", "not ", "cannot", "can't", "pending",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn has_proof_or_success_context(normalized: &str) -> bool {
    if [
        "not passed",
        "not fixed",
        "not closed",
        "not implemented",
        "not succeeded",
        "without proof",
        "no proof",
        "lacked proof",
        "lacking proof",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
    {
        return false;
    }
    [
        "proof:",
        "proofs:",
        "proof target",
        "proof command",
        "proof commands",
        "evidence:",
        "validated",
        "verified",
        "tests passed",
        "test passed",
        "proof passed",
        "proofs passed",
        "passed",
        "fixed",
        "closed",
        "implemented",
        "succeeded",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn has_historical_or_success_evidence_context(normalized: &str) -> bool {
    let has_historical_context = [
        "previous",
        "previously",
        "prior",
        "historical",
        "history",
        "earlier",
        "former",
        "formerly",
        "past",
        "old ",
        "quoted",
        "quote",
        "repro",
        "reproduction",
        "reproduced",
        "attempt",
        "attempts",
        "retry",
        "returned",
        "logged",
        "log ",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));
    let has_evidence_context = ["proof", "evidence", "coverage", "validated", "verified"]
        .iter()
        .any(|phrase| normalized.contains(phrase));
    let has_success_context = [
        "proof passed",
        "proofs passed",
        "proof commands passed",
        "test passed",
        "tests passed",
        "validated",
        "verified",
        "fixed",
        "closed",
        "complete",
        "succeeded",
        "green",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));

    has_historical_context
        || (has_evidence_context
            && has_success_context
            && has_failure_state_artifact_language(normalized))
}

fn has_failure_state_artifact_language(normalized: &str) -> bool {
    [
        "canonical_gate_blocked",
        "canonical_status_blocked",
        "close_feedback_canonical_status_blocked",
        "status=blocked",
        "status: blocked",
        "blocker details",
        "blocker code",
        "blocked flag",
        "continuation_blocked",
        "failure-state",
        "failure state",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn has_current_failure_outcome_language(normalized: &str) -> bool {
    let trimmed = normalized.trim_start_matches(['"', '\'', '`', ' ']);
    let starts_with_failure_state = [
        "blocked:",
        "blocker:",
        "blocker_code=",
        "blocker_code:",
        "blocker code:",
        "blocker details:",
        "blocked flag:",
        "continuation blocker:",
        "continuation_blocked:",
        "continuation_blocked flag:",
        "awaiting_approval:",
        "approval_wait:",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix));
    starts_with_failure_state
        || has_concrete_canonical_close_phrase(trimmed)
        || trimmed.contains("current blocker")
        || trimmed.contains("current blocked")
        || trimmed.contains("currently blocked")
        || trimmed.contains("currently failing")
        || has_contrastive_blocker_clause(trimmed)
}

const CONCRETE_CANONICAL_CLOSE_PHRASES: &[&str] = &[
    "still blocked",
    "remains blocked",
    "remained blocked",
    "is blocked",
    "stays blocked",
    "blocked pending",
    "blocked by",
    "blocked on",
    "blocked due",
    "blocked because",
    "blocked until",
    "blocker:",
    "blocker remains",
    "blocker_code=",
    "blocker_code:",
    "blocker_code ",
    "blocker code",
    "blocker code:",
    "approval required",
    "pending approval",
    "pending operator approval",
    "awaiting approval",
    "approval_wait",
    "awaiting_approval",
];

fn has_concrete_canonical_close_phrase(normalized: &str) -> bool {
    CONCRETE_CANONICAL_CLOSE_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

const CONCRETE_CANONICAL_CLOSE_FIELD_LABELS: &[&str] = &[
    "blocked flag",
    "blocked field",
    "blocker detail",
    "blocker details",
    "blocker field",
    "blocker fields",
    "blocker flag",
    "blocker flags",
    "continuation blocker",
    "continuation blockers",
    "continuation_blocked",
    "continuation_blocked flag",
];

fn has_concrete_canonical_close_field_label(normalized: &str) -> bool {
    CONCRETE_CANONICAL_CLOSE_FIELD_LABELS.iter().any(|label| {
        [format!("{label}:"), format!("{label}=")]
            .iter()
            .any(|field_label| normalized.contains(field_label))
    })
}

fn has_contrastive_blocker_clause(normalized: &str) -> bool {
    normalized
        .split_once(", but ")
        .or_else(|| normalized.split_once(" but "))
        .is_some_and(|(_, blocker_clause)| has_concrete_canonical_close_phrase(blocker_clause))
}

fn ignored_canonical_close_meta_segments(reason: &str) -> Vec<String> {
    let blocker_keywords = ["blocked", "blocker", "approval_wait", "awaiting_approval"];
    let meta_keywords = [
        "fixed",
        "fix",
        "implemented",
        "implemented after",
        "closed after implementing",
        "closed after validating",
        "commit",
        "committed",
        "pushed",
        "regression",
        "proofs:",
        "proof:",
        "proof commands passed",
        "reported",
        "reports",
        "not a ",
        "not an ",
        "no current",
        "not current",
        "rather than",
        "no longer",
        "does not",
        "returns",
        "return",
        "preserves",
        "preserve",
        "mirrors",
        "mirror",
        "diagnostic context",
        "diagnostic",
        "canonical",
        "integration coverage",
        "coverage for",
        "wording",
        "artifact/status/blocker/action",
        "cargo ",
        "installed vida ",
        "vida task next",
    ];

    reason
        .split(['.', ';'])
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = trimmed.to_ascii_lowercase();
            let has_blocker_keyword = blocker_keywords
                .iter()
                .any(|keyword| normalized.contains(keyword));
            let starts_with_blocked_status = blocker_keywords
                .iter()
                .any(|keyword| normalized.starts_with(&format!("{keyword}:")));
            let has_meta_keyword = meta_keywords
                .iter()
                .any(|keyword| normalized.contains(keyword));
            if has_blocker_keyword
                && has_meta_keyword
                && !starts_with_blocked_status
                && !has_contrastive_blocker_clause(&normalized)
                && !has_concrete_canonical_close_phrase(&normalized)
            {
                Some(normalized)
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn canonical_close_status_from_reason(
    reason: &str,
) -> Option<(&'static str, &'static str)> {
    let mut normalized = reason.to_ascii_lowercase();
    for phrase in ignored_canonical_close_historical_context(reason) {
        normalized = normalized.replace(&phrase, " canonical_close_context_language ");
    }
    if has_concrete_canonical_close_field_label(&normalized) {
        return Some(("blocked", "blocked"));
    }
    for phrase in ignored_canonical_close_meta_language(reason) {
        normalized = normalized.replace(&phrase, " canonical_close_context_language ");
    }
    let approval_keywords = [
        "approval_wait".to_string(),
        "awaiting_approval".to_string(),
        "approval required".to_string(),
        "pending approval".to_string(),
    ];
    if !super::contains_keywords(&normalized, &approval_keywords).is_empty() {
        return Some((
            "awaiting_approval",
            crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str(),
        ));
    }

    let blocker_keywords = [
        "blocked".to_string(),
        "blocker".to_string(),
        "lane_blocked".to_string(),
        "blocked_pending".to_string(),
    ];
    if !super::contains_keywords(&normalized, &blocker_keywords).is_empty() {
        return Some(("blocked", "blocked"));
    }

    None
}

fn host_agent_ids(carrier_catalog: &[serde_json::Value]) -> Vec<String> {
    carrier_catalog
        .iter()
        .filter_map(|row| row["role_id"].as_str())
        .map(ToString::to_string)
        .collect()
}

fn host_agent_id_exists(carrier_catalog: &[serde_json::Value], agent_id: &str) -> bool {
    carrier_catalog
        .iter()
        .any(|row| row["role_id"].as_str() == Some(agent_id))
}

fn host_agent_id_for_tier(carrier_catalog: &[serde_json::Value], tier: &str) -> Option<String> {
    carrier_catalog
        .iter()
        .find(|row| row["tier"].as_str() == Some(tier))
        .and_then(|row| row["role_id"].as_str())
        .map(ToString::to_string)
}

fn resolve_feedback_host_agent_id(
    assignment: &serde_json::Value,
    carrier_catalog: &[serde_json::Value],
) -> Result<(String, String), serde_json::Value> {
    let mut attempted = Vec::new();
    for (source, candidate) in [
        (
            "selected_agent_id",
            assignment["selected_agent_id"].as_str(),
        ),
        (
            "selected_carrier_agent_id",
            assignment["selected_carrier_agent_id"].as_str(),
        ),
        (
            "selected_carrier_id",
            assignment["selected_carrier_id"].as_str(),
        ),
        (
            "selected_backend_id",
            assignment["selected_backend_id"].as_str(),
        ),
        (
            "activation_agent_type",
            assignment["activation_agent_type"].as_str(),
        ),
        ("selected_tier", assignment["selected_tier"].as_str()),
    ] {
        let Some(candidate) = candidate.map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        attempted.push(serde_json::json!({
            "source": source,
            "candidate": candidate,
        }));
        if host_agent_id_exists(carrier_catalog, candidate) {
            return Ok((candidate.to_string(), source.to_string()));
        }
        if let Some(role_id) = host_agent_id_for_tier(carrier_catalog, candidate) {
            return Ok((role_id, format!("{source}.tier_match")));
        }
    }

    Err(serde_json::json!({
        "status": "blocked",
        "reason": "selected_feedback_carrier_unavailable",
        "attempted_candidates": attempted,
        "available_host_agent_ids": host_agent_ids(carrier_catalog),
    }))
}

pub(crate) fn maybe_record_task_close_host_agent_feedback(
    project_root: &std::path::Path,
    task: &serde_json::Value,
    close_reason: &str,
    source: &str,
) -> serde_json::Value {
    let task_class = super::infer_task_class_from_task_payload(task);
    let runtime_role = super::runtime_role_for_task_class(&task_class);
    if let Some((canonical_status, canonical_gate)) =
        canonical_close_status_from_reason(close_reason)
    {
        return serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "task_class": task_class,
            "runtime_role": runtime_role,
            "canonical_status": canonical_status,
            "canonical_gate": canonical_gate,
        });
    }
    if std::env::var_os("VIDA_TASK_CLOSE_FULL_FEEDBACK").is_none() {
        let outcome = infer_feedback_outcome_from_close_reason(close_reason);
        let score = default_feedback_score(outcome, &task_class);
        let outcome_inference = close_feedback_outcome_inference(close_reason, outcome, score);
        let safety_gate = if outcome == "failure" {
            "hold"
        } else {
            "observe"
        };
        return serde_json::json!({
            "status": "recorded",
            "task_class": task_class,
            "runtime_role": runtime_role,
            "feedback_agent_id": "lightweight_task_close_feedback",
            "feedback_selection_source": "operator_latency_budget_fast_path",
            "feedback_outcome_inference": outcome_inference,
            "feedback": {
                "mode": "lightweight_task_close_feedback",
                "recorded_outcome": outcome,
                "recorded_score": score,
                "recorded_task_class": task_class,
                "recorded_notes": "automatic task-close feedback",
                "safety_baseline": {
                    "safety_gate": safety_gate,
                    "status": "baseline_recorded",
                },
                "source": source,
                "task_id": task["id"].as_str().unwrap_or(""),
                "task_title": task["title"].as_str().unwrap_or(""),
            },
        });
    }
    let overlay = match super::project_activator_surface::read_yaml_file_checked(
        &project_root.join("vida.config.yaml"),
    ) {
        Ok(overlay) => overlay,
        Err(error) => {
            return serde_json::json!({
                "status": "skipped",
                "reason": format!("overlay_unavailable: {error}")
            });
        }
    };
    let (_selected_cli_system, carrier_catalog) =
        match super::project_activator_surface::resolved_host_cli_agent_catalog_for_root(
            project_root,
            &overlay,
        ) {
            Ok(resolved) => resolved,
            Err(error) => {
                return serde_json::json!({
                    "status": "skipped",
                    "reason": format!("host_cli_catalog_unavailable: {error}")
                });
            }
        };
    if carrier_catalog.is_empty() {
        return serde_json::json!({
            "status": "skipped",
            "reason": "host_cli_catalog_empty"
        });
    }

    let compiled_bundle =
        match super::build_compiled_agent_extension_bundle_for_root(&overlay, project_root) {
            Ok(bundle) => bundle,
            Err(error) => {
                return serde_json::json!({
                    "status": "error",
                    "reason": format!("compiled_bundle_failed: {error}")
                });
            }
        };
    let assignment = super::build_runtime_assignment_from_resolved_constraints(
        &compiled_bundle,
        "orchestrator",
        &task_class,
        runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        return serde_json::json!({
            "status": "skipped",
            "reason": assignment["reason"].as_str().unwrap_or("runtime_assignment_disabled"),
            "task_class": task_class,
            "runtime_role": runtime_role,
        });
    }

    let (agent_id, feedback_selection_source) =
        match resolve_feedback_host_agent_id(&assignment, &carrier_catalog) {
            Ok(selection) => selection,
            Err(blocked) => {
                return serde_json::json!({
                    "status": "blocked",
                    "reason": "selected_feedback_carrier_unavailable",
                    "task_class": task_class,
                    "runtime_role": runtime_role,
                    "assignment": assignment,
                    "feedback_selection": blocked,
                });
            }
        };
    if let Some((canonical_status, canonical_gate)) =
        canonical_close_status_from_reason(close_reason)
    {
        return serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "task_class": task_class,
            "runtime_role": runtime_role,
            "assignment": assignment,
            "canonical_status": canonical_status,
            "canonical_gate": canonical_gate,
        });
    }
    let outcome = infer_feedback_outcome_from_close_reason(close_reason);
    let score = default_feedback_score(outcome, &task_class);
    let outcome_inference = close_feedback_outcome_inference(close_reason, outcome, score);
    let input = super::HostAgentFeedbackInput {
        agent_id: &agent_id,
        score,
        outcome,
        task_class: &task_class,
        notes: Some("automatic task-close feedback"),
        source,
        task_id: task["id"].as_str(),
        task_display_id: task["display_id"].as_str(),
        task_title: task["title"].as_str(),
        runtime_role: assignment["runtime_role"].as_str(),
        selected_tier: assignment["selected_tier"].as_str(),
        estimated_task_price_units: assignment["estimated_task_price_units"].as_u64(),
        lifecycle_state: assignment["lifecycle_state"].as_str(),
        effective_score: assignment["effective_score"].as_u64(),
        reason: Some(close_reason),
    };
    match append_host_agent_feedback(project_root, &input) {
        Ok(view) => serde_json::json!({
            "status": "recorded",
            "task_class": task_class,
            "runtime_role": runtime_role,
            "feedback_agent_id": agent_id,
            "feedback_selection_source": feedback_selection_source,
            "assignment": assignment,
            "feedback_outcome_inference": outcome_inference,
            "feedback": view,
        }),
        Err(error) => serde_json::json!({
            "status": "error",
            "reason": error,
            "task_class": task_class,
            "runtime_role": runtime_role,
            "assignment": assignment,
        }),
    }
}

fn append_host_agent_feedback(
    project_root: &std::path::Path,
    input: &super::HostAgentFeedbackInput<'_>,
) -> Result<serde_json::Value, String> {
    if input.score > 100 {
        return Err("Feedback score must be between 0 and 100.".to_string());
    }
    if let Some(notes) = input.notes {
        if notes.len() > MAX_FEEDBACK_NOTES_BYTES {
            return Err(format!(
                "Feedback notes exceed bounded ingestion contract: {} bytes > {} bytes.",
                notes.len(),
                MAX_FEEDBACK_NOTES_BYTES
            ));
        }
    }
    let overlay = super::project_activator_surface::read_yaml_file_checked(
        &project_root.join("vida.config.yaml"),
    )
    .map_err(|error| format!("Failed to read project overlay: {error}"))?;
    let (selected_cli_system, carrier_catalog) =
        super::project_activator_surface::resolved_host_cli_agent_catalog_for_root(
            project_root,
            &overlay,
        )?;
    if !carrier_catalog
        .iter()
        .any(|row| row["role_id"].as_str() == Some(input.agent_id))
    {
        return Err(format!(
            "Unknown host agent `{}` for selected CLI system `{}`.",
            input.agent_id, selected_cli_system
        ));
    }
    let scorecards_path = super::worker_scorecards_state_path(project_root);
    let mut scorecards =
        super::load_or_initialize_worker_scorecards(project_root, &carrier_catalog);
    if !scorecards["agents"][input.agent_id]["feedback"].is_array() {
        scorecards["agents"][input.agent_id]["feedback"] = serde_json::json!([]);
    }
    let feedback_rows = scorecards["agents"][input.agent_id]["feedback"]
        .as_array_mut()
        .expect("feedback array should initialize");
    feedback_rows.push(serde_json::json!({
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "score": input.score,
        "outcome": input.outcome,
        "task_class": input.task_class,
        "notes": input.notes.unwrap_or(""),
        "source": input.source,
        "task_id": input.task_id.unwrap_or(""),
        "task_display_id": input.task_display_id.unwrap_or(""),
        "task_title": input.task_title.unwrap_or(""),
    }));
    scorecards["updated_at"] = serde_json::Value::String(
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    );
    if let Some(parent) = scorecards_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(
        &scorecards_path,
        serde_json::to_string_pretty(&scorecards).expect("scorecards json should render"),
    )
    .map_err(|error| format!("Failed to write {}: {error}", scorecards_path.display()))?;
    let scoring_policy = serde_json::to_value(
        super::yaml_lookup(&overlay, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let strategy = super::refresh_worker_strategy(project_root, &carrier_catalog, &scoring_policy);
    let observability_event = super::append_host_agent_observability_event(project_root, input)?;
    Ok(serde_json::json!({
        "surface": "vida agent-feedback",
        "host_cli_system": selected_cli_system,
        "agent_id": input.agent_id,
        "recorded_score": input.score,
        "recorded_outcome": input.outcome,
        "recorded_task_class": input.task_class,
        "recorded_notes": input.notes.unwrap_or(""),
        "scorecards_store": super::WORKER_SCORECARDS_STATE,
        "strategy_store": super::WORKER_STRATEGY_STATE,
        "observability_store": super::HOST_AGENT_OBSERVABILITY_STATE,
        "prompt_lifecycle_store": super::PROMPT_LIFECYCLE_STATE,
        "strategy_row": strategy["agents"][input.agent_id],
        "observability_event": observability_event,
        "feedback_event": observability_event["feedback_event"].clone(),
        "evaluation_baseline": observability_event["evaluation_baseline"].clone(),
        "prompt_lifecycle_baseline": observability_event["prompt_lifecycle_baseline"].clone(),
        "safety_baseline": observability_event["safety_baseline"].clone()
    }))
}

#[cfg(test)]
mod tests {
    use crate::read_json_file_if_present;
    use crate::run;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir};
    use crate::HOST_AGENT_OBSERVABILITY_STATE;
    use crate::WORKER_SCORECARDS_STATE;
    use crate::WORKER_STRATEGY_STATE;
    use std::process::ExitCode;

    #[test]
    fn agent_feedback_records_scorecard_and_refreshes_strategy() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-feedback",
                "--agent-id",
                "junior",
                "--score",
                "92",
                "--outcome",
                "success",
                "--task-class",
                "implementation",
                "--notes",
                "clean bounded closure",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let scorecards = read_json_file_if_present(&harness.path().join(WORKER_SCORECARDS_STATE))
            .expect("scorecards should exist");
        let rows = scorecards["agents"]["junior"]["feedback"]
            .as_array()
            .expect("feedback rows should render");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["score"], 92);
        assert_eq!(rows[0]["outcome"], "success");
        assert_eq!(rows[0]["task_class"], "implementation");

        let strategy = read_json_file_if_present(&harness.path().join(WORKER_STRATEGY_STATE))
            .expect("strategy should exist");
        assert!(
            strategy["agents"]["junior"]["effective_score"]
                .as_u64()
                .unwrap_or_default()
                >= 80
        );
        let observability =
            read_json_file_if_present(&harness.path().join(HOST_AGENT_OBSERVABILITY_STATE))
                .expect("observability ledger should exist");
        assert_eq!(
            observability["events"]
                .as_array()
                .expect("events should be an array")
                .len(),
            1
        );
        assert_eq!(observability["events"][0]["agent_id"], "junior");
    }

    #[test]
    fn agent_feedback_records_scorecard_for_non_default_selected_system() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "vida-test",
                "--project-name",
                "VIDA Test",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-feedback",
                "--agent-id",
                "qwen-primary",
                "--score",
                "81",
                "--outcome",
                "success",
                "--task-class",
                "implementation",
                "--notes",
                "external carrier feedback",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let scorecards = read_json_file_if_present(&harness.path().join(WORKER_SCORECARDS_STATE))
            .expect("scorecards should exist");
        let rows = scorecards["agents"]["qwen-primary"]["feedback"]
            .as_array()
            .expect("feedback rows should render");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["score"], 81);
        assert_eq!(rows[0]["outcome"], "success");
    }

    #[test]
    fn close_feedback_selection_maps_internal_backend_to_configured_codex_carrier() {
        let assignment = serde_json::json!({
            "selected_agent_id": "internal_subagents",
            "selected_carrier_agent_id": "internal_subagents",
            "selected_carrier_id": "internal_subagents",
            "selected_backend_id": "internal_subagents",
            "activation_agent_type": "internal_subagents",
            "selected_tier": "middle",
        });
        let carrier_catalog = vec![
            serde_json::json!({
                "role_id": "junior",
                "tier": "junior",
            }),
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
            }),
        ];

        let (agent_id, source) =
            super::resolve_feedback_host_agent_id(&assignment, &carrier_catalog)
                .expect("internal backend should resolve through selected_tier");

        assert_eq!(agent_id, "middle");
        assert_eq!(source, "selected_tier");
    }

    #[test]
    fn close_feedback_selection_blocks_before_unknown_host_agent_execution() {
        let assignment = serde_json::json!({
            "selected_agent_id": "internal_subagents",
            "selected_carrier_id": "internal_subagents",
            "selected_backend_id": "internal_subagents",
            "selected_tier": "senior",
        });
        let carrier_catalog = vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
        })];

        let blocked = super::resolve_feedback_host_agent_id(&assignment, &carrier_catalog)
            .expect_err("unavailable backend should fail before append_host_agent_feedback");

        assert_eq!(blocked["status"], "blocked");
        assert_eq!(blocked["reason"], "selected_feedback_carrier_unavailable");
        assert!(blocked["available_host_agent_ids"]
            .as_array()
            .expect("available ids should render")
            .iter()
            .any(|value| value == "junior"));
        assert!(blocked["attempted_candidates"]
            .as_array()
            .expect("attempted candidates should render")
            .iter()
            .any(|row| row["candidate"] == "internal_subagents"));
    }

    #[test]
    fn close_feedback_inference_does_not_treat_fail_closed_contract_language_as_failure() {
        let reason = "Added execution-preparation artifact registry contract foundation with owner/id/path/status validation and fail-closed checks; taskflow_artifacts tests passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 90);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["tests passed"])
        );
        assert_eq!(
            inference["ignored_contract_language"],
            serde_json::json!(["fail-closed"])
        );
    }

    #[test]
    fn close_feedback_inference_still_scores_explicit_failed_reason_as_failure() {
        let reason = "Validation failed after proof commands.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "failure");
        assert_eq!(score, 35);
        assert_eq!(inference["outcome"], "failure");
        assert_eq!(inference["failure_markers"], serde_json::json!(["failed"]));
    }

    #[test]
    fn close_feedback_inference_ignores_post_close_blocker_field_context() {
        let reason = "Fixed task close contract so successful mutation returns command success while continuation blockers remain separate. JSON now reports continuation_blocked flag, blocker details, and next actions. Proof commands passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 90);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["proof commands passed"])
        );
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "continuation blockers remain separate"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "continuation_blocked flag"));
        assert!(ignored.iter().any(|phrase| phrase == "blocker details"));
    }

    #[test]
    fn close_feedback_inference_still_scores_direct_blocked_reason_as_failure() {
        let reason = "Task is still blocked by missing verification proof.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "failure");
        assert_eq!(score, 35);
        assert_eq!(inference["failure_markers"], serde_json::json!(["blocked"]));
    }

    #[test]
    fn close_feedback_inference_ignores_failure_marker_meta_language() {
        let reason = "Added scoring guard; tests passed; explicit failed markers still fail.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 90);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["tests passed"])
        );
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "explicit failed markers still fail"));
    }

    #[test]
    fn close_feedback_inference_ignores_negated_failure_language() {
        let reason = "Validation did not fail and proof commands passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored.iter().any(|phrase| phrase == "did not fail"));
    }

    #[test]
    fn close_feedback_inference_ignores_rejected_alternatives_audit_language() {
        let reason = "Added model-profile readiness audit payload with selected overrides, rejected alternatives, and readiness blockers; model_profile_readiness_audit tests passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 90);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["tests passed"])
        );
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "rejected alternatives"));
    }

    #[test]
    fn close_feedback_inference_ignores_failure_case_coverage_language() {
        let reason = "Added close-feedback smoke coverage for rejected alternatives and concrete rejected patch wording records failure; task_smoke test passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["test passed"])
        );
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored.iter().any(|phrase| phrase == "records failure"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "concrete rejected patch wording"));
    }

    #[test]
    fn close_feedback_inference_ignores_failure_case_coverage_terms() {
        let reason =
            "Added failure-case coverage and rejected wording coverage; proof commands passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["success_markers"],
            serde_json::json!(["proof commands passed"])
        );
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "failure-case coverage"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "rejected wording coverage"));
    }

    #[test]
    fn close_feedback_inference_ignores_failed_subprocess_diagnostic_wording() {
        let reason = "Migrated task_smoke VIDA command construction to vida-test-support bounded_binary_command; improved helper diagnostics for failed subprocess status/stdout/stderr; protocol_binding_check_statuses_are_canonical and protocol_binding_check_lock_retry_preserves_blocker_codes tests passed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(inference["success_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "failed subprocess status"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "failed subprocess status/stdout/stderr"));
    }

    #[test]
    fn canonical_close_status_ignores_passed_invariant_rejection_wording() {
        for (reason, expected_ignored_phrase) in [
            (
                "Rejected parent closure while child remains open invariant was covered; proof commands passed.",
                "rejected parent closure while child remains open",
            ),
            (
                "Rejected close invariant proof passed.",
                "rejected close invariant",
            ),
            (
                "Rejected parent closure path proved; tests passed.",
                "rejected parent closure path proved",
            ),
        ] {
            let outcome = super::infer_feedback_outcome_from_close_reason(reason);
            let score = super::default_feedback_score(outcome, "verification");
            let inference = super::close_feedback_outcome_inference(reason, outcome, score);

            assert_eq!(super::canonical_close_status_from_reason(reason), None);
            assert_eq!(outcome, "success");
            assert_eq!(score, 88);
            assert_eq!(inference["failure_markers"], serde_json::json!([]));
            assert!(inference["ignored_meta_language"]
                .as_array()
                .expect("ignored meta language should render")
                .iter()
                .any(|phrase| phrase == expected_ignored_phrase));
            assert!(inference["success_markers"]
                .as_array()
                .expect("success markers should render")
                .iter()
                .any(|phrase| phrase == "proof commands passed"
                    || phrase == "proof passed"
                    || phrase == "tests passed"));
        }
    }

    #[test]
    fn close_feedback_inference_preserves_concrete_rejected_outcomes() {
        for reason in [
            "Task was rejected by verifier after review.",
            "Rejected patch because it changed unrelated files.",
            "Concrete rejected patch because it removed operator evidence.",
        ] {
            let outcome = super::infer_feedback_outcome_from_close_reason(reason);
            let score = super::default_feedback_score(outcome, "verification");
            let inference = super::close_feedback_outcome_inference(reason, outcome, score);

            assert_eq!(outcome, "failure");
            assert_eq!(score, 35);
            assert_eq!(
                inference["failure_markers"],
                serde_json::json!(["rejected"])
            );
        }
    }

    #[test]
    fn canonical_close_status_ignores_readiness_blockers_audit_language() {
        let reason = "Added model-profile readiness audit payload with selected overrides, rejected alternatives, and readiness blockers; model_profile_readiness_audit tests passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored.iter().any(|phrase| phrase == "readiness blockers"));
    }

    #[test]
    fn canonical_close_status_ignores_proved_blocked_receipt_rejection_policy() {
        let reason = "Rejected materialization-only blocked task-ensure receipts before terminal closure and persisted final-snapshot resume paths. Proof: cargo test -p vida taskflow_consume_continue_rejects_materialization_only_receipt_before_final_snapshot_replay passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase.as_str().is_some_and(|value| value
                .contains("rejected materialization-only blocked task-ensure receipts"))));
    }

    #[test]
    fn canonical_close_status_preserves_concrete_blocked_reasons() {
        let reason = "Task remains blocked pending operator evidence.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }

    #[test]
    fn canonical_close_status_preserves_historical_current_blocker_reasons() {
        for reason in [
            "Blocked by previous dependency not complete",
            "Blocked on prior approval",
        ] {
            assert_eq!(
                super::canonical_close_status_from_reason(reason),
                Some(("blocked", "blocked")),
                "historical wording must not hide a current blocker reason: {reason}"
            );
        }
    }

    #[test]
    fn canonical_close_status_preserves_blocker_field_label_reasons() {
        for reason in [
            "Blocker details: missing verifier receipt",
            "blocker field=missing verifier receipt",
            "blocked flag: true",
            "continuation blocker: missing follow-up dispatch",
            "continuation_blocked flag=true",
        ] {
            assert_eq!(
                super::canonical_close_status_from_reason(reason),
                Some(("blocked", "blocked")),
                "concrete blocker field-label reason should remain fail-closed: {reason}"
            );
        }
    }

    #[test]
    fn canonical_close_status_preserves_current_blocked_receipt_reason() {
        let reason = "Rejected receipt because task is blocked by missing execution evidence.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }

    #[test]
    fn canonical_close_status_preserves_negated_success_blocked_receipt_reason() {
        let reason =
            "Rejected receipt after blocked task lacked execution evidence; tests not passed.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
        assert!(
            !super::ignored_canonical_close_historical_context(reason)
                .iter()
                .any(|segment| segment.contains("rejected receipt after blocked task")),
            "current blocked receipt wording must not be stripped by unrelated negated success text"
        );
    }

    #[test]
    fn canonical_close_status_ignores_fix_description_meta_blocked_phrases() {
        let reason = "Fixed false canonical close feedback derivation: classifier strips audit and fix-description phrases before keyword matching while preserving concrete blocked reason detection. Task close JSON now exposes deferred canonical-close telemetry as actionable blocked output only when the close reason is genuinely blocked. Proofs: ...";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "close feedback derivation"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocked reason detection"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "actionable blocked output"));
        assert!(ignored.iter().any(|phrase| phrase == "genuinely blocked"));
    }

    #[test]
    fn canonical_close_status_ignores_blocked_task_outcomes_meta_language() {
        let reason = "Fixed task-close feedback outcome meta-language classification: feedback outcome normalization now shares canonical close meta-language stripping, so audit/fix-description phrases such as blocked reason detection, actionable blocked output, and genuinely blocked are treated as context while concrete blocked task outcomes remain failure evidence. Proofs: cargo test -p vida close_feedback_inference -- --quiet --test-threads=1; cargo fmt --check; cargo build -p vida --release.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocked reason detection"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "actionable blocked output"));
        assert!(ignored.iter().any(|phrase| phrase == "genuinely blocked"));
        assert!(ignored.iter().any(|phrase| phrase == "failure evidence"));
    }

    #[test]
    fn canonical_close_status_ignores_blocker_code_and_proof_meta_language() {
        let reason = "Fixed task next run-graph gate: next now blocks ready-head dispatch when latest run-graph state is held, returns canonical latest_run_graph_status_blocked, preserves ready-head only as diagnostic context, and mirrors artifact/status/blocker/action fields across shared and operator contract output. Proofs: cargo test -p vida taskflow_next_decision_blocks_ready_head_when_latest_run_graph_is_blocked -- --quiet --test-threads=1; cargo test -p vida taskflow_next_decision -- --quiet --test-threads=1; cargo fmt --check; cargo build -p vida --release; installed vida task next --json returns blocked with recovery action.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase.contains("latest_run_graph_status_blocked")));
        assert!(ignored
            .iter()
            .any(|phrase| phrase.contains("diagnostic context")));
        assert!(ignored.iter().any(|phrase| phrase
            .contains("installed vida task next --json returns blocked with recovery action")));
    }

    #[test]
    fn canonical_close_status_ignores_task_acceptance_blocker_wording() {
        let reason = "Implemented TaskFlow work item kind schema. Acceptance covered: invalid parent/child kind combinations fail closed with actionable blockers; ready/blocked/progress/list/tree surfaces include canonical kind without breaking existing JSON consumers. Proofs passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "fail closed with actionable blockers"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "ready/blocked/progress/list/tree"));
    }

    #[test]
    fn canonical_close_status_ignores_verifier_rework_regression_proof_context() {
        let reason = "Separated receipt-backed verification execution evidence from closure-ready proof. Verification evidence that is receipt-backed but explicitly reports closure_ready=false, blocker_code/blockers, or blocker/rework/not-approved verdict text no longer bridges a blocked verification lane into executed closure handoff. Added regression maybe_bridge_receipt_backed_verification_rework_does_not_open_closure. Proof: cargo test -p vida maybe_bridge_receipt_backed_verification_rework_does_not_open_closure -- --nocapture --test-threads=1.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocker_code/blockers"));
        assert!(ignored.iter().any(|phrase| phrase == "blocker/rework"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocked verification lane"));
    }

    #[test]
    fn canonical_close_status_ignores_commit_push_proof_with_historical_blocker_context() {
        let reason = "Committed and pushed verifier blocker closure regression fix. Commit f770f70aa77bf95883d7ac9af6da2c680409d739 is on main and origin/main; git status clean. Proof: git log -1 --oneline --decorate; git show --stat --oneline --name-only --no-renames HEAD; git rev-parse HEAD equals origin/main.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "verifier blocker closure regression fix"));
    }

    #[test]
    fn canonical_close_status_ignores_historical_failure_state_evidence_segments() {
        let reason = "Closed after verification: current implementation is complete. Evidence: previous task close output quoted blocker details: close_feedback_canonical_status_blocked/canonical_gate_blocked and failure-state wording; proof commands passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase.as_str().is_some_and(
                |value| value.contains("previous task close output quoted blocker details")
            )));
    }

    #[test]
    fn canonical_close_status_ignores_structured_blocked_json_proof_context() {
        let reason = "Added invalid graph JSON envelope coverage and repairs. Current CLI boundary now proves invalid JSONL import returns structured blocked JSON with dependency graph issue code; graph-summary invalid graph computation paths now also emit structured blocked JSON instead of stderr-only exit. Proof commands passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "structured blocked json"));
        assert!(ignored.iter().any(|phrase| phrase == "stderr-only exit"));
    }

    #[test]
    fn canonical_close_status_ignores_passed_blocked_projection_wording() {
        let reason = "CASE-03 applied spawn-blocker ordering through next-lawful and blocked task projections; proof commands passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "spawn-blocker ordering"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocked task projections"));
    }

    #[test]
    fn canonical_close_status_ignores_successful_blocked_pass_coverage_wording() {
        let reason = "Closed after implementing docflow check JSON mode and validating direct CLI plus VIDA proxy integration coverage for help, blocked, pass, and installed runtime smoke.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored.iter().any(|phrase| {
            phrase
                == "closed after implementing docflow check json mode and validating direct cli plus vida proxy integration coverage for help, blocked, pass, and installed runtime smoke"
        }));
    }

    #[test]
    fn canonical_close_status_preserves_contrastive_blocked_clause_after_meta_language() {
        let reason = "Closed after implementing, but still blocked pending operator approval";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
        assert!(!super::ignored_canonical_close_meta_language(reason)
            .iter()
            .any(|phrase| phrase.contains("still blocked pending operator approval")));
    }

    #[test]
    fn canonical_close_status_ignores_contrastive_coverage_wording() {
        let reason =
            "Closed after implementing JSON mode, but kept coverage for help, blocked, pass.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
    }

    #[test]
    fn canonical_close_status_ignores_implemented_blocker_gate_proof_wording() {
        let reason = "Implemented verifier blocker summary gate with tests; focused proofs passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let ignored = super::ignored_canonical_close_meta_language(reason);
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "implemented verifier blocker summary gate with tests"));
    }

    #[test]
    fn canonical_close_status_preserves_implemented_change_with_remaining_blocker() {
        let reason = "Implemented the change but blocker remains pending verifier evidence";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
        assert!(!super::ignored_canonical_close_meta_language(reason)
            .iter()
            .any(|phrase| phrase.contains("blocker remains pending verifier evidence")));
    }

    #[test]
    fn canonical_close_status_preserves_positive_blocker_code_diagnostics() {
        for reason in [
            "Diagnostics: blocked with blocker_code=missing_verifier_receipt",
            "Diagnostic: blocker code missing approval evidence",
        ] {
            assert_eq!(
                super::canonical_close_status_from_reason(reason),
                Some(("blocked", "blocked"))
            );
            assert!(!super::ignored_canonical_close_meta_language(reason)
                .iter()
                .any(|phrase| phrase.contains("blocker_code")
                    || phrase.contains("blocker code missing")));
        }
    }

    #[test]
    fn close_feedback_inference_ignores_fail_closes_contract_wording() {
        let reason = "Successful task-close status/proof context must dominate lexical failure words such as fail-closes if missing; proof commands passed.";

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(
            inference["ignored_contract_language"],
            serde_json::json!(["fail-closes"])
        );
    }

    #[test]
    fn close_feedback_inference_ignores_blocked_fix_description_meta_language() {
        let reason = "Fixed false canonical close feedback derivation: classifier strips audit and fix-description phrases before keyword matching while preserving concrete blocked reason detection. Task close JSON now exposes deferred canonical-close telemetry as actionable blocked output only when the close reason is genuinely blocked. Proofs: canonical_close_status_ignores_readiness_blockers_audit_language, canonical_close_status_ignores_fix_description_meta_blocked_phrases, canonical_close_status_preserves_concrete_blocked_reasons, task_close_feedback_blocker_summary_surfaces_deferred_canonical_close, cargo fmt --check.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "architecture");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 90);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "close feedback derivation"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "blocked reason detection"));
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "actionable blocked output"));
        assert!(ignored.iter().any(|phrase| phrase == "genuinely blocked"));
    }

    #[test]
    fn close_feedback_inference_ignores_blocker_code_and_proof_meta_language() {
        let reason = "Fixed task next run-graph gate: next now blocks ready-head dispatch when latest run-graph state is held, returns canonical latest_run_graph_status_blocked, preserves ready-head only as diagnostic context, and mirrors artifact/status/blocker/action fields across shared and operator contract output. Proofs: cargo test -p vida taskflow_next_decision_blocks_ready_head_when_latest_run_graph_is_blocked -- --quiet --test-threads=1; cargo test -p vida taskflow_next_decision -- --quiet --test-threads=1; cargo fmt --check; cargo build -p vida --release; installed vida task next --json returns blocked with recovery action.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        let ignored = inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render");
        assert!(ignored.iter().any(|phrase| phrase
            .as_str()
            .is_some_and(|value| value.contains("latest_run_graph_status_blocked"))));
        assert!(ignored.iter().any(|phrase| phrase
            .as_str()
            .is_some_and(|value| value.contains("diagnostic context"))));
        assert!(ignored
            .iter()
            .any(|phrase| phrase.as_str().is_some_and(|value| value.contains(
                "installed vida task next --json returns blocked with recovery action"
            ))));
    }

    #[test]
    fn close_feedback_inference_ignores_no_blocker_codes_diagnostic_proof_wording() {
        let reason = "Post-merge diagnostics passed with no blocker codes and git status clean.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert_eq!(inference["success_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase
                .as_str()
                .is_some_and(|value| value.contains("no blocker codes"))));
    }

    #[test]
    fn close_feedback_inference_ignores_negated_blocker_baseline_context() {
        let reason = "PR #470 processed and merged. Evidence: gh pr view reports state MERGED, mergedAt 2026-06-23T22:25:19Z, mergeCommit 9b92c28b0ca7df81c4dfcd15dd77bac520dbf91a, head 2384afec2fcfb3ba089f121f93a5da005cfd10c1. Local merged-base proof before merge: git diff --check passed; cargo test -p vida --bin vida runtime_defect_design_backed_seed_uses_configured_first_step --locked -- --test-threads=1 passed. rustfmt --check on taskflow_run_graph.rs failed equally on origin/main baseline, so not a PR-specific blocker. vida task validate-graph passed after merge.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase
                .as_str()
                .is_some_and(|value| value.contains("not a pr-specific blocker"))));
    }

    #[test]
    fn close_feedback_inference_ignores_empty_blocker_list_proof_wording() {
        let reason = "Lane reconciliation finished: lane_completed, dispatch_status executed, receipt paths recorded, and blocker list empty; the reconciliation todo can now end before selecting the next lawful task.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(super::canonical_close_status_from_reason(reason), None);
        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase
                .as_str()
                .is_some_and(|value| value.contains("blocker list empty"))));
    }

    #[test]
    fn close_feedback_inference_ignores_failed_result_defect_description() {
        let reason = "Fixed host bridge result ingestion so failed/tampered parent adapter results fail closed instead of being promoted to pass. Added regression test host_bridge_rejects_failed_parent_result. Proofs passed: cargo test -p vida host_bridge_rejects_failed_parent_result -- --nocapture --test-threads=1; cargo test -p vida host_bridge -- --nocapture --test-threads=1; cargo fmt -p vida -- --check; git diff --check.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 88);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["success_markers"]
            .as_array()
            .expect("success markers should render")
            .iter()
            .any(|marker| marker == "proofs passed"));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase == "failed/tampered parent adapter results"));
    }

    #[test]
    fn close_feedback_inference_ignores_failed_result_hyphenated_context() {
        let reason = "Fixed task-close feedback inference for contextual failed-result defect descriptions. Proofs passed: cargo test -p vida close_feedback_inference_ignores_failed_result_defect_description -- --nocapture --test-threads=1; cargo test -p vida close_feedback_inference -- --nocapture --test-threads=1; cargo fmt -p vida -- --check; git diff --check. Commit 6c0cc646e pushed.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "implementation");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "success");
        assert_eq!(score, 82);
        assert_eq!(inference["outcome"], "success");
        assert_eq!(inference["failure_markers"], serde_json::json!([]));
        assert!(inference["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase == "contextual failed-result defect descriptions"));
    }

    #[test]
    fn close_feedback_inference_preserves_failed_subprocess_status_reasons() {
        let reason = "Task failed subprocess status 101 while running proofs.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "failure");
        assert_eq!(score, 35);
        assert_eq!(inference["outcome"], "failure");
        assert_eq!(inference["failure_markers"], serde_json::json!(["failed"]));
    }

    #[test]
    fn close_feedback_inference_preserves_concrete_failed_result_reasons() {
        let reason = "Task failed result after verification.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "failure");
        assert_eq!(score, 35);
        assert_eq!(inference["outcome"], "failure");
        assert_eq!(inference["failure_markers"], serde_json::json!(["failed"]));
        assert_eq!(inference["ignored_meta_language"], serde_json::json!([]));
    }

    #[test]
    fn close_feedback_inference_preserves_concrete_blocked_reasons() {
        let reason = "Task remains blocked pending operator evidence.";
        let outcome = super::infer_feedback_outcome_from_close_reason(reason);
        let score = super::default_feedback_score(outcome, "verification");
        let inference = super::close_feedback_outcome_inference(reason, outcome, score);

        assert_eq!(outcome, "failure");
        assert_eq!(score, 35);
        assert_eq!(inference["outcome"], "failure");
        assert_eq!(inference["failure_markers"], serde_json::json!(["blocked"]));
    }

    #[test]
    fn close_feedback_inference_preserves_concrete_blocked_outcomes() {
        for reason in [
            "Task remains blocked pending operator evidence.",
            "Blocked: cargo test failed",
            "The lane is blocked pending a verifier receipt.",
            "Task remains blocked pending coverage.",
        ] {
            let outcome = super::infer_feedback_outcome_from_close_reason(reason);
            let score = super::default_feedback_score(outcome, "verification");
            let inference = super::close_feedback_outcome_inference(reason, outcome, score);

            assert_eq!(
                super::canonical_close_status_from_reason(reason),
                Some(("blocked", "blocked"))
            );
            assert_eq!(outcome, "failure");
            assert_eq!(score, 35);
            assert_eq!(inference["outcome"], "failure");
            assert!(inference["failure_markers"]
                .as_array()
                .expect("failure markers should render")
                .iter()
                .any(|marker| marker == "blocked"));
        }
    }

    #[test]
    fn canonical_close_status_preserves_blocked_json_parser_failures() {
        let reason = "Blocked JSON parser failure; cannot proceed";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }

    #[test]
    fn canonical_close_status_preserves_blocked_prefix_with_meta_keywords() {
        let reason = "Blocked: cargo test failed";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }

    #[test]
    fn canonical_close_status_preserves_approval_prefix_with_meta_keywords() {
        let reason = "Awaiting_approval: return to operator with proof artifact";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some((
                "awaiting_approval",
                crate::release1_contracts::ApprovalStatus::ApprovalRequired.as_str()
            ))
        );
    }

    #[test]
    fn canonical_close_status_still_preserves_concrete_blocked_reasons() {
        let reason = "Task remains blocked pending operator evidence.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }

    #[test]
    fn canonical_close_status_preserves_rework_blocker_verdicts() {
        for reason in [
            "blocker/rework/not-approved",
            "Cannot proceed because blocker missing credentials",
            "Prevents closure: blocker missing credentials",
        ] {
            assert_eq!(
                super::canonical_close_status_from_reason(reason),
                Some(("blocked", "blocked")),
                "reason should remain a blocked close verdict: {reason}"
            );
        }
    }
}
