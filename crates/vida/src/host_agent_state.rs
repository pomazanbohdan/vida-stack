use std::fs;
use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use crate::carrier_runtime_metadata::{
    selection_policy_rule, selection_policy_u64, DEFAULT_CONSECUTIVE_FAILURE_LIMIT,
    DEFAULT_DEMOTION_SCORE, DEFAULT_PROBATION_TASK_RUNS, DEFAULT_PROMOTION_SCORE,
    DEFAULT_RETIREMENT_FAILURE_LIMIT,
};

use super::{WORKER_SCORECARDS_STATE, WORKER_STRATEGY_STATE};

pub(crate) const HOST_AGENT_OBSERVABILITY_STATE: &str = ".vida/state/host-agent-observability.json";

pub(crate) fn worker_scorecards_state_path(project_root: &Path) -> PathBuf {
    project_root.join(WORKER_SCORECARDS_STATE)
}

pub(crate) fn worker_strategy_state_path(project_root: &Path) -> PathBuf {
    project_root.join(WORKER_STRATEGY_STATE)
}

pub(crate) fn host_agent_observability_state_path(project_root: &Path) -> PathBuf {
    project_root.join(HOST_AGENT_OBSERVABILITY_STATE)
}

pub(crate) fn prompt_lifecycle_state_path(project_root: &Path) -> PathBuf {
    project_root.join(crate::PROMPT_LIFECYCLE_STATE)
}

pub(crate) fn read_json_file_if_present(path: &Path) -> Option<serde_json::Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub(crate) fn load_or_initialize_host_agent_observability_state(
    project_root: &Path,
) -> serde_json::Value {
    read_json_file_if_present(&host_agent_observability_state_path(project_root)).unwrap_or_else(
        || {
            serde_json::json!({
                "schema_version": 1,
                "updated_at": time::OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .expect("rfc3339 timestamp should render"),
                "store_path": HOST_AGENT_OBSERVABILITY_STATE,
                "budget": {
                    "total_estimated_units": 0,
                    "by_agent_id": {},
                    "by_task_class": {},
                    "event_count": 0,
                    "latest_event_at": serde_json::Value::Null,
                },
                "events": []
            })
        },
    )
}

pub(crate) fn load_or_initialize_prompt_lifecycle_state(project_root: &Path) -> serde_json::Value {
    read_json_file_if_present(&prompt_lifecycle_state_path(project_root)).unwrap_or_else(|| {
        serde_json::json!({
            "schema_version": 1,
            "updated_at": time::OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("rfc3339 timestamp should render"),
            "store_path": crate::PROMPT_LIFECYCLE_STATE,
            "workflows": {}
        })
    })
}

#[derive(Debug, Clone)]
pub(crate) struct HostAgentFeedbackInput<'a> {
    pub(crate) agent_id: &'a str,
    pub(crate) score: u64,
    pub(crate) outcome: &'a str,
    pub(crate) task_class: &'a str,
    pub(crate) notes: Option<&'a str>,
    pub(crate) source: &'a str,
    pub(crate) task_id: Option<&'a str>,
    pub(crate) task_display_id: Option<&'a str>,
    pub(crate) task_title: Option<&'a str>,
    pub(crate) runtime_role: Option<&'a str>,
    pub(crate) selected_tier: Option<&'a str>,
    pub(crate) estimated_task_price_units: Option<u64>,
    pub(crate) lifecycle_state: Option<&'a str>,
    pub(crate) effective_score: Option<u64>,
    pub(crate) reason: Option<&'a str>,
}

fn increment_object_counter(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    delta: u64,
) {
    let current = object
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    object.insert(
        key.to_string(),
        serde_json::Value::Number(serde_json::Number::from(current + delta)),
    );
}

