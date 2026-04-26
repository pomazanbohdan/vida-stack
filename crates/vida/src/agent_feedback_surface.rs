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
    for phrase in ignored_feedback_contract_language(reason)
        .into_iter()
        .chain(ignored_canonical_close_meta_language(reason))
        .chain(ignored_feedback_meta_language(reason))
    {
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
    ignored_feedback_phrases(reason, &["fail-closed", "fail closed", "fail_closed"])
}

fn ignored_feedback_meta_language(reason: &str) -> Vec<String> {
    ignored_feedback_phrases(
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
    )
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
            "concrete blocked task outcomes",
            "blocked task outcomes",
            "failure evidence",
            "concrete blocked reasons",
            "top-level blocked/actionable",
            "top level blocked/actionable",
            "actionable blocked output",
            "genuinely blocked",
            "readiness blockers",
            "readiness blocker",
            "blocker coverage",
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
        ],
    );
    ignored.extend(ignored_canonical_close_meta_segments(reason));
    ignored.sort();
    ignored.dedup();
    ignored
}

fn ignored_canonical_close_meta_segments(reason: &str) -> Vec<String> {
    let blocker_keywords = ["blocked", "blocker", "approval_wait", "awaiting_approval"];
    let meta_keywords = [
        "fixed",
        "proofs:",
        "proof:",
        "returns",
        "return",
        "preserves",
        "preserve",
        "mirrors",
        "mirror",
        "diagnostic context",
        "diagnostic",
        "canonical",
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
            let has_meta_keyword = meta_keywords
                .iter()
                .any(|keyword| normalized.contains(keyword));
            if has_blocker_keyword && has_meta_keyword {
                Some(normalized)
            } else {
                None
            }
        })
        .collect()
}

fn canonical_close_status_from_reason(reason: &str) -> Option<(&'static str, &'static str)> {
    let mut normalized = reason.to_ascii_lowercase();
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
    let task_class = super::infer_task_class_from_task_payload(task);
    let runtime_role = super::runtime_role_for_task_class(&task_class);
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
    fn canonical_close_status_preserves_concrete_blocked_reasons() {
        let reason = "Task remains blocked pending operator evidence.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
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
        assert!(ignored
            .iter()
            .any(|phrase| phrase == "concrete blocked task outcomes"));
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
    fn canonical_close_status_still_preserves_concrete_blocked_reasons() {
        let reason = "Task remains blocked pending operator evidence.";

        assert_eq!(
            super::canonical_close_status_from_reason(reason),
            Some(("blocked", "blocked"))
        );
    }
}
