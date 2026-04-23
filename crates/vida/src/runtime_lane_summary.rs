use crate::taskflow_routing::{
    backend_selection_source, explicit_executor_backend_from_route,
    fallback_executor_backend_from_route, fanout_executor_backends_from_route,
    route_primary_backend_hint_from_route, runtime_assignment_backend_for_route,
};
use crate::{
    json_bool, json_lookup, json_string, json_string_list,
    read_or_sync_launcher_activation_snapshot, StateStore,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct RuntimeConsumptionLaneSelection {
    pub(crate) ok: bool,
    pub(crate) activation_source: String,
    pub(crate) selection_mode: String,
    pub(crate) fallback_role: String,
    pub(crate) request: String,
    pub(crate) selected_role: String,
    pub(crate) conversational_mode: Option<String>,
    pub(crate) single_task_only: bool,
    pub(crate) tracked_flow_entry: Option<String>,
    pub(crate) allow_freeform_chat: bool,
    pub(crate) confidence: String,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) compiled_bundle: serde_json::Value,
    pub(crate) execution_plan: serde_json::Value,
    pub(crate) reason: String,
}

pub(crate) fn build_runtime_execution_plan_from_snapshot(
    compiled_bundle: &serde_json::Value,
    selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    crate::development_flow_orchestration::build_runtime_execution_plan_from_snapshot(
        compiled_bundle,
        selection,
    )
}

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

    let feature_terms = feature_delivery_design_terms(&normalized_request);
    let implementation_terms = explicit_implementation_request_terms(&normalized_request);
    let verification_terms = explicit_verification_request_terms(&normalized_request);
    let bounded_repair_terms = explicit_bounded_code_repair_terms(&normalized_request);
    candidates.sort_by(|a, b| b.5.len().cmp(&a.5.len()).then_with(|| a.0.cmp(&b.0)));
    let selected = &candidates[0];
    if selected.5.is_empty() {
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

        if !verification_terms.is_empty() && role_exists_in_lane_bundle(bundle, "verifier") {
            result.selected_role = "verifier".to_string();
            result.matched_terms = verification_terms.clone();
            result.confidence = if verification_terms.len() >= 3 {
                "high".to_string()
            } else {
                "medium".to_string()
            };
            result.reason = "auto_explicit_verification_request".to_string();
            result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
            return Ok(result);
        }

        if !implementation_terms.is_empty()
            && verification_terms.is_empty()
            && role_exists_in_lane_bundle(bundle, "worker")
        {
            result.selected_role = "worker".to_string();
            result.matched_terms = implementation_terms.clone();
            result.confidence = if implementation_terms.len() >= 3 {
                "high".to_string()
            } else {
                "medium".to_string()
            };
            result.reason = "auto_explicit_implementation_request".to_string();
            result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
            return Ok(result);
        }

        if !bounded_repair_terms.is_empty()
            && verification_terms.is_empty()
            && role_exists_in_lane_bundle(bundle, "worker")
        {
            result.selected_role = "worker".to_string();
            result.matched_terms = bounded_repair_terms.clone();
            result.confidence = if bounded_repair_terms.len() >= 3 {
                "high".to_string()
            } else {
                "medium".to_string()
            };
            result.reason = "auto_explicit_implementation_request".to_string();
            result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
            return Ok(result);
        }

        result.reason = "auto_no_keyword_match".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }
    let selected_mode = selected.0.as_str();
    let selected_is_weak_pbi_discussion = selected_mode == "pbi_discussion"
        && selected
            .5
            .iter()
            .all(|term| weak_pbi_discussion_term(term.as_str()));
    if selected_is_weak_pbi_discussion
        && !verification_terms.is_empty()
        && role_exists_in_lane_bundle(bundle, "verifier")
    {
        result.selected_role = "verifier".to_string();
        result.matched_terms = verification_terms.clone();
        result.confidence = if verification_terms.len() >= 3 {
            "high".to_string()
        } else {
            "medium".to_string()
        };
        result.reason = "auto_explicit_verification_request_override".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }
    if selected_is_weak_pbi_discussion
        && !implementation_terms.is_empty()
        && verification_terms.is_empty()
        && role_exists_in_lane_bundle(bundle, "worker")
    {
        result.selected_role = "worker".to_string();
        result.matched_terms = implementation_terms.clone();
        result.confidence = if implementation_terms.len() >= 3 {
            "high".to_string()
        } else {
            "medium".to_string()
        };
        result.reason = "auto_explicit_implementation_request_override".to_string();
        result.execution_plan = build_runtime_execution_plan_from_snapshot(bundle, &result);
        return Ok(result);
    }
    if matches!(selected_mode, "scope_discussion" | "pbi_discussion")
        && !bounded_repair_terms.is_empty()
        && verification_terms.is_empty()
        && role_exists_in_lane_bundle(bundle, "worker")
    {
        result.selected_role = "worker".to_string();
        result.matched_terms = bounded_repair_terms.clone();
        result.confidence = if bounded_repair_terms.len() >= 3 {
            "high".to_string()
        } else {
            "medium".to_string()
        };
        result.reason = "auto_explicit_implementation_request_override".to_string();
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

fn backend_capability_list(entry: &serde_json::Value, field: &str) -> Vec<String> {
    json_string_list(json_lookup(entry, &[field]))
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn backend_has_any(values: &[String], candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| values.iter().any(|value| value == candidate))
}

fn build_backend_lane_admissibility(entry: &serde_json::Value) -> serde_json::Value {
    let backend_class = json_string(json_lookup(entry, &["subagent_backend_class"]))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let write_scope = json_string(json_lookup(entry, &["write_scope"]))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let capabilities = backend_capability_list(entry, "capability_band");
    let specialties = backend_capability_list(entry, "specialties");
    let internal_backend = backend_class == "internal";
    let read_only_capable = internal_backend
        || backend_has_any(
            &capabilities,
            &[
                "read_only",
                "review_safe",
                "web_search",
                "architecture_safe",
            ],
        )
        || !specialties.is_empty();
    let execution_preparation_capable = internal_backend
        || read_only_capable
        || backend_has_any(
            &specialties,
            &["planning", "spec", "architecture", "long_context"],
        );
    let implementation_capable = write_scope != "none"
        && (internal_backend
            || backend_has_any(&capabilities, &["implementation_safe"])
            || backend_has_any(&specialties, &["implementation", "integration"]));
    let coach_capable = internal_backend
        || backend_has_any(&capabilities, &["review_safe"])
        || backend_has_any(&specialties, &["review", "planning", "spec"]);
    let review_capable = coach_capable;
    let verification_capable = internal_backend || backend_has_any(&specialties, &["verification"]);
    let review_only_backend = (coach_capable || review_capable) && !implementation_capable;

    serde_json::json!({
        "analysis": read_only_capable,
        "execution_preparation": execution_preparation_capable,
        "implementation": implementation_capable,
        "coach": coach_capable,
        "review": review_capable,
        "verification": verification_capable,
        "policy_flags": {
            "internal_only_backend": internal_backend,
            "read_only_backend": write_scope == "none",
            "review_only_backend": review_only_backend,
            "scoped_write_backend": write_scope == "scoped_only",
        }
    })
}

fn backend_policy_entry(backend_id: &str, entry: &serde_json::Value) -> serde_json::Value {
    let capabilities = backend_capability_list(entry, "capability_band");
    let specialties = backend_capability_list(entry, "specialties");
    serde_json::json!({
        "backend_id": backend_id,
        "backend_class": json_string(json_lookup(entry, &["subagent_backend_class"])).unwrap_or_default(),
        "write_scope": json_string(json_lookup(entry, &["write_scope"])).unwrap_or_default(),
        "capability_band": capabilities,
        "specialties": specialties,
        "lane_admissibility": build_backend_lane_admissibility(entry),
    })
}

fn backend_policy_by_id(agent_system: &serde_json::Value, backend_id: &str) -> serde_json::Value {
    json_lookup(agent_system, &["subagents", backend_id])
        .filter(|entry| json_bool(json_lookup(entry, &["enabled"]), false))
        .map(|entry| backend_policy_entry(backend_id, entry))
        .unwrap_or(serde_json::Value::Null)
}

fn backend_policy_from_execution_plan(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> serde_json::Value {
    if backend_id.trim().is_empty() {
        return serde_json::Value::Null;
    }
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn backend_policy_class(policy: &serde_json::Value) -> Option<&str> {
    policy
        .get("backend_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn route_has_external_backend(
    route_primary_policy: &serde_json::Value,
    fallback_policy: &serde_json::Value,
    fanout_policies: &[serde_json::Value],
) -> bool {
    backend_policy_class(route_primary_policy) == Some("external_cli")
        || backend_policy_class(fallback_policy) == Some("external_cli")
        || fanout_policies
            .iter()
            .any(|policy| backend_policy_class(policy) == Some("external_cli"))
}

fn effective_execution_posture(
    selected_execution_class: Option<&str>,
    route_has_external_backend: bool,
) -> &'static str {
    match selected_execution_class.unwrap_or("unknown") {
        "external" => "external_only",
        "internal" if route_has_external_backend => "hybrid",
        "internal" => "internal_only",
        "unknown" if route_has_external_backend => "hybrid_unknown_host",
        _ => "unknown",
    }
}

pub(crate) fn summarize_execution_truth_for_route(
    execution_plan: &serde_json::Value,
    route: Option<&serde_json::Value>,
    selected_execution_class: Option<&str>,
    effective_selected_backend: Option<&str>,
    activation_kind: Option<&str>,
    execution_evidence_status: Option<&str>,
) -> serde_json::Value {
    let route_primary_backend = route.and_then(route_primary_backend_hint_from_route);
    let runtime_assignment_backend =
        route.and_then(|route| runtime_assignment_backend_for_route(execution_plan, route));
    let fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let effective_selected_backend = effective_selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| route_primary_backend.clone());
    let route_primary_policy = route_primary_backend
        .as_deref()
        .map(|backend_id| backend_policy_from_execution_plan(execution_plan, backend_id))
        .unwrap_or(serde_json::Value::Null);
    let fallback_policy = fallback_backend
        .as_deref()
        .map(|backend_id| backend_policy_from_execution_plan(execution_plan, backend_id))
        .unwrap_or(serde_json::Value::Null);
    let fanout_policies = fanout_backends
        .iter()
        .map(|backend_id| backend_policy_from_execution_plan(execution_plan, backend_id))
        .collect::<Vec<_>>();
    let selected_backend_policy = effective_selected_backend
        .as_deref()
        .map(|backend_id| backend_policy_from_execution_plan(execution_plan, backend_id))
        .unwrap_or(serde_json::Value::Null);
    let route_uses_external_backend =
        route_has_external_backend(&route_primary_policy, &fallback_policy, &fanout_policies);
    let inherited_selected_backend = effective_selected_backend
        .as_deref()
        .filter(|backend_id| route_primary_backend.as_deref() != Some(*backend_id))
        .filter(|backend_id| fallback_backend.as_deref() != Some(*backend_id))
        .filter(|backend_id| {
            !fanout_backends
                .iter()
                .any(|candidate| candidate == *backend_id)
        });
    let selected_backend_source = backend_selection_source(
        effective_selected_backend.as_deref(),
        inherited_selected_backend,
        runtime_assignment_backend.as_deref(),
        route_primary_backend.as_deref(),
        fallback_backend.as_deref(),
        &fanout_backends,
        None,
        None,
    );

    serde_json::json!({
        "effective_execution_posture": effective_execution_posture(
            selected_execution_class,
            route_uses_external_backend,
        ),
        "selected_execution_class": selected_execution_class.unwrap_or("unknown"),
        "route_primary_backend": route_primary_backend,
        "effective_selected_backend": effective_selected_backend,
        "selected_backend_source": selected_backend_source,
        "backend_selection_source": selected_backend_source,
        "fallback_backend": fallback_backend,
        "fanout_backends": fanout_backends,
        "route_uses_external_backend": route_uses_external_backend,
        "selected_backend_policy": selected_backend_policy,
        "route_primary_backend_policy": route_primary_policy,
        "fallback_backend_policy": fallback_policy,
        "fanout_backend_policies": fanout_policies,
        "activation_evidence": {
            "activation_kind": activation_kind.unwrap_or("unknown"),
            "execution_evidence_status": execution_evidence_status.unwrap_or("missing"),
            "receipt_backed": execution_evidence_status == Some("recorded"),
        },
    })
}

pub(crate) fn build_executor_backend_admissibility_matrix(
    agent_system: &serde_json::Value,
) -> serde_json::Value {
    let Some(entries) =
        json_lookup(agent_system, &["subagents"]).and_then(serde_json::Value::as_object)
    else {
        return serde_json::json!([]);
    };
    let mut rows = entries
        .iter()
        .filter(|(_, entry)| json_bool(json_lookup(entry, &["enabled"]), false))
        .map(|(backend_id, entry)| backend_policy_entry(backend_id, entry))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["backend_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["backend_id"].as_str().unwrap_or_default())
    });
    serde_json::Value::Array(rows)
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
        "analysis" | "read_only_prep" | "research" | "research_fast" | "research_deep" => {
            ("business_analyst", "analysis")
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
        "executor_backend_policy": backend_policy_by_id(agent_system, &executor_backend),
        "fallback_executor_backend_policy": backend_policy_by_id(agent_system, &fallback_executor_backend),
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
    use super::{
        build_executor_backend_admissibility_matrix, build_runtime_execution_plan_from_snapshot,
        build_runtime_lane_selection_from_bundle, summarize_agent_route_from_snapshot,
        summarize_execution_truth_for_route,
    };
    use crate::launcher_activation_snapshot::pack_router_keywords_json;
    use crate::project_activator_surface::read_yaml_file_checked;
    use crate::temp_state::TempStateHarness;
    use crate::{build_compiled_agent_extension_bundle_for_root, run, Cli};
    use clap::Parser;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::ExitCode;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    struct RecoveringMutex(Mutex<()>);

    impl RecoveringMutex {
        fn lock(&self) -> MutexGuard<'_, ()> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    fn current_dir_lock() -> &'static RecoveringMutex {
        static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
        LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
    }

    struct CurrentDirGuard {
        _lock: MutexGuard<'static, ()>,
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &Path) -> Self {
            let lock = current_dir_lock().lock();
            let original = env::current_dir().expect("current dir should resolve");
            env::set_current_dir(path).expect("current dir should change");
            Self {
                _lock: lock,
                original,
            }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.original).expect("current dir should restore");
        }
    }

    fn guard_current_dir(path: &Path) -> CurrentDirGuard {
        CurrentDirGuard::change_to(path)
    }

    fn cli(args: &[&str]) -> Cli {
        let mut argv = vec!["vida"];
        argv.extend(args.iter().copied());
        Cli::parse_from(argv)
    }

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
            },
            "subagents": {
                "internal_subagents": {
                    "enabled": true,
                    "subagent_backend_class": "internal",
                    "write_scope": "orchestrator_native",
                    "capability_band": ["implementation_safe", "review_safe"],
                    "specialties": ["implementation", "verification"]
                },
                "internal_review": {
                    "enabled": true,
                    "subagent_backend_class": "internal",
                    "write_scope": "orchestrator_native",
                    "capability_band": ["review_safe"],
                    "specialties": ["review"]
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
        assert_eq!(
            summary["executor_backend_policy"]["lane_admissibility"]["implementation"],
            true
        );
        assert_eq!(
            summary["fallback_executor_backend_policy"]["lane_admissibility"]["review"],
            true
        );
    }

    #[test]
    fn build_executor_backend_admissibility_matrix_marks_review_only_external_backends() {
        let agent_system = serde_json::json!({
            "subagents": {
                "internal_subagents": {
                    "enabled": true,
                    "subagent_backend_class": "internal",
                    "write_scope": "orchestrator_native",
                    "capability_band": ["implementation_safe", "review_safe"],
                    "specialties": ["implementation", "verification"]
                },
                "hermes_cli": {
                    "enabled": true,
                    "subagent_backend_class": "external_cli",
                    "write_scope": "none",
                    "capability_band": ["read_only", "review_safe", "web_search"],
                    "specialties": ["review", "agentic_coding"]
                }
            }
        });

        let matrix = build_executor_backend_admissibility_matrix(&agent_system);
        let rows = matrix.as_array().expect("matrix should be an array");
        assert_eq!(rows.len(), 2);

        let internal = rows
            .iter()
            .find(|row| row["backend_id"] == "internal_subagents")
            .expect("internal row should exist");
        assert_eq!(internal["lane_admissibility"]["implementation"], true);
        assert_eq!(internal["lane_admissibility"]["verification"], true);

        let hermes_review_backend = rows
            .iter()
            .find(|row| row["backend_id"] == "hermes_cli")
            .expect("hermes row should exist");
        assert_eq!(
            hermes_review_backend["lane_admissibility"]["analysis"],
            true
        );
        assert_eq!(hermes_review_backend["lane_admissibility"]["coach"], true);
        assert_eq!(
            hermes_review_backend["lane_admissibility"]["implementation"],
            false
        );
        assert_eq!(
            hermes_review_backend["lane_admissibility"]["verification"],
            false
        );
        assert_eq!(
            hermes_review_backend["lane_admissibility"]["policy_flags"]["review_only_backend"],
            true
        );
    }

    #[test]
    fn real_project_config_expands_analysis_and_review_routes_beyond_single_review_backend() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vida.config.yaml");
        let overlay: serde_yaml::Value =
            serde_yaml::from_str(&fs::read_to_string(&config_path).expect("config should read"))
                .expect("config should parse");
        let overlay_json =
            serde_json::to_value(overlay).expect("yaml config should convert to json");
        let agent_system = &overlay_json["agent_system"];

        let analysis =
            summarize_agent_route_from_snapshot(&serde_json::Value::Null, agent_system, "analysis");
        assert_eq!(analysis["executor_backend"], "opencode_cli");
        assert_eq!(
            analysis["fanout_executor_backends"],
            serde_json::json!(["hermes_cli", "opencode_cli", "kilo_cli"])
        );
        assert_eq!(
            analysis["executor_backend_policy"]["lane_admissibility"]["implementation"],
            false
        );

        let coach =
            summarize_agent_route_from_snapshot(&serde_json::Value::Null, agent_system, "coach");
        assert_eq!(coach["executor_backend"], "hermes_cli");
        let coach_fanout = coach["fanout_executor_backends"]
            .as_array()
            .expect("coach fanout should be an array");
        assert!(coach_fanout.iter().any(|value| value == "hermes_cli"));
        assert!(coach_fanout.iter().any(|value| value == "opencode_cli"));

        let verification = summarize_agent_route_from_snapshot(
            &serde_json::Value::Null,
            agent_system,
            "verification",
        );
        assert_eq!(verification["executor_backend"], "opencode_cli");

        let review_ensemble = summarize_agent_route_from_snapshot(
            &serde_json::Value::Null,
            agent_system,
            "review_ensemble",
        );
        let review_ensemble_fanout = review_ensemble["fanout_executor_backends"]
            .as_array()
            .expect("review ensemble fanout should be an array");
        assert!(review_ensemble_fanout
            .iter()
            .any(|value| value == "hermes_cli"));
        assert!(review_ensemble_fanout
            .iter()
            .any(|value| value == "opencode_cli"));
    }

    #[test]
    fn summarize_execution_truth_for_route_marks_hybrid_posture_and_fallback_selection() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "opencode_cli",
                    "backend_class": "external_cli"
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal"
                },
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli"
                }
            ]
        });
        let route = serde_json::json!({
            "executor_backend": "opencode_cli",
            "fallback_executor_backend": "internal_subagents",
            "fanout_executor_backends": ["hermes_cli"]
        });

        let summary = summarize_execution_truth_for_route(
            &execution_plan,
            Some(&route),
            Some("internal"),
            Some("internal_subagents"),
            Some("activation_view"),
            Some("missing"),
        );

        assert_eq!(summary["effective_execution_posture"], "hybrid");
        assert_eq!(summary["route_primary_backend"], "opencode_cli");
        assert_eq!(summary["effective_selected_backend"], "internal_subagents");
        assert_eq!(summary["selected_backend_source"], "route_fallback_hint");
        assert_eq!(summary["fallback_backend"], "internal_subagents");
        assert_eq!(summary["fanout_backends"][0], "hermes_cli");
        assert_eq!(
            summary["activation_evidence"]["execution_evidence_status"],
            "missing"
        );
        assert_eq!(
            summary["selected_backend_policy"]["backend_class"],
            "internal"
        );
    }

    #[test]
    fn runtime_assignment_uses_overlay_ladder_for_all_four_tiers() {
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

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);

        let assignment_for = |request: &str| {
            let selection = build_runtime_lane_selection_from_bundle(
                &bundle,
                "state_store",
                &pack_router,
                request,
            )
            .expect("selection should build");
            let plan = build_runtime_execution_plan_from_snapshot(&bundle, &selection);
            let carrier_runtime_assignment = plan["carrier_runtime_assignment"].clone();
            let runtime_assignment = plan["runtime_assignment"].clone();
            assert_eq!(carrier_runtime_assignment, runtime_assignment);
            assert!(plan.get("codex_runtime_assignment").is_none());
            runtime_assignment
        };
        let implementation = assignment_for("write one bounded implementation patch");
        assert_eq!(implementation["enabled"], true);
        assert_eq!(implementation["runtime_role"], "worker");
        assert_eq!(implementation["activation_agent_type"], "junior");
        assert_eq!(implementation["activation_runtime_role"], "worker");
        assert_eq!(implementation["selected_tier"], "junior");
        assert_eq!(implementation["selected_runtime_role"], "worker");
        assert_eq!(implementation["tier_default_runtime_role"], "worker");
        assert_eq!(implementation["rate"], 1);
        assert_eq!(implementation["estimated_task_price_units"], 1);

        let specification = assignment_for(
            "research the feature, write the specification, and develop an implementation plan",
        );
        assert_eq!(specification["enabled"], true);
        assert_eq!(specification["runtime_role"], "business_analyst");
        assert_eq!(specification["activation_agent_type"], "middle");
        assert_eq!(specification["activation_runtime_role"], "business_analyst");
        assert_eq!(specification["selected_tier"], "middle");
        assert_eq!(specification["selected_runtime_role"], "business_analyst");
        assert_eq!(specification["tier_default_runtime_role"], "coach");
        assert_eq!(specification["rate"], 4);
        assert_eq!(specification["estimated_task_price_units"], 8);

        let coach = assignment_for(
            "review the implemented result against the spec, acceptance criteria, and definition of done; request rework if it drifts",
        );
        assert_eq!(coach["enabled"], true);
        assert_eq!(coach["runtime_role"], "coach");
        assert_eq!(coach["activation_agent_type"], "middle");
        assert_eq!(coach["activation_runtime_role"], "coach");
        assert_eq!(coach["selected_tier"], "middle");
        assert_eq!(coach["selected_runtime_role"], "coach");
        assert_eq!(coach["tier_default_runtime_role"], "coach");
        assert_eq!(coach["rate"], 4);
        assert_eq!(coach["estimated_task_price_units"], 8);

        let verification = assignment_for("review one bounded patch and verify release readiness");
        assert_eq!(verification["enabled"], true);
        assert_eq!(verification["runtime_role"], "verifier");
        assert_eq!(verification["activation_agent_type"], "senior");
        assert_eq!(verification["activation_runtime_role"], "verifier");
        assert_eq!(verification["selected_tier"], "senior");
        assert_eq!(verification["selected_runtime_role"], "verifier");
        assert_eq!(verification["tier_default_runtime_role"], "verifier");
        assert_eq!(verification["rate"], 16);
        assert_eq!(verification["estimated_task_price_units"], 32);

        let architecture = assignment_for(
            "prepare the architecture and hard escalation plan for a cross cutting migration conflict",
        );
        assert_eq!(architecture["enabled"], true);
        assert_eq!(architecture["runtime_role"], "solution_architect");
        assert_eq!(architecture["activation_agent_type"], "architect");
        assert_eq!(
            architecture["activation_runtime_role"],
            "solution_architect"
        );
        assert_eq!(architecture["selected_tier"], "architect");
        assert_eq!(architecture["selected_runtime_role"], "solution_architect");
        assert_eq!(
            architecture["tier_default_runtime_role"],
            "solution_architect"
        );
        assert_eq!(architecture["rate"], 32);
        assert_eq!(architecture["estimated_task_price_units"], 128);
    }

    #[test]
    fn explicit_implementation_request_selects_worker_without_conversation_mode() {
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

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "Implement exactly one bounded write-producing code change: move the test and make only minimal compile-fix adjustments.",
        )
        .expect("selection should build");

        assert_eq!(selection.selected_role, "worker");
        assert!(selection.conversational_mode.is_none());
        assert_eq!(selection.reason, "auto_explicit_implementation_request");
        assert!(selection
            .matched_terms
            .iter()
            .any(|term| term == "write-producing" || term == "move the test"));
    }

    #[test]
    fn weak_pbi_discussion_match_does_not_override_explicit_fix_patch_intent() {
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

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "Take the next backlog task and implement a bounded patch to fix the runtime code path with minimal code change.",
        )
        .expect("selection should build");

        assert_eq!(selection.selected_role, "worker");
        assert!(selection.conversational_mode.is_none());
        assert_eq!(
            selection.reason,
            "auto_explicit_implementation_request_override"
        );
        assert!(selection
            .matched_terms
            .iter()
            .any(|term| term == "implement" || term == "bounded patch" || term == "code change"));
    }

    #[test]
    fn feature_design_request_keeps_scope_discussion_route_even_with_fix_wording() {
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

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "Research the feature scope, write the specification and acceptance criteria, then clarify what code fix should follow for the backlog task.",
        )
        .expect("selection should build");

        assert_eq!(selection.selected_role, "business_analyst");
        assert_eq!(
            selection.conversational_mode.as_deref(),
            Some("scope_discussion")
        );
        assert_eq!(selection.reason, "auto_keyword_match");
        assert!(selection
            .matched_terms
            .iter()
            .any(|term| term == "scope" || term == "spec" || term == "acceptance"));
    }

    #[test]
    fn bounded_rust_file_repair_request_overrides_scope_discussion_route() {
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

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "Repair selector precedence in crates/vida/src/runtime_lane_summary.rs, fix the bug, and add a regression test so this Rust file stops routing to specification planning.",
        )
        .expect("selection should build");

        assert_eq!(selection.selected_role, "worker");
        assert!(selection.conversational_mode.is_none());
        assert_eq!(
            selection.reason,
            "auto_explicit_implementation_request_override"
        );
        assert!(selection
            .matched_terms
            .iter()
            .any(|term| term == "repair" || term == "fix" || term == "regression test"));
        assert!(selection
            .matched_terms
            .iter()
            .any(|term| term == ".rs" || term == "crates/" || term == "rust file"));
    }

    #[test]
    fn dispatch_aliases_are_loaded_from_overlay_not_rust_catalog() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace("development_implementer:", "custom_impl_lane:");
        fs::write(&config_path, updated).expect("config should be rewritten");

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

        let codex_config = fs::read_to_string(harness.path().join(".codex/config.toml"))
            .expect("rendered codex config should exist");
        assert!(!codex_config.contains("[agents.custom_impl_lane]"));
        assert!(!codex_config.contains("[agents.development_implementer]"));

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let pack_router = pack_router_keywords_json(&config);
        let selection = build_runtime_lane_selection_from_bundle(
            &bundle,
            "state_store",
            &pack_router,
            "write one bounded implementation patch",
        )
        .expect("selection should build");
        let plan = build_runtime_execution_plan_from_snapshot(&bundle, &selection);

        let carrier_runtime_assignment = plan["carrier_runtime_assignment"].clone();
        let runtime_assignment = plan["runtime_assignment"].clone();
        assert_eq!(carrier_runtime_assignment, runtime_assignment);
        assert!(plan.get("codex_runtime_assignment").is_none());
        assert!(runtime_assignment.get("internal_named_lane_id").is_none());
        assert_eq!(
            plan["development_flow"]["dispatch_contract"]["implementer_activation"]
                ["activation_agent_type"],
            "junior"
        );
        assert!(
            plan["development_flow"]["dispatch_contract"]["implementer_activation"]
                .get("internal_named_lane_id")
                .is_none()
        );
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

