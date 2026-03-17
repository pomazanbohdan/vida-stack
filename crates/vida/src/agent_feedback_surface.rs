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
    let normalized = reason.to_ascii_lowercase();
    let inferred = if super::contains_keywords(
        &normalized,
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
    .len()
        >= 1
    {
        "failure"
    } else if super::contains_keywords(
        &normalized,
        &[
            "neutral".to_string(),
            "partial".to_string(),
            "handoff".to_string(),
            "handoff pending".to_string(),
        ],
    )
    .len()
        >= 1
    {
        "neutral"
    } else {
        "success"
    };
    canonical_feedback_outcome(inferred).expect("inferred feedback outcome must be canonical")
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

fn canonical_close_status_from_reason(reason: &str) -> Option<(&'static str, &'static str)> {
    let normalized = reason.to_ascii_lowercase();
    let approval_keywords = [
        "approval_wait".to_string(),
        "awaiting_approval".to_string(),
        "approval required".to_string(),
        "pending approval".to_string(),
    ];
    if !super::contains_keywords(&normalized, &approval_keywords).is_empty() {
        return Some(("awaiting_approval", "approval_required"));
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

pub(crate) fn maybe_record_task_close_host_agent_feedback(
    project_root: &std::path::Path,
    task: &serde_json::Value,
    close_reason: &str,
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
    let selected_cli_system = super::yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(super::project_activator_surface::normalize_host_cli_system);
    if selected_cli_system != Some("codex") {
        return serde_json::json!({
            "status": "skipped",
            "reason": "host_cli_not_selected_or_unsupported"
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
    let task_class = super::infer_codex_task_class_from_task_payload(task);
    let runtime_role = super::codex_runtime_role_for_task_class(&task_class);
    let assignment = super::build_codex_runtime_assignment_from_resolved_constraints(
        &compiled_bundle,
        "orchestrator",
        &task_class,
        runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        return serde_json::json!({
            "status": "skipped",
            "reason": assignment["reason"].as_str().unwrap_or("codex_runtime_assignment_disabled"),
            "task_class": task_class,
            "runtime_role": runtime_role,
        });
    }

    let agent_id = assignment["selected_agent_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if agent_id.is_empty() {
        return serde_json::json!({
            "status": "error",
            "reason": "selected_agent_id_missing",
            "task_class": task_class,
            "runtime_role": runtime_role,
        });
    }
    if let Some((canonical_status, canonical_gate)) = canonical_close_status_from_reason(close_reason) {
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
    let input = super::HostAgentFeedbackInput {
        agent_id: &agent_id,
        score,
        outcome,
        task_class: &task_class,
        notes: Some("automatic task-close feedback"),
        source: "vida taskflow task close",
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
            "assignment": assignment,
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
    let selected_cli_system = super::yaml_lookup(&overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .and_then(super::project_activator_surface::normalize_host_cli_system)
        .ok_or_else(|| {
            "Host CLI system is missing or unsupported in vida.config.yaml.".to_string()
        })?;
    match selected_cli_system {
        "codex" => {
            let codex_roles = {
                let overlay_roles =
                    super::project_activator_surface::overlay_codex_agent_catalog(&overlay);
                if overlay_roles.is_empty() {
                    super::project_activator_surface::read_codex_agent_catalog(
                        &project_root.join(".codex"),
                    )
                } else {
                    overlay_roles
                }
            };
            if !codex_roles
                .iter()
                .any(|row| row["role_id"].as_str() == Some(input.agent_id))
            {
                return Err(format!(
                    "Unknown host agent `{}` for selected CLI system `codex`.",
                    input.agent_id
                ));
            }
            let scorecards_path = super::codex_worker_scorecards_state_path(project_root);
            let mut scorecards =
                super::load_or_initialize_codex_worker_scorecards(project_root, &codex_roles);
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
            let strategy =
                super::refresh_codex_worker_strategy(project_root, &codex_roles, &scoring_policy);
            let observability_event =
                super::append_host_agent_observability_event(project_root, input)?;
            Ok(serde_json::json!({
                "surface": "vida agent-feedback",
                "host_cli_system": "codex",
                "agent_id": input.agent_id,
                "recorded_score": input.score,
                "recorded_outcome": input.outcome,
                "recorded_task_class": input.task_class,
                "recorded_notes": input.notes.unwrap_or(""),
                "scorecards_store": super::CODEX_WORKER_SCORECARDS_STATE,
                "strategy_store": super::CODEX_WORKER_STRATEGY_STATE,
                "observability_store": super::HOST_AGENT_OBSERVABILITY_STATE,
                "strategy_row": strategy["agents"][input.agent_id],
                "observability_event": observability_event
            }))
        }
        other => Err(format!(
            "Feedback writeback is not implemented for host CLI system `{other}`."
        )),
    }
}
