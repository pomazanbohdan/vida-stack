use crate::semantic_routing_features::{
    SemanticRoutingFeatureInput, SemanticScoreInputs, extract_semantic_routing_features,
    score_semantic_route,
};
use crate::{
    RuntimeConsumptionLaneSelection, carrier_runtime_section,
    declared_task_class_supports_requested, infer_execution_runtime_role, infer_runtime_task_class,
    json_lookup, json_u64, role_supports_task_class, runtime_role_for_task_class,
    task_complexity_multiplier,
};

fn selection_strategy(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["model_selection"]["default_strategy"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("balanced_cost_quality")
        .to_string()
}

fn selection_strategy_supported(selection_strategy: &str) -> bool {
    matches!(
        selection_strategy,
        "balanced_cost_quality" | "quality_first" | "risk_aware" | "free_first_with_quality_floor"
    )
}

fn semantic_routing_enabled(compiled_bundle: &serde_json::Value) -> bool {
    json_lookup(
        &compiled_bundle["agent_system"]["semantic_routing"],
        &["enabled"],
    )
    .and_then(serde_json::Value::as_bool)
    .unwrap_or(false)
}

fn configured_model_selection_score(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    score_kind: &str,
    raw: &str,
) -> Option<f64> {
    let raw = raw.trim();
    (!raw.is_empty())
        .then(|| {
            model_selection_policy_value(
                compiled_bundle,
                carrier_runtime,
                &["semantic_scores", score_kind, raw],
            )
        })
        .flatten()
        .and_then(serde_json::Value::as_f64)
}

fn configured_model_selection_rank(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    rank_kind: &str,
    raw: &str,
) -> Option<u8> {
    let raw = raw.trim();
    (!raw.is_empty())
        .then(|| {
            model_selection_policy_value(
                compiled_bundle,
                carrier_runtime,
                &["ordinal_ranks", rank_kind, raw],
            )
        })
        .flatten()
        .and_then(|value| json_u64(Some(value)))
        .and_then(|value| u8::try_from(value).ok())
}

fn model_selection_ranking_policy_is_complete(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
) -> bool {
    ["reasoning_effort", "quality_tier", "speed_tier"]
        .into_iter()
        .all(|kind| {
            model_selection_policy_value(
                compiled_bundle,
                carrier_runtime,
                &["semantic_scores", kind],
            )
            .is_some_and(|value| {
                value.is_object() && !value.as_object().is_some_and(|map| map.is_empty())
            })
        })
        && ["reasoning_effort", "quality_tier"]
            .into_iter()
            .all(|kind| {
                model_selection_policy_value(
                    compiled_bundle,
                    carrier_runtime,
                    &["ordinal_ranks", kind],
                )
                .is_some_and(|value| {
                    value.is_object() && !value.as_object().is_some_and(|map| map.is_empty())
                })
            })
        && model_selection_policy_value(
            compiled_bundle,
            carrier_runtime,
            &["missing_reasoning_effort_policy", "mode"],
        )
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn insert_semantic_routing_diagnostics(
    assignment: &mut serde_json::Value,
    compiled_bundle: &serde_json::Value,
    request_text: &str,
    task_class: &str,
    runtime_role: &str,
    route_key: &str,
) {
    if assignment["enabled"].as_bool() != Some(true) {
        return;
    }
    let feature_vector = extract_semantic_routing_features(&SemanticRoutingFeatureInput {
        request_text,
        task_title: None,
        task_description: None,
        runtime_role: Some(runtime_role),
        task_class: Some(task_class),
        route_key: Some(route_key),
        max_text_chars: 12_000,
    });
    let selected_quality_tier = assignment["selected_quality_tier"]
        .as_str()
        .unwrap_or_default();
    let selected_reasoning_effort = assignment["selected_reasoning_effort"]
        .as_str()
        .unwrap_or_default();
    let selected_speed_tier = assignment["selected_speed_tier"]
        .as_str()
        .unwrap_or_default();
    let selected_over_budget = assignment["selected_over_budget"]
        .as_bool()
        .unwrap_or(false);
    let selected_readiness = assignment["selected_model_profile_readiness_status"]
        .as_str()
        .unwrap_or_default();
    let carrier_runtime = carrier_runtime_section(compiled_bundle);
    let score_breakdown = score_semantic_route(
        &feature_vector,
        SemanticScoreInputs {
            quality: configured_model_selection_score(
                compiled_bundle,
                carrier_runtime,
                "quality_tier",
                selected_quality_tier,
            )
            .unwrap_or_default(),
            reasoning: configured_model_selection_score(
                compiled_bundle,
                carrier_runtime,
                "reasoning_effort",
                selected_reasoning_effort,
            )
            .unwrap_or_default(),
            reliability: assignment["effective_score"].as_u64().unwrap_or(50) as f64,
            speed: configured_model_selection_score(
                compiled_bundle,
                carrier_runtime,
                "speed_tier",
                selected_speed_tier,
            )
            .unwrap_or_default(),
            cost: assignment["normalized_cost_units"].as_u64().unwrap_or(0) as f64,
            domain_fit: if feature_vector.detected_domain == task_class {
                85.0
            } else if feature_vector
                .matched_terms
                .domain
                .iter()
                .any(|term| task_class.contains(term))
            {
                60.0
            } else {
                25.0
            },
            complexity_mismatch_penalty: 0.0,
            lifecycle_penalty: 0.0,
            over_budget_penalty: if selected_over_budget { 25.0 } else { 0.0 },
            readiness_penalty: if selected_readiness == "blocked" {
                25.0
            } else {
                0.0
            },
        },
    );

    if let Some(map) = assignment.as_object_mut() {
        map.insert(
            "semantic_routing".to_string(),
            serde_json::json!({
                "enabled": true,
                "mode": "deterministic_rules",
                "feature_source": feature_vector.feature_source,
                "complexity_score": feature_vector.complexity_score,
                "complexity_band": feature_vector.complexity_band,
                "detected_domain": feature_vector.detected_domain,
                "guardrail_risk": serde_json::to_value(&feature_vector.guardrail_risk).unwrap_or(serde_json::Value::Null),
                "feature_vector": serde_json::to_value(&feature_vector).unwrap_or(serde_json::Value::Null),
                "score_applied": true,
                "strategy": "semantic_balanced_cost_quality",
                "config_source_path": "agent_system.model_selection.semantic_scores",
                "advisory_only": true
            }),
        );
        map.insert(
            "semantic_route_score".to_string(),
            serde_json::json!(score_breakdown.semantic_route_score),
        );
        map.insert(
            "semantic_score_breakdown".to_string(),
            serde_json::to_value(score_breakdown).unwrap_or(serde_json::Value::Null),
        );
    }
}

fn model_selection_enabled(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
) -> bool {
    json_lookup(
        &compiled_bundle["agent_system"]["model_selection"],
        &["enabled"],
    )
    .or_else(|| json_lookup(&carrier_runtime["model_selection"], &["enabled"]))
    .and_then(serde_json::Value::as_bool)
    .unwrap_or(true)
}

fn candidate_scope(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
) -> String {
    json_lookup(
        &compiled_bundle["agent_system"]["model_selection"],
        &["candidate_scope"],
    )
    .or_else(|| json_lookup(&carrier_runtime["model_selection"], &["candidate_scope"]))
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or("unified_carrier_model_profiles")
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

fn model_selection_policy_value<'a>(
    compiled_bundle: &'a serde_json::Value,
    carrier_runtime: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    json_lookup(&carrier_runtime["model_selection"], path)
        .or_else(|| json_lookup(&compiled_bundle["agent_system"]["model_selection"], path))
}

fn budget_policy_enforces_max_budget_units(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
) -> bool {
    model_selection_policy_value(
        compiled_bundle,
        carrier_runtime,
        &["budget_policy", "enforce_max_budget_units"],
    )
    .and_then(serde_json::Value::as_bool)
    .unwrap_or(false)
}

fn budget_policy_allows_over_budget_escalation(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
) -> bool {
    model_selection_policy_value(
        compiled_bundle,
        carrier_runtime,
        &["budget_policy", "allow_escalation_over_budget_with_blocker"],
    )
    .and_then(serde_json::Value::as_bool)
    .unwrap_or(false)
}

fn route_budget_policy(
    compiled_bundle: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<String> {
    json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["budget_policy"],
    )
    .or_else(|| {
        json_lookup(
            &compiled_bundle["agent_system"]["routing"][conversation_role],
            &["budget_policy"],
        )
    })
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(|value| value.to_ascii_lowercase())
}

fn route_budget_policy_source_path(
    compiled_bundle: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<String> {
    if json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["budget_policy"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some(format!("agent_system.routing.{route_key}.budget_policy"));
    }
    if json_lookup(
        &compiled_bundle["agent_system"]["routing"][conversation_role],
        &["budget_policy"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some(format!(
            "agent_system.routing.{conversation_role}.budget_policy"
        ));
    }
    if json_lookup(
        &compiled_bundle["agent_system"]["model_selection"],
        &["budget_policy"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some("agent_system.model_selection.budget_policy".to_string());
    }
    None
}

fn assignment_budget_policy(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> String {
    route_budget_policy(compiled_bundle, route_key, conversation_role)
        .or_else(|| {
            model_selection_policy_value(compiled_bundle, carrier_runtime, &["budget_policy"])
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
        })
        .unwrap_or_else(|| {
            if budget_policy_enforces_max_budget_units(compiled_bundle, carrier_runtime) {
                "strict".to_string()
            } else {
                "informational".to_string()
            }
        })
}

fn assignment_budget_policy_source_path(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
    budget_policy: &str,
) -> String {
    route_budget_policy_source_path(compiled_bundle, route_key, conversation_role)
        .or_else(|| {
            model_selection_policy_value(compiled_bundle, carrier_runtime, &["budget_policy"])
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|_| "carrier_runtime.model_selection.budget_policy".to_string())
        })
        .unwrap_or_else(|| {
            if budget_policy == "strict" {
                "derived_from_model_selection.budget_policy.enforce_max_budget_units".to_string()
            } else {
                "default:informational".to_string()
            }
        })
}

fn assignment_max_budget_units(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<u64> {
    json_u64(json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["max_budget_units"],
    ))
    .or_else(|| {
        json_u64(json_lookup(
            &compiled_bundle["agent_system"]["routing"][conversation_role],
            &["max_budget_units"],
        ))
    })
    .or_else(|| {
        json_u64(json_lookup(
            &compiled_bundle["agent_system"]["routing"]["default"],
            &["max_budget_units"],
        ))
    })
    .or_else(|| {
        json_u64(json_lookup(
            &compiled_bundle["agent_system"]["routing"][conversation_role],
            &["budget_policy", "max_budget_units"],
        ))
    })
    .or_else(|| {
        json_u64(model_selection_policy_value(
            compiled_bundle,
            carrier_runtime,
            &["budget_policy", "max_budget_units"],
        ))
    })
    .or_else(|| {
        json_u64(model_selection_policy_value(
            compiled_bundle,
            carrier_runtime,
            &["max_budget_units"],
        ))
    })
}

fn assignment_max_budget_units_source_path(
    compiled_bundle: &serde_json::Value,
    carrier_runtime: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<String> {
    if json_u64(json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["max_budget_units"],
    ))
    .is_some()
    {
        return Some(format!("agent_system.routing.{route_key}.max_budget_units"));
    }
    if json_u64(json_lookup(
        &compiled_bundle["agent_system"]["routing"][conversation_role],
        &["max_budget_units"],
    ))
    .is_some()
    {
        return Some(format!(
            "agent_system.routing.{conversation_role}.max_budget_units"
        ));
    }
    if json_u64(json_lookup(
        &compiled_bundle["agent_system"]["routing"]["default"],
        &["max_budget_units"],
    ))
    .is_some()
    {
        return Some("agent_system.routing.default.max_budget_units".to_string());
    }
    if json_u64(json_lookup(
        &compiled_bundle["agent_system"]["routing"][conversation_role],
        &["budget_policy", "max_budget_units"],
    ))
    .is_some()
    {
        return Some(format!(
            "agent_system.routing.{conversation_role}.budget_policy.max_budget_units"
        ));
    }
    if json_u64(model_selection_policy_value(
        compiled_bundle,
        carrier_runtime,
        &["budget_policy", "max_budget_units"],
    ))
    .is_some()
    {
        return Some("model_selection.budget_policy.max_budget_units".to_string());
    }
    if json_u64(model_selection_policy_value(
        compiled_bundle,
        carrier_runtime,
        &["max_budget_units"],
    ))
    .is_some()
    {
        return Some("model_selection.max_budget_units".to_string());
    }
    None
}

fn route_profile_mapping<'a>(
    compiled_bundle: &'a serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<&'a serde_json::Value> {
    json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["profiles"],
    )
    .or_else(|| {
        json_lookup(
            &compiled_bundle["agent_system"]["routing"][conversation_role],
            &["profiles"],
        )
    })
}

fn route_profile_mapping_source_path(
    compiled_bundle: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
    carrier_id: &str,
) -> Option<String> {
    let route_profiles = json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["profiles"],
    );
    if let Some(profiles) = route_profiles {
        if profiles
            .get(carrier_id)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Some(format!(
                "agent_system.routing.{route_key}.profiles.{carrier_id}"
            ));
        }
        if profiles
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Some(format!("agent_system.routing.{route_key}.profiles"));
        }
    }
    let role_profiles = json_lookup(
        &compiled_bundle["agent_system"]["routing"][conversation_role],
        &["profiles"],
    );
    if let Some(profiles) = role_profiles {
        if profiles
            .get(carrier_id)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Some(format!(
                "agent_system.routing.{conversation_role}.profiles.{carrier_id}"
            ));
        }
        if profiles
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Some(format!("agent_system.routing.{conversation_role}.profiles"));
        }
    }
    None
}

fn route_executor_backend(
    compiled_bundle: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<String> {
    json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["executor_backend"],
    )
    .or_else(|| {
        json_lookup(
            &compiled_bundle["agent_system"]["routing"][conversation_role],
            &["executor_backend"],
        )
    })
    .or_else(|| {
        json_lookup(
            &compiled_bundle["agent_system"]["routing"]["default"],
            &["executor_backend"],
        )
    })
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(str::to_string)
}

