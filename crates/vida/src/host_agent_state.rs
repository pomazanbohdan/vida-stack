use std::fs;
use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use super::{json_lookup, WORKER_SCORECARDS_STATE, WORKER_STRATEGY_STATE};

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
    let promotion_score = json_u64(json_lookup(scoring_policy, &["promotion_score"])).unwrap_or(75);
    let demotion_score = json_u64(json_lookup(scoring_policy, &["demotion_score"])).unwrap_or(45);
    let consecutive_failure_limit =
        json_u64(json_lookup(scoring_policy, &["consecutive_failure_limit"])).unwrap_or(3);
    let probation_task_runs =
        json_u64(json_lookup(scoring_policy, &["probation_task_runs"])).unwrap_or(2);
    let retirement_failure_limit =
        json_u64(json_lookup(scoring_policy, &["retirement_failure_limit"])).unwrap_or(8);

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
            "rule": "capability_first_then_score_guard_then_cheapest_tier",
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
        "selection_rule": "choose the cheapest configured agent that satisfies runtime-role admissibility, task-class fit, and the local score guard; escalate only when the cheaper candidate is degraded or score-insufficient",
        "rates": rate_map,
        "vendor_basis": {
            "openai": "structured role configs and reasoning-effort tuning in Codex project config; prefer explicit task/runtime settings over implicit chat heuristics",
            "anthropic": "structured prompt templates, explicit sections, and evaluation-backed refinement loops",
            "microsoft": "record one bounded decision/design artifact per change and route on explicit cost-quality tradeoffs instead of ad hoc escalation"
        },
        "local_strategy_store": worker_strategy["store_path"],
        "local_scorecards_store": worker_strategy["scorecards_path"],
        "tiers": tiers
    })
}
