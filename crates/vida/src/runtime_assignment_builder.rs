use crate::{
    carrier_runtime_section, infer_execution_runtime_role, infer_runtime_task_class, json_lookup,
    json_u64, role_supports_task_class, runtime_role_for_task_class,
    task_complexity_multiplier, RuntimeConsumptionLaneSelection,
};

fn selection_strategy(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["model_selection"]["default_strategy"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("balanced_cost_quality")
        .to_string()
}

fn free_profiles_allowed(carrier_runtime: &serde_json::Value) -> bool {
    carrier_runtime["model_selection"]["free_profiles_allowed"]
        .as_bool()
        .or_else(|| carrier_runtime["model_selection"]["zero_cost_profiles_allowed"].as_bool())
        .unwrap_or(true)
}

fn quality_floor_for_runtime_role(
    carrier_runtime: &serde_json::Value,
    runtime_role: &str,
) -> Option<String> {
    carrier_runtime["model_selection"]["quality_floor_by_runtime_role"][runtime_role]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn reasoning_floor_for_task_class(
    carrier_runtime: &serde_json::Value,
    task_class: &str,
) -> Option<String> {
    carrier_runtime["model_selection"]["reasoning_floor_by_task_class"][task_class]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn selection_rule_for_runtime(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["model_selection"]["selection_rule"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("role_task_then_readiness_then_score_then_cost_quality")
        .to_string()
}

fn quality_tier_rank(raw: &str) -> u8 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "very_high" | "veryhigh" => 5,
        "high" | "medium_high" | "mediumhigh" => 4,
        "medium" => 3,
        "medium_low" | "mediumlow" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn reasoning_effort_rank(raw: &str) -> u8 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "xhigh" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "minimal" => 1,
        "provider_default" | "provider-configured" => 2,
        _ => 0,
    }
}

fn profile_runtime_roles(
    role: &serde_json::Value,
    profile: &serde_json::Value,
) -> Vec<String> {
    let profile_roles = profile["runtime_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if profile_roles.is_empty() {
        role["runtime_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect()
    } else {
        profile_roles
    }
}

fn profile_task_classes(
    role: &serde_json::Value,
    profile: &serde_json::Value,
) -> Vec<String> {
    let profile_tasks = profile["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if profile_tasks.is_empty() {
        role["task_classes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect()
    } else {
        profile_tasks
    }
}

fn profile_supports_runtime_role(
    role: &serde_json::Value,
    profile: &serde_json::Value,
    runtime_role: &str,
) -> bool {
    let runtime_roles = profile_runtime_roles(role, profile);
    runtime_roles.is_empty() || runtime_roles.iter().any(|value| value == runtime_role)
}

fn profile_supports_task_class(
    role: &serde_json::Value,
    profile: &serde_json::Value,
    task_class: &str,
) -> bool {
    let task_classes = profile_task_classes(role, profile);
    task_classes.is_empty() || task_classes.iter().any(|value| value == task_class)
}

fn profile_readiness_status(profile: &serde_json::Value) -> String {
    if let Some(status) = profile["readiness"]["status"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return status.to_string();
    }
    if profile["readiness"]["required"].as_bool() == Some(true)
        && profile["readiness"]["ready"].as_bool() == Some(false)
    {
        return "blocked".to_string();
    }
    "ready".to_string()
}

fn task_class_requires_write_scope(task_class: &str) -> bool {
    matches!(
        task_class,
        "implementation" | "implementation_medium" | "delivery_task" | "execution_block"
    )
}

fn write_scope_allows_task_class(write_scope: &str, task_class: &str) -> bool {
    if !task_class_requires_write_scope(task_class) {
        return true;
    }
    !matches!(
        write_scope.trim().to_ascii_lowercase().as_str(),
        "" | "none" | "read-only" | "read_only" | "readorreview" | "read_or_review"
    )
}

#[derive(Clone)]
struct ProfileCandidate {
    role: serde_json::Value,
    profile: serde_json::Value,
    rate: u64,
    effective_score: u64,
    lifecycle_state: String,
    quality_rank: u8,
    reasoning_rank: u8,
    supports_runtime_role: bool,
    supports_task_class: bool,
    readiness_status: String,
}

pub(crate) fn dispatch_alias_row<'a>(
    compiled_bundle: &'a serde_json::Value,
    alias_id: &str,
) -> Option<&'a serde_json::Value> {
    carrier_runtime_section(compiled_bundle)["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["role_id"].as_str() == Some(alias_id))
}

pub(crate) fn dispatch_alias_runtime_roles(alias: &serde_json::Value) -> Vec<String> {
    let mut runtime_roles = alias["runtime_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if runtime_roles.is_empty() {
        if let Some(default_runtime_role) = alias["default_runtime_role"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            runtime_roles.push(default_runtime_role.to_string());
        }
    }
    runtime_roles
}

pub(crate) fn build_runtime_assignment_from_dispatch_alias(
    compiled_bundle: &serde_json::Value,
    alias_id: &str,
    fallback_task_class: &str,
) -> serde_json::Value {
    let Some(alias) = dispatch_alias_row(compiled_bundle, alias_id) else {
        return serde_json::json!({
            "enabled": false,
            "reason": "dispatch_alias_missing",
            "dispatch_alias_id": alias_id,
            "task_class": fallback_task_class,
        });
    };
    let runtime_role = dispatch_alias_runtime_roles(alias).into_iter().next();
    let Some(runtime_role) = runtime_role else {
        return serde_json::json!({
            "enabled": false,
            "reason": "dispatch_alias_runtime_role_missing",
            "dispatch_alias_id": alias_id,
            "task_class": fallback_task_class,
        });
    };
    let task_class = alias["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .find(|value| !value.is_empty())
        .unwrap_or(fallback_task_class)
        .to_string();
    let mut assignment = build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        alias_id,
        &task_class,
        &runtime_role,
    );
    if let Some(map) = assignment.as_object_mut() {
        map.insert(
            "dispatch_alias_id".to_string(),
            serde_json::Value::String(alias_id.to_string()),
        );
        map.insert(
            "dispatch_alias_runtime_role".to_string(),
            serde_json::Value::String(runtime_role),
        );
        map.insert(
            "dispatch_alias_task_class".to_string(),
            serde_json::Value::String(task_class),
        );
        map.insert(
            "dispatch_alias_description".to_string(),
            alias
                .get("description")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "preferred_carrier_tier".to_string(),
            alias
                .get("carrier_tier")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "developer_instructions".to_string(),
            alias
                .get("developer_instructions")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
    }
    assignment
}

pub(crate) fn resolve_dispatch_alias_id(
    compiled_bundle: &serde_json::Value,
    preferred_alias_id: &str,
    task_class: &str,
) -> Option<String> {
    if !preferred_alias_id.is_empty()
        && dispatch_alias_row(compiled_bundle, preferred_alias_id).is_some()
    {
        return Some(preferred_alias_id.to_string());
    }
    let runtime_role = runtime_role_for_task_class(task_class);
    carrier_runtime_section(compiled_bundle)["dispatch_aliases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|alias| {
            let alias_runtime_roles = dispatch_alias_runtime_roles(alias);
            (!alias_runtime_roles.is_empty()
                && alias_runtime_roles
                    .iter()
                    .any(|value| value == runtime_role))
                && role_supports_task_class(alias, task_class)
        })
        .and_then(|alias| alias["role_id"].as_str().map(str::to_string))
}

pub(crate) fn build_runtime_assignment_from_resolved_constraints(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
) -> serde_json::Value {
    let carrier_runtime = carrier_runtime_section(compiled_bundle);
    let Some(roles) = carrier_runtime["roles"].as_array() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing"
        });
    };
    if roles.is_empty() {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing"
        });
    }

    let demotion_score = json_u64(json_lookup(
        &carrier_runtime["worker_strategy"],
        &["selection_policy", "demotion_score"],
    ))
    .unwrap_or(crate::carrier_runtime_metadata::DEFAULT_DEMOTION_SCORE);
    let selection_strategy = selection_strategy(carrier_runtime);
    let free_profiles_allowed = free_profiles_allowed(carrier_runtime);
    let quality_floor = quality_floor_for_runtime_role(carrier_runtime, execution_runtime_role);
    let reasoning_floor = reasoning_floor_for_task_class(carrier_runtime, task_class);
    let mut rejected_candidates = Vec::new();
    let mut candidates = roles
        .iter()
        .flat_map(|role| {
            let role_id = role["role_id"].as_str().unwrap_or_default();
            let strategy = &carrier_runtime["worker_strategy"]["agents"][role_id];
            let effective_score =
                json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
            let lifecycle_state = strategy["lifecycle_state"]
                .as_str()
                .unwrap_or("probation")
                .to_string();
            crate::model_profile_contract::model_profiles_from_json_row(role)
                .into_iter()
                .map(move |profile| {
                    let rate = profile["normalized_cost_units"]
                        .as_u64()
                        .or_else(|| role["normalized_cost_units"].as_u64())
                        .or_else(|| role["rate"].as_u64())
                        .unwrap_or(0);
                    let reasoning_effort = profile["reasoning_effort"]
                        .as_str()
                        .or_else(|| role["model_reasoning_effort"].as_str())
                        .unwrap_or_default();
                    let quality_tier = profile["quality_tier"]
                        .as_str()
                        .or_else(|| role["quality_tier"].as_str())
                        .unwrap_or_default();
                    let readiness_status = profile_readiness_status(&profile);
                    ProfileCandidate {
                        supports_runtime_role: profile_supports_runtime_role(
                            role,
                            &profile,
                            execution_runtime_role,
                        ),
                        supports_task_class: profile_supports_task_class(role, &profile, task_class),
                        rate,
                        effective_score,
                        lifecycle_state: lifecycle_state.clone(),
                        quality_rank: quality_tier_rank(quality_tier),
                        reasoning_rank: reasoning_effort_rank(reasoning_effort),
                        readiness_status,
                        role: role.clone(),
                        profile,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let has_exact_match = candidates
        .iter()
        .any(|candidate| candidate.supports_runtime_role && candidate.supports_task_class);
    if !has_exact_match {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_carrier_declares_runtime_role_and_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "rejected_candidates": rejected_candidates
        });
    }

    candidates.retain(|candidate| {
        let mut reasons = Vec::new();
        if !candidate.supports_runtime_role {
            reasons.push("runtime_role_not_supported".to_string());
        }
        if !candidate.supports_task_class {
            reasons.push("task_class_not_supported".to_string());
        }
        if candidate.effective_score < demotion_score || candidate.lifecycle_state == "retired" {
            reasons.push("carrier_score_or_lifecycle_blocked".to_string());
        }
        if candidate.rate == 0 && !free_profiles_allowed {
            reasons.push("zero_cost_profiles_disabled".to_string());
        }
        if candidate.readiness_status == "blocked" {
            reasons.push("profile_not_ready".to_string());
        }
        let write_scope = candidate.profile["write_scope"]
            .as_str()
            .or_else(|| candidate.role["write_scope"].as_str())
            .unwrap_or_default();
        if !write_scope_allows_task_class(write_scope, task_class) {
            reasons.push("write_scope_inadmissible_for_task_class".to_string());
        }
        if let Some(floor) = quality_floor.as_deref() {
            if candidate.quality_rank < quality_tier_rank(floor) {
                reasons.push("quality_floor_not_met".to_string());
            }
        }
        if let Some(floor) = reasoning_floor.as_deref() {
            if candidate.reasoning_rank < reasoning_effort_rank(floor) {
                reasons.push("reasoning_floor_not_met".to_string());
            }
        }
        if reasons.is_empty() {
            true
        } else {
            rejected_candidates.push(serde_json::json!({
                "carrier_id": candidate.role["role_id"],
                "carrier_tier": candidate.role["tier"],
                "model_profile_id": candidate.profile["profile_id"],
                "model_ref": candidate.profile["model_ref"],
                "reasons": reasons,
                "reason": reasons.first().cloned().unwrap_or_default(),
            }));
            false
        }
    });

    if selection_strategy == "quality_first" || selection_strategy == "risk_aware" {
        candidates.sort_by(|left, right| {
            right
                .quality_rank
                .cmp(&left.quality_rank)
                .then_with(|| right.reasoning_rank.cmp(&left.reasoning_rank))
                .then_with(|| left.rate.cmp(&right.rate))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
        });
    } else if selection_strategy == "free_first_with_quality_floor" {
        candidates.sort_by(|left, right| {
            (left.rate != 0)
                .cmp(&(right.rate != 0))
                .then_with(|| left.rate.cmp(&right.rate))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
        });
    } else {
        candidates.sort_by(|left, right| {
            left.rate
                .cmp(&right.rate)
                .then_with(|| right.quality_rank.cmp(&left.quality_rank))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
                .then_with(|| right.reasoning_rank.cmp(&left.reasoning_rank))
        });
    }

    let Some(selected_candidate) = candidates.first() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_carrier_satisfies_runtime_role_or_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_strategy": selection_strategy,
            "rejected_candidates": rejected_candidates,
        });
    };

    let selected_role = &selected_candidate.role;
    let selected_profile = &selected_candidate.profile;
    let tier = selected_role["tier"].as_str().unwrap_or_default();
    let rate = selected_candidate.rate;
    let complexity_multiplier = task_complexity_multiplier(task_class);
    let effective_score = selected_candidate.effective_score;
    let lifecycle_state = selected_candidate.lifecycle_state.as_str();
    let selection_rule = selection_rule_for_runtime(carrier_runtime);
    let rationale = vec![
        format!("task_class={task_class}"),
        format!("conversation_role={conversation_role}"),
        format!("execution_runtime_role={execution_runtime_role}"),
        format!("selected_tier={tier}"),
        format!(
            "selected_model_profile={}",
            selected_profile["profile_id"].as_str().unwrap_or_default()
        ),
        format!("effective_score={effective_score}"),
        format!("lifecycle_state={lifecycle_state}"),
        format!("selection_rule={selection_rule}"),
    ];

    serde_json::json!({
        "enabled": true,
        "task_class": task_class,
        "runtime_role": execution_runtime_role,
        "conversation_role": conversation_role,
        "activation_agent_type": selected_role["role_id"],
        "activation_runtime_role": execution_runtime_role,
        "selected_agent_id": selected_role["role_id"],
        "selected_carrier_id": selected_role["role_id"],
        "selected_backend_id": selected_role["role_id"],
        "selected_carrier_agent_id": selected_role["role_id"],
        "selected_tier": selected_role["tier"],
        "selected_carrier_tier": selected_role["tier"],
        "selected_runtime_role": execution_runtime_role,
        "selected_model_profile_id": selected_profile["profile_id"],
        "selected_model_ref": selected_profile["model_ref"],
        "selected_model_provider": selected_profile["provider"],
        "selected_reasoning_effort": selected_profile["reasoning_effort"],
        "selected_plan_mode_reasoning_effort": selected_profile["plan_mode_reasoning_effort"],
        "selected_sandbox_mode": selected_profile["sandbox_mode"],
        "selected_quality_tier": selected_profile["quality_tier"],
        "selected_speed_tier": selected_profile["speed_tier"],
        "selected_model_profile_readiness_status": selected_candidate.readiness_status,
        "tier_default_runtime_role": selected_role["default_runtime_role"],
        "reasoning_band": selected_role["reasoning_band"],
        "model": selected_profile["model_ref"],
        "model_provider": selected_profile["provider"],
        "model_reasoning_effort": selected_profile["reasoning_effort"],
        "sandbox_mode": selected_profile["sandbox_mode"],
        "rate": rate,
        "normalized_cost_units": rate,
        "estimated_task_price_units": rate * complexity_multiplier,
        "complexity_multiplier": complexity_multiplier,
        "effective_score": effective_score,
        "lifecycle_state": lifecycle_state,
        "strategy_store": carrier_runtime["worker_strategy"]["store_path"],
        "scorecards_store": carrier_runtime["worker_strategy"]["scorecards_path"],
        "selection_strategy": selection_strategy,
        "selection_rule": selection_rule,
        "rejected_candidates": rejected_candidates,
        "rationale": rationale
    })
}