fn host_feedback_artifact_suffix(recorded_at: &str) -> String {
    recorded_at
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn host_feedback_workflow_class(task_class: &str) -> &'static str {
    match task_class.trim() {
        "implementation" => {
            crate::release1_contracts::WorkflowClass::DelegatedDevelopmentPacket.as_str()
        }
        "verification" | "review" | "quality_gate" | "release_readiness" => {
            crate::release1_contracts::WorkflowClass::ToolAssistedRead.as_str()
        }
        "architecture" | "specification" => {
            crate::release1_contracts::WorkflowClass::DocumentationMutation.as_str()
        }
        "memory" => crate::release1_contracts::WorkflowClass::MemoryWrite.as_str(),
        _ => crate::release1_contracts::WorkflowClass::InformationalAnswer.as_str(),
    }
}

fn host_feedback_source_kind(source: &str) -> &'static str {
    if source.trim() == "vida task close" {
        "automatic_task_close_feedback"
    } else {
        "manual_feedback"
    }
}

fn host_feedback_severity(outcome: &str) -> &'static str {
    match outcome.trim() {
        "failure" => "high",
        "neutral" => "medium",
        _ => "low",
    }
}

fn host_feedback_defect_cluster(input: &HostAgentFeedbackInput<'_>) -> &'static str {
    let joined = format!(
        "{} {} {}",
        input.notes.unwrap_or(""),
        input.reason.unwrap_or(""),
        input.task_class
    )
    .to_ascii_lowercase();
    if joined.contains("prompt") || joined.contains("regression") {
        "prompt_regression"
    } else if joined.contains("safety") || joined.contains("adversarial") {
        "safety_review"
    } else if joined.contains("approval") {
        "approval_gate"
    } else if joined.contains("retrieval")
        || joined.contains("citation")
        || joined.contains("freshness")
    {
        "retrieval_trust"
    } else if joined.contains("tool") || joined.contains("contract") {
        "tool_contract"
    } else if joined.contains("timeout") || joined.contains("latency") {
        "runtime_timeout"
    } else {
        "general_runtime_feedback"
    }
}

fn host_feedback_summary_text(input: &HostAgentFeedbackInput<'_>) -> String {
    input
        .notes
        .or(input.reason)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(input.outcome)
        .to_string()
}

fn host_feedback_feedback_event(
    input: &HostAgentFeedbackInput<'_>,
    recorded_at: &str,
) -> serde_json::Value {
    let workflow_class = host_feedback_workflow_class(input.task_class);
    let suffix = host_feedback_artifact_suffix(recorded_at);
    let trace_id = format!("host-agent-feedback.trace.{}.{}", input.agent_id, suffix);
    serde_json::to_value(crate::release1_contracts::CanonicalFeedbackArtifact {
        feedback_event: crate::release1_contracts::CanonicalFeedbackEvent {
            header: crate::release1_contracts::CanonicalArtifactHeader::new(
                format!("host-agent-feedback.event.{}.{}", input.agent_id, suffix),
                crate::release1_contracts::CanonicalArtifactType::FeedbackEvent,
                recorded_at.to_string(),
                recorded_at.to_string(),
                "recorded",
                "host_agent_observability",
                Some(trace_id),
                Some(workflow_class.to_string()),
            ),
            feedback_id: format!("host-agent-feedback.{}.{}", input.agent_id, suffix),
            source_kind: host_feedback_source_kind(input.source).to_string(),
            severity: host_feedback_severity(input.outcome).to_string(),
            feedback_type: "agent_runtime_feedback".to_string(),
            summary: host_feedback_summary_text(input),
            linked_defect_or_remediation_id: input.task_id.map(str::to_string),
        },
    })
    .expect("feedback artifact should serialize")
}

fn host_feedback_evaluation_baseline(
    input: &HostAgentFeedbackInput<'_>,
    recorded_at: &str,
) -> serde_json::Value {
    let workflow_class = host_feedback_workflow_class(input.task_class);
    let suffix = host_feedback_artifact_suffix(recorded_at);
    let trace_id = format!("host-agent-feedback.trace.{}.{}", input.agent_id, suffix);
    let safety_defect_rate = if input.outcome == "failure" {
        100.0
    } else {
        0.0
    };
    serde_json::to_value(crate::release1_contracts::CanonicalEvaluationArtifact {
        evaluation_run: crate::release1_contracts::CanonicalEvaluationRun {
            header: crate::release1_contracts::CanonicalArtifactHeader::new(
                format!(
                    "host-agent-feedback.evaluation.{}.{}",
                    input.agent_id, suffix
                ),
                crate::release1_contracts::CanonicalArtifactType::EvaluationRun,
                recorded_at.to_string(),
                recorded_at.to_string(),
                "recorded",
                "host_agent_observability",
                Some(trace_id.clone()),
                Some(workflow_class.to_string()),
            ),
            evaluation_id: format!("host-agent-evaluation.{}.{}", input.agent_id, suffix),
            evaluation_profile: "host_agent_feedback_baseline".to_string(),
            target_surface: input.source.to_string(),
            dataset_or_sample_window: input
                .task_id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("single_feedback_event")
                .to_string(),
            metric_results: std::collections::BTreeMap::from([
                ("feedback_score".to_string(), input.score as f64),
                ("safety_defect_rate".to_string(), safety_defect_rate),
            ]),
            regression_summary: format!(
                "defect_cluster={} outcome={}",
                host_feedback_defect_cluster(input),
                input.outcome
            ),
            decision: match input.outcome {
                "failure" => "hold",
                "neutral" => "review",
                _ => "observe",
            }
            .to_string(),
            decision_reason: input.reason.unwrap_or(input.outcome).to_string(),
            run_at: recorded_at.to_string(),
            trace_sample_refs: vec![trace_id],
        },
    })
    .expect("evaluation artifact should serialize")
}

fn host_feedback_prompt_lifecycle_baseline(
    input: &HostAgentFeedbackInput<'_>,
    recorded_at: &str,
) -> serde_json::Value {
    let workflow_class = host_feedback_workflow_class(input.task_class);
    let suffix = host_feedback_artifact_suffix(recorded_at);
    serde_json::json!({
        "artifact_id": format!("host-agent-feedback.prompt-lifecycle.{}.{}", input.agent_id, suffix),
        "status": "baseline_only",
        "workflow_class": workflow_class,
        "lifecycle_state": "draft",
        "promotion_gate": "benchmark_required",
        "canary_status": "not_started",
        "rollback_posture": "manual_rollback_ready",
        "source_feedback_id": format!("host-agent-feedback.{}.{}", input.agent_id, suffix),
        "recorded_at": recorded_at,
    })
}

fn host_feedback_safety_baseline(
    input: &HostAgentFeedbackInput<'_>,
    recorded_at: &str,
) -> serde_json::Value {
    serde_json::json!({
        "artifact_id": format!(
            "host-agent-feedback.safety.{}.{}",
            input.agent_id,
            host_feedback_artifact_suffix(recorded_at)
        ),
        "status": if input.outcome == "failure" { "review_required" } else { "baseline_recorded" },
        "workflow_class": host_feedback_workflow_class(input.task_class),
        "safety_gate": match input.outcome {
            "failure" => "hold",
            "neutral" => "review",
            _ => "observe",
        },
        "defect_cluster": host_feedback_defect_cluster(input),
        "defect_rate_signal": if input.outcome == "failure" { 100.0 } else { 0.0 },
        "recorded_at": recorded_at,
    })
}

fn persist_prompt_lifecycle_baseline(
    project_root: &Path,
    prompt_lifecycle_baseline: &serde_json::Value,
    evaluation_baseline: &serde_json::Value,
    safety_baseline: &serde_json::Value,
    feedback_event: &serde_json::Value,
    recorded_at: &str,
) -> Result<(), String> {
    let mut registry = load_or_initialize_prompt_lifecycle_state(project_root);
    if !registry["workflows"].is_object() {
        registry["workflows"] = serde_json::json!({});
    }
    let workflow_class = prompt_lifecycle_baseline["workflow_class"]
        .as_str()
        .unwrap_or("informational_answer")
        .to_string();
    registry["workflows"][&workflow_class] = serde_json::json!({
        "artifact_id": format!("prompt-lifecycle-registry.{workflow_class}"),
        "workflow_class": workflow_class,
        "lifecycle_state": prompt_lifecycle_baseline["lifecycle_state"].clone(),
        "promotion_gate": prompt_lifecycle_baseline["promotion_gate"].clone(),
        "canary_status": prompt_lifecycle_baseline["canary_status"].clone(),
        "rollback_posture": prompt_lifecycle_baseline["rollback_posture"].clone(),
        "latest_feedback_event_id": feedback_event["feedback_id"].clone(),
        "latest_evaluation_artifact_id": evaluation_baseline["artifact_id"].clone(),
        "latest_safety_gate": safety_baseline["safety_gate"].clone(),
        "last_updated_at": recorded_at,
    });
    registry["updated_at"] = serde_json::Value::String(recorded_at.to_string());

    let path = prompt_lifecycle_state_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&registry).expect("prompt lifecycle json should render"),
    )
    .map_err(|error| format!("Failed to write {}: {error}", path.display()))
}

