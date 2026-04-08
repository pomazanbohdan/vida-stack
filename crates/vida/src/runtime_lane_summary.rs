use crate::{
    build_runtime_execution_plan_from_snapshot, json_bool, json_lookup, json_string,
    json_string_list, read_or_sync_launcher_activation_snapshot, RuntimeConsumptionLaneSelection,
    StateStore,
};
use crate::taskflow_routing::{
    explicit_executor_backend_from_route, fallback_executor_backend_from_route,
    fanout_executor_backends_from_route,
};

pub(crate) fn build_runtime_lane_selection_from_bundle(
    bundle: &serde_json::Value,
    activation_source: &str,
    pack_router_keywords: &serde_json::Value,
    request: &str,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let selection_mode = json_string(json_lookup(bundle, &["role_selection", "mode"]))
        .unwrap_or_else(|| "fixed".to_string());
    let configured_fallback =
        json_string(json_lookup(bundle, &["role_selection", "fallback_role"]))
            .unwrap_or_else(|| "orchestrator".to_string());
    if !role_exists_in_lane_bundle(bundle, &configured_fallback) {
        return Err(format!(
            "Agent extension bundle validation failed: fallback role `{configured_fallback}` is unresolved."
        ));
    }
    let fallback_role = configured_fallback;
    let normalized_request = request.to_lowercase();
    let mut result = RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: activation_source.to_string(),
        selection_mode: selection_mode.clone(),
        fallback_role: fallback_role.clone(),
        request: request.to_string(),
        selected_role: fallback_role.clone(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "fallback".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: String::new(),
    };

    if selection_mode != "auto" {
        result.reason = "fixed_mode".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }

    let Some(serde_json::Value::Object(conversation_modes)) =
        json_lookup(bundle, &["role_selection", "conversation_modes"])
    else {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    };
    if normalized_request.trim().is_empty() {
        result.reason = "auto_no_modes_or_empty_request".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }

    let mut candidates = Vec::new();
    for (mode_key, mode_value) in conversation_modes {
        let mode_id = mode_key.as_str();
        let serde_json::Value::Object(_) = mode_value else {
            continue;
        };
        if !json_bool(json_lookup(mode_value, &["enabled"]), true) {
            continue;
        }

        let mut keywords = match mode_id {
            "scope_discussion" => vec![
                "scope",
                "scoping",
                "requirement",
                "requirements",
                "acceptance",
                "constraint",
                "constraints",
                "clarify",
                "clarification",
                "discover",
                "discovery",
                "spec",
                "specification",
                "user story",
                "ac",
            ]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
            "pbi_discussion" => vec![
                "pbi",
                "backlog",
                "priority",
                "prioritize",
                "prioritization",
                "task",
                "ticket",
                "delivery cut",
                "estimate",
                "estimation",
                "roadmap",
                "decompose",
                "decomposition",
                "work pool",
            ]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let extra_keys: &[&str] = match mode_id {
            "scope_discussion" => &["spec"],
            "pbi_discussion" => &["pool", "pool_strong", "pool_dependency"],
            _ => &[],
        };
        for key in extra_keys {
            for keyword in json_string_list(json_lookup(pack_router_keywords, &[*key])) {
                if !keywords.contains(&keyword) {
                    keywords.push(keyword);
                }
            }
        }

        let matched_terms = contains_keywords(&normalized_request, &keywords);
        let selected_role = json_string(json_lookup(mode_value, &["role"]))
            .unwrap_or_else(|| fallback_role.clone());
        if !role_exists_in_lane_bundle(bundle, &selected_role) {
            return Err(format!(
                "Agent extension bundle validation failed: conversation mode `{mode_id}` references unresolved role `{selected_role}`."
            ));
        }
        let tracked_flow_entry = json_string(json_lookup(mode_value, &["tracked_flow_entry"]));
        if let Some(flow_id) = tracked_flow_entry.as_deref() {
            if !tracked_flow_target_exists(bundle, flow_id) {
                return Err(format!(
                    "Agent extension bundle validation failed: conversation mode `{mode_id}` references unresolved tracked flow entry `{flow_id}`."
                ));
            }
        }
        candidates.push((
            mode_id.to_string(),
            selected_role,
            json_bool(json_lookup(mode_value, &["single_task_only"]), false),
            tracked_flow_entry,
            json_bool(json_lookup(mode_value, &["allow_freeform_chat"]), false),
            matched_terms,
        ));
    }

    if candidates.is_empty() {
        result.reason = "auto_no_enabled_modes".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }

    candidates.sort_by(|a, b| b.5.len().cmp(&a.5.len()).then_with(|| a.0.cmp(&b.0)));
    let selected = &candidates[0];
    if selected.5.is_empty() {
        let feature_terms = feature_delivery_design_terms(&normalized_request);
        if !feature_terms.is_empty() {
            if let Some(scope_candidate) = candidates.iter().find(|row| row.0 == "scope_discussion")
            {
                result.selected_role = scope_candidate.1.clone();
                result.conversational_mode = Some(scope_candidate.0.clone());
                result.single_task_only = scope_candidate.2;
                result.tracked_flow_entry = scope_candidate.3.clone();
                result.allow_freeform_chat = scope_candidate.4;
                result.matched_terms = feature_terms.clone();
                result.confidence = if feature_terms.len() >= 4 {
                    "high".to_string()
                } else {
                    "medium".to_string()
                };
                result.reason = "auto_feature_design_request".to_string();
                result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
                return Ok(result);
            }
        }

        result.reason = "auto_no_keyword_match".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }
    if !role_exists_in_lane_bundle(bundle, &selected.1) {
        result.reason = "auto_selected_unknown_role".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }

    result.selected_role = selected.1.clone();
    result.conversational_mode = Some(selected.0.clone());
    result.single_task_only = selected.2;
    result.tracked_flow_entry = selected.3.clone();
    result.allow_freeform_chat = selected.4;
    result.matched_terms = selected.5.clone();
    result.confidence = match selected.5.len() {
        0 => "fallback".to_string(),
        1 => "low".to_string(),
        2 => "medium".to_string(),
        _ => "high".to_string(),
    };
    result.reason = "auto_keyword_match".to_string();
    result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
    Ok(result)
}

pub(crate) async fn build_runtime_lane_selection_with_store(
    store: &StateStore,
    request: &str,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
    build_runtime_lane_selection_from_bundle(
        &snapshot.compiled_bundle,
        &snapshot.source,
        &snapshot.pack_router_keywords,
        request,
    )
}

pub(crate) fn summarize_agent_route_from_snapshot(
    compiled_bundle: &serde_json::Value,
    agent_system: &serde_json::Value,
    route_id: &str,
) -> serde_json::Value {
    let Some(route) = json_lookup(agent_system, &["routing", route_id]) else {
        return serde_json::Value::Null;
    };
    let (runtime_role, task_class) = match route_id {
        "implementation" | "small_patch" | "small_patch_write" | "ui_patch" => {
            ("worker", "implementation")
        }
        "coach" => ("coach", "coach"),
        "verification" | "verification_ensemble" | "review_ensemble" => {
            ("verifier", "verification")
        }
        "architecture" => ("solution_architect", "architecture"),
        _ => ("", ""),
    };
    let runtime_assignment = if runtime_role.is_empty() || task_class.is_empty() {
        serde_json::Value::Null
    } else {
        crate::runtime_assignment_builder::build_runtime_assignment_from_resolved_constraints(
            compiled_bundle,
            route_id,
            task_class,
            runtime_role,
        )
    };
    let executor_backend = crate::taskflow_routing::selected_backend_from_execution_plan_route(
        &serde_json::Value::Null,
        route,
    )
    .unwrap_or_default();
    let fallback_executor_backend = fallback_executor_backend_from_route(route).unwrap_or_default();
    let fanout_executor_backends = fanout_executor_backends_from_route(route);
    let legacy_fanout_subagents = if fanout_executor_backends.is_empty() {
        json_string_list(json_lookup(route, &["fanout_subagents"]))
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
    } else {
        fanout_executor_backends.clone()
    };
    let mut route_summary = serde_json::json!({
        "route_id": route_id,
        "carrier_backend_hint": executor_backend.clone(),
        "executor_backend": explicit_executor_backend_from_route(route)
            .or_else(|| json_string(json_lookup(route, &["subagents"])))
            .or_else(|| json_string(json_lookup(route, &["carrier_backend_hint"])))
            .unwrap_or_default(),
        "fallback_executor_backend": fallback_executor_backend.clone(),
        "fanout_executor_backends": serde_json::json!(fanout_executor_backends),
        "subagents": json_string(json_lookup(route, &["subagents"])).unwrap_or_default(),
        "bridge_fallback_subagent": json_string(json_lookup(route, &["bridge_fallback_subagent"]))
            .unwrap_or_default(),
        "fanout_subagents": legacy_fanout_subagents.join(", "),
        "preferred_agent_type": runtime_assignment["selected_agent_id"],
        "preferred_agent_tier": runtime_assignment["selected_tier"],
        "preferred_runtime_role": runtime_assignment["runtime_role"],
        "profiles": json_lookup(route, &["profiles"]).cloned().unwrap_or(serde_json::Value::Null),
        "write_scope": json_string(json_lookup(route, &["write_scope"])).unwrap_or_default(),
        "dispatch_required": json_string(json_lookup(route, &["dispatch_required"])).unwrap_or_default(),
        "verification_gate": json_string(json_lookup(route, &["verification_gate"])).unwrap_or_default(),
        "analysis_required": json_bool(json_lookup(route, &["analysis_required"]), false),
        "analysis_route_task_class": json_string(json_lookup(route, &["analysis_route_task_class"])).unwrap_or_default(),
        "coach_required": json_bool(json_lookup(route, &["coach_required"]), false),
        "coach_route_task_class": json_string(json_lookup(route, &["coach_route_task_class"])).unwrap_or_default(),
        "verification_route_task_class": json_string(json_lookup(route, &["verification_route_task_class"])).unwrap_or_default(),
        "independent_verification_required": json_bool(json_lookup(route, &["independent_verification_required"]), false),
        "graph_strategy": json_string(json_lookup(route, &["graph_strategy"])).unwrap_or_default(),
        "internal_escalation_trigger": json_string(json_lookup(route, &["internal_escalation_trigger"])).unwrap_or_default(),
    });
    if let Some(summary) = route_summary.as_object_mut() {
        summary.extend(crate::runtime_assignment_alias_fields(&runtime_assignment));
    }
    route_summary
}

#[cfg(test)]
mod tests {
    use super::summarize_agent_route_from_snapshot;

    #[test]
    fn summarize_agent_route_prefers_explicit_executor_fields() {
        let compiled_bundle = serde_json::json!({
            "carrier_runtime": {
                "roles": []
            }
        });
        let agent_system = serde_json::json!({
            "routing": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "internal_review",
                    "fanout_executor_backends": ["internal_fast", "internal_arch"],
                    "carrier_backend_hint": "legacy_hint",
                    "subagents": "legacy_subagents",
                    "bridge_fallback_subagent": "legacy_bridge",
                    "fanout_subagents": "legacy_fanout",
                    "profiles": {
                        "internal_subagents": "internal_fast"
                    },
                    "write_scope": "none",
                    "dispatch_required": "external_first_when_eligible",
                    "verification_gate": "source_backed_summary",
                    "analysis_required": false,
                    "analysis_route_task_class": "",
                    "coach_required": false,
                    "coach_route_task_class": "",
                    "verification_route_task_class": "",
                    "independent_verification_required": false,
                    "graph_strategy": "deterministic_then_escalate",
                    "internal_escalation_trigger": "provider_exhausted_or_decision_conflict"
                }
            }
        });

        let summary =
            summarize_agent_route_from_snapshot(&compiled_bundle, &agent_system, "implementation");

        assert_eq!(summary["carrier_backend_hint"], "internal_subagents");
        assert_eq!(summary["executor_backend"], "internal_subagents");
        assert_eq!(summary["fallback_executor_backend"], "internal_review");
        assert_eq!(
            summary["fanout_executor_backends"],
            serde_json::json!(["internal_fast", "internal_arch"])
        );
        assert_eq!(summary["subagents"], "legacy_subagents");
        assert_eq!(summary["bridge_fallback_subagent"], "legacy_bridge");
        assert_eq!(summary["fanout_subagents"], "internal_fast, internal_arch");
    }
}

pub(crate) fn role_exists_in_lane_bundle(bundle: &serde_json::Value, role_id: &str) -> bool {
    if role_id.is_empty() {
        return false;
    }

    bundle["enabled_framework_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .any(|value| value == role_id)
        || bundle["project_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role_id"].as_str())
            .any(|value| value == role_id)
}

fn known_tracked_flow_targets() -> &'static [&'static str] {
    &[
        "research-pack",
        "spec-pack",
        "work-pool-pack",
        "dev-pack",
        "bug-pool-pack",
        "reflection-pack",
    ]
}

fn bundle_project_flow_exists(bundle: &serde_json::Value, flow_id: &str) -> bool {
    bundle["project_flows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["flow_id"].as_str())
        .any(|value| value == flow_id)
}

pub(crate) fn tracked_flow_target_exists(bundle: &serde_json::Value, flow_id: &str) -> bool {
    known_tracked_flow_targets().contains(&flow_id) || bundle_project_flow_exists(bundle, flow_id)
}

fn contains_keywords(request: &str, keywords: &[String]) -> Vec<String> {
    crate::contains_keywords(request, keywords)
}

fn feature_delivery_design_terms(request: &str) -> Vec<String> {
    crate::feature_delivery_design_terms(request)
}