pub(crate) fn build_runtime_assignment(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let task_class = infer_runtime_task_class(selection, requires_design_gate);
    let execution_runtime_role =
        infer_execution_runtime_role(selection, &task_class, requires_design_gate);
    build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        &selection.selected_role,
        &task_class,
        &execution_runtime_role,
    )
}

#[cfg(test)]
mod tests {
    use super::build_runtime_assignment_from_resolved_constraints;

    fn compiled_bundle_with_roles(roles: Vec<serde_json::Value>) -> serde_json::Value {
        let worker_agents = roles
            .iter()
            .filter_map(|role| {
                role["role_id"].as_str().map(|role_id| {
                    (
                        role_id.to_string(),
                        serde_json::json!({
                            "effective_score": 70,
                            "lifecycle_state": "active",
                        }),
                    )
                })
            })
            .collect::<serde_json::Map<_, _>>();
        serde_json::json!({
            "carrier_runtime": {
                "roles": roles,
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier",
                        "demotion_score": 45
                    },
                    "agents": worker_agents,
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "free_profiles_allowed": true,
                    "quality_floor_by_runtime_role": {
                        "worker": "medium",
                        "coach": "medium",
                        "verifier": "high"
                    },
                    "reasoning_floor_by_task_class": {
                        "implementation": "low",
                        "review": "medium"
                    }
                }
            }
        })
    }

    #[test]
    fn free_external_model_profile_with_cost_zero_is_not_dropped() {
        let compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "codex_gpt54_medium_write",
                "model_profiles": {
                    "codex_gpt54_medium_write": {
                        "profile_id": "codex_gpt54_medium_write",
                        "model_ref": "gpt-5.4",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "workspace-write",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "opencode_cli",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "opencode_minimax_free_review",
                "model_profiles": {
                    "opencode_minimax_free_review": {
                        "profile_id": "opencode_minimax_free_review",
                        "model_ref": "opencode/minimax-m2.5-free",
                        "provider": "opencode",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 0,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "opencode_cli");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "opencode_minimax_free_review"
        );
        assert_eq!(
            assignment["selected_model_ref"],
            "opencode/minimax-m2.5-free"
        );
        assert_eq!(assignment["selected_reasoning_effort"], "medium");
        assert_eq!(assignment["selection_strategy"], "balanced_cost_quality");
        assert_eq!(
            assignment["selection_rule"],
            "role_task_then_readiness_then_score_then_cost_quality"
        );
        assert_eq!(assignment["normalized_cost_units"], 0);
    }

    #[test]
    fn write_scope_none_external_profile_is_rejected_for_implementation() {
        let compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "junior",
                "tier": "junior",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.4",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "plan_mode_reasoning_effort": "medium",
                        "sandbox_mode": "workspace-write",
                        "normalized_cost_units": 1,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "opencode_cli",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "opencode_minimax_free_review",
                "model_profiles": {
                    "opencode_minimax_free_review": {
                        "profile_id": "opencode_minimax_free_review",
                        "model_ref": "opencode/minimax-m2.5-free",
                        "provider": "opencode",
                        "reasoning_effort": "provider_default",
                        "normalized_cost_units": 0,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "junior");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "codex_gpt54_low_write"
        );
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "opencode_cli"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str()
                                    == Some("write_scope_inadmissible_for_task_class")
                            })
                })
        );
    }
}