pub(crate) fn append_host_agent_observability_event(
    project_root: &Path,
    input: &HostAgentFeedbackInput<'_>,
) -> Result<serde_json::Value, String> {
    let mut ledger = load_or_initialize_host_agent_observability_state(project_root);
    if !ledger["events"].is_array() {
        ledger["events"] = serde_json::json!([]);
    }
    if !ledger["budget"].is_object() {
        ledger["budget"] = serde_json::json!({
            "total_estimated_units": 0,
            "by_agent_id": {},
            "by_task_class": {},
            "event_count": 0,
            "latest_event_at": serde_json::Value::Null,
        });
    }

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let estimated_units = input.estimated_task_price_units.unwrap_or_default();
    let feedback_event = host_feedback_feedback_event(input, &recorded_at);
    let evaluation_baseline = host_feedback_evaluation_baseline(input, &recorded_at);
    let prompt_lifecycle_baseline = host_feedback_prompt_lifecycle_baseline(input, &recorded_at);
    let safety_baseline = host_feedback_safety_baseline(input, &recorded_at);
    persist_prompt_lifecycle_baseline(
        project_root,
        &prompt_lifecycle_baseline,
        &evaluation_baseline,
        &safety_baseline,
        &feedback_event,
        &recorded_at,
    )?;
    let event = serde_json::json!({
        "recorded_at": recorded_at,
        "event_kind": "feedback",
        "source": input.source,
        "agent_id": input.agent_id,
        "selected_tier": input.selected_tier.unwrap_or(input.agent_id),
        "runtime_role": input.runtime_role.unwrap_or(""),
        "task_id": input.task_id.unwrap_or(""),
        "task_display_id": input.task_display_id.unwrap_or(""),
        "task_title": input.task_title.unwrap_or(""),
        "task_class": input.task_class,
        "outcome": input.outcome,
        "score": input.score,
        "notes": input.notes.unwrap_or(""),
        "reason": input.reason.unwrap_or(""),
        "estimated_task_price_units": estimated_units,
        "effective_score": input.effective_score,
        "lifecycle_state": input.lifecycle_state.unwrap_or(""),
        "feedback_event": feedback_event,
        "evaluation_baseline": evaluation_baseline,
        "prompt_lifecycle_baseline": prompt_lifecycle_baseline,
        "safety_baseline": safety_baseline,
    });
    let event_count = {
        let events = ledger["events"]
            .as_array_mut()
            .expect("events array should initialize");
        events.push(event.clone());
        events.len() as u64
    };

    let budget = ledger["budget"]
        .as_object_mut()
        .expect("budget object should initialize");
    let total_estimated_units = budget
        .get("total_estimated_units")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    budget.insert(
        "total_estimated_units".to_string(),
        serde_json::Value::Number(serde_json::Number::from(
            total_estimated_units + estimated_units,
        )),
    );
    if budget
        .get("by_agent_id")
        .and_then(serde_json::Value::as_object)
        .is_none()
    {
        budget.insert("by_agent_id".to_string(), serde_json::json!({}));
    }
    if budget
        .get("by_task_class")
        .and_then(serde_json::Value::as_object)
        .is_none()
    {
        budget.insert("by_task_class".to_string(), serde_json::json!({}));
    }
    if let Some(by_agent_id) = budget
        .get_mut("by_agent_id")
        .and_then(serde_json::Value::as_object_mut)
    {
        increment_object_counter(by_agent_id, input.agent_id, estimated_units);
    }
    if let Some(by_task_class) = budget
        .get_mut("by_task_class")
        .and_then(serde_json::Value::as_object_mut)
    {
        increment_object_counter(by_task_class, input.task_class, estimated_units);
    }
    budget.insert(
        "event_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(event_count)),
    );
    budget.insert(
        "latest_event_at".to_string(),
        serde_json::Value::String(recorded_at.clone()),
    );
    ledger["updated_at"] = serde_json::Value::String(recorded_at.clone());

    let ledger_path = host_agent_observability_state_path(project_root);
    if let Some(parent) = ledger_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &ledger_path,
        serde_json::to_string_pretty(&ledger).expect("observability json should render"),
    )
    .map_err(|error| format!("Failed to write {}: {error}", ledger_path.display()))?;
    Ok(event)
}

