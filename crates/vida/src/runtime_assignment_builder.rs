use crate::{
    carrier_runtime_section, infer_execution_runtime_role, infer_runtime_task_class, json_lookup,
    json_u64, role_supports_runtime_role, role_supports_task_class, runtime_role_for_task_class,
    task_complexity_multiplier, RuntimeConsumptionLaneSelection,
};

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

    let mut candidates = roles
        .iter()
        .filter_map(|role| {
            let role_id = role["role_id"].as_str()?;
            let rate = role["rate"].as_u64().unwrap_or(0);
            if rate == 0 {
                return None;
            }
            let strategy = &carrier_runtime["worker_strategy"]["agents"][role_id];
            let effective_score =
                json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
            let lifecycle_state = strategy["lifecycle_state"].as_str().unwrap_or("probation");
            let supports_runtime_role = role_supports_runtime_role(role, execution_runtime_role);
            let supports_task_class = role_supports_task_class(role, task_class);
            Some((
                !supports_runtime_role,
                !supports_task_class,
                effective_score < demotion_score || lifecycle_state == "retired",
                rate,
                std::cmp::Reverse(effective_score),
                role.clone(),
                strategy.clone(),
            ))
        })
        .collect::<Vec<_>>();

    let has_exact_match =
        candidates
            .iter()
            .any(|(runtime_role_miss, task_class_miss, _, _, _, _, _)| {
                !*runtime_role_miss && !*task_class_miss
            });
    if !has_exact_match {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_carrier_declares_runtime_role_and_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role
        });
    }

    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
    });
    let Some((_, _, _, _, _, selected_role, strategy)) = candidates.first() else {
        return serde_json::json!({
            "enabled": false,
            "reason": "no_carrier_satisfies_runtime_role_or_task_class",
            "task_class": task_class,
            "runtime_role": execution_runtime_role,
            "conversation_role": conversation_role
        });
    };

    let tier = selected_role["tier"].as_str().unwrap_or_default();
    let rate = selected_role["rate"].as_u64().unwrap_or(0);
    let complexity_multiplier = task_complexity_multiplier(task_class);
    let effective_score = json_u64(json_lookup(strategy, &["effective_score"])).unwrap_or(70);
    let lifecycle_state = strategy["lifecycle_state"].as_str().unwrap_or("probation");
    let rationale = vec![
        format!("task_class={task_class}"),
        format!("conversation_role={conversation_role}"),
        format!("execution_runtime_role={execution_runtime_role}"),
        format!("selected_tier={tier}"),
        format!("effective_score={effective_score}"),
        format!("lifecycle_state={lifecycle_state}"),
        "selection_rule=capability_first_then_score_guard_then_cheapest_tier".to_string(),
    ];

    serde_json::json!({
        "enabled": true,
        "task_class": task_class,
        "runtime_role": execution_runtime_role,
        "conversation_role": conversation_role,
        "activation_agent_type": selected_role["role_id"],
        "activation_runtime_role": execution_runtime_role,
        "selected_agent_id": selected_role["role_id"],
        "selected_carrier_agent_id": selected_role["role_id"],
        "selected_tier": selected_role["tier"],
        "selected_carrier_tier": selected_role["tier"],
        "selected_runtime_role": execution_runtime_role,
        "tier_default_runtime_role": selected_role["default_runtime_role"],
        "reasoning_band": selected_role["reasoning_band"],
        "model_reasoning_effort": selected_role["model_reasoning_effort"],
        "sandbox_mode": selected_role["sandbox_mode"],
        "rate": rate,
        "estimated_task_price_units": rate * complexity_multiplier,
        "complexity_multiplier": complexity_multiplier,
        "effective_score": effective_score,
        "lifecycle_state": lifecycle_state,
        "strategy_store": carrier_runtime["worker_strategy"]["store_path"],
        "scorecards_store": carrier_runtime["worker_strategy"]["scorecards_path"],
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