pub(crate) fn feature_delivery_design_terms(request: &str) -> Vec<String> {
    crate::feature_delivery_design_terms(request)
}

pub(crate) fn explicit_implementation_request_terms(request: &str) -> Vec<String> {
    crate::contains_keywords(
        request,
        &[
            "implement".to_string(),
            "implementation patch".to_string(),
            "bounded patch".to_string(),
            "write-producing".to_string(),
            "code change".to_string(),
            "move the test".to_string(),
            "minimal compile-fix".to_string(),
            "fix the code".to_string(),
            "edit the code".to_string(),
            "update the code".to_string(),
            "refactor".to_string(),
            "patch".to_string(),
        ],
    )
}

pub(crate) fn explicit_bounded_code_repair_terms(request: &str) -> Vec<String> {
    let repair_terms = crate::contains_keywords(
        request,
        &[
            "repair".to_string(),
            "fix".to_string(),
            "bug".to_string(),
            "regression".to_string(),
            "regression test".to_string(),
            "regression-test".to_string(),
        ],
    );
    if repair_terms.is_empty() {
        return Vec::new();
    }

    let scope_terms = crate::contains_keywords(
        request,
        &[
            ".rs".to_string(),
            "crates/".to_string(),
            "src/".to_string(),
            "src".to_string(),
            "rust file".to_string(),
            "bounded rust file".to_string(),
            "single rust file".to_string(),
            "file scope".to_string(),
            "exact file".to_string(),
            "test".to_string(),
            "tests".to_string(),
            "unit test".to_string(),
            "integration test".to_string(),
            "proof".to_string(),
            "proof of".to_string(),
        ],
    );
    if scope_terms.is_empty() {
        return Vec::new();
    }

    let mut combined = Vec::new();
    for term in repair_terms.into_iter().chain(scope_terms.into_iter()) {
        if !combined.contains(&term) {
            combined.push(term);
        }
    }
    combined
}

fn explicit_verification_request_terms(request: &str) -> Vec<String> {
    crate::contains_keywords(
        request,
        &[
            "verify".to_string(),
            "verification".to_string(),
            "verifier".to_string(),
            "review readiness".to_string(),
            "release readiness".to_string(),
            "verify release readiness".to_string(),
            "release-ready".to_string(),
            "release ready".to_string(),
        ],
    )
}

fn weak_pbi_discussion_term(term: &str) -> bool {
    matches!(
        term,
        "pbi" | "backlog" | "priority" | "task" | "ticket" | "work pool"
    )
}