pub(crate) fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    })
}

fn clamp_score(value: f64) -> u64 {
    value.round().clamp(0.0, 100.0) as u64
}

fn worker_scorecard_feedback_rows(
    scorecards: &serde_json::Value,
    role_id: &str,
) -> Vec<serde_json::Value> {
    scorecards["agents"][role_id]["feedback"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn load_or_initialize_worker_scorecards(
    project_root: &Path,
    carrier_roles: &[serde_json::Value],
) -> serde_json::Value {
    let path = worker_scorecards_state_path(project_root);
    let mut scorecards = read_json_file_if_present(&path).unwrap_or_else(|| {
        serde_json::json!({
            "schema_version": 1,
            "updated_at": time::OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("rfc3339 timestamp should render"),
            "agents": {}
        })
    });

    let Some(agents) = scorecards["agents"].as_object_mut() else {
        scorecards["agents"] = serde_json::json!({});
        let agents = scorecards["agents"]
            .as_object_mut()
            .expect("agents object should exist");
        for row in carrier_roles {
            if let Some(role_id) = row["role_id"].as_str() {
                agents.insert(role_id.to_string(), serde_json::json!({"feedback": []}));
            }
        }
        let body =
            serde_json::to_string_pretty(&scorecards).expect("scorecards json should render");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, body);
        return scorecards;
    };

    let mut changed = false;
    for row in carrier_roles {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        if !agents.contains_key(role_id) {
            agents.insert(role_id.to_string(), serde_json::json!({"feedback": []}));
            changed = true;
        }
    }
    if changed {
        scorecards["updated_at"] = serde_json::Value::String(
            time::OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("rfc3339 timestamp should render"),
        );
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let body =
            serde_json::to_string_pretty(&scorecards).expect("scorecards json should render");
        let _ = fs::write(&path, body);
    }
    scorecards
}

pub(crate) fn refresh_worker_strategy(
    project_root: &Path,
    carrier_roles: &[serde_json::Value],
    scoring_policy: &serde_json::Value,
) -> serde_json::Value {
    let scorecards = load_or_initialize_worker_scorecards(project_root, carrier_roles);
    let selection_rule = selection_policy_rule(scoring_policy);
    let promotion_score =
        selection_policy_u64(scoring_policy, "promotion_score", DEFAULT_PROMOTION_SCORE);
    let demotion_score =
        selection_policy_u64(scoring_policy, "demotion_score", DEFAULT_DEMOTION_SCORE);
    let consecutive_failure_limit = selection_policy_u64(
        scoring_policy,
        "consecutive_failure_limit",
        DEFAULT_CONSECUTIVE_FAILURE_LIMIT,
    );
    let probation_task_runs = selection_policy_u64(
        scoring_policy,
        "probation_task_runs",
        DEFAULT_PROBATION_TASK_RUNS,
    );
    let retirement_failure_limit = selection_policy_u64(
        scoring_policy,
        "retirement_failure_limit",
        DEFAULT_RETIREMENT_FAILURE_LIMIT,
    );

    let mut agents = serde_json::Map::new();
    for row in carrier_roles {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let tier = row["tier"].as_str().unwrap_or(role_id);
        let rate = row["rate"].as_u64().unwrap_or(0);
        let feedback_rows = worker_scorecard_feedback_rows(&scorecards, role_id);
        let feedback_count = feedback_rows.len() as u64;
        let mut success_runs = 0u64;
        let mut failure_runs = 0u64;
        let mut last_feedback_score = None::<u64>;
        let mut total_feedback_score = 0f64;
        let mut consecutive_failures = 0u64;

        for item in &feedback_rows {
            let outcome = item["outcome"].as_str().unwrap_or("neutral");
            let score = json_u64(item.get("score")).unwrap_or(70);
            last_feedback_score = Some(score);
            total_feedback_score += score as f64;
            if outcome == "success" {
                success_runs += 1;
                consecutive_failures = 0;
            } else if outcome == "failure" {
                failure_runs += 1;
                consecutive_failures += 1;
            } else {
                consecutive_failures = 0;
            }
        }

        let average_feedback = if feedback_count > 0 {
            total_feedback_score / feedback_count as f64
        } else {
            70.0
        };
        let base_score = 70.0;
        let success_bonus = (success_runs as f64 * 2.5).min(15.0);
        let failure_penalty = (failure_runs as f64 * 4.0).min(28.0);
        let feedback_adjustment = (average_feedback - 70.0) * 0.45;
        let effective_score =
            clamp_score(base_score + success_bonus - failure_penalty + feedback_adjustment);
        let lifecycle_state = if consecutive_failures >= retirement_failure_limit {
            "retired"
        } else if consecutive_failures >= consecutive_failure_limit
            || effective_score < demotion_score
        {
            "degraded"
        } else if feedback_count < probation_task_runs {
            "probation"
        } else if effective_score >= promotion_score {
            "promoted"
        } else {
            "active"
        };

        agents.insert(
            role_id.to_string(),
            serde_json::json!({
                "tier": tier,
                "rate": rate,
                "reasoning_band": row["reasoning_band"],
                "default_runtime_role": row["default_runtime_role"],
                "task_classes": row["task_classes"],
                "feedback_count": feedback_count,
                "success_runs": success_runs,
                "failure_runs": failure_runs,
                "consecutive_failures": consecutive_failures,
                "average_feedback": (average_feedback * 100.0).round() / 100.0,
                "last_feedback_score": last_feedback_score,
                "effective_score": effective_score,
                "lifecycle_state": lifecycle_state,
            }),
        );
    }

    let strategy = serde_json::json!({
        "schema_version": 1,
        "updated_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "store_path": WORKER_STRATEGY_STATE,
        "scorecards_path": WORKER_SCORECARDS_STATE,
        "selection_policy": {
            "rule": selection_rule,
            "promotion_score": promotion_score,
            "demotion_score": demotion_score,
            "consecutive_failure_limit": consecutive_failure_limit,
            "retirement_failure_limit": retirement_failure_limit
        },
        "agents": agents
    });
    let path = worker_strategy_state_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(&strategy).expect("strategy json should render");
    let _ = fs::write(&path, body);
    strategy
}

pub(crate) fn build_carrier_pricing_policy(
    carrier_roles: &[serde_json::Value],
    worker_strategy: &serde_json::Value,
    vendor_basis: &serde_json::Value,
) -> serde_json::Value {
    let mut tiers = carrier_roles.to_vec();
    tiers.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
    });
    let mut rate_map = serde_json::Map::new();
    for row in &tiers {
        if let (Some(tier), Some(rate)) = (row["tier"].as_str(), row["rate"].as_u64()) {
            rate_map.insert(tier.to_string(), serde_json::Value::Number(rate.into()));
        }
    }
    serde_json::json!({
        "selection_rule": crate::carrier_runtime_metadata::selection_policy_rule(
            &worker_strategy["selection_policy"],
        ),
        "rates": rate_map,
        "vendor_basis": vendor_basis,
        "local_strategy_store": worker_strategy["store_path"],
        "local_scorecards_store": worker_strategy["scorecards_path"],
        "tiers": tiers
    })
}