fn route_executor_backend_source_path(
    compiled_bundle: &serde_json::Value,
    route_key: &str,
    conversation_role: &str,
) -> Option<String> {
    if json_lookup(
        &compiled_bundle["agent_system"]["routing"][route_key],
        &["executor_backend"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some(format!("agent_system.routing.{route_key}.executor_backend"));
    }
    if json_lookup(
        &compiled_bundle["agent_system"]["routing"][conversation_role],
        &["executor_backend"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some(format!(
            "agent_system.routing.{conversation_role}.executor_backend"
        ));
    }
    if json_lookup(
        &compiled_bundle["agent_system"]["routing"]["default"],
        &["executor_backend"],
    )
    .and_then(serde_json::Value::as_str)
    .map(str::trim)
    .is_some_and(|value| !value.is_empty())
    {
        return Some("agent_system.routing.default.executor_backend".to_string());
    }
    None
}

fn mapped_profile_for_carrier(
    route_profiles: Option<&serde_json::Value>,
    carrier_id: &str,
) -> Option<String> {
    let route_profiles = route_profiles?;
    route_profiles
        .get(carrier_id)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            route_profiles
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn selection_rule_for_runtime(carrier_runtime: &serde_json::Value) -> String {
    carrier_runtime["model_selection"]["selection_rule"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("role_task_then_readiness_then_score_then_cost_quality")
        .to_string()
}

fn profile_runtime_roles(role: &serde_json::Value, profile: &serde_json::Value) -> Vec<String> {
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

fn profile_task_classes(role: &serde_json::Value, profile: &serde_json::Value) -> Vec<String> {
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

fn non_empty_string_field<'a>(value: &'a serde_json::Value) -> Option<&'a str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn candidate_metadata_source_path(role_id: &str, profile_id: &str, field: &str) -> Option<String> {
    if role_id.is_empty() || profile_id.is_empty() {
        return None;
    }
    if field.trim().is_empty() {
        return None;
    }
    Some(format!(
        "carrier_runtime.roles[{role_id}].model_profiles.{profile_id}.{field}"
    ))
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
    task_classes.is_empty()
        || task_classes
            .iter()
            .any(|declared| declared_task_class_supports_requested(declared, task_class))
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

fn pricing_metadata_for_profile(
    compiled_bundle: &serde_json::Value,
    role_id: &str,
    profile_id: &str,
    profile: &serde_json::Value,
    provider: Option<&str>,
) -> (serde_json::Value, Option<String>) {
    if let Some(pricing) = profile.get("pricing").filter(|value| !value.is_null()) {
        return (
            pricing.clone(),
            candidate_metadata_source_path(role_id, profile_id, "pricing"),
        );
    }
    if let Some(provider) = provider {
        let provider_pricing = json_lookup(
            &compiled_bundle["agent_system"]["pricing"]["providers"],
            &[provider],
        );
        if provider_pricing.is_some() {
            return (
                provider_pricing.cloned().unwrap_or(serde_json::Value::Null),
                Some(format!("agent_system.pricing.providers[{provider}]")),
            );
        }
    }
    if let Some(default_pricing) = json_lookup(
        &compiled_bundle["agent_system"]["pricing"],
        &["model_profile_defaults", "pricing"],
    ) {
        return (
            default_pricing.clone(),
            Some("agent_system.pricing.model_profile_defaults.pricing".to_string()),
        );
    }
    (serde_json::Value::Null, None)
}

fn pricing_freshness_status_and_reject_reasons(
    pricing: &serde_json::Value,
) -> (serde_json::Value, String, Vec<String>) {
    let freshness = pricing
        .get("freshness")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let freshness_status = if freshness.is_null() {
        "missing".to_string()
    } else if freshness.get("max_age_days").is_none() {
        "missing".to_string()
    } else if freshness
        .get("max_age_days")
        .and_then(serde_json::Value::as_u64)
        == Some(0)
    {
        "stale".to_string()
    } else {
        "ready".to_string()
    };
    let required = freshness
        .get("required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let enforced = freshness
        .get("enforced")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let diagnostic_only = freshness
        .get("diagnostic_only")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let should_enforce = (required || enforced) && !diagnostic_only;
    let reasons = if should_enforce {
        match freshness_status.as_str() {
            "missing" => vec!["model_price_freshness_policy_incomplete".to_string()],
            "stale" => vec!["model_price_freshness_stale".to_string()],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    (freshness, freshness_status, reasons)
}

fn configured_executor_backend_class(backend_class: &str) -> bool {
    matches!(
        backend_class.trim(),
        "external_cli" | "service" | "service_executor"
    )
}

fn service_executor_backend_class(backend_class: &str) -> bool {
    matches!(backend_class.trim(), "service" | "service_executor")
}

fn candidate_backend_class(candidate: &ProfileCandidate) -> String {
    candidate.role["backend_class"]
        .as_str()
        .or_else(|| candidate.role["subagent_backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn configured_backend_readiness_guard_active(readiness: Option<&serde_json::Value>) -> bool {
    readiness
        .and_then(|readiness| readiness.pointer("/write_scope_guard/pre_write_enforcement"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && readiness
            .and_then(|readiness| readiness.pointer("/write_scope_guard/status"))
            .and_then(serde_json::Value::as_str)
            == Some("active")
}

fn service_executor_write_candidate_ready(
    backend_class: &str,
    task_class: &str,
    readiness: Option<&serde_json::Value>,
) -> bool {
    if !service_executor_backend_class(backend_class)
        || !task_class_requires_write_scope(task_class)
    {
        return true;
    }
    readiness.is_some_and(|readiness| {
        readiness
            .get("blocked")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
            && configured_backend_readiness_guard_active(Some(readiness))
    })
}

fn legacy_external_backend_readiness_alias(
    readiness: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(readiness) = readiness else {
        return serde_json::Value::Null;
    };
    let mut alias = readiness.clone();
    if let Some(map) = alias.as_object_mut() {
        if map.get("status").and_then(serde_json::Value::as_str)
            == Some("configured_backend_dispatch_blocked")
        {
            map.insert(
                "status".to_string(),
                serde_json::Value::String("external_backend_dispatch_blocked".to_string()),
            );
        }
    }
    alias
}

fn configured_backend_dispatch_blocker(
    backend_id: &str,
    backend_class: &str,
    backend_entry: &serde_yaml::Value,
) -> Option<String> {
    if backend_class == "external_cli" {
        return crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            backend_id,
            backend_entry,
        );
    }
    if crate::yaml_lookup(backend_entry, &["enabled"]).is_some()
        && !crate::yaml_bool(crate::yaml_lookup(backend_entry, &["enabled"]), false)
    {
        return Some(format!(
            "configured backend `{backend_id}` is disabled; service executor dispatch is forbidden"
        ));
    }
    None
}

fn configured_backend_readiness_verdict_for_candidate(
    compiled_bundle: &serde_json::Value,
    role: &serde_json::Value,
    profile: &serde_json::Value,
) -> Option<serde_json::Value> {
    let (backend_id, backend_class, backend_entry) =
        configured_backend_entry_for_role(compiled_bundle, role)?;
    let profile_id = profile["profile_id"].as_str().map(str::trim);
    if let Some(blocker_reason) =
        configured_backend_dispatch_blocker(&backend_id, &backend_class, &backend_entry)
    {
        let selected_model_profile = profile_id
            .map(|profile_id| serde_json::Value::String(profile_id.to_string()))
            .unwrap_or(serde_json::Value::Null);
        return Some(serde_json::json!({
            "backend_id": backend_id,
            "backend_class": backend_class,
            "status": "configured_backend_dispatch_blocked",
            "blocked": true,
            "blocker_code": "configured_backend_dispatch_failed",
            "blocker_reason": blocker_reason,
            "selected_model_profile": selected_model_profile,
            "raw_provider_dispatch_forbidden": service_executor_backend_class(&backend_class),
            "next_actions": [
                format!("Enable and repair external backend `{backend_id}` in `vida.config.yaml`, or reroute this lane to a receipt-backed backend before dispatch.")
            ],
        }));
    }
    let mut verdict =
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            &backend_id,
            &backend_entry,
            profile_id,
        );
    if let Some(map) = verdict.as_object_mut() {
        map.insert(
            "backend_class".to_string(),
            serde_json::Value::String(backend_class.clone()),
        );
        map.insert(
            "raw_provider_dispatch_forbidden".to_string(),
            serde_json::Value::Bool(service_executor_backend_class(&backend_class)),
        );
        if service_executor_backend_class(&backend_class)
            && map.get("status").and_then(serde_json::Value::as_str)
                == Some("external_cli_command_not_found")
        {
            map.insert(
                "status".to_string(),
                serde_json::Value::String("service_executor_command_not_found".to_string()),
            );
        }
    }
    Some(verdict)
}

fn configured_backend_entry_for_role(
    compiled_bundle: &serde_json::Value,
    role: &serde_json::Value,
) -> Option<(String, String, serde_yaml::Value)> {
    let backend_id = role["role_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let backend_entry_json =
        json_lookup(&compiled_bundle["agent_system"], &["subagents", backend_id])?;
    let backend_entry = serde_yaml::to_value(backend_entry_json).ok()?;
    let backend_class = role["backend_class"]
        .as_str()
        .or_else(|| role["subagent_backend_class"].as_str())
        .or_else(|| backend_entry_json["subagent_backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    if !configured_executor_backend_class(backend_class) {
        return None;
    }
    Some((
        backend_id.to_string(),
        backend_class.to_string(),
        backend_entry,
    ))
}

fn configured_backend_dispatch_blocker_verdict_for_candidate(
    compiled_bundle: &serde_json::Value,
    role: &serde_json::Value,
    profile: &serde_json::Value,
) -> Option<serde_json::Value> {
    let (backend_id, backend_class, backend_entry) =
        configured_backend_entry_for_role(compiled_bundle, role)?;
    let blocker_reason =
        configured_backend_dispatch_blocker(&backend_id, &backend_class, &backend_entry)?;
    let selected_model_profile = profile["profile_id"]
        .as_str()
        .map(str::trim)
        .filter(|profile_id| !profile_id.is_empty())
        .map(|profile_id| serde_json::Value::String(profile_id.to_string()))
        .unwrap_or(serde_json::Value::Null);
    Some(serde_json::json!({
        "backend_id": backend_id,
        "backend_class": backend_class,
        "status": "configured_backend_dispatch_blocked",
        "blocked": true,
        "blocker_code": "configured_backend_dispatch_failed",
        "blocker_reason": blocker_reason,
        "selected_model_profile": selected_model_profile,
        "raw_provider_dispatch_forbidden": service_executor_backend_class(&backend_class),
        "next_actions": [
            format!("Enable and repair external backend `{backend_id}` in `vida.config.yaml`, or reroute this lane to a receipt-backed backend before dispatch.")
        ],
    }))
}

fn task_class_requires_write_scope(task_class: &str) -> bool {
    matches!(
        task_class,
        "implementation"
            | "implementation_medium"
            | "delivery_task"
            | "execution_block"
            | "test_authoring"
            | "regression_test"
    )
}

fn write_scope_allows_task_class(
    write_scope: &str,
    task_class: &str,
    external_backend_readiness: Option<&serde_json::Value>,
) -> bool {
    if !task_class_requires_write_scope(task_class) {
        return true;
    }
    write_scope_allows_write_task_class(write_scope, external_backend_readiness)
}

fn write_scope_allows_write_task_class(
    write_scope: &str,
    external_backend_readiness: Option<&serde_json::Value>,
) -> bool {
    let normalized = write_scope.trim().to_ascii_lowercase();
    if write_scope_is_readonly(&normalized) {
        return false;
    }

    if write_scope_requires_live_guard(&normalized) {
        return external_backend_readiness
            .and_then(|readiness| readiness.pointer("/write_scope_guard/pre_write_enforcement"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && external_backend_readiness
                .and_then(|readiness| readiness.pointer("/write_scope_guard/status"))
                .and_then(serde_json::Value::as_str)
                == Some("active");
    }

    true
}

fn isolation_requires_readonly_scope(isolation: &str) -> bool {
    let normalized = isolation.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "readonly"
            | "read-only"
            | "read_only"
            | "internal_readonly_complete"
            | "external_readonly_complete"
            | "root_validate_only"
    )
}

fn write_scope_is_readonly(write_scope: &str) -> bool {
    let normalized = write_scope.trim().to_ascii_lowercase();
    normalized.is_empty()
        || matches!(
            normalized.as_str(),
            "none" | "read-only" | "read_only" | "readonly" | "readorreview"
        )
        || normalized.starts_with("read_or_")
}

fn isolation_allows_write_scope(isolation: &str, write_scope: &str) -> bool {
    !isolation_requires_readonly_scope(isolation) || write_scope_is_readonly(write_scope)
}

fn write_scope_requires_live_guard(write_scope: &str) -> bool {
    let normalized = write_scope.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "guard_required"
            | "guard-required"
            | "guard_required_owned_paths"
            | "guard-required-owned-paths"
            | "guard_required_packet_owned_paths"
            | "guard-required-packet-owned-paths"
    )
}

fn reasoning_control_mode(profile: &serde_json::Value) -> Option<&'static str> {
    let reasoning_control = &profile["reasoning_control"];
    match reasoning_control {
        serde_json::Value::Null => None,
        serde_json::Value::Object(entries) if entries.is_empty() => None,
        _ => Some("metadata_only"),
    }
}

fn first_rejected_metadata_reason(rejected_candidates: &[serde_json::Value]) -> Option<String> {
    let metadata_reasons = [
        "selected_model_profile_id_missing",
        "selected_model_ref_missing",
        "selected_rate_missing",
        "model_price_freshness_policy_incomplete",
        "model_price_freshness_stale",
    ];
    rejected_candidates.iter().find_map(|candidate| {
        candidate["reasons"].as_array().and_then(|reasons| {
            reasons
                .iter()
                .filter_map(serde_json::Value::as_str)
                .find(|reason| metadata_reasons.contains(reason))
                .map(|reason| reason.to_string())
        })
    })
}

#[derive(Clone)]
struct ProfileCandidate {
    role: serde_json::Value,
    profile: serde_json::Value,
    model_profile_id_source_path: Option<String>,
    model_ref_source_path: Option<String>,
    rate_source_path: Option<String>,
    rate_present: bool,
    rate: u64,
    pricing: serde_json::Value,
    pricing_source_path: Option<String>,
    pricing_freshness: serde_json::Value,
    pricing_freshness_source_path: Option<String>,
    pricing_freshness_status: String,
    pricing_rejection_reasons: Vec<String>,
    effective_score: u64,
    lifecycle_state: String,
    quality_rank: u8,
    reasoning_rank: u8,
    supports_runtime_role: bool,
    supports_task_class: bool,
    readiness_status: String,
    external_backend_readiness: Option<serde_json::Value>,
}

impl ProfileCandidate {
    fn over_budget(&self, max_budget_units: Option<u64>) -> bool {
        max_budget_units
            .map(|max_budget_units| self.rate > max_budget_units)
            .unwrap_or(false)
    }
}

fn load_configured_backend_readiness_for_candidate(
    compiled_bundle: &serde_json::Value,
    candidate: &mut ProfileCandidate,
) {
    if candidate.external_backend_readiness.is_some() {
        return;
    }
    candidate.external_backend_readiness = configured_backend_readiness_verdict_for_candidate(
        compiled_bundle,
        &candidate.role,
        &candidate.profile,
    );
    if let Some(status) = candidate
        .external_backend_readiness
        .as_ref()
        .and_then(|verdict| verdict["status"].as_str())
        .map(str::to_string)
    {
        candidate.readiness_status = status;
    }
}

fn load_static_configured_backend_dispatch_blocker_for_candidate(
    compiled_bundle: &serde_json::Value,
    candidate: &mut ProfileCandidate,
) {
    if candidate.external_backend_readiness.is_some() {
        return;
    }
    candidate.external_backend_readiness =
        configured_backend_dispatch_blocker_verdict_for_candidate(
            compiled_bundle,
            &candidate.role,
            &candidate.profile,
        );
    if let Some(status) = candidate
        .external_backend_readiness
        .as_ref()
        .and_then(|verdict| verdict["status"].as_str())
        .map(str::to_string)
    {
        candidate.readiness_status = status;
    }
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
    build_runtime_assignment_from_dispatch_alias_with_readiness(
        compiled_bundle,
        alias_id,
        fallback_task_class,
        true,
    )
}

pub(crate) fn build_runtime_assignment_preview_from_dispatch_alias(
    compiled_bundle: &serde_json::Value,
    alias_id: &str,
    fallback_task_class: &str,
) -> serde_json::Value {
    build_runtime_assignment_from_dispatch_alias_with_readiness(
        compiled_bundle,
        alias_id,
        fallback_task_class,
        false,
    )
}

fn build_runtime_assignment_from_dispatch_alias_with_readiness(
    compiled_bundle: &serde_json::Value,
    alias_id: &str,
    fallback_task_class: &str,
    probe_external_readiness: bool,
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
    let mut assignment = if probe_external_readiness {
        build_runtime_assignment_from_resolved_constraints(
            compiled_bundle,
            alias_id,
            &task_class,
            &runtime_role,
        )
    } else {
        build_runtime_assignment_preview_from_resolved_constraints(
            compiled_bundle,
            alias_id,
            &task_class,
            &runtime_role,
        )
    };
    let alias_carrier_tier = alias
        .get("carrier_tier")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(map) = assignment.as_object_mut() {
        if let Some(carrier_tier) = alias_carrier_tier.as_ref() {
            map.insert(
                "activation_agent_type".to_string(),
                serde_json::Value::String(carrier_tier.clone()),
            );
            map.insert(
                "selected_tier".to_string(),
                serde_json::Value::String(carrier_tier.clone()),
            );
            map.insert(
                "selected_carrier_tier".to_string(),
                serde_json::Value::String(carrier_tier.clone()),
            );
        }
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
    build_runtime_assignment_from_resolved_constraints_with_readiness(
        compiled_bundle,
        conversation_role,
        task_class,
        execution_runtime_role,
        true,
        None,
    )
}

pub(crate) fn build_runtime_assignment_preview_from_resolved_constraints(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
) -> serde_json::Value {
    build_runtime_assignment_from_resolved_constraints_with_readiness(
        compiled_bundle,
        conversation_role,
        task_class,
        execution_runtime_role,
        false,
        None,
    )
}

fn build_runtime_assignment_from_resolved_constraints_with_stage_isolation(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
    stage_isolation: Option<&str>,
) -> serde_json::Value {
    build_runtime_assignment_from_resolved_constraints_with_readiness(
        compiled_bundle,
        conversation_role,
        task_class,
        execution_runtime_role,
        true,
        stage_isolation,
    )
}

fn build_runtime_assignment_from_resolved_constraints_with_readiness(
    compiled_bundle: &serde_json::Value,
    conversation_role: &str,
    task_class: &str,
    execution_runtime_role: &str,
    probe_external_readiness: bool,
    scoped_row_isolation: Option<&str>,
) -> serde_json::Value {
    let carrier_runtime = carrier_runtime_section(compiled_bundle);
    let selection_rule = selection_rule_for_runtime(carrier_runtime);
    let selection_strategy = selection_strategy(carrier_runtime);
    let model_selection_enabled = model_selection_enabled(compiled_bundle, carrier_runtime);
    let candidate_scope = candidate_scope(compiled_bundle, carrier_runtime);
    if !selection_strategy_supported(&selection_strategy) {
        return serde_json::json!({
            "enabled": false,
            "reason": "selection_strategy_not_supported",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_strategy": selection_strategy,
            "supported_selection_strategies": [
                "balanced_cost_quality",
                "quality_first",
                "risk_aware",
                "free_first_with_quality_floor"
            ],
            "selection_strategy_source_path": "carrier_runtime.model_selection.default_strategy",
            "selection_rule": selection_rule,
            "model_selection_enabled": model_selection_enabled,
            "candidate_scope": candidate_scope,
            "rejected_candidates": [],
        });
    }
    if !model_selection_enabled {
        return serde_json::json!({
            "enabled": false,
            "reason": "model_selection_disabled",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "model_selection_enabled": false,
            "candidate_scope": candidate_scope,
            "rejected_candidates": [],
        });
    }
    if !model_selection_ranking_policy_is_complete(compiled_bundle, carrier_runtime) {
        return serde_json::json!({
            "enabled": false,
            "reason": "model_selection_ranking_policy_missing",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_rule": selection_rule,
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
            "blocker_codes": ["model_selection_ranking_policy_missing"],
            "next_actions": [
                "Configure non-empty semantic_scores and ordinal_ranks under agent_system.model_selection.",
                "Configure missing_reasoning_effort_policy.mode before retrying runtime assignment."
            ],
            "rejected_candidates": [],
        });
    }
    if candidate_scope != "unified_carrier_model_profiles" {
        return serde_json::json!({
            "enabled": false,
            "reason": "candidate_scope_not_supported",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
            "supported_candidate_scope": "unified_carrier_model_profiles",
            "rejected_candidates": [],
        });
    }
    let Some(roles) = carrier_runtime["roles"].as_array() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing",
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
        });
    };
    if roles.is_empty() {
        return serde_json::json!({
            "enabled": false,
            "reason": "carrier_runtime_roles_missing",
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
        });
    }

    let demotion_score = json_u64(json_lookup(
        &carrier_runtime["worker_strategy"],
        &["selection_policy", "demotion_score"],
    ))
    .unwrap_or(crate::carrier_runtime_metadata::DEFAULT_DEMOTION_SCORE);
    let free_profiles_allowed = free_profiles_allowed(carrier_runtime);
    let quality_floor = quality_floor_for_runtime_role(carrier_runtime, execution_runtime_role);
    let reasoning_floor = reasoning_floor_for_task_class(carrier_runtime, task_class);
    let budget_policy = assignment_budget_policy(
        compiled_bundle,
        carrier_runtime,
        task_class,
        conversation_role,
    );
    let budget_policy_source_path = assignment_budget_policy_source_path(
        compiled_bundle,
        carrier_runtime,
        task_class,
        conversation_role,
        &budget_policy,
    );
    let allow_over_budget_escalation =
        budget_policy_allows_over_budget_escalation(compiled_bundle, carrier_runtime);
    let route_profiles = route_profile_mapping(compiled_bundle, task_class, conversation_role);
    let enforce_max_budget_units =
        budget_policy_enforces_max_budget_units(compiled_bundle, carrier_runtime)
            || matches!(budget_policy.as_str(), "strict" | "balanced");
    let max_budget_units = enforce_max_budget_units
        .then(|| {
            assignment_max_budget_units(
                compiled_bundle,
                carrier_runtime,
                task_class,
                conversation_role,
            )
        })
        .flatten();
    let max_budget_units_source_path = max_budget_units.and_then(|_| {
        assignment_max_budget_units_source_path(
            compiled_bundle,
            carrier_runtime,
            task_class,
            conversation_role,
        )
    });
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
                    let profile_id = non_empty_string_field(&profile["profile_id"])
                        .map(str::to_string)
                        .unwrap_or_default();
                    let raw_rate = profile
                        .get("normalized_cost_units")
                        .or_else(|| role.get("normalized_cost_units"))
                        .or_else(|| role.get("rate"));
                    let rate = json_u64(raw_rate).unwrap_or(0);
                    let rate_present = json_u64(raw_rate).is_some();
                    let rate_source_path = if profile.get("normalized_cost_units").is_some() {
                        candidate_metadata_source_path(
                            role_id,
                            &profile_id,
                            "normalized_cost_units",
                        )
                    } else if role.get("normalized_cost_units").is_some() {
                        Some(format!(
                            "carrier_runtime.roles[{role_id}].normalized_cost_units"
                        ))
                    } else if role.get("rate").is_some() {
                        Some(format!("carrier_runtime.roles[{role_id}].rate"))
                    } else {
                        None
                    };
                    let provider = profile["provider"]
                        .as_str()
                        .or_else(|| role["model_provider"].as_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty());
                    let (pricing, pricing_source_path) = pricing_metadata_for_profile(
                        compiled_bundle,
                        role_id,
                        &profile_id,
                        &profile,
                        provider,
                    );
                    let (pricing_freshness, pricing_freshness_status, pricing_rejection_reasons) =
                        pricing_freshness_status_and_reject_reasons(&pricing);
                    let pricing_freshness_source_path = pricing_source_path
                        .as_ref()
                        .map(|path| format!("{path}.freshness"));
                    let model_profile_id_source_path =
                        candidate_metadata_source_path(role_id, &profile_id, "profile_id");
                    let model_ref_source_path =
                        candidate_metadata_source_path(role_id, &profile_id, "model_ref");
                    let reasoning_effort = profile["reasoning_effort"]
                        .as_str()
                        .or_else(|| role["model_reasoning_effort"].as_str())
                        .unwrap_or_default();
                    let quality_tier = profile["quality_tier"]
                        .as_str()
                        .or_else(|| role["quality_tier"].as_str())
                        .unwrap_or_default();
                    ProfileCandidate {
                        supports_runtime_role: profile_supports_runtime_role(
                            role,
                            &profile,
                            execution_runtime_role,
                        ),
                        supports_task_class: profile_supports_task_class(
                            role, &profile, task_class,
                        ),
                        model_profile_id_source_path,
                        model_ref_source_path,
                        rate_present,
                        rate_source_path,
                        rate,
                        pricing,
                        pricing_source_path,
                        pricing_freshness,
                        pricing_freshness_source_path,
                        pricing_freshness_status,
                        pricing_rejection_reasons,
                        effective_score,
                        lifecycle_state: lifecycle_state.clone(),
                        quality_rank: configured_model_selection_rank(
                            compiled_bundle,
                            carrier_runtime,
                            "quality_tier",
                            quality_tier,
                        )
                        .unwrap_or_default(),
                        reasoning_rank: configured_model_selection_rank(
                            compiled_bundle,
                            carrier_runtime,
                            "reasoning_effort",
                            reasoning_effort,
                        )
                        .unwrap_or_default(),
                        readiness_status: profile_readiness_status(&profile),
                        external_backend_readiness: None,
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
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "next_actions": [
                "Add or enable a carrier under `vida.config.yaml -> host_environment.systems.<system>.carriers` that declares both the requested runtime_role and task_class.",
                "Run `vida project-activator --repair --host-cli-system <system> --json` to refresh selected host materialization, then retry the runtime assignment preview."
            ],
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
            "rejected_candidates": rejected_candidates
        });
    }

    candidates.retain_mut(|candidate| {
        let mut reasons = Vec::new();
        let mut metadata_source_paths = serde_json::Map::new();
        let profile_id = candidate.profile["profile_id"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let model_ref = candidate.profile["model_ref"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if profile_id.is_none() {
            reasons.push("selected_model_profile_id_missing".to_string());
            if let Some(path) = candidate.model_profile_id_source_path.as_ref() {
                metadata_source_paths.insert(
                    "selected_model_profile_id".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
        }
        if model_ref.is_none() {
            reasons.push("selected_model_ref_missing".to_string());
            if let Some(path) = candidate.model_ref_source_path.as_ref() {
                metadata_source_paths.insert(
                    "selected_model_ref".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
        }
        if !candidate.rate_present {
            reasons.push("selected_rate_missing".to_string());
            if let Some(path) = candidate.rate_source_path.as_ref() {
                metadata_source_paths.insert(
                    "selected_rate".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
        }
        if !candidate.pricing_rejection_reasons.is_empty()
            || candidate.pricing_freshness_status != "ready"
        {
            if let Some(path) = candidate.pricing_source_path.as_ref() {
                metadata_source_paths.insert(
                    "selected_pricing".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
            if let Some(path) = candidate.pricing_freshness_source_path.as_ref() {
                metadata_source_paths.insert(
                    "selected_pricing_freshness".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
            reasons.extend(candidate.pricing_rejection_reasons.clone());
        }
        if !candidate.supports_runtime_role {
            reasons.push("runtime_role_not_supported".to_string());
        }
        if !candidate.supports_task_class {
            reasons.push("task_class_not_supported".to_string());
        }
        if candidate.quality_rank == 0 {
            reasons.push("quality_tier_rank_unconfigured".to_string());
        }
        if candidate.reasoning_rank == 0 {
            reasons.push("reasoning_effort_rank_unconfigured".to_string());
        }
        if candidate.effective_score < demotion_score || candidate.lifecycle_state == "retired" {
            reasons.push("carrier_score_or_lifecycle_blocked".to_string());
        }
        if candidate.rate == 0 && !free_profiles_allowed {
            reasons.push("zero_cost_profiles_disabled".to_string());
        }
        if budget_policy == "strict"
            && candidate.over_budget(max_budget_units)
            && !allow_over_budget_escalation
        {
            reasons.push("over_budget".to_string());
        }
        let write_scope = candidate.profile["write_scope"]
            .as_str()
            .or_else(|| candidate.role["write_scope"].as_str())
            .unwrap_or_default()
            .to_string();
        let backend_class = candidate_backend_class(candidate);
        let live_guard_required = task_class_requires_write_scope(task_class)
            && write_scope_requires_live_guard(&write_scope);
        let service_write_guard_required = service_executor_backend_class(&backend_class)
            && task_class_requires_write_scope(task_class);
        load_static_configured_backend_dispatch_blocker_for_candidate(compiled_bundle, candidate);
        if reasons.is_empty()
            && (probe_external_readiness || live_guard_required || service_write_guard_required)
        {
            load_configured_backend_readiness_for_candidate(compiled_bundle, candidate);
        }
        if candidate.readiness_status == "blocked" {
            reasons.push("profile_not_ready".to_string());
        }
        if candidate
            .external_backend_readiness
            .as_ref()
            .and_then(|verdict| verdict["blocked"].as_bool())
            .unwrap_or(false)
        {
            reasons.push("external_backend_not_ready".to_string());
        }
        if let Some(mapped_profile_id) = mapped_profile_for_carrier(
            route_profiles,
            candidate.role["role_id"].as_str().unwrap_or_default(),
        ) {
            if candidate.profile["profile_id"].as_str() != Some(mapped_profile_id.as_str()) {
                reasons.push("route_profile_mapping_mismatch".to_string());
            }
        }
        if !write_scope_allows_task_class(
            &write_scope,
            task_class,
            candidate.external_backend_readiness.as_ref(),
        ) {
            reasons.push("write_scope_inadmissible_for_task_class".to_string());
        }
        if scoped_row_isolation
            .is_some_and(|isolation| !isolation_allows_write_scope(isolation, &write_scope))
        {
            reasons.push("write_scope_inadmissible_for_isolation".to_string());
        }
        if !service_executor_write_candidate_ready(
            &backend_class,
            task_class,
            candidate.external_backend_readiness.as_ref(),
        ) {
            reasons.push("service_executor_guard_not_ready".to_string());
        }
        if let Some(floor) = quality_floor.as_deref() {
            if candidate.quality_rank
                < configured_model_selection_rank(
                    compiled_bundle,
                    carrier_runtime,
                    "quality_tier",
                    floor,
                )
                .unwrap_or_default()
            {
                reasons.push("quality_floor_not_met".to_string());
            }
        }
        if let Some(floor) = reasoning_floor.as_deref() {
            if candidate.reasoning_rank
                < configured_model_selection_rank(
                    compiled_bundle,
                    carrier_runtime,
                    "reasoning_effort",
                    floor,
                )
                .unwrap_or_default()
            {
                reasons.push("reasoning_floor_not_met".to_string());
            }
        }
        if reasons.is_empty() {
            true
        } else {
            let mut rejected_candidate = serde_json::json!({
                "carrier_id": candidate.role["role_id"],
                "carrier_tier": candidate.role["tier"],
                "model_profile_id": candidate.profile["profile_id"],
                "model_ref": candidate.profile["model_ref"],
                "reasons": reasons,
                "reason": reasons.first().cloned().unwrap_or_default(),
            });
            if let Some(readiness) = candidate.external_backend_readiness.clone() {
                if let Some(row) = rejected_candidate.as_object_mut() {
                    row.insert(
                        "configured_backend_readiness".to_string(),
                        readiness.clone(),
                    );
                    row.insert(
                        "external_backend_readiness".to_string(),
                        legacy_external_backend_readiness_alias(Some(&readiness)),
                    );
                }
            }
            if let Some(row) = rejected_candidate.as_object_mut() {
                if !metadata_source_paths.is_empty() {
                    row.insert(
                        "selection_source_paths".to_string(),
                        serde_json::Value::Object(metadata_source_paths),
                    );
                }
            }
            rejected_candidates.push(rejected_candidate);
            false
        }
    });

    let prefer_in_budget_first = max_budget_units.is_some()
        && (budget_policy == "balanced"
            || (budget_policy == "strict" && allow_over_budget_escalation));
    if selection_strategy == "quality_first" || selection_strategy == "risk_aware" {
        candidates.sort_by(|left, right| {
            prefer_in_budget_first
                .then(|| {
                    left.over_budget(max_budget_units)
                        .cmp(&right.over_budget(max_budget_units))
                })
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.quality_rank.cmp(&left.quality_rank))
                .then_with(|| right.reasoning_rank.cmp(&left.reasoning_rank))
                .then_with(|| left.rate.cmp(&right.rate))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
        });
    } else if selection_strategy == "free_first_with_quality_floor" {
        candidates.sort_by(|left, right| {
            prefer_in_budget_first
                .then(|| {
                    left.over_budget(max_budget_units)
                        .cmp(&right.over_budget(max_budget_units))
                })
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| (left.rate != 0).cmp(&(right.rate != 0)))
                .then_with(|| left.rate.cmp(&right.rate))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
        });
    } else {
        candidates.sort_by(|left, right| {
            prefer_in_budget_first
                .then(|| {
                    left.over_budget(max_budget_units)
                        .cmp(&right.over_budget(max_budget_units))
                })
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.rate.cmp(&right.rate))
                .then_with(|| right.quality_rank.cmp(&left.quality_rank))
                .then_with(|| right.effective_score.cmp(&left.effective_score))
                .then_with(|| right.reasoning_rank.cmp(&left.reasoning_rank))
        });
    }

    let Some(selected_candidate) = candidates.first() else {
        let reason = first_rejected_metadata_reason(&rejected_candidates)
            .unwrap_or_else(|| "no_carrier_satisfies_runtime_role_or_task_class".to_string());
        return serde_json::json!({
            "enabled": false,
            "reason": reason,
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role,
            "selection_strategy": selection_strategy,
            "selection_rule": selection_rule,
            "model_selection_enabled": true,
            "candidate_scope": candidate_scope,
            "rejected_candidates": rejected_candidates,
        });
    };

    let selected_role = &selected_candidate.role;
    let selected_profile = &selected_candidate.profile;
    let selected_role_id = selected_role["role_id"].as_str().unwrap_or_default();
    let selected_profile_id = selected_profile["profile_id"].as_str().unwrap_or_default();
    let selected_dispatch_backend =
        route_executor_backend(compiled_bundle, task_class, conversation_role).unwrap_or_else(
            || {
                selected_role["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            },
        );
    let selected_dispatch_backend_source_path =
        route_executor_backend_source_path(compiled_bundle, task_class, conversation_role)
            .unwrap_or_else(|| format!("carrier_runtime.roles[{selected_role_id}].role_id"));
    let selected_route_profile_mapping = mapped_profile_for_carrier(
        route_profiles,
        selected_role["role_id"].as_str().unwrap_or_default(),
    );
    let selected_route_profile_mapping_source_path =
        selected_route_profile_mapping.as_ref().and_then(|_| {
            route_profile_mapping_source_path(
                compiled_bundle,
                task_class,
                conversation_role,
                selected_role_id,
            )
        });
    let route_profile_mapping_applied = selected_route_profile_mapping.is_some();
    let tier = selected_role["tier"].as_str().unwrap_or_default();
    let rate = selected_candidate.rate;
    let selected_over_budget = selected_candidate.over_budget(max_budget_units);
    let selected_rate_source_path = selected_candidate.rate_source_path.clone();
    let budget_verdict = if !enforce_max_budget_units || max_budget_units.is_none() {
        "not_enforced"
    } else if selected_over_budget && budget_policy == "strict" && allow_over_budget_escalation {
        "strict_over_budget_escalation"
    } else if selected_over_budget && budget_policy == "balanced" {
        "balanced_over_budget_escalation"
    } else if selected_over_budget {
        "over_budget"
    } else {
        "in_budget"
    };
    let complexity_multiplier = task_complexity_multiplier(task_class);
    let effective_score = selected_candidate.effective_score;
    let lifecycle_state = selected_candidate.lifecycle_state.as_str();
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
        format!("budget_policy={budget_policy}"),
        format!("budget_verdict={budget_verdict}"),
    ];
    let mut selection_override_reasons = Vec::new();
    if route_profile_mapping_applied {
        selection_override_reasons.push(serde_json::json!({
            "field": "selected_model_profile_id",
            "reason": "route_profile_mapping_applied",
            "source_path": selected_route_profile_mapping_source_path.clone(),
            "selected_profile_id": selected_profile_id,
        }));
    }
    if max_budget_units.is_some() {
        selection_override_reasons.push(serde_json::json!({
            "field": "selected_carrier_id",
            "reason": if prefer_in_budget_first {
                "selection_budget_preferred_in_budget_candidate"
            } else {
                "selection_budget_filtered_over_budget_candidates"
            },
            "source_path": max_budget_units_source_path.clone(),
            "budget_policy": budget_policy.clone(),
            "max_budget_units": max_budget_units,
        }));
    }
    let selected_write_scope = selected_profile["write_scope"]
        .as_str()
        .or_else(|| selected_role["write_scope"].as_str())
        .unwrap_or_default()
        .to_string();
    let selected_write_scope_source_path = if selected_profile["write_scope"].as_str().is_some() {
        format!(
            "carrier_runtime.roles[{selected_role_id}].model_profiles.{selected_profile_id}.write_scope"
        )
    } else {
        format!("carrier_runtime.roles[{selected_role_id}].write_scope")
    };

    let mut assignment = serde_json::json!({
        "enabled": true,
        "task_class": task_class,
        "runtime_role": execution_runtime_role,
        "conversation_role": conversation_role,
        "activation_agent_type": selected_role["role_id"],
        "activation_runtime_role": execution_runtime_role,
        "selected_agent_id": selected_role["role_id"],
        "selected_carrier_id": selected_role["role_id"],
        "selected_backend_id": selected_dispatch_backend,
        "selected_dispatch_backend_id": selected_dispatch_backend,
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
        "selected_write_scope": selected_write_scope,
        "selected_quality_tier": selected_profile["quality_tier"],
        "selected_speed_tier": selected_profile["speed_tier"],
            "selected_model_profile_readiness_status": selected_candidate.readiness_status,
            "selected_configured_backend_readiness": selected_candidate.external_backend_readiness,
        "selected_external_backend_readiness": legacy_external_backend_readiness_alias(selected_candidate.external_backend_readiness.as_ref()),
        "pricing_readiness": serde_json::json!({
            "pricing": selected_candidate.pricing,
            "pricing_source_path": selected_candidate.pricing_source_path,
            "pricing_freshness": selected_candidate.pricing_freshness,
            "pricing_freshness_source_path": selected_candidate.pricing_freshness_source_path,
            "pricing_freshness_status": selected_candidate.pricing_freshness_status,
            "pricing_rejection_reasons": selected_candidate.pricing_rejection_reasons,
        }),
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
        "scorecards_store": carrier_runtime["worker_strategy"]["scorecards_path"]
    });
    if let Some(map) = assignment.as_object_mut() {
        map.insert(
            "selection_precedence".to_string(),
            serde_json::json!([
                "runtime_role_and_task_class_admissibility",
                "carrier_score_lifecycle_readiness",
                "route_profile_mapping",
                "write_scope",
                "quality_and_reasoning_floors",
                "selection_budget_policy",
                "selection_strategy_sort"
            ]),
        );
        let mut selection_source_paths = serde_json::json!({
            "selected_carrier_id": format!("carrier_runtime.roles[{selected_role_id}].role_id"),
            "selected_backend_id": selected_dispatch_backend_source_path,
            "selected_dispatch_backend_id": selected_dispatch_backend_source_path,
            "selected_carrier_tier": format!("carrier_runtime.roles[{selected_role_id}].tier"),
            "selected_model_profile_id": format!("carrier_runtime.roles[{selected_role_id}].model_profiles.{selected_profile_id}.profile_id"),
            "selected_model_ref": format!("carrier_runtime.roles[{selected_role_id}].model_profiles.{selected_profile_id}.model_ref"),
            "selected_rate": selected_rate_source_path,
            "selected_reasoning_effort": format!("carrier_runtime.roles[{selected_role_id}].model_profiles.{selected_profile_id}.reasoning_effort"),
            "selected_write_scope": selected_write_scope_source_path,
            "selection_strategy": "carrier_runtime.model_selection.default_strategy",
            "selection_rule": "carrier_runtime.model_selection.selection_rule",
            "candidate_scope": "model_selection.candidate_scope",
                "budget_policy": budget_policy_source_path.clone(),
                "max_budget_units": max_budget_units_source_path.clone(),
                "selected_route_profile_mapping": selected_route_profile_mapping_source_path.clone(),
            "selected_pricing": selected_candidate
                .pricing_source_path
                .clone()
                .unwrap_or_default(),
            "selected_pricing_freshness": selected_candidate
                .pricing_freshness_source_path
                .clone()
                .unwrap_or_default(),
        });
        if let Some(selection_source_paths_map) = selection_source_paths.as_object_mut() {
            if selected_candidate.pricing_source_path.is_none() {
                selection_source_paths_map.remove("selected_pricing");
            }
            if selected_candidate.pricing_freshness_source_path.is_none() {
                selection_source_paths_map.remove("selected_pricing_freshness");
            }
        }
        map.insert("selection_source_paths".to_string(), selection_source_paths);
        map.insert(
            "selection_override_reasons".to_string(),
            serde_json::Value::Array(selection_override_reasons),
        );
        map.insert(
            "selection_strategy".to_string(),
            serde_json::Value::String(selection_strategy),
        );
        map.insert(
            "selection_rule".to_string(),
            serde_json::Value::String(selection_rule),
        );
        map.insert(
            "model_selection_enabled".to_string(),
            serde_json::Value::Bool(true),
        );
        map.insert(
            "candidate_scope".to_string(),
            serde_json::Value::String(candidate_scope),
        );
        map.insert(
            "rejected_candidates".to_string(),
            serde_json::Value::Array(rejected_candidates),
        );
        map.insert(
            "rationale".to_string(),
            serde_json::Value::Array(
                rationale
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
        map.insert(
            "selected_reasoning_control_mode".to_string(),
            reasoning_control_mode(selected_profile)
                .map(|value| serde_json::Value::String(value.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "budget_policy".to_string(),
            serde_json::Value::String(budget_policy.clone()),
        );
        map.insert(
            "budget_scope".to_string(),
            serde_json::Value::String("selection_filter_only".to_string()),
        );
        map.insert(
            "selection_budget".to_string(),
            serde_json::json!({
                "scope": "selection_filter_only",
                "policy": budget_policy,
                "policy_source_path": budget_policy_source_path,
                "enforces_candidate_rejection": budget_policy == "strict",
                "prefers_in_budget_candidates": prefer_in_budget_first,
                "max_budget_units": max_budget_units,
                "max_budget_units_source_path": max_budget_units_source_path,
                "selected_candidate_rate": rate,
                "selected_over_budget": selected_over_budget,
                "budget_verdict": budget_verdict,
                "over_budget_escalation_allowed": allow_over_budget_escalation,
            }),
        );
        map.insert(
            "runtime_budget_ledger".to_string(),
            serde_json::json!({
                "status": "not_tracked_by_runtime_assignment",
                "scope": "runtime_spend_ledger",
                "enforcement": "not_implemented_in_this_assignment_builder",
                "message": "selection budget limits candidate choice only; no runtime spend ledger is debited here",
            }),
        );
        map.insert(
            "over_budget_escalation_allowed".to_string(),
            serde_json::Value::Bool(allow_over_budget_escalation),
        );
        map.insert(
            "max_budget_units".to_string(),
            max_budget_units
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "budget_verdict".to_string(),
            serde_json::Value::String(budget_verdict.to_string()),
        );
        map.insert(
            "selected_over_budget".to_string(),
            serde_json::Value::Bool(selected_over_budget),
        );
        map.insert(
            "selected_route_profile_mapping".to_string(),
            selected_route_profile_mapping
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "route_profile_mapping_applied".to_string(),
            serde_json::Value::Bool(route_profile_mapping_applied),
        );
    }
    assignment
}

pub(crate) fn build_runtime_assignment(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let task_class = infer_runtime_task_class(selection, requires_design_gate);
    let execution_runtime_role =
        infer_execution_runtime_role(selection, &task_class, requires_design_gate);
    let mut assignment = build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        &selection.selected_role,
        &task_class,
        &execution_runtime_role,
    );
    if semantic_routing_enabled(compiled_bundle) {
        insert_semantic_routing_diagnostics(
            &mut assignment,
            compiled_bundle,
            &selection.request,
            &task_class,
            &execution_runtime_role,
            &selection.selected_role,
        );
    }
    assignment
}

fn stage_policy_string(value: &serde_json::Value, field: &str, default_value: &str) -> String {
    value[field]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .to_string()
}

fn stage_policy_optional_string(value: &serde_json::Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        value[*field]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn stage_policy_scoped_bundle(
    compiled_bundle: &serde_json::Value,
    row: &serde_json::Value,
) -> serde_json::Value {
    let mut scoped_bundle = compiled_bundle.clone();
    let requested_carrier_id =
        stage_policy_optional_string(row, &["carrier_id", "agent_id", "role_id"]);
    let requested_model_profile_id =
        stage_policy_optional_string(row, &["model_profile_id", "model_profile"]);

    if let Some(carrier_id) = requested_carrier_id.as_deref() {
        if let Some(agents) =
            scoped_bundle["carrier_runtime"]["worker_strategy"]["agents"].as_object_mut()
        {
            for (agent_id, agent) in agents {
                if agent_id != carrier_id {
                    agent["lifecycle_state"] = serde_json::json!("retired");
                }
            }
        }
        if let Some(roles) = scoped_bundle["carrier_runtime"]["roles"].as_array_mut() {
            for role in roles {
                if role["role_id"].as_str() != Some(carrier_id) {
                    role["stage_attempt_scoped_out"] = serde_json::json!(true);
                    continue;
                }
                if let Some(model_profile_id) = requested_model_profile_id.as_deref() {
                    role["default_model_profile"] = serde_json::json!(model_profile_id);
                }
            }
        }
        if let Some(model_profile_id) = requested_model_profile_id.as_deref() {
            if let Some(subagent) = scoped_bundle
                .get_mut("agent_system")
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|agent_system| agent_system.get_mut("subagents"))
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|subagents| subagents.get_mut(carrier_id))
                .and_then(serde_json::Value::as_object_mut)
            {
                subagent.insert(
                    "default_model_profile".to_string(),
                    serde_json::json!(model_profile_id),
                );
            }
        }
    }

    scoped_bundle
}

fn stage_policy_assignment(
    compiled_bundle: &serde_json::Value,
    stage_id: &str,
    row: &serde_json::Value,
    index: usize,
) -> serde_json::Value {
    let task_class = stage_policy_string(row, "task_class", stage_id);
    let runtime_role = stage_policy_string(row, "runtime_role", "worker");
    let conversation_role = stage_policy_string(row, "conversation_role", &runtime_role);
    let scoped_bundle = stage_policy_scoped_bundle(compiled_bundle, row);
    let isolation = row["isolation"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("readonly");
    let mut assignment = build_runtime_assignment_from_resolved_constraints_with_stage_isolation(
        &scoped_bundle,
        &conversation_role,
        &task_class,
        &runtime_role,
        Some(isolation),
    );
    if let Some(map) = assignment.as_object_mut() {
        map.insert("stage_id".to_string(), serde_json::json!(stage_id));
        map.insert("attempt_index".to_string(), serde_json::json!(index));
        map.insert(
            "attempt_id".to_string(),
            row["attempt_id"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(serde_json::Value::from)
                .unwrap_or_else(|| serde_json::json!(format!("{stage_id}-attempt-{index}"))),
        );
        map.insert(
            "policy_source_path".to_string(),
            serde_json::json!(format!(
                "agent_system.stage_attempt_policies.{stage_id}.attempts[{index}]"
            )),
        );
        map.insert(
            "isolation".to_string(),
            serde_json::json!(isolation),
        );
        map.insert(
            "requested_carrier_id".to_string(),
            stage_policy_optional_string(row, &["carrier_id", "agent_id", "role_id"])
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "requested_model_profile_id".to_string(),
            stage_policy_optional_string(row, &["model_profile_id", "model_profile"])
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    assignment
}

fn stage_policy_blocker_reason(assignment: &serde_json::Value) -> String {
    let reason = assignment["reason"]
        .as_str()
        .unwrap_or("assignment_blocked")
        .to_string();
    let Some(requested_carrier_id) = assignment["requested_carrier_id"].as_str() else {
        return reason;
    };
    if reason != "no_carrier_satisfies_runtime_role_or_task_class" {
        return reason;
    }
    assignment["rejected_candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|candidate| candidate["carrier_id"].as_str() == Some(requested_carrier_id))
        .and_then(|candidate| candidate["reason"].as_str())
        .map(str::to_string)
        .unwrap_or(reason)
}

pub(crate) fn build_stage_attempt_policy_from_config(
    compiled_bundle: &serde_json::Value,
    stage_id: &str,
) -> serde_json::Value {
    let stage_id = stage_id.trim();
    if stage_id.is_empty() {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["stage_id_missing"],
            "reason": "stage_id_missing",
        });
    }
    let policy = &compiled_bundle["agent_system"]["stage_attempt_policies"][stage_id];
    if !policy.is_object() {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["stage_attempt_policy_missing"],
            "stage_id": stage_id,
            "reason": "stage_attempt_policy_missing",
            "source_path": format!("agent_system.stage_attempt_policies.{stage_id}"),
        });
    }

    let attempts = policy["attempts"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .enumerate()
                .map(|(index, row)| stage_policy_assignment(compiled_bundle, stage_id, row, index))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let consolidator = policy
        .get("consolidator")
        .filter(|value| value.is_object())
        .map(|row| stage_policy_assignment(compiled_bundle, stage_id, row, attempts.len()));

    let mut blocker_codes = Vec::new();
    if attempts.is_empty() {
        blocker_codes.push("stage_attempt_policy_attempts_missing".to_string());
    }
    for attempt in &attempts {
        if attempt["enabled"].as_bool() != Some(true) {
            blocker_codes.push(format!("attempt:{}", stage_policy_blocker_reason(attempt)));
        }
    }
    match consolidator.as_ref() {
        Some(assignment) if assignment["enabled"].as_bool() == Some(true) => {}
        Some(assignment) => blocker_codes.push(format!(
            "consolidator:{}",
            stage_policy_blocker_reason(assignment)
        )),
        None => blocker_codes.push("stage_attempt_policy_consolidator_missing".to_string()),
    }

    serde_json::json!({
        "status": if blocker_codes.is_empty() { "pass" } else { "blocked" },
        "blocker_codes": blocker_codes,
        "stage_id": stage_id,
        "source_path": format!("agent_system.stage_attempt_policies.{stage_id}"),
        "attempt_count": attempts.len(),
        "attempts": attempts,
        "consolidator": consolidator,
        "fanout": policy["fanout"].clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_assignment, build_runtime_assignment_from_resolved_constraints,
        build_runtime_assignment_preview_from_resolved_constraints,
        build_stage_attempt_policy_from_config,
    };
    use crate::RuntimeConsumptionLaneSelection;

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
                    "semantic_scores": {
                        "reasoning_effort": {"minimal": 15.0, "low": 35.0, "provider_default": 50.0, "provider-configured": 50.0, "medium": 60.0, "high": 82.0, "xhigh": 95.0},
                        "quality_tier": {"low": 35.0, "medium_low": 50.0, "medium": 60.0, "medium_high": 82.0, "high": 82.0, "very_high": 95.0},
                        "speed_tier": {"slow": 35.0, "medium": 60.0, "fast": 82.0, "instant": 95.0}
                    },
                    "ordinal_ranks": {
                        "reasoning_effort": {"minimal": 1, "low": 2, "provider_default": 2, "provider-configured": 2, "medium": 3, "high": 4, "xhigh": 5},
                        "quality_tier": {"low": 1, "medium_low": 2, "medium": 3, "medium_high": 4, "high": 4, "very_high": 5}
                    },
                    "missing_reasoning_effort_policy": {"mode": "use_configured_default", "default": "medium"},
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
    fn stage_attempt_policy_config() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "analysis_internal_low",
                "tier": "internal_low",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["analysis"],
                "reasoning_band": "low",
                "default_model_profile": "codex_low_analysis",
                "model_profiles": {
                    "codex_low_analysis": {
                        "profile_id": "codex_low_analysis",
                        "model_ref": "codex/low-analysis",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 1,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["worker"],
                        "task_classes": ["analysis"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "vibe_medium_analysis",
                "tier": "external_medium",
                "rate": 2,
                "normalized_cost_units": 2,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["analysis"],
                "reasoning_band": "medium",
                "backend_class": "external_cli",
                "subagent_backend_class": "external_cli",
                "default_model_profile": "mistral_medium_analysis",
                "model_profiles": {
                    "mistral_medium_analysis": {
                        "profile_id": "mistral_medium_analysis",
                        "model_ref": "mistralai/mistral-medium-3.5-128b",
                        "provider": "nim",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 2,
                        "speed_tier": "medium",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["worker"],
                        "task_classes": ["analysis"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "coach_consolidator_high",
                "tier": "coach_high",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "high",
                "default_model_profile": "codex_high_consolidator",
                "model_profiles": {
                    "codex_high_consolidator": {
                        "profile_id": "codex_high_consolidator",
                        "model_ref": "codex/high-consolidator",
                        "provider": "openai",
                        "reasoning_effort": "high",
                        "normalized_cost_units": 4,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "none",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "stage_attempt_policies": {
                "analysis": {
                    "fanout": { "mode": "parallel", "max_attempts": 2 },
                    "attempts": [
                        {
                            "attempt_id": "analysis-internal-low",
                            "carrier_id": "analysis_internal_low",
                            "model_profile_id": "codex_low_analysis",
                            "conversation_role": "worker",
                            "runtime_role": "worker",
                            "task_class": "analysis",
                            "isolation": "external_readonly_complete"
                        },
                        {
                            "attempt_id": "analysis-external-medium",
                            "carrier_id": "vibe_medium_analysis",
                            "model_profile_id": "mistral_medium_analysis",
                            "conversation_role": "worker",
                            "runtime_role": "worker",
                            "task_class": "analysis",
                            "isolation": "external_readonly_complete"
                        }
                    ],
                    "consolidator": {
                        "attempt_id": "analysis-consolidator",
                        "carrier_id": "coach_consolidator_high",
                        "model_profile_id": "codex_high_consolidator",
                        "conversation_role": "coach",
                        "runtime_role": "coach",
                        "task_class": "review",
                        "isolation": "root_validate_only"
                    }
                }
            }
        });

        let policy = build_stage_attempt_policy_from_config(&compiled_bundle, "analysis");

        assert_eq!(policy["status"], "pass");
        assert!(
            policy["blocker_codes"]
                .as_array()
                .expect("blocker codes should render")
                .is_empty()
        );
        assert_eq!(policy["attempt_count"], 2);
        assert_eq!(
            policy["source_path"],
            "agent_system.stage_attempt_policies.analysis"
        );
        assert_eq!(policy["fanout"]["mode"], "parallel");
        assert_eq!(
            policy["attempts"][0]["policy_source_path"],
            "agent_system.stage_attempt_policies.analysis.attempts[0]"
        );
        assert_eq!(policy["attempts"][0]["enabled"], true);
        assert_eq!(
            policy["attempts"][0]["selected_carrier_id"],
            "analysis_internal_low"
        );
        assert_eq!(
            policy["attempts"][0]["selected_model_profile_id"],
            "codex_low_analysis"
        );
        assert_eq!(
            policy["attempts"][0]["isolation"],
            "external_readonly_complete"
        );
        assert_eq!(policy["attempts"][1]["enabled"], true);
        assert_eq!(
            policy["attempts"][1]["selected_carrier_id"],
            "vibe_medium_analysis"
        );
        assert_eq!(
            policy["attempts"][1]["selected_model_profile_id"],
            "mistral_medium_analysis"
        );
        assert_eq!(policy["consolidator"]["enabled"], true);
        assert_eq!(
            policy["consolidator"]["selected_carrier_id"],
            "coach_consolidator_high"
        );
        assert_eq!(
            policy["consolidator"]["selected_model_profile_id"],
            "codex_high_consolidator"
        );
        assert_ne!(
            policy["consolidator"]["selected_model_profile_id"],
            policy["attempts"][0]["selected_model_profile_id"]
        );
    }

    #[test]
    fn readonly_stage_isolation_rejects_write_enabled_profile() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "analysis_writer",
                "tier": "middle",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "business_analyst",
                "runtime_roles": ["business_analyst"],
                "task_classes": ["analysis"],
                "reasoning_band": "xhigh",
                "default_model_profile": "codex_write_analysis",
                "model_profiles": {
                    "codex_write_analysis": {
                        "profile_id": "codex_write_analysis",
                        "model_ref": "codex/write-analysis",
                        "provider": "openai",
                        "reasoning_effort": "xhigh",
                        "normalized_cost_units": 1,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["analysis"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "coach_consolidator",
                "tier": "coach",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "coach_review",
                "model_profiles": {
                    "coach_review": {
                        "profile_id": "coach_review",
                        "model_ref": "coach/review",
                        "provider": "test",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
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
        compiled_bundle["agent_system"] = serde_json::json!({
            "stage_attempt_policies": {
                "analysis": {
                    "fanout": { "mode": "single", "max_attempts": 1 },
                    "attempts": [
                        {
                            "attempt_id": "analysis-readonly-write-profile",
                            "carrier_id": "analysis_writer",
                            "model_profile_id": "codex_write_analysis",
                            "conversation_role": "analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "analysis",
                            "isolation": "internal_readonly_complete"
                        }
                    ],
                    "consolidator": {
                        "attempt_id": "analysis-consolidator",
                        "carrier_id": "coach_consolidator",
                        "model_profile_id": "coach_review",
                        "conversation_role": "coach",
                        "runtime_role": "coach",
                        "task_class": "review",
                        "isolation": "root_validate_only"
                    }
                }
            }
        });

        let policy = build_stage_attempt_policy_from_config(&compiled_bundle, "analysis");

        assert_eq!(policy["status"], "blocked");
        assert!(
            policy["blocker_codes"]
                .as_array()
                .expect("blocker codes should render")
                .iter()
                .any(|reason| {
                    reason.as_str() == Some("attempt:write_scope_inadmissible_for_isolation")
                }),
            "policy={policy:#}"
        );
        assert_eq!(policy["attempts"][0]["enabled"], false);
        assert!(
            policy["attempts"][0]["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|candidate| {
                    candidate["carrier_id"] == "analysis_writer"
                        && candidate["reason"] == "write_scope_inadmissible_for_isolation"
                })
        );
    }

    #[test]
    fn inadmissible_stage_attempt_fails_closed() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "readonly_implementer",
                "tier": "readonly",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "readonly_implementation",
                "model_profiles": {
                    "readonly_implementation": {
                        "profile_id": "readonly_implementation",
                        "model_ref": "readonly/implementation",
                        "provider": "test",
                        "reasoning_effort": "low",
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
            serde_json::json!({
                "role_id": "coach_consolidator",
                "tier": "coach",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "coach_review",
                "model_profiles": {
                    "coach_review": {
                        "profile_id": "coach_review",
                        "model_ref": "coach/review",
                        "provider": "test",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
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
        compiled_bundle["agent_system"] = serde_json::json!({
            "stage_attempt_policies": {
                "implementation": {
                    "fanout": { "mode": "single", "max_attempts": 1 },
                    "attempts": [
                        {
                            "attempt_id": "implementation-readonly",
                            "carrier_id": "readonly_implementer",
                            "model_profile_id": "readonly_implementation",
                            "conversation_role": "worker",
                            "runtime_role": "worker",
                            "task_class": "implementation",
                            "isolation": "external_patch_proposal"
                        }
                    ],
                    "consolidator": {
                        "attempt_id": "implementation-consolidator",
                        "carrier_id": "coach_consolidator",
                        "model_profile_id": "coach_review",
                        "conversation_role": "coach",
                        "runtime_role": "coach",
                        "task_class": "review",
                        "isolation": "root_validate_only"
                    }
                }
            }
        });

        let policy = build_stage_attempt_policy_from_config(&compiled_bundle, "implementation");

        assert_eq!(policy["status"], "blocked");
        assert!(
            policy["blocker_codes"]
                .as_array()
                .expect("blocker codes should render")
                .iter()
                .any(|reason| {
                    reason.as_str() == Some("attempt:write_scope_inadmissible_for_task_class")
                }),
            "policy={policy:#}"
        );
        assert_eq!(policy["attempts"][0]["enabled"], false);
        assert_eq!(
            policy["attempts"][0]["reason"],
            "no_carrier_satisfies_runtime_role_or_task_class"
        );
        assert!(
            policy["attempts"][0]["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|candidate| {
                    candidate["carrier_id"] == "readonly_implementer"
                        && candidate["reason"] == "write_scope_inadmissible_for_task_class"
                })
        );
        assert_eq!(
            policy["attempts"][0]["requested_carrier_id"],
            "readonly_implementer"
        );
        assert_eq!(policy["consolidator"]["enabled"], true);
        assert_eq!(
            policy["consolidator"]["selected_model_profile_id"],
            "coach_review"
        );
    }

    #[test]
    fn configured_executor_backend_stays_separate_from_selected_carrier() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "middle",
            "tier": "middle",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["test_authoring"],
            "reasoning_band": "medium",
            "default_model_profile": "codex_gpt55_medium_write",
            "model_profiles": {
                "codex_gpt55_medium_write": {
                    "profile_id": "codex_gpt55_medium_write",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "plan_mode_reasoning_effort": "high",
                    "sandbox_mode": "workspace-write",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["test_authoring"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({
                "routing": {
                    "default": {
                        "executor_backend": "internal_subagents"
                    }
                }
            }),
        );

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "test_author",
            "test_authoring",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "middle");
        assert_eq!(assignment["activation_agent_type"], "middle");
        assert_eq!(assignment["selected_backend_id"], "internal_subagents");
        assert_eq!(
            assignment["selected_dispatch_backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            assignment["selection_source_paths"]["selected_backend_id"],
            "agent_system.routing.default.executor_backend"
        );
    }

    #[test]
    fn enabled_semantic_routing_adds_advisory_assignment_diagnostics() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "worker",
            "tier": "middle",
            "rate": 6,
            "normalized_cost_units": 6,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "medium",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "developer",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "plan_mode_reasoning_effort": "high",
                    "sandbox_mode": "workspace-write",
                    "normalized_cost_units": 6,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({
                "semantic_routing": {
                    "enabled": true,
                    "mode": "deterministic_rules"
                }
            }),
        );
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "worker".to_string(),
            request: "Implement deterministic routing feature extraction with receipt-aware scoring helpers.".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: compiled_bundle.clone(),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };

        let assignment = build_runtime_assignment(&compiled_bundle, &selection, false);

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["task_class"], "implementation");
        assert_eq!(assignment["semantic_routing"]["enabled"], true);
        assert_eq!(
            assignment["semantic_routing"]["feature_source"],
            "vida_native_deterministic_rules"
        );
        assert_eq!(assignment["semantic_routing"]["score_applied"], true);
        assert_eq!(assignment["semantic_routing"]["advisory_only"], true);
        assert_eq!(
            assignment["semantic_routing"]["guardrail_risk"]["advisory_only"],
            true
        );
        assert_eq!(
            assignment["semantic_score_breakdown"]["advisory_only"],
            true
        );
        assert!(assignment["semantic_route_score"].as_f64().is_some());
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
                        "model_ref": "gpt-5.5",
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
    fn missing_model_ref_rejects_candidate_and_fails_closed() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "developer",
                    "model_ref": "",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "selected_model_ref_missing");
        assert_eq!(assignment["selected_carrier_id"].is_null(), true);
        assert_eq!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .next()
                .and_then(|row| row["selection_source_paths"]["selected_model_ref"].as_str())
                .unwrap_or_default(),
            "carrier_runtime.roles[junior].model_profiles.developer.model_ref"
        );
    }

    #[test]
    fn missing_model_profile_id_rejects_candidate_and_fails_closed() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "selected_model_profile_id_missing");
        assert_eq!(assignment["selected_carrier_id"].is_null(), true);
        assert_eq!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .next()
                .and_then(|row| row["reasons"][0].as_str())
                .unwrap_or_default(),
            "selected_model_profile_id_missing"
        );
    }

    #[test]
    fn missing_rate_metadata_rejects_candidate_and_fails_closed() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "developer",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "selected_rate_missing");
        assert_eq!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .next()
                .and_then(|row| row["selection_source_paths"]["selected_rate"].as_str())
                .unwrap_or_default(),
            ""
        );
    }

    #[test]
    fn enforced_pricing_staleness_is_blocking_and_actionable() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "developer",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "pricing": {
                "providers": {
                    "openai": {
                        "freshness": {
                            "required": true,
                            "max_age_days": 0,
                            "enforced": true
                        }
                    }
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "model_price_freshness_stale");
        assert_eq!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .next()
                .and_then(|row| row["selection_source_paths"]["selected_pricing"].as_str())
                .unwrap_or_default(),
            "agent_system.pricing.providers[openai]"
        );
        assert_eq!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .next()
                .and_then(
                    |row| row["selection_source_paths"]["selected_pricing_freshness"].as_str()
                )
                .unwrap_or_default(),
            "agent_system.pricing.providers[openai].freshness"
        );
    }

    #[test]
    fn diagnostic_pricing_staleness_does_not_block_selection_but_is_visible() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "developer",
            "model_profiles": {
                "developer": {
                    "profile_id": "developer",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "pricing": {
                "providers": {
                    "openai": {
                        "freshness": {
                            "required": true,
                            "max_age_days": 0,
                            "diagnostic_only": true
                        }
                    }
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "junior");
        assert_eq!(
            assignment["pricing_readiness"]["pricing_freshness_status"],
            "stale"
        );
        assert_eq!(
            assignment["selection_source_paths"]["selected_pricing"],
            "agent_system.pricing.providers[openai]"
        );
        assert_eq!(
            assignment["selection_source_paths"]["selected_pricing_freshness"],
            "agent_system.pricing.providers[openai].freshness"
        );
    }

    #[test]
    fn enforced_budget_rejects_over_budget_candidates() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
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
                        "model_ref": "gpt-5.5",
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
                "role_id": "senior",
                "tier": "senior",
                "rate": 16,
                "normalized_cost_units": 16,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "high",
                "default_model_profile": "codex_gpt54_high_write",
                "model_profiles": {
                    "codex_gpt54_high_write": {
                        "profile_id": "codex_gpt54_high_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "high",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "workspace-write",
                        "normalized_cost_units": 16,
                        "speed_tier": "fast",
                        "quality_tier": "high",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["carrier_runtime"]["model_selection"]["default_strategy"] =
            serde_json::json!("quality_first");
        compiled_bundle["agent_system"] = serde_json::json!({
            "model_selection": {
                "budget_policy": {
                    "enforce_max_budget_units": true
                }
            },
            "routing": {
                "worker": {
                    "max_budget_units": 1
                }
            }
        });

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
                    row["carrier_id"] == "senior"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| reason.as_str() == Some("over_budget"))
                })
        );
    }

    #[test]
    fn selected_candidate_surface_includes_rate_source_path() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "low",
            "default_model_profile": "codex_gpt54_low_write",
            "model_profiles": {
                "codex_gpt54_low_write": {
                    "profile_id": "codex_gpt54_low_write",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["carrier_runtime"]["model_selection"]["default_strategy"] =
            serde_json::json!("free_first_with_quality_floor");

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(
            assignment["selection_source_paths"]["selected_rate"],
            "carrier_runtime.roles[junior].model_profiles.codex_gpt54_low_write.normalized_cost_units"
        );
    }

    #[test]
    fn strict_budget_can_escalate_to_only_admissible_verifier_when_configured() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "opencode_cli",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "verifier",
                "runtime_roles": ["verifier"],
                "task_classes": ["verification"],
                "reasoning_band": "medium",
                "default_model_profile": "opencode_codex_mini_review",
                "model_profiles": {
                    "opencode_codex_mini_review": {
                        "profile_id": "opencode_codex_mini_review",
                        "model_ref": "opencode/gpt-5.1-codex-mini",
                        "provider": "opencode",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 0,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["verifier"],
                        "task_classes": ["verification"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "senior",
                "tier": "senior",
                "rate": 16,
                "normalized_cost_units": 16,
                "default_runtime_role": "verifier",
                "runtime_roles": ["verifier"],
                "task_classes": ["verification"],
                "reasoning_band": "high",
                "default_model_profile": "codex_spark_high_readonly",
                "model_profiles": {
                    "codex_spark_high_readonly": {
                        "profile_id": "codex_spark_high_readonly",
                        "model_ref": "gpt-5.3-codex-spark",
                        "provider": "openai",
                        "reasoning_effort": "high",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "read-only",
                        "normalized_cost_units": 16,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "read-only",
                        "runtime_roles": ["verifier"],
                        "task_classes": ["verification"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["carrier_runtime"]["model_selection"]["reasoning_floor_by_task_class"]["verification"] =
            serde_json::json!("high");
        compiled_bundle["agent_system"] = serde_json::json!({
            "model_selection": {
                "budget_policy": {
                    "enforce_max_budget_units": true,
                    "allow_escalation_over_budget_with_blocker": true
                }
            },
            "routing": {
                "verification": {
                    "max_budget_units": 4
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "verification",
            "verification",
            "verifier",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "senior");
        assert_eq!(assignment["selected_over_budget"], true);
        assert_eq!(
            assignment["budget_verdict"],
            "strict_over_budget_escalation"
        );
        assert_eq!(assignment["over_budget_escalation_allowed"], true);
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
                            .any(|reason| reason.as_str() == Some("reasoning_floor_not_met"))
                })
        );
    }

    #[test]
    fn route_level_budget_cap_applies_when_conversation_role_is_worker() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
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
                        "model_ref": "gpt-5.5",
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
                "role_id": "senior",
                "tier": "senior",
                "rate": 16,
                "normalized_cost_units": 16,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "high",
                "default_model_profile": "codex_gpt54_high_write",
                "model_profiles": {
                    "codex_gpt54_high_write": {
                        "profile_id": "codex_gpt54_high_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "high",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "workspace-write",
                        "normalized_cost_units": 16,
                        "speed_tier": "fast",
                        "quality_tier": "high",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["carrier_runtime"]["model_selection"]["default_strategy"] =
            serde_json::json!("quality_first");
        compiled_bundle["agent_system"] = serde_json::json!({
            "model_selection": {
                "budget_policy": {
                    "enforce_max_budget_units": true
                }
            },
            "routing": {
                "implementation": {
                    "max_budget_units": 1
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "junior");
        assert_eq!(assignment["max_budget_units"], 1);
        assert_eq!(assignment["budget_verdict"], "in_budget");
        assert_eq!(assignment["budget_scope"], "selection_filter_only");
        assert_eq!(
            assignment["selection_budget"]["scope"],
            "selection_filter_only"
        );
        assert_eq!(
            assignment["selection_budget"]["max_budget_units_source_path"],
            "agent_system.routing.implementation.max_budget_units"
        );
        assert_eq!(
            assignment["runtime_budget_ledger"]["status"],
            "not_tracked_by_runtime_assignment"
        );
        assert_eq!(
            assignment["selection_source_paths"]["selected_carrier_id"],
            "carrier_runtime.roles[junior].role_id"
        );
        assert_eq!(
            assignment["selection_source_paths"]["selected_model_profile_id"],
            "carrier_runtime.roles[junior].model_profiles.codex_gpt54_low_write.profile_id"
        );
        assert!(
            assignment["selection_override_reasons"]
                .as_array()
                .expect("selection override reasons should render")
                .iter()
                .any(|row| {
                    row["reason"] == "selection_budget_filtered_over_budget_candidates"
                        && row["field"] == "selected_carrier_id"
                })
        );
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "senior"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| reason.as_str() == Some("over_budget"))
                })
        );
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
                        "model_ref": "gpt-5.5",
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
                                reason.as_str() == Some("write_scope_inadmissible_for_task_class")
                            })
                })
        );
    }

    #[test]
    fn readonly_profile_is_rejected_for_regression_test_task_class() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "internal_subagents",
            "tier": "senior",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["regression_test"],
            "reasoning_band": "xhigh",
            "default_model_profile": "codex_gpt56_luna_xhigh_write",
            "model_profiles": {
                "codex_gpt56_luna_xhigh_readonly": {
                    "profile_id": "codex_gpt56_luna_xhigh_readonly",
                    "model_ref": "gpt-5.6-luna",
                    "provider": "openai-codex",
                    "reasoning_effort": "xhigh",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "read-only",
                    "runtime_roles": ["worker"],
                    "task_classes": ["regression_test"],
                    "readiness": { "required": true, "ready": true }
                },
                "codex_gpt56_luna_xhigh_write": {
                    "profile_id": "codex_gpt56_luna_xhigh_write",
                    "model_ref": "gpt-5.6-luna",
                    "provider": "openai-codex",
                    "reasoning_effort": "xhigh",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["regression_test"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "regression_test",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "codex_gpt56_luna_xhigh_write"
        );
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["model_profile_id"] == "codex_gpt56_luna_xhigh_readonly"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str() == Some("write_scope_inadmissible_for_task_class")
                            })
                })
        );
    }

    #[test]
    fn guard_required_write_scope_is_rejected_for_implementation_until_guard_is_active() {
        let compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
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
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
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
                "role_id": "pi_cli",
                "tier": "external_write_guarded",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "pi_gpt54_mini_low_guarded",
                "backend_class": "external_cli",
                "model_profiles": {
                    "pi_gpt54_mini_low_guarded": {
                        "profile_id": "pi_gpt54_mini_low_guarded",
                        "model_ref": "openai-codex/gpt-5.4-mini",
                        "provider": "pi",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 0,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "guard_required_owned_paths",
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
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
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
                    row["carrier_id"] == "pi_cli"
                        && row["model_profile_id"] == "pi_gpt54_mini_low_guarded"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str() == Some("write_scope_inadmissible_for_task_class")
                            })
                })
        );
    }

    #[test]
    fn guard_required_write_scope_is_admitted_when_pi_prewrite_guard_is_active() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 4,
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
                "role_id": "pi_cli",
                "tier": "external_write_guarded",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "pi_gpt54_mini_low_guarded",
                "backend_class": "external_cli",
                "subagent_backend_class": "external_cli",
                "dispatch": { "command": "sh" },
                "readiness": {
                    "adapter": { "mode": "command_found", "command": "sh" },
                    "provider": { "mode": "command_found", "command": "sh" },
                    "write_scope_guard": {
                        "mode": "adapter_feature_required",
                        "required_for_write_profiles": true,
                        "fail_closed_until_available": true
                    }
                },
                "model_profiles": {
                    "pi_gpt54_mini_low_guarded": {
                        "profile_id": "pi_gpt54_mini_low_guarded",
                        "model_ref": "provider-configured/test",
                        "provider": "pi",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 1,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "guard_required_owned_paths",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        let pi_backend_entry = compiled_bundle["carrier_runtime"]["roles"][1].clone();
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({"subagents": {"pi_cli": pi_backend_entry}}),
        );

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "pi_cli");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "pi_gpt54_mini_low_guarded"
        );
        assert_eq!(
            assignment["selected_external_backend_readiness"]["write_scope_guard"]["status"],
            "active"
        );
    }

    #[test]
    fn service_executor_backend_is_selected_only_through_configured_readiness_and_guard() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 4,
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
                "role_id": "vida_coder",
                "tier": "service_write_guarded",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "medium",
                "default_model_profile": "vida_coder_medium_write",
                "backend_class": "service_executor",
                "subagent_backend_class": "service_executor",
                "dispatch": { "command": "sh", "mode": "service" },
                "readiness": {
                    "adapter": { "mode": "command_found", "command": "sh" },
                    "provider": { "mode": "command_found", "command": "sh" },
                    "write_scope_guard": {
                        "mode": "adapter_feature_required",
                        "required_for_write_profiles": true,
                        "fail_closed_until_available": true
                    }
                },
                "model_profiles": {
                    "vida_coder_medium_write": {
                        "profile_id": "vida_coder_medium_write",
                        "model_ref": "vida-coder/medium",
                        "provider": "vida-coder",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "guard_required_packet_owned_paths",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        let service_backend_entry = compiled_bundle["carrier_runtime"]["roles"][1].clone();
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({"subagents": {"vida_coder": service_backend_entry}}),
        );

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "vida_coder");
        assert_eq!(assignment["selected_backend_id"], "vida_coder");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "vida_coder_medium_write"
        );
        assert_eq!(
            assignment["selected_configured_backend_readiness"]["backend_class"],
            "service_executor"
        );
        assert_eq!(
            assignment["selected_configured_backend_readiness"]["raw_provider_dispatch_forbidden"],
            true
        );
        assert_eq!(
            assignment["selected_configured_backend_readiness"]["write_scope_guard"]["status"],
            "active"
        );
        assert_eq!(
            assignment["selected_external_backend_readiness"],
            assignment["selected_configured_backend_readiness"]
        );
    }

    #[test]
    fn service_executor_backend_without_guard_fails_closed_for_implementation() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 4,
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
                "role_id": "vida_coder",
                "tier": "service_write_guarded",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "medium",
                "default_model_profile": "vida_coder_medium_write",
                "backend_class": "service_executor",
                "subagent_backend_class": "service_executor",
                "dispatch": { "command": "sh", "mode": "service" },
                "readiness": {
                    "adapter": {
                        "mode": "command_found",
                        "command": "vida-definitely-missing-service-executor-guard-command"
                    },
                    "provider": { "mode": "command_found", "command": "sh" },
                    "write_scope_guard": {
                        "mode": "adapter_feature_required",
                        "required_for_write_profiles": true,
                        "fail_closed_until_available": true,
                        "status": "blocked"
                    }
                },
                "model_profiles": {
                    "vida_coder_medium_write": {
                        "profile_id": "vida_coder_medium_write",
                        "model_ref": "vida-coder/medium",
                        "provider": "vida-coder",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "guard_required_packet_owned_paths",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        let service_backend_entry = compiled_bundle["carrier_runtime"]["roles"][1].clone();
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({"subagents": {"vida_coder": service_backend_entry}}),
        );

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "vida_coder"
                        && row["configured_backend_readiness"]["backend_class"]
                            == "service_executor"
                        && row["configured_backend_readiness"]["raw_provider_dispatch_forbidden"]
                            == true
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str() == Some("service_executor_guard_not_ready")
                            })
                })
        );
    }

    #[test]
    fn unconfigured_service_executor_backend_fails_closed_for_implementation() {
        let compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 4,
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
                "role_id": "vida_coder",
                "tier": "service_write_guarded",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "medium",
                "default_model_profile": "vida_coder_medium_write",
                "backend_class": "service_executor",
                "subagent_backend_class": "service_executor",
                "model_profiles": {
                    "vida_coder_medium_write": {
                        "profile_id": "vida_coder_medium_write",
                        "model_ref": "vida-coder/medium",
                        "provider": "vida-coder",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "guard_required_packet_owned_paths",
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
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "vida_coder"
                        && row["configured_backend_readiness"].is_null()
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str() == Some("service_executor_guard_not_ready")
                            })
                })
        );
    }

    #[test]
    fn unguarded_service_executor_write_scope_still_requires_service_guard() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_subagents",
                "tier": "senior",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "low",
                "default_model_profile": "codex_gpt54_low_write",
                "model_profiles": {
                    "codex_gpt54_low_write": {
                        "profile_id": "codex_gpt54_low_write",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "low",
                        "normalized_cost_units": 4,
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
                "role_id": "vida_coder",
                "tier": "service_write_guarded",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "worker",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"],
                "reasoning_band": "medium",
                "default_model_profile": "vida_coder_medium_write",
                "backend_class": "service_executor",
                "subagent_backend_class": "service_executor",
                "dispatch": { "command": "sh", "mode": "service" },
                "readiness": {
                    "adapter": {
                        "mode": "command_found",
                        "command": "vida-definitely-missing-service-executor-guard-command"
                    },
                    "provider": { "mode": "command_found", "command": "sh" },
                    "write_scope_guard": {
                        "mode": "adapter_feature_required",
                        "required_for_write_profiles": true,
                        "fail_closed_until_available": true
                    }
                },
                "model_profiles": {
                    "vida_coder_medium_write": {
                        "profile_id": "vida_coder_medium_write",
                        "model_ref": "vida-coder/medium",
                        "provider": "vida-coder",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "workspace-write",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        let service_backend_entry = compiled_bundle["carrier_runtime"]["roles"][1].clone();
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({"subagents": {"vida_coder": service_backend_entry}}),
        );

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "vida_coder"
                        && row["configured_backend_readiness"]["backend_class"]
                            == "service_executor"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| {
                                reason.as_str() == Some("service_executor_guard_not_ready")
                            })
                })
        );
    }

    #[test]
    fn legacy_external_backend_readiness_alias_preserves_external_blocked_status() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_support",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "internal_review",
                "model_profiles": {
                    "internal_review": {
                        "profile_id": "internal_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "vida_coder",
                "tier": "service_review",
                "rate": 1,
                "normalized_cost_units": 1,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "vida_coder_review",
                "backend_class": "service_executor",
                "subagent_backend_class": "service_executor",
                "model_profiles": {
                    "vida_coder_review": {
                        "profile_id": "vida_coder_review",
                        "model_ref": "vida-coder/review",
                        "provider": "vida-coder",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 1,
                        "speed_tier": "medium",
                        "quality_tier": "high",
                        "write_scope": "none",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        let mut service_backend_entry = compiled_bundle["carrier_runtime"]["roles"][1].clone();
        service_backend_entry["enabled"] = serde_json::Value::Bool(false);
        compiled_bundle.as_object_mut().unwrap().insert(
            "agent_system".to_string(),
            serde_json::json!({"subagents": {"vida_coder": service_backend_entry}}),
        );

        let assignment = build_runtime_assignment_preview_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_support");
        let rejected = assignment["rejected_candidates"]
            .as_array()
            .expect("rejected candidates should render")
            .iter()
            .find(|row| row["carrier_id"] == "vida_coder")
            .expect("service candidate should be rejected");
        assert_eq!(
            rejected["configured_backend_readiness"]["status"],
            "configured_backend_dispatch_blocked"
        );
        assert_eq!(
            rejected["external_backend_readiness"]["status"],
            "external_backend_dispatch_blocked"
        );
    }

    #[test]
    fn write_scope_none_remains_admissible_for_review_task_class() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "readonly_review",
            "tier": "review",
            "rate": 0,
            "normalized_cost_units": 0,
            "default_runtime_role": "coach",
            "runtime_roles": ["coach"],
            "task_classes": ["review"],
            "reasoning_band": "medium",
            "default_model_profile": "readonly_review_profile",
            "model_profiles": {
                "readonly_review_profile": {
                    "profile_id": "readonly_review_profile",
                    "model_ref": "review/model",
                    "provider": "review",
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
        })]);

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "readonly_review");
        assert_eq!(
            assignment["selected_model_profile_id"],
            "readonly_review_profile"
        );
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .is_empty()
        );
    }

    #[test]
    fn blocked_external_cli_readiness_is_rejected_before_selection() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "codex_gpt54_medium_review",
                "model_profiles": {
                    "codex_gpt54_medium_review": {
                        "profile_id": "codex_gpt54_medium_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
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
                "default_model_profile": "opencode_free_review",
                "backend_class": "external_cli",
                "model_profiles": {
                    "opencode_free_review": {
                        "profile_id": "opencode_free_review",
                        "model_ref": "opencode/free-review",
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
        compiled_bundle["agent_system"] = serde_json::json!({
            "subagents": {
                "opencode_cli": {
                    "enabled": true,
                    "subagent_backend_class": "external_cli",
                    "default_model_profile": "opencode_free_review",
                    "model_profiles": {
                        "opencode_free_review": {
                            "profile_id": "opencode_free_review",
                            "model_ref": "opencode/free-review",
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
                    },
                    "readiness": {
                        "auth": {
                            "mode": "env_present",
                            "env_var": "VIDA_TEST_MISSING_EXTERNAL_AUTH"
                        }
                    }
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "middle");
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
                            .any(|reason| reason.as_str() == Some("external_backend_not_ready"))
                        && row["external_backend_readiness"]["status"]
                            == "interactive_auth_required"
                })
        );
    }

    #[test]
    fn recent_dispatch_result_auth_failure_rejects_vibe_cli_before_selection() {
        struct CurrentDirGuard(std::path::PathBuf);
        impl Drop for CurrentDirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }

        let project_root = std::env::temp_dir().join(format!(
            "vida-vibe-dispatch-auth-assignment-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&project_root);
        let dispatch_dir =
            project_root.join(".vida/data/state/runtime-consumption/dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir should exist");
        std::fs::write(
            dispatch_dir.join("vibe.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Invalid API key",
              "recorded_at": "2026-06-25T14:00:00Z"
            }"#,
        )
        .expect("dispatch result should write");
        std::fs::write(project_root.join("AGENTS.md"), "project").expect("AGENTS should write");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            "project:\n  id: test\n",
        )
        .expect("config should write");
        for relative in [".vida/config", ".vida/db", ".vida/project"] {
            std::fs::create_dir_all(project_root.join(relative)).expect("marker dir should exist");
        }
        let _cwd_guard = CurrentDirGuard(std::env::current_dir().expect("cwd should resolve"));
        std::env::set_current_dir(&project_root).expect("cwd should switch");

        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "internal_review",
                "model_profiles": {
                    "internal_review": {
                        "profile_id": "internal_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "vibe_cli",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "backend_class": "external_cli",
                "default_model_profile": "vibe_review",
                "model_profiles": {
                    "vibe_review": {
                        "profile_id": "vibe_review",
                        "model_ref": "vibe/provider-configured",
                        "provider": "vibe",
                        "reasoning_effort": "provider_default",
                        "normalized_cost_units": 0,
                        "speed_tier": "medium",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "subagents": {
                "vibe_cli": {
                    "enabled": true,
                    "subagent_backend_class": "external_cli",
                    "default_model_profile": "vibe_review",
                    "model_profiles": compiled_bundle["carrier_runtime"]["roles"][1]["model_profiles"].clone()
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["selected_carrier_id"], "middle");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "vibe_cli"
                        && row["external_backend_readiness"]["status"] == "provider_auth_failed"
                        && row["external_backend_readiness"]["blocked"] == true
                        && row["external_backend_readiness"]["blocker_code"]
                            == "provider_auth_failed"
                })
        );
        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn missing_external_cli_detect_command_is_rejected_before_selection() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "codex_gpt54_medium_review",
                "model_profiles": {
                    "codex_gpt54_medium_review": {
                        "profile_id": "codex_gpt54_medium_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "hermes_cli",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "hermes_provider_configured_review",
                "backend_class": "external_cli",
                "model_profiles": {
                    "hermes_provider_configured_review": {
                        "profile_id": "hermes_provider_configured_review",
                        "model_ref": "hermes/provider-configured",
                        "provider": "hermes",
                        "reasoning_effort": "provider_default",
                        "normalized_cost_units": 0,
                        "speed_tier": "medium",
                        "quality_tier": "medium",
                        "write_scope": "none",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
        ]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "subagents": {
                "hermes_cli": {
                    "enabled": true,
                    "subagent_backend_class": "external_cli",
                    "detect_command": "vida-definitely-missing-external-cli-command-for-test",
                    "default_model_profile": "hermes_provider_configured_review",
                    "model_profiles": {
                        "hermes_provider_configured_review": {
                            "profile_id": "hermes_provider_configured_review",
                            "model_ref": "hermes/provider-configured",
                            "provider": "hermes",
                            "reasoning_effort": "provider_default",
                            "normalized_cost_units": 0,
                            "speed_tier": "medium",
                            "quality_tier": "medium",
                            "write_scope": "none",
                            "runtime_roles": ["coach"],
                            "task_classes": ["review"],
                            "readiness": { "required": true, "ready": true }
                        }
                    }
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "middle");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "hermes_cli"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| reason.as_str() == Some("external_backend_not_ready"))
                        && row["external_backend_readiness"]["status"]
                            == "external_cli_command_not_found"
                        && row["external_backend_readiness"]["blocker_code"]
                            == "tool_execution_failed"
                })
        );
    }

    #[test]
    fn disabled_configured_external_backend_is_rejected_before_preview_selection() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "internal_support",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "internal_review",
                "model_profiles": {
                    "internal_review": {
                        "profile_id": "internal_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
                        "runtime_roles": ["coach"],
                        "task_classes": ["review"],
                        "readiness": { "required": true, "ready": true }
                    }
                }
            }),
            serde_json::json!({
                "role_id": "external_bridge",
                "tier": "external_free",
                "rate": 0,
                "normalized_cost_units": 0,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "external_review",
                "model_profiles": {
                    "external_review": {
                        "profile_id": "external_review",
                        "model_ref": "provider/free-review",
                        "provider": "provider",
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
        compiled_bundle["agent_system"] = serde_json::json!({
            "subagents": {
                "external_bridge": {
                    "enabled": false,
                    "subagent_backend_class": "external_cli",
                    "default_model_profile": "external_review",
                    "model_profiles": {
                        "external_review": {
                            "profile_id": "external_review",
                            "model_ref": "provider/free-review",
                            "provider": "provider",
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
                }
            }
        });

        let assignment = build_runtime_assignment_preview_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "review",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_support");
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "external_bridge"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| reason.as_str() == Some("external_backend_not_ready"))
                        && row["external_backend_readiness"]["blocked"] == true
                        && row["external_backend_readiness"]["blocker_code"]
                            == "configured_backend_dispatch_failed"
                })
        );
    }

    #[test]
    fn external_cli_readiness_override_remains_admissible() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![
            serde_json::json!({
                "role_id": "middle",
                "tier": "middle",
                "rate": 4,
                "normalized_cost_units": 4,
                "default_runtime_role": "coach",
                "runtime_roles": ["coach"],
                "task_classes": ["review"],
                "reasoning_band": "medium",
                "default_model_profile": "codex_gpt54_medium_review",
                "model_profiles": {
                    "codex_gpt54_medium_review": {
                        "profile_id": "codex_gpt54_medium_review",
                        "model_ref": "gpt-5.5",
                        "provider": "openai",
                        "reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "speed_tier": "fast",
                        "quality_tier": "medium",
                        "write_scope": "read_or_review",
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
                "default_model_profile": "opencode_free_review",
                "backend_class": "external_cli",
                "model_profiles": {
                    "opencode_free_review": {
                        "profile_id": "opencode_free_review",
                        "model_ref": "opencode/free-review",
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
        compiled_bundle["agent_system"] = serde_json::json!({
            "subagents": {
                "opencode_cli": {
                    "enabled": true,
                    "subagent_backend_class": "external_cli",
                    "dispatch": {
                        "model_flag": "--model"
                    },
                    "default_model_profile": "opencode_free_review",
                    "model_profiles": {
                        "opencode_free_review": {
                            "profile_id": "opencode_free_review",
                            "model_ref": "opencode/free-review",
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
                    },
                    "readiness": {
                        "auth": {
                            "mode": "none"
                        },
                        "model": {
                            "mode": "none",
                            "allow_dispatch_override": true
                        }
                    }
                }
            }
        });

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
            "opencode_free_review"
        );
        assert_eq!(
            assignment["selected_model_profile_readiness_status"],
            "carrier_ready_with_override"
        );
        assert_eq!(
            assignment["selected_external_backend_readiness"]["status"],
            "carrier_ready_with_override"
        );
    }

    #[test]
    fn canonical_task_class_declaration_satisfies_alias_assignment_request() {
        let compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "middle",
            "tier": "middle",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "coach",
            "runtime_roles": ["coach"],
            "task_classes": ["coach"],
            "reasoning_band": "medium",
            "default_model_profile": "coach_validation",
            "model_profiles": {
                "coach_validation": {
                    "profile_id": "coach_validation",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "read_or_review",
                    "runtime_roles": ["coach"],
                    "task_classes": ["coach"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);

        let assignment = build_runtime_assignment_preview_from_resolved_constraints(
            &compiled_bundle,
            "coach",
            "validation",
            "coach",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "middle");
        assert_eq!(assignment["task_class"], "validation");
        assert_eq!(assignment["runtime_role"], "coach");
    }

    #[test]
    fn route_level_profile_mapping_constrains_selected_carrier_profile() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "internal_subagents",
            "tier": "senior_internal",
            "rate": 10,
            "normalized_cost_units": 10,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker", "verifier"],
            "task_classes": ["implementation", "verification"],
            "reasoning_band": "high",
            "default_model_profile": "internal_review",
            "model_profiles": {
                "internal_fast": {
                    "profile_id": "internal_fast",
                    "model_ref": "internal_fast",
                    "provider": "internal",
                    "reasoning_effort": "low",
                    "normalized_cost_units": 6,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "orchestrator_native",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                },
                "internal_review": {
                    "profile_id": "internal_review",
                    "model_ref": "internal_review",
                    "provider": "internal",
                    "reasoning_effort": "high",
                    "normalized_cost_units": 8,
                    "speed_tier": "medium",
                    "quality_tier": "high",
                    "write_scope": "read_only",
                    "runtime_roles": ["verifier"],
                    "task_classes": ["verification"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "routing": {
                "implementation": {
                    "profiles": {
                        "internal_subagents": "internal_fast"
                    }
                }
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "implementation",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["selected_carrier_id"], "internal_subagents");
        assert_eq!(assignment["selected_model_profile_id"], "internal_fast");
        assert_eq!(
            assignment["selected_route_profile_mapping"],
            "internal_fast"
        );
        assert_eq!(assignment["route_profile_mapping_applied"], true);
        assert_eq!(
            assignment["selection_source_paths"]["selected_route_profile_mapping"],
            "agent_system.routing.implementation.profiles.internal_subagents"
        );
        assert!(
            assignment["selection_override_reasons"]
                .as_array()
                .expect("selection override reasons should render")
                .iter()
                .any(|row| {
                    row["reason"] == "route_profile_mapping_applied"
                        && row["field"] == "selected_model_profile_id"
                        && row["source_path"]
                            == "agent_system.routing.implementation.profiles.internal_subagents"
                })
        );
        assert!(
            assignment["rejected_candidates"]
                .as_array()
                .expect("rejected candidates should render")
                .iter()
                .any(|row| {
                    row["carrier_id"] == "internal_subagents"
                        && row["model_profile_id"] == "internal_review"
                        && row["reasons"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|reason| reason.as_str() == Some("route_profile_mapping_mismatch"))
                })
        );
    }

    #[test]
    fn disabled_model_selection_returns_disabled_runtime_assignment() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "middle",
            "tier": "middle",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "medium",
            "default_model_profile": "codex_gpt54_medium_write",
            "model_profiles": {
                "codex_gpt54_medium_write": {
                    "profile_id": "codex_gpt54_medium_write",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "model_selection": {
                "enabled": false,
                "candidate_scope": "unified_carrier_model_profiles"
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "model_selection_disabled");
        assert_eq!(assignment["model_selection_enabled"], false);
        assert_eq!(
            assignment["candidate_scope"],
            "unified_carrier_model_profiles"
        );
        assert!(assignment["selected_carrier_id"].is_null());
    }

    #[test]
    fn unsupported_candidate_scope_fail_closes_runtime_assignment() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "middle",
            "tier": "middle",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "medium",
            "default_model_profile": "codex_gpt54_medium_write",
            "model_profiles": {
                "codex_gpt54_medium_write": {
                    "profile_id": "codex_gpt54_medium_write",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["agent_system"] = serde_json::json!({
            "model_selection": {
                "enabled": true,
                "candidate_scope": "legacy_route_backends_only"
            }
        });

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "candidate_scope_not_supported");
        assert_eq!(assignment["model_selection_enabled"], true);
        assert_eq!(assignment["candidate_scope"], "legacy_route_backends_only");
        assert_eq!(
            assignment["supported_candidate_scope"],
            "unified_carrier_model_profiles"
        );
    }

    #[test]
    fn unsupported_selection_strategy_fail_closes_runtime_assignment() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "middle",
            "tier": "middle",
            "rate": 4,
            "normalized_cost_units": 4,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "reasoning_band": "medium",
            "default_model_profile": "codex_gpt54_medium_write",
            "model_profiles": {
                "codex_gpt54_medium_write": {
                    "profile_id": "codex_gpt54_medium_write",
                    "model_ref": "gpt-5.5",
                    "provider": "openai",
                    "reasoning_effort": "medium",
                    "normalized_cost_units": 4,
                    "speed_tier": "fast",
                    "quality_tier": "medium",
                    "write_scope": "workspace-write",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": { "required": true, "ready": true }
                }
            }
        })]);
        compiled_bundle["carrier_runtime"]["model_selection"]["default_strategy"] =
            serde_json::json!("unknown_strategy");

        let assignment = build_runtime_assignment_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(assignment["reason"], "selection_strategy_not_supported");
        assert_eq!(assignment["selection_strategy"], "unknown_strategy");
        assert_eq!(
            assignment["selection_strategy_source_path"],
            "carrier_runtime.model_selection.default_strategy"
        );
        assert!(
            assignment["supported_selection_strategies"]
                .as_array()
                .expect("supported strategies should render")
                .iter()
                .any(|strategy| strategy == "balanced_cost_quality")
        );
    }

    #[test]
    fn missing_ranking_policy_fail_closes_runtime_assignment() {
        let mut compiled_bundle = compiled_bundle_with_roles(vec![serde_json::json!({
            "role_id": "junior",
            "tier": "junior",
            "rate": 1,
            "normalized_cost_units": 1,
            "default_runtime_role": "worker",
            "runtime_roles": ["worker"],
            "task_classes": ["implementation"],
            "default_model_profile": "configured",
            "model_profiles": {
                "configured": {
                    "profile_id": "configured",
                    "model_ref": "config/model",
                    "reasoning_effort": "medium",
                    "quality_tier": "medium",
                    "speed_tier": "fast",
                    "normalized_cost_units": 1,
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"],
                    "readiness": {"required": true, "ready": true}
                }
            }
        })]);
        compiled_bundle["carrier_runtime"]["model_selection"]
            .as_object_mut()
            .expect("test policy should be object")
            .remove("ordinal_ranks");

        let assignment = build_runtime_assignment_preview_from_resolved_constraints(
            &compiled_bundle,
            "worker",
            "implementation",
            "worker",
        );

        assert_eq!(assignment["enabled"], false);
        assert_eq!(
            assignment["reason"],
            "model_selection_ranking_policy_missing"
        );
        assert!(assignment["blocker_codes"].as_array().is_some_and(|codes| {
            codes
                .iter()
                .any(|code| code == "model_selection_ranking_policy_missing")
        }));
    }
}