#[cfg(test)]
mod tests {
    use super::{
        append_host_agent_observability_event, load_or_initialize_host_agent_observability_state,
        load_or_initialize_prompt_lifecycle_state, HostAgentFeedbackInput,
    };
    use crate::temp_state::TempStateHarness;

    #[test]
    fn append_host_agent_observability_event_records_control_baseline_artifacts() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let input = HostAgentFeedbackInput {
            agent_id: "junior",
            score: 92,
            outcome: "success",
            task_class: "implementation",
            notes: Some("clean bounded closure"),
            source: "vida agent-feedback",
            task_id: Some("task-1"),
            task_display_id: None,
            task_title: Some("Example Task"),
            runtime_role: Some("implementer"),
            selected_tier: Some("junior"),
            estimated_task_price_units: Some(4),
            lifecycle_state: Some("promoted"),
            effective_score: Some(81),
            reason: None,
        };

        let event = append_host_agent_observability_event(harness.path(), &input)
            .expect("event should record");
        assert_eq!(event["feedback_event"]["artifact_type"], "feedback_event");
        assert_eq!(
            event["feedback_event"]["workflow_class"],
            "delegated_development_packet"
        );
        assert_eq!(
            event["evaluation_baseline"]["artifact_type"],
            "evaluation_run"
        );
        assert_eq!(
            event["prompt_lifecycle_baseline"]["lifecycle_state"],
            "draft"
        );
        assert_eq!(event["safety_baseline"]["safety_gate"], "observe");

        let ledger = load_or_initialize_host_agent_observability_state(harness.path());
        assert_eq!(
            ledger["events"][0]["feedback_event"]["artifact_type"],
            "feedback_event"
        );
        let prompt_lifecycle = load_or_initialize_prompt_lifecycle_state(harness.path());
        assert_eq!(
            prompt_lifecycle["workflows"]["delegated_development_packet"]["lifecycle_state"],
            "draft"
        );
    }

    #[test]
    fn append_host_agent_observability_event_marks_failure_as_hold_with_prompt_regression_cluster()
    {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let input = HostAgentFeedbackInput {
            agent_id: "senior",
            score: 34,
            outcome: "failure",
            task_class: "verification",
            notes: Some("prompt regression triggered safety review"),
            source: "vida task close",
            task_id: Some("task-2"),
            task_display_id: None,
            task_title: Some("Regression Task"),
            runtime_role: Some("verifier"),
            selected_tier: Some("senior"),
            estimated_task_price_units: Some(8),
            lifecycle_state: Some("promoted"),
            effective_score: Some(79),
            reason: Some("critical prompt regression"),
        };

        let event = append_host_agent_observability_event(harness.path(), &input)
            .expect("event should record");
        assert_eq!(
            event["feedback_event"]["source_kind"],
            "automatic_task_close_feedback"
        );
        assert_eq!(event["feedback_event"]["severity"], "high");
        assert_eq!(event["evaluation_baseline"]["decision"], "hold");
        assert_eq!(event["safety_baseline"]["status"], "review_required");
        assert_eq!(
            event["safety_baseline"]["defect_cluster"],
            "prompt_regression"
        );
    }
}
