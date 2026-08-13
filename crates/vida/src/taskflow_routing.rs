use std::sync::OnceLock;

use crate::runtime_assignment_policy::canonical_dispatch_target_name;
use crate::team_flow_state_machine::DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE;
use crate::{json_string, json_string_list};

const REJECTED_NON_BEHAVIORAL_ROUTE_FIELDS: &[&str] = &[
    "external_costs_authoritative",
    "embedding_semantic_cache",
    "embedding_semantic_cache_provider",
    "hot_path_price_fetch",
    "hot_path_price_catalog_fetch",
    "imported_price_authority_override",
    "coach_executor_backend",
    "deterministic_first",
    "external_first_required",
    "gateway_audit_logs",
    "gateway_credentials",
    "gateway_price_override",
    "gateway_proxy_adapter",
    "gateway_virtual_keys",
    "imported_price_override",
    "live_price_catalog_fetch",
    "local_execution_allowed",
    "local_execution_preferred",
    "max_cli_subagent_calls",
    "max_coach_passes",
    "max_fallback_hops",
    "max_total_runtime_seconds",
    "max_verification_passes",
    "merge_policy",
    "min_output_bytes",
    "price_catalog_provider_fetch",
    "semantic_cache_authoritative",
    "semantic_cache_bypass_hard_filters",
    "semantic_cache_closure_authority",
    "semantic_cache_embedding_provider",
    "semantic_cache_receipt_authority",
    "semantic_cache_remote_provider",
    "semantic_cache_selected_candidate_authority",
    "semantic_score_bypass_hard_filters",
    "semantic_score_disable_hard_filters",
    "semantic_score_override_authority",
    "semantic_score_resurrect_rejected_candidate",
    "web_search_required",
    "workflow_learning_enabled",
    "workflow_rework_learning",
    "workflow_verification_learning",
];

const DIAGNOSTIC_ONLY_ROUTE_FIELDS: &[&str] = &[
    "provider_price_snapshot",
    "dispatch_required",
    "graph_strategy",
    "internal_escalation_trigger",
    "semantic_scoring_order",
    "semantic_route_cache",
    "write_scope",
];

/// Diagnostic-only raw catalog lookup. Executable callers must resolve through
/// `TeamFlowExecutionAuthority` and a typed `Result` projection.
pub(crate) fn dispatch_contract_lane_diagnostic<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    let canonical_target = canonical_dispatch_target_name(dispatch_target);
    if let Some(lane) = execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"]
        .get(canonical_target.as_str())
    {
        return Some(lane);
    }
    if canonical_target != dispatch_target {
        if let Some(lane) = execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"]
            .get(dispatch_target)
        {
            return Some(lane);
        }
    }
    None
}

/// Diagnostic projection only; raw `development_flow` entries never authorize dispatch.
pub(crate) fn direct_development_flow_route_entries<'a>(
    execution_plan: &'a serde_json::Value,
) -> Vec<(String, &'a serde_json::Value)> {
    direct_development_flow_route_records(execution_plan)
        .into_iter()
        .map(|entry| (entry.dispatch_target, entry.route))
        .collect()
}

/// Diagnostic projection only; selectors are not TeamFlow node identities.
pub(crate) fn direct_development_flow_route_selectors<'a>(
    execution_plan: &'a serde_json::Value,
) -> Vec<(String, &'a serde_json::Value)> {
    direct_development_flow_route_records(execution_plan)
        .into_iter()
        .flat_map(|entry| {
            let route_id = canonical_dispatch_target_name(&entry.route_id);
            if route_id == entry.dispatch_target {
                vec![(entry.dispatch_target, entry.route)]
            } else {
                vec![
                    (route_id, entry.route),
                    (entry.dispatch_target, entry.route),
                ]
            }
        })
        .collect()
}

struct DirectDevelopmentFlowRouteEntry<'a> {
    route_id: String,
    dispatch_target: String,
    route: &'a serde_json::Value,
}

fn direct_development_flow_route_records<'a>(
    execution_plan: &'a serde_json::Value,
) -> Vec<DirectDevelopmentFlowRouteEntry<'a>> {
    execution_plan["development_flow"]
        .as_object()
        .into_iter()
        .flat_map(|flow| flow.iter())
        .filter(|(route_id, route)| direct_development_flow_entry_is_route(route_id, route))
        .map(|(route_id, route)| DirectDevelopmentFlowRouteEntry {
            route_id: route_id.to_string(),
            dispatch_target: development_flow_route_dispatch_target(route_id, route),
            route,
        })
        .collect()
}

fn direct_development_flow_entry_is_route(route_id: &str, route: &serde_json::Value) -> bool {
    if matches!(route_id, "dispatch_contract" | "default_route") || !route.is_object() {
        return false;
    }
    route.get("dispatch_target").is_some()
        || route.get("target").is_some()
        || route
            .get("activation")
            .is_some_and(serde_json::Value::is_object)
        || route
            .get("runtime_assignment")
            .is_some_and(serde_json::Value::is_object)
        || route.get("executor_backend").is_some()
        || route.get("fallback_executor_backend").is_some()
        || route.get("fanout_executor_backends").is_some()
}

fn development_flow_route_dispatch_target(route_id: &str, route: &serde_json::Value) -> String {
    json_string(route.get("dispatch_target"))
        .or_else(|| json_string(route.get("target")))
        .map(|target| canonical_dispatch_target_name(&target))
        .unwrap_or_else(|| canonical_dispatch_target_name(route_id))
}

pub(crate) fn dispatch_contract_lane_activation(lane: &serde_json::Value) -> &serde_json::Value {
    static EMPTY_ACTIVATION: OnceLock<serde_json::Value> = OnceLock::new();
    lane.get("activation")
        .unwrap_or_else(|| EMPTY_ACTIVATION.get_or_init(|| serde_json::Value::Null))
}

pub(crate) fn dispatch_contract_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    // Diagnostic-only compatibility projection. Executable callers must use
    // `dispatch_contract_lane_sequence_result` so blockers remain visible.
    match dispatch_contract_lane_sequence_result(dispatch_contract) {
        Ok(sequence) => sequence,
        Err(blocker) => vec![format!("blocked:{}", blocker.code)],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispatchContractResolutionBlocker {
    pub(crate) code: &'static str,
    pub(crate) sequence_field: String,
}

pub(crate) fn dispatch_contract_lane_sequence_result(
    dispatch_contract: &serde_json::Value,
) -> Result<Vec<String>, DispatchContractResolutionBlocker> {
    crate::team_flow_state_machine::TeamFlowStateMachine::resolve_dispatch_contract(
        dispatch_contract,
        "lane_sequence",
    )
    .and_then(|state_machine| {
        state_machine
            .resolve_execution_lane_sequence_status()
            .map_err(
                |blocker| crate::team_flow_state_machine::TeamFlowResolutionBlocker {
                    code: blocker.code,
                    sequence_field: blocker.sequence_field,
                },
            )
    })
    .map_err(|blocker| DispatchContractResolutionBlocker {
        code: blocker.code,
        sequence_field: blocker.sequence_field,
    })
}

pub(crate) fn dispatch_contract_execution_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    // Diagnostic-only compatibility projection; execution uses the Result API.
    match dispatch_contract_execution_lane_sequence_result(dispatch_contract) {
        Ok(sequence) => sequence,
        Err(blocker) => vec![format!("blocked:{}", blocker.code)],
    }
}

pub(crate) fn dispatch_contract_execution_lane_sequence_result(
    dispatch_contract: &serde_json::Value,
) -> Result<Vec<String>, DispatchContractResolutionBlocker> {
    crate::team_flow_state_machine::TeamFlowStateMachine::resolve_dispatch_contract(
        dispatch_contract,
        "execution_lane_sequence",
    )
    .map_err(|blocker| DispatchContractResolutionBlocker {
        code: blocker.code,
        sequence_field: blocker.sequence_field,
    })?
    .resolve_execution_lane_sequence_status()
    .map_err(|blocker| DispatchContractResolutionBlocker {
        code: blocker.code,
        sequence_field: blocker.sequence_field,
    })
}

pub(crate) fn dispatch_contract_allowed_next_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    // Diagnostic-only compatibility projection; execution uses the Result API.
    dispatch_contract_lane_sequence(dispatch_contract)
}

/// Diagnostic-only projection. Executable callers resolve a target through the
/// typed TeamFlow authority projection, never by scanning raw lane metadata.
pub(crate) fn dispatch_target_for_runtime_role_diagnostic(
    execution_plan: &serde_json::Value,
    runtime_role: &str,
) -> Option<String> {
    let runtime_role = runtime_role.trim();
    if runtime_role.is_empty() {
        return None;
    }
    if let Some(lane_catalog) =
        execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"].as_object()
    {
        for (dispatch_target, lane) in lane_catalog {
            let activation = dispatch_contract_lane_activation(lane);
            let lane_runtime_role = json_string(activation.get("activation_runtime_role"))
                .or_else(|| json_string(lane.get("runtime_role")));
            if lane_runtime_role.as_deref() == Some(runtime_role) {
                return Some(dispatch_target.clone());
            }
        }
    }
    None
}

fn carrier_backend_from_assignment(assignment: &serde_json::Value) -> Option<String> {
    json_string(assignment.get("effective_selected_backend"))
        .or_else(|| json_string(assignment.get("selected_dispatch_backend_id")))
        .or_else(|| json_string(assignment.get("dispatch_backend_id")))
        .or_else(|| json_string(assignment.get("selected_backend_id")))
        .or_else(|| json_string(assignment.get("selected_backend")))
        .filter(|value| !value.is_empty())
}

fn execution_plan_backend_metadata_present(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> bool {
    let backend_id = backend_id.trim();
    if backend_id.is_empty() {
        return false;
    }
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["backend_id"].as_str() == Some(backend_id))
}

fn dispatch_backend_from_assignment(
    execution_plan: &serde_json::Value,
    assignment: &serde_json::Value,
) -> Option<String> {
    let readiness_backend = assignment["selected_external_backend_readiness"]["backend_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|backend_id| execution_plan_backend_metadata_present(execution_plan, backend_id))
        .map(str::to_string);
    if readiness_backend.is_some() {
        return readiness_backend;
    }

    [
        "effective_selected_backend",
        "selected_dispatch_backend_id",
        "dispatch_backend_id",
        "selected_backend_id",
    ]
    .iter()
    .filter_map(|key| json_string(assignment.get(*key)))
    .find(|backend_id| execution_plan_backend_metadata_present(execution_plan, backend_id))
}

pub(crate) fn activation_backend_from_route(route: &serde_json::Value) -> Option<String> {
    carrier_backend_from_assignment(dispatch_contract_lane_activation(route))
}

fn activation_agent_type_from_route(route: &serde_json::Value) -> Option<String> {
    json_string(dispatch_contract_lane_activation(route).get("activation_agent_type"))
        .filter(|value| !value.trim().is_empty())
}

fn route_backend_value(route: &serde_json::Value, key: &str) -> Option<String> {
    json_string(route.get(key))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            crate::json_string_list(route.get(key))
                .into_iter()
                .find(|value| !value.trim().is_empty())
        })
}

fn route_field_is_configured(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Null) | None => false,
        Some(serde_json::Value::String(raw)) => !raw.trim().is_empty(),
        Some(serde_json::Value::Array(rows)) => !rows.is_empty(),
        Some(serde_json::Value::Object(entries)) => !entries.is_empty(),
        Some(_) => true,
    }
}

fn route_non_behavioral_fields(route: &serde_json::Value) -> Vec<String> {
    REJECTED_NON_BEHAVIORAL_ROUTE_FIELDS
        .iter()
        .filter(|field| route_field_is_configured(route.get(**field)))
        .map(|field| field.to_string())
        .collect()
}

fn route_diagnostic_only_fields(route: &serde_json::Value) -> Vec<String> {
    DIAGNOSTIC_ONLY_ROUTE_FIELDS
        .iter()
        .filter(|field| route_field_is_configured(route.get(**field)))
        .map(|field| field.to_string())
        .collect()
}

fn route_field_truth(route: &serde_json::Value) -> serde_json::Value {
    let rejected = REJECTED_NON_BEHAVIORAL_ROUTE_FIELDS
        .iter()
        .filter(|field| route_field_is_configured(route.get(**field)))
        .map(|field| {
            serde_json::json!({
                "field": field,
                "truth": "rejected_no_runtime_consumer",
                "knob_class": "unsupported_non_behavioral",
                "effect": "validate-routing blocks the route until the field is removed or wired to a concrete consumer",
            })
        });
    let diagnostic_only = DIAGNOSTIC_ONLY_ROUTE_FIELDS
        .iter()
        .filter(|field| route_field_is_configured(route.get(**field)))
        .map(|field| {
            serde_json::json!({
                "field": field,
                "truth": "diagnostic_only_no_execution_actuation",
                "knob_class": "diagnostic_only",
                "effect": "surface/explain metadata only; runtime execution selection does not change from this field",
            })
        });
    serde_json::Value::Array(rejected.chain(diagnostic_only).collect())
}

pub(crate) fn runtime_assignment_from_route<'a>(
    route: &'a serde_json::Value,
) -> &'a serde_json::Value {
    route
        .get("carrier_runtime_assignment")
        .or_else(|| route.get("runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

#[allow(dead_code)]
pub(crate) fn runtime_assignment_source_from_route(route: &serde_json::Value) -> &'static str {
    if route.get("carrier_runtime_assignment").is_some() {
        "carrier_runtime_assignment"
    } else if route.get("runtime_assignment").is_some() {
        "runtime_assignment"
    } else {
        "missing"
    }
}

pub(crate) fn runtime_assignment_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
) -> &'a serde_json::Value {
    execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("carrier_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

pub(crate) fn runtime_assignment_source_from_execution_plan(
    execution_plan: &serde_json::Value,
) -> &'static str {
    if execution_plan.get("runtime_assignment").is_some() {
        "runtime_assignment"
    } else if execution_plan.get("carrier_runtime_assignment").is_some() {
        "carrier_runtime_assignment"
    } else {
        "missing"
    }
}

#[allow(dead_code)]
fn carrier_backend_from_route(route: &serde_json::Value) -> Option<String> {
    json_string(route.get("preferred_agent_tier"))
        .or_else(|| json_string(route.get("preferred_agent_type")))
        .or_else(|| carrier_backend_from_assignment(runtime_assignment_from_route(route)))
        .filter(|value| !value.is_empty())
}

pub(crate) fn route_primary_backend_hint_from_route(route: &serde_json::Value) -> Option<String> {
    explicit_executor_backend_from_route(route).or_else(|| activation_backend_from_route(route))
}

pub(crate) fn runtime_assignment_backend_for_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    dispatch_backend_from_assignment(execution_plan, runtime_assignment_from_route(route)).or_else(
        || {
            dispatch_backend_from_assignment(
                execution_plan,
                runtime_assignment_from_execution_plan(execution_plan),
            )
        },
    )
}

pub(crate) fn explicit_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "executor_backend")
}

pub(crate) fn fallback_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "fallback_executor_backend")
}

pub(crate) fn fanout_executor_backends_from_route(route: &serde_json::Value) -> Vec<String> {
    json_string_list(route.get("fanout_executor_backends"))
}

pub(crate) fn selected_backend_from_execution_plan_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    runtime_assignment_backend_for_route(execution_plan, route)
        .or_else(|| explicit_executor_backend_from_route(route))
        .or_else(|| route_backend_value(route, "fallback_executor_backend"))
        .or_else(|| route_backend_value(route, "fanout_executor_backends"))
        .or_else(|| activation_backend_from_route(route))
        .filter(|value| !value.is_empty())
}

pub(crate) fn backend_selection_source(
    effective_selected_backend: Option<&str>,
    inherited_selected_backend: Option<&str>,
    runtime_assignment_backend: Option<&str>,
    route_primary_backend: Option<&str>,
    route_fallback_backend: Option<&str>,
    route_fanout_backends: &[String],
    activation_agent_type: Option<&str>,
    explicit_selected_backend_override: Option<&str>,
) -> &'static str {
    match effective_selected_backend {
        Some(backend_id) if explicit_selected_backend_override == Some(backend_id) => {
            "explicit_retry_override"
        }
        Some(backend_id) if inherited_selected_backend == Some(backend_id) => {
            "dynamic_runtime_selection"
        }
        Some(backend_id) if runtime_assignment_backend == Some(backend_id) => "runtime_assignment",
        Some(backend_id) if route_primary_backend == Some(backend_id) => "route_primary_hint",
        Some(backend_id) if route_fallback_backend == Some(backend_id) => "route_fallback_hint",
        Some(backend_id)
            if route_fanout_backends
                .iter()
                .any(|candidate| candidate == backend_id) =>
        {
            "route_fanout_hint"
        }
        Some(backend_id) if activation_agent_type == Some(backend_id) => "activation_agent_type",
        Some(_) => "derived_selection",
        None => "unknown",
    }
}

fn selected_candidate_from_assignment<F>(assignment_field: &F) -> serde_json::Value
where
    F: Fn(&str) -> serde_json::Value,
{
    let carrier_id = assignment_field("selected_carrier_id");
    if carrier_id.is_null() {
        return serde_json::Value::Null;
    }
    serde_json::json!({
        "status": "selected",
        "carrier_id": carrier_id,
        "carrier_tier": assignment_field("selected_carrier_tier"),
        "model_profile_id": assignment_field("selected_model_profile_id"),
        "model_ref": assignment_field("selected_model_ref"),
        "model_provider": assignment_field("selected_model_provider"),
        "reasoning_effort": assignment_field("selected_reasoning_effort"),
        "reasoning_control_mode": assignment_field("selected_reasoning_control_mode"),
        "quality_tier": assignment_field("selected_quality_tier"),
        "speed_tier": assignment_field("selected_speed_tier"),
        "readiness_status": assignment_field("selected_model_profile_readiness_status"),
        "external_backend_readiness": assignment_field("selected_external_backend_readiness"),
        "budget_policy": assignment_field("budget_policy"),
        "budget_verdict": assignment_field("budget_verdict"),
        "max_budget_units": assignment_field("max_budget_units"),
        "selected_over_budget": assignment_field("selected_over_budget"),
        "route_profile_mapping": assignment_field("selected_route_profile_mapping"),
        "selection_source_paths": assignment_field("selection_source_paths"),
        "selection_override_reasons": assignment_field("selection_override_reasons"),
        "selection_budget": assignment_field("selection_budget"),
        "runtime_budget_ledger": assignment_field("runtime_budget_ledger"),
    })
}

fn candidate_pool_from_assignment(
    selected_candidate: &serde_json::Value,
    rejected_candidates: serde_json::Value,
) -> serde_json::Value {
    let mut candidates = Vec::new();
    if !selected_candidate.is_null() {
        candidates.push(selected_candidate.clone());
    }
    candidates.extend(
        rejected_candidates
            .as_array()
            .into_iter()
            .flatten()
            .cloned()
            .map(|mut candidate| {
                if let Some(row) = candidate.as_object_mut() {
                    row.insert(
                        "status".to_string(),
                        serde_json::Value::String("rejected".to_string()),
                    );
                }
                candidate
            }),
    );
    serde_json::Value::Array(candidates)
}

pub(crate) fn route_explain_payload(
    execution_plan: &serde_json::Value,
    compiled_bundle: &serde_json::Value,
    dispatch_target: &str,
    route: Option<&serde_json::Value>,
) -> serde_json::Value {
    let route_runtime_assignment = route
        .map(runtime_assignment_from_route)
        .filter(|value| value.is_object());
    let plan_runtime_assignment = runtime_assignment_from_execution_plan(execution_plan);
    let assignment_field = |key: &str| {
        route_runtime_assignment
            .and_then(|assignment| assignment.get(key))
            .filter(|value| !value.is_null())
            .cloned()
            .or_else(|| plan_runtime_assignment.get(key).cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    let route_primary_backend = route.and_then(route_primary_backend_hint_from_route);
    let runtime_assignment_backend =
        route.and_then(|route| runtime_assignment_backend_for_route(execution_plan, route));
    let fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_else(Vec::new);
    let activation_agent_type = route.and_then(activation_agent_type_from_route);
    let selected_backend = route
        .and_then(|route| {
            crate::runtime_dispatch_state::admissible_selected_backend_for_dispatch_target(
                execution_plan,
                dispatch_target,
                activation_backend_from_route(route).as_deref(),
                runtime_assignment_backend.as_deref(),
            )
        })
        .or_else(|| {
            route
                .and_then(|route| selected_backend_from_execution_plan_route(execution_plan, route))
        });
    let non_behavioral_route_fields = route
        .map(route_non_behavioral_fields)
        .unwrap_or_else(Vec::new);
    let diagnostic_only_route_fields = route
        .map(route_diagnostic_only_fields)
        .unwrap_or_else(Vec::new);
    let route_field_truth = route
        .map(route_field_truth)
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    let selection_source = backend_selection_source(
        selected_backend.as_deref(),
        None,
        runtime_assignment_backend.as_deref(),
        route_primary_backend.as_deref(),
        fallback_backend.as_deref(),
        &fanout_backends,
        activation_agent_type.as_deref(),
        None,
    );
    let selected_candidate = selected_candidate_from_assignment(&assignment_field);
    let rejected_candidates = assignment_field("rejected_candidates");
    let candidate_pool =
        candidate_pool_from_assignment(&selected_candidate, rejected_candidates.clone());
    let dispatch_contract_validation =
        typed_team_flow_authority_validation(execution_plan, compiled_bundle);

    serde_json::json!({
        "dispatch_target": dispatch_target,
        "route_present": route.is_some(),
        "selected_backend": selected_backend,
        "selection_source": selection_source,
        "runtime_assignment_source": route
            .map(runtime_assignment_source_from_route)
            .unwrap_or("missing"),
        "runtime_assignment_enabled": assignment_field("enabled"),
        "runtime_assignment_reason": assignment_field("reason"),
        "model_selection_enabled": assignment_field("model_selection_enabled"),
        "candidate_scope": assignment_field("candidate_scope"),
        "selected_carrier_id": assignment_field("selected_carrier_id"),
        "selected_model_profile_id": assignment_field("selected_model_profile_id"),
        "selected_model_ref": assignment_field("selected_model_ref"),
        "selected_model_provider": assignment_field("selected_model_provider"),
        "selected_reasoning_effort": assignment_field("selected_reasoning_effort"),
        "selected_reasoning_control_mode": assignment_field("selected_reasoning_control_mode"),
        "budget_policy": assignment_field("budget_policy"),
        "budget_verdict": assignment_field("budget_verdict"),
        "max_budget_units": assignment_field("max_budget_units"),
        "selected_over_budget": assignment_field("selected_over_budget"),
        "budget_scope": assignment_field("budget_scope"),
        "selection_budget": assignment_field("selection_budget"),
        "runtime_budget_ledger": assignment_field("runtime_budget_ledger"),
        "route_profile_mapping_applied": assignment_field("route_profile_mapping_applied"),
        "selected_route_profile_mapping": assignment_field("selected_route_profile_mapping"),
        "selection_source_paths": assignment_field("selection_source_paths"),
        "selection_override_reasons": assignment_field("selection_override_reasons"),
        "selection_precedence": assignment_field("selection_precedence"),
        "selection_inputs": {
            "selection_strategy": assignment_field("selection_strategy"),
            "selection_rule": assignment_field("selection_rule"),
            "model_selection_enabled": assignment_field("model_selection_enabled"),
            "candidate_scope": assignment_field("candidate_scope"),
            "budget_policy": assignment_field("budget_policy"),
            "max_budget_units": assignment_field("max_budget_units"),
            "route_profile_mapping_applied": assignment_field("route_profile_mapping_applied"),
            "route_primary_backend": route_primary_backend,
            "fallback_backend": fallback_backend,
            "fanout_backends": fanout_backends,
        },
        "selected_candidate": selected_candidate,
        "candidate_pool": candidate_pool,
        "rejected_candidates": rejected_candidates,
        "runtime_assignment_backend": runtime_assignment_backend,
        "route_primary_backend": route_primary_backend,
        "fallback_backend": fallback_backend,
        "fanout_backends": fanout_backends,
        "readiness_blockers": [],
        "activation_agent_type": activation_agent_type,
        "non_behavioral_route_fields": non_behavioral_route_fields,
        "rejected_route_fields": non_behavioral_route_fields,
        "diagnostic_only_route_fields": diagnostic_only_route_fields,
        "route_field_truth": route_field_truth,
        "dispatch_contract_validation": dispatch_contract_validation,
    })
}

fn typed_team_flow_authority_validation(
    execution_plan: &serde_json::Value,
    compiled_bundle: &serde_json::Value,
) -> serde_json::Value {
    let authority = match crate::team_flow_authority_adapter::require_team_flow_execution_authority(
        compiled_bundle,
        None,
        None,
    ) {
        Ok(authority) => authority,
        Err(blocker) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_code": blocker.code,
            });
        }
    };
    if authority.ordered_node_ids().is_empty() {
        return serde_json::json!({
            "status": "blocked",
            "blocker_code": DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE,
        });
    }
    let raw_authority_id = execution_plan["development_flow"]["dispatch_contract"]
        .get("team_flow_authority_id")
        .and_then(serde_json::Value::as_str);
    if raw_authority_id != Some(authority.authority_id.as_str()) {
        return serde_json::json!({
            "status": "blocked",
            "blocker_code": "team_flow_authority_plan_identity_mismatch",
        });
    }
    serde_json::json!({
        "status": "pass",
        "blocker_code": serde_json::Value::Null,
        "authority_id": authority.authority_id,
        "source": "persisted_team_flow_execution_authority",
    })
}

pub(crate) fn route_explain_status(
    payload: &serde_json::Value,
    admissible: Option<bool>,
) -> String {
    if payload["route_present"].as_bool() != Some(true) {
        return "blocked".to_string();
    }
    if payload["dispatch_contract_validation"]["status"].as_str() == Some("blocked") {
        return "blocked".to_string();
    }
    if payload["selected_backend"].as_str().is_none() {
        return "blocked".to_string();
    }
    if payload["runtime_assignment_enabled"].as_bool() == Some(false) {
        return "blocked".to_string();
    }
    if payload["model_selection_enabled"].as_bool() == Some(false) {
        return "blocked".to_string();
    }
    if payload["candidate_scope"].as_str() == Some("unified_carrier_model_profiles")
        || payload["candidate_scope"].is_null()
    {
    } else {
        return "blocked".to_string();
    }
    if payload["selected_backend_readiness"]["blocked"].as_bool() == Some(true) {
        return "blocked".to_string();
    }
    if payload["non_behavioral_route_fields"]
        .as_array()
        .is_some_and(|rows| !rows.is_empty())
    {
        return "blocked".to_string();
    }
    if admissible == Some(false) {
        return "blocked".to_string();
    }
    "pass".to_string()
}

pub(crate) fn route_explain_blocker_codes(
    payload: &serde_json::Value,
    admissible: Option<bool>,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if payload["route_present"].as_bool() != Some(true) {
        blockers.push("route_missing".to_string());
    }
    if payload["dispatch_contract_validation"]["status"].as_str() == Some("blocked") {
        blockers.push(
            payload["dispatch_contract_validation"]["blocker_code"]
                .as_str()
                .unwrap_or(DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE)
                .to_string(),
        );
    }
    if payload["selected_backend"].as_str().is_none() {
        blockers.push("selected_backend_missing".to_string());
    }
    if payload["runtime_assignment_enabled"].as_bool() == Some(false) {
        blockers.push(
            payload["runtime_assignment_reason"]
                .as_str()
                .unwrap_or("runtime_assignment_disabled")
                .to_string(),
        );
    }
    if payload["model_selection_enabled"].as_bool() == Some(false) {
        blockers.push("model_selection_disabled".to_string());
    }
    if !matches!(
        payload["candidate_scope"].as_str(),
        Some("unified_carrier_model_profiles") | None
    ) {
        blockers.push("candidate_scope_not_supported".to_string());
    }
    if payload["selected_backend_readiness"]["blocked"].as_bool() == Some(true) {
        blockers.push("selected_backend_not_ready".to_string());
    }
    if payload["non_behavioral_route_fields"]
        .as_array()
        .is_some_and(|rows| !rows.is_empty())
    {
        blockers.push("route_fields_not_behavioral".to_string());
    }
    if admissible == Some(false) {
        blockers.push("selected_backend_not_admissible_for_dispatch_target".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

#[cfg(test)]
mod tests {
    use super::{
        dispatch_contract_allowed_next_lane_sequence, dispatch_contract_execution_lane_sequence,
        dispatch_contract_lane_activation, dispatch_contract_lane_diagnostic,
        dispatch_contract_lane_sequence, explicit_executor_backend_from_route,
        fallback_executor_backend_from_route, fanout_executor_backends_from_route,
        route_explain_blocker_codes, route_explain_payload as route_explain_payload_with_authority,
        route_explain_status, selected_backend_from_execution_plan_route,
    };

    const FIXTURE_BACKEND_A: &str = "fixture-backend-a";
    const FIXTURE_BACKEND_B: &str = "fixture-backend-b";
    const FIXTURE_CARRIER_A: &str = "fixture-carrier-a";
    const FIXTURE_RUNTIME_ROLE_A: &str = "fixture-runtime-role-a";
    const FIXTURE_RUNTIME_ROLE_B: &str = "fixture-runtime-role-b";
    const FIXTURE_TIER_A: &str = "fixture-tier-a";
    const FIXTURE_TIER_B: &str = "fixture-tier-b";
    const FIXTURE_TASK_CLASS_A: &str = "fixture-task-class-a";
    const FIXTURE_TARGET_A: &str = "fixture-target-a";
    const FIXTURE_TARGET_B: &str = "fixture-target-b";
    const FIXTURE_TARGET_C: &str = "fixture-target-c";
    const FIXTURE_MODEL_REF: &str = "fixture-model-ref";
    const FIXTURE_MODEL_PROFILE: &str = "fixture-model-profile";
    const FIXTURE_PROVIDER: &str = "fixture-provider";
    const FIXTURE_REASONING: &str = "fixture-reasoning";

    fn route_explain_payload(
        execution_plan: &serde_json::Value,
        dispatch_target: &str,
        route: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &compiled_bundle,
            None,
            None,
        )
        .expect("canonical fixture authority should compile");
        let mut execution_plan = execution_plan.clone();
        execution_plan["development_flow"]["dispatch_contract"]["team_flow_authority_id"] =
            serde_json::Value::String(authority.authority_id.clone());
        route_explain_payload_with_authority(
            &execution_plan,
            &compiled_bundle,
            dispatch_target,
            route,
        )
    }

    #[test]
    fn selected_backend_prefers_configured_executor_backend_over_internal_carrier() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_A,
                "activation_agent_type": FIXTURE_TIER_A,
            },
            "development_flow": {
                "implementation": {
                    "preferred_agent_tier": FIXTURE_TIER_B,
                    "preferred_agent_type": FIXTURE_TIER_B,
                    "subagents": FIXTURE_BACKEND_A,
                    "runtime_assignment": {
                        "selected_tier": FIXTURE_TIER_B,
                        "activation_agent_type": FIXTURE_TIER_B,
                    }
                }
            },
            "default_route": {
                "subagents": FIXTURE_BACKEND_A
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some(FIXTURE_BACKEND_A)
        );
    }

    #[test]
    fn selected_backend_does_not_treat_internal_carrier_as_dispatch_backend() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_A,
                "activation_agent_type": FIXTURE_TIER_A,
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": FIXTURE_BACKEND_A,
                    "subagents": FIXTURE_BACKEND_B,
                    "runtime_assignment": {
                        "selected_tier": FIXTURE_TIER_B,
                        "activation_agent_type": FIXTURE_TIER_B,
                    }
                }
            },
            "default_route": {
                "subagents": FIXTURE_BACKEND_B
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some(FIXTURE_BACKEND_A)
        );
    }

    #[test]
    fn selected_backend_keeps_configured_external_backend_from_runtime_assignment() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": FIXTURE_BACKEND_A,
                    "backend_class": "fixture-external-class",
                    "lane_admissibility": {
                        "fixture-class-a": true,
                        "fixture-class-b": true,
                        "fixture-class-c": false,
                        "fixture-class-d": true,
                        "fixture-class-e": false
                    }
                },
                {
                    "backend_id": FIXTURE_BACKEND_B,
                    "backend_class": "fixture-internal-class",
                    "lane_admissibility": {
                        "fixture-class-a": true,
                        "fixture-class-b": true,
                        "fixture-class-c": true,
                        "fixture-class-d": true,
                        "fixture-class-e": true
                    }
                }
            ],
            "runtime_assignment": {
                "selected_backend_id": FIXTURE_BACKEND_A,
                "selected_carrier_id": FIXTURE_CARRIER_A,
                "selected_external_backend_readiness": {
                    "backend_id": FIXTURE_BACKEND_A,
                    "status": "carrier_ready",
                    "blocked": false
                }
            },
            "development_flow": {
                "coach": {
                    "executor_backend": FIXTURE_BACKEND_B,
                    "fallback_executor_backend": FIXTURE_BACKEND_B
                }
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["coach"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some(FIXTURE_BACKEND_A)
        );
    }

    #[test]
    fn runtime_assignment_source_ignores_legacy_execution_plan_alias() {
        let execution_plan = serde_json::json!({
            "codex_runtime_assignment": {
                "selected_tier": FIXTURE_TIER_B,
                "activation_agent_type": FIXTURE_TIER_B,
            }
        });

        assert_eq!(
            super::runtime_assignment_source_from_execution_plan(&execution_plan),
            "missing"
        );
        assert_eq!(
            super::runtime_assignment_from_execution_plan(&execution_plan),
            &serde_json::Value::Null
        );
    }

    #[test]
    fn runtime_assignment_source_ignores_legacy_route_alias() {
        let route = serde_json::json!({
            "codex_runtime_assignment": {
                "selected_tier": FIXTURE_TIER_A,
                "activation_agent_type": FIXTURE_TIER_A,
            }
        });

        assert_eq!(
            super::runtime_assignment_source_from_route(&route),
            "missing"
        );
        assert_eq!(
            super::runtime_assignment_from_route(&route),
            &serde_json::Value::Null
        );
    }

    #[test]
    fn selected_backend_ignores_legacy_alias_when_runtime_assignment_has_no_backend() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_A,
                "activation_agent_type": FIXTURE_TIER_A,
            },
            "codex_runtime_assignment": {
                "selected_tier": FIXTURE_TIER_B,
                "activation_agent_type": FIXTURE_TIER_B,
            },
            "development_flow": {
                "implementation": {
                    "subagents": FIXTURE_BACKEND_A
                }
            },
            "default_route": {
                "subagents": FIXTURE_BACKEND_A
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route),
            None
        );
        assert_eq!(
            super::runtime_assignment_source_from_execution_plan(&execution_plan),
            "runtime_assignment"
        );
    }

    #[test]
    fn selected_backend_ignores_legacy_carrier_and_subagent_hints() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "carrier_backend_hint": "neutral_hint",
                    "subagents": FIXTURE_BACKEND_A
                }
            },
            "default_route": {
                "subagents": FIXTURE_BACKEND_A
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route),
            None
        );
    }

    #[test]
    fn selected_backend_keeps_dispatch_backend_separate_from_selected_carrier() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_A,
                "activation_agent_type": FIXTURE_TIER_A,
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": FIXTURE_BACKEND_A,
                    "fallback_executor_backend": FIXTURE_BACKEND_B,
                    "fanout_executor_backends": [FIXTURE_BACKEND_A, FIXTURE_BACKEND_B],
                    "preferred_agent_tier": FIXTURE_TIER_B,
                    "preferred_agent_type": FIXTURE_TIER_B,
                    "carrier_backend_hint": "legacy_hint",
                    "subagents": "legacy_subagents",
                    "bridge_fallback_subagent": "legacy_bridge",
                    "fanout_subagents": "legacy_fanout",
                }
            },
            "default_route": {
                "subagents": "legacy_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some(FIXTURE_BACKEND_A)
        );
    }

    #[test]
    fn explicit_executor_helpers_preserve_fallback_and_fanout_fields() {
        let route = serde_json::json!({
            "executor_backend": FIXTURE_BACKEND_A,
            "fallback_executor_backend": FIXTURE_BACKEND_B,
            "fanout_executor_backends": [FIXTURE_BACKEND_A, FIXTURE_BACKEND_B]
        });

        assert_eq!(
            explicit_executor_backend_from_route(&route).as_deref(),
            Some(FIXTURE_BACKEND_A)
        );
        assert_eq!(
            fallback_executor_backend_from_route(&route).as_deref(),
            Some(FIXTURE_BACKEND_B)
        );
        assert_eq!(
            fanout_executor_backends_from_route(&route),
            vec![FIXTURE_BACKEND_A.to_string(), FIXTURE_BACKEND_B.to_string()]
        );
    }

    #[test]
    fn explicit_executor_helpers_ignore_legacy_hints() {
        let route = serde_json::json!({
            "carrier_backend_hint": "legacy_hint",
            "subagents": "legacy_subagents",
            "bridge_fallback_subagent": "legacy_bridge",
            "fanout_subagents": "legacy_fanout"
        });

        assert_eq!(explicit_executor_backend_from_route(&route), None);
        assert_eq!(fallback_executor_backend_from_route(&route), None);
        assert!(fanout_executor_backends_from_route(&route).is_empty());
        assert_eq!(
            selected_backend_from_execution_plan_route(&serde_json::json!({}), &route),
            None
        );
    }

    #[test]
    fn selected_backend_does_not_use_activation_agent_type_without_executor_hints() {
        let execution_plan = serde_json::json!({});
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": FIXTURE_TIER_A,
                "activation_runtime_role": FIXTURE_RUNTIME_ROLE_A
            }
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            None
        );
    }

    #[test]
    fn route_payload_keeps_test_author_carrier_separate_from_dispatch_backend() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": FIXTURE_BACKEND_A,
                    "backend_class": "fixture-internal-class",
                    "lane_admissibility": {
                        "fixture-class-a": true,
                        "fixture-class-b": true,
                        "fixture-class-c": true,
                        "fixture-class-d": true,
                        "fixture-class-e": true
                    }
                }
            ],
            "development_flow": {
                "test_author": {
                    "runtime_assignment": {
                        "selected_carrier_id": FIXTURE_CARRIER_A,
                        "activation_agent_type": FIXTURE_TIER_A,
                        "selected_backend_id": FIXTURE_BACKEND_A,
                        "selected_model_ref": FIXTURE_MODEL_REF,
                        "selected_model_profile_id": FIXTURE_MODEL_PROFILE,
                        "selected_model_provider": FIXTURE_PROVIDER,
                        "selected_reasoning_effort": FIXTURE_REASONING,
                        "enabled": true,
                        "model_selection_enabled": true
                    },
                    "activation": {
                        "activation_agent_type": FIXTURE_TIER_A,
                        "activation_runtime_role": FIXTURE_RUNTIME_ROLE_A
                    }
                }
            }
        });
        let route = &execution_plan["development_flow"]["test_author"];
        let payload = route_explain_payload(&execution_plan, "test_author", Some(route));

        assert_eq!(
            payload["selected_carrier_id"].as_str(),
            Some(FIXTURE_CARRIER_A)
        );
        assert_eq!(
            payload["selected_model_ref"].as_str(),
            Some(FIXTURE_MODEL_REF)
        );
        assert_eq!(
            payload["activation_agent_type"].as_str(),
            Some(FIXTURE_TIER_A)
        );
        assert_eq!(
            payload["selected_backend"].as_str(),
            Some(FIXTURE_BACKEND_A)
        );
        assert_ne!(
            payload["selected_backend"].as_str(),
            payload["activation_agent_type"].as_str()
        );
        assert_eq!(route_explain_status(&payload, Some(true)), "pass");
    }

    #[test]
    fn dispatch_contract_lane_rejects_development_flow_route_entries_as_authority() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                (FIXTURE_TARGET_A): {
                    "dispatch_target": FIXTURE_TARGET_A,
                    "task_class": FIXTURE_TASK_CLASS_A,
                    "closure_class": FIXTURE_TASK_CLASS_A,
                    "executor_backend": FIXTURE_BACKEND_A,
                    "activation": {
                        "activation_agent_type": FIXTURE_TIER_A,
                        "activation_runtime_role": FIXTURE_RUNTIME_ROLE_A
                    }
                }
            }
        });

        assert!(dispatch_contract_lane_diagnostic(&execution_plan, FIXTURE_TARGET_A).is_none());
        assert_eq!(
            super::dispatch_target_for_runtime_role_diagnostic(
                &execution_plan,
                FIXTURE_RUNTIME_ROLE_A
            ),
            None
        );

        let payload = route_explain_payload(
            &execution_plan,
            FIXTURE_TARGET_A,
            Some(&execution_plan["development_flow"][FIXTURE_TARGET_A]),
        );
        assert_eq!(payload["route_present"].as_bool(), Some(true));
        assert_eq!(
            payload["selected_backend"].as_str(),
            Some(FIXTURE_BACKEND_A)
        );
        assert_eq!(route_explain_status(&payload, Some(true)), "pass");
    }

    #[test]
    fn route_explain_ignores_raw_dispatch_contract_catalog_tamper() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                (FIXTURE_TARGET_A): {"executor_backend": FIXTURE_BACKEND_A},
                "dispatch_contract": {"execution_lane_sequence": [FIXTURE_TARGET_A]}
            }
        });
        let route = &execution_plan["development_flow"][FIXTURE_TARGET_A];
        let payload = route_explain_payload(&execution_plan, FIXTURE_TARGET_A, Some(route));

        assert_eq!(
            payload["dispatch_contract_validation"]["status"].as_str(),
            Some("pass")
        );
        assert_eq!(route_explain_status(&payload, Some(true)), "pass");
        assert!(!route_explain_blocker_codes(&payload, Some(true))
            .iter()
            .any(|code| code == super::DISPATCH_CONTRACT_LANE_CATALOG_INCOMPLETE));
    }

    #[test]
    fn route_explain_blocks_missing_persisted_team_flow_authority() {
        let payload = route_explain_payload_with_authority(
            &serde_json::json!({}),
            &serde_json::Value::Null,
            FIXTURE_TARGET_A,
            None,
        );
        assert_eq!(
            payload["dispatch_contract_validation"]["blocker_code"].as_str(),
            Some("team_flow_authority_missing")
        );
        assert_eq!(route_explain_status(&payload, Some(true)), "blocked");
    }

    #[test]
    fn route_explain_blocks_mismatched_persisted_team_flow_authority() {
        let compiled_bundle =
            crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle();
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "team_flow_authority_id": "team-flow-authority:tampered"
                }
            }
        });
        let payload = route_explain_payload_with_authority(
            &execution_plan,
            &compiled_bundle,
            FIXTURE_TARGET_A,
            None,
        );
        assert_eq!(
            payload["dispatch_contract_validation"]["blocker_code"].as_str(),
            Some("team_flow_authority_plan_identity_mismatch")
        );
        assert_eq!(route_explain_status(&payload, Some(true)), "blocked");
    }

    #[test]
    fn direct_route_key_aliases_remain_diagnostic_only() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                (FIXTURE_TARGET_A): {
                    "dispatch_target": FIXTURE_TARGET_B,
                    "task_class": FIXTURE_TASK_CLASS_A,
                    "activation": {
                        "activation_runtime_role": FIXTURE_RUNTIME_ROLE_A
                    }
                }
            }
        });

        assert!(dispatch_contract_lane_diagnostic(&execution_plan, FIXTURE_TARGET_A).is_none());
        assert!(dispatch_contract_lane_diagnostic(&execution_plan, FIXTURE_TARGET_B).is_none());
        assert_eq!(
            super::dispatch_target_for_runtime_role_diagnostic(
                &execution_plan,
                FIXTURE_RUNTIME_ROLE_A
            ),
            None
        );
        let selectors: Vec<_> = super::direct_development_flow_route_selectors(&execution_plan)
            .into_iter()
            .map(|(selector, _)| selector)
            .collect();
        assert_eq!(
            selectors,
            vec![FIXTURE_TARGET_A.to_string(), FIXTURE_TARGET_B.to_string()]
        );
    }

    #[test]
    fn direct_development_flow_route_is_diagnostic_only() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "aaa": {
                    "dispatch_target": FIXTURE_TARGET_B,
                    "task_class": FIXTURE_TASK_CLASS_A,
                    "executor_backend": FIXTURE_BACKEND_B,
                    "activation": {
                        "activation_runtime_role": FIXTURE_RUNTIME_ROLE_B
                    }
                }
            }
        });

        assert_eq!(
            super::dispatch_target_for_runtime_role_diagnostic(
                &execution_plan,
                FIXTURE_RUNTIME_ROLE_B
            ),
            None
        );
        assert_eq!(
            dispatch_contract_lane_diagnostic(&execution_plan, FIXTURE_TARGET_B),
            None
        );
    }

    #[test]
    fn selected_backend_prefers_backend_fallback_over_internal_carrier_hint() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_B,
            }
        });
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": FIXTURE_TIER_A,
            },
            "fallback_executor_backend": FIXTURE_BACKEND_B
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            Some(FIXTURE_BACKEND_B)
        );
    }

    #[test]
    fn route_explain_payload_surfaces_hybrid_selection_sources() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": FIXTURE_TIER_B,
                "selected_carrier_tier": FIXTURE_TIER_B,
                "selected_carrier_id": FIXTURE_CARRIER_A,
                "selected_model_profile_id": FIXTURE_MODEL_PROFILE,
                "selected_model_ref": FIXTURE_MODEL_REF,
                "selected_model_provider": FIXTURE_PROVIDER,
                "selected_reasoning_effort": FIXTURE_REASONING,
                "selected_quality_tier": FIXTURE_TIER_A,
                "selected_speed_tier": FIXTURE_TIER_A,
                "selected_model_profile_readiness_status": "ready",
                "budget_verdict": "in_budget",
                "budget_policy": "balanced",
                "budget_scope": "selection_filter_only",
                "selection_budget": {
                    "scope": "selection_filter_only",
                    "policy": "balanced",
                    "max_budget_units": 16,
                    "budget_verdict": "in_budget"
                },
                "runtime_budget_ledger": {
                    "status": "not_tracked_by_runtime_assignment"
                },
                "selection_source_paths": {
                    "selected_carrier_id": "carrier_runtime.roles[fixture].role_id",
                    "selected_model_profile_id": "carrier_runtime.roles[fixture].model_profiles.fixture.profile_id"
                },
                "selection_override_reasons": [],
                "selection_strategy": "balanced_cost_quality",
                "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                "model_selection_enabled": true,
                "candidate_scope": "unified_carrier_model_profiles",
                "rejected_candidates": [
                    {
                        "carrier_id": FIXTURE_TIER_B,
                        "model_profile_id": FIXTURE_MODEL_PROFILE,
                        "reason": "quality_floor_not_met",
                        "reasons": ["quality_floor_not_met"]
                    }
                ],
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": FIXTURE_BACKEND_A,
                    "fallback_executor_backend": FIXTURE_BACKEND_B,
                    "fanout_executor_backends": [FIXTURE_BACKEND_A, FIXTURE_BACKEND_B],
                    "activation": {
                        "activation_agent_type": FIXTURE_TIER_B
                    }
                }
            }
        });
        let route = &execution_plan["development_flow"]["implementation"];
        let payload = route_explain_payload(&execution_plan, "implementation", Some(route));

        assert_eq!(payload["route_present"].as_bool(), Some(true));
        assert_eq!(
            payload["selected_backend"].as_str(),
            Some(FIXTURE_BACKEND_A)
        );
        assert_eq!(
            payload["selected_carrier_id"].as_str(),
            Some(FIXTURE_CARRIER_A)
        );
        assert_eq!(
            payload["selected_model_profile_id"].as_str(),
            Some(FIXTURE_MODEL_PROFILE)
        );
        assert_eq!(payload["budget_verdict"].as_str(), Some("in_budget"));
        assert_eq!(
            payload["budget_scope"].as_str(),
            Some("selection_filter_only")
        );
        assert_eq!(
            payload["selection_budget"]["scope"].as_str(),
            Some("selection_filter_only")
        );
        assert_eq!(
            payload["runtime_budget_ledger"]["status"].as_str(),
            Some("not_tracked_by_runtime_assignment")
        );
        assert_eq!(
            payload["selection_source_paths"]["selected_carrier_id"].as_str(),
            Some("carrier_runtime.roles[fixture].role_id")
        );
        assert_eq!(
            payload["selected_candidate"]["selection_source_paths"]["selected_model_profile_id"]
                .as_str(),
            Some("carrier_runtime.roles[fixture].model_profiles.fixture.profile_id")
        );
        assert_eq!(
            payload["selection_inputs"]["selection_rule"].as_str(),
            Some("role_task_then_readiness_then_score_then_cost_quality")
        );
        assert_eq!(
            payload["selected_candidate"]["model_profile_id"].as_str(),
            Some(FIXTURE_MODEL_PROFILE)
        );
        assert_eq!(
            payload["candidate_pool"]
                .as_array()
                .expect("candidate pool should render")
                .len(),
            2
        );
        assert!(payload["candidate_pool"]
            .as_array()
            .expect("candidate pool should render")
            .iter()
            .any(|row| {
                row["status"].as_str() == Some("rejected")
                    && row["carrier_id"].as_str() == Some(FIXTURE_TIER_B)
            }));
        assert_eq!(
            payload["selection_source"].as_str(),
            Some("route_primary_hint")
        );
        assert_eq!(
            payload["route_primary_backend"].as_str(),
            Some("internal_subagents")
        );
        assert_eq!(payload["fallback_backend"].as_str(), Some("hermes_cli"));
        assert_eq!(route_explain_status(&payload, Some(true)), "pass");
        assert!(route_explain_blocker_codes(&payload, Some(true)).is_empty());
    }

    #[test]
    fn route_explain_status_blocks_missing_route_or_inadmissible_backend() {
        let payload = route_explain_payload(&serde_json::json!({}), "implementation", None);
        assert_eq!(route_explain_status(&payload, None), "blocked");
        assert_eq!(
            route_explain_blocker_codes(&payload, None),
            vec![
                "route_missing".to_string(),
                "selected_backend_missing".to_string()
            ]
        );

        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "executor_backend": "external_cli"
                }
            }
        });
        let route = &execution_plan["development_flow"]["implementation"];
        let payload = route_explain_payload(&execution_plan, "implementation", Some(route));
        assert_eq!(route_explain_status(&payload, Some(false)), "blocked");
        assert!(route_explain_blocker_codes(&payload, Some(false))
            .iter()
            .any(|code| code == "selected_backend_not_admissible_for_dispatch_target"));
    }

    #[test]
    fn route_explain_status_blocks_disabled_model_selection_and_nonbehavioral_route_fields() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "enabled": false,
                "reason": "model_selection_disabled",
                "model_selection_enabled": false,
                "candidate_scope": "unified_carrier_model_profiles"
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "analysis_executor_backend": "opencode_cli",
                    "analysis_fanout_executor_backends": ["hermes_cli", "opencode_cli"],
                    "dispatch_required": "diagnostic_summary_only",
                    "graph_strategy": "diagnostic_summary_only",
                    "provider_price_snapshot": {
                        "source_kind": "external_provider_config_snapshot",
                        "trust_class": "diagnostic_only_until_validated"
                    },
                    "imported_price_authority_override": true,
                    "semantic_score_override_authority": true,
                    "semantic_scoring_order": {
                        "hard_filters_before_semantic_score": true,
                        "semantic_score_can_resurrect_rejected_candidate": false
                    },
                    "semantic_cache_authoritative": true,
                    "semantic_cache_embedding_provider": "remote",
                    "semantic_route_cache": {
                        "validity_scope": {
                            "diagnostic_only": true,
                            "not_runtime_authority": true
                        },
                        "invalidation_tuple": [
                            "request_fingerprint",
                            "compiled_bundle_revision",
                            "carrier_runtime_hash"
                        ]
                    },
                    "gateway_proxy_adapter": "future-only",
                    "workflow_learning_enabled": true,
                    "price_catalog_provider_fetch": "hot_path",
                    "write_scope": "diagnostic_summary_only",
                    "max_cli_subagent_calls": 3
                }
            }
        });
        let route = &execution_plan["development_flow"]["implementation"];
        let payload = route_explain_payload(&execution_plan, "implementation", Some(route));

        assert_eq!(
            payload["selected_backend"].as_str(),
            Some("internal_subagents")
        );
        assert_eq!(payload["runtime_assignment_enabled"].as_bool(), Some(false));
        assert_eq!(
            payload["non_behavioral_route_fields"],
            serde_json::json!([
                "imported_price_authority_override",
                "gateway_proxy_adapter",
                "max_cli_subagent_calls",
                "price_catalog_provider_fetch",
                "semantic_cache_authoritative",
                "semantic_cache_embedding_provider",
                "semantic_score_override_authority",
                "workflow_learning_enabled"
            ])
        );
        assert_eq!(
            payload["rejected_route_fields"],
            serde_json::json!([
                "imported_price_authority_override",
                "gateway_proxy_adapter",
                "max_cli_subagent_calls",
                "price_catalog_provider_fetch",
                "semantic_cache_authoritative",
                "semantic_cache_embedding_provider",
                "semantic_score_override_authority",
                "workflow_learning_enabled"
            ])
        );
        assert_eq!(
            payload["diagnostic_only_route_fields"],
            serde_json::json!([
                "provider_price_snapshot",
                "dispatch_required",
                "graph_strategy",
                "semantic_scoring_order",
                "semantic_route_cache",
                "write_scope"
            ])
        );
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("imported_price_authority_override")
                    && row["truth"].as_str() == Some("rejected_no_runtime_consumer")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("max_cli_subagent_calls")
                    && row["truth"].as_str() == Some("rejected_no_runtime_consumer")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("semantic_cache_authoritative")
                    && row["truth"].as_str() == Some("rejected_no_runtime_consumer")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("semantic_score_override_authority")
                    && row["truth"].as_str() == Some("rejected_no_runtime_consumer")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("provider_price_snapshot")
                    && row["truth"].as_str() == Some("diagnostic_only_no_execution_actuation")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("dispatch_required")
                    && row["truth"].as_str() == Some("diagnostic_only_no_execution_actuation")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("semantic_scoring_order")
                    && row["truth"].as_str() == Some("diagnostic_only_no_execution_actuation")
            }));
        assert!(payload["route_field_truth"]
            .as_array()
            .expect("route field truth should render")
            .iter()
            .any(|row| {
                row["field"].as_str() == Some("semantic_route_cache")
                    && row["truth"].as_str() == Some("diagnostic_only_no_execution_actuation")
            }));
        assert_eq!(route_explain_status(&payload, Some(true)), "blocked");
        let blockers = route_explain_blocker_codes(&payload, Some(true));
        assert!(blockers
            .iter()
            .any(|code| code == "model_selection_disabled"));
        assert!(blockers
            .iter()
            .any(|code| code == "route_fields_not_behavioral"));
    }

    #[test]
    fn dispatch_contract_lane_sequence_canonicalizes_target_aliases() {
        let dispatch_contract = serde_json::json!({
            "lane_sequence": [
                "writer",
                "business_analyst",
                "coach",
                "prover",
                "escalation",
                "release/closure"
            ],
            "execution_lane_sequence": [
                "writer",
                "business_analyst",
                "verifier",
                "escalation"
            ]
        });

        assert_eq!(
            dispatch_contract_lane_sequence(&dispatch_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
        assert_eq!(
            dispatch_contract_execution_lane_sequence(&dispatch_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
    }

    #[test]
    fn dispatch_contract_allowed_next_sequence_prefers_full_lane_order() {
        let full_contract = serde_json::json!({
            "lane_sequence": ["analyst", "designer", "autotester"],
            "execution_lane_sequence": ["analyst", "autotester"]
        });
        assert_eq!(
            dispatch_contract_allowed_next_lane_sequence(&full_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );

        let execution_only_contract = serde_json::json!({
            "execution_lane_sequence": ["analyst", "autotester"]
        });
        assert_eq!(
            dispatch_contract_allowed_next_lane_sequence(&execution_only_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );

        let invalid_catalog_contract = serde_json::json!({
            "lane_sequence": ["analyst", "designer", "autotester"],
            "execution_lane_sequence": ["analyst", "autotester"],
            "lane_catalog": {
                "analyst": {"task_class": "analysis", "stage": "design_gate"},
                "designer": {"task_class": "design", "stage": "design_gate"},
                "autotester": {"task_class": "regression_test", "stage": "verification"}
            }
        });
        assert_eq!(
            dispatch_contract_allowed_next_lane_sequence(&invalid_catalog_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
    }

    #[test]
    fn dispatch_contract_does_not_bypass_later_required_lane_after_terminal_closure() {
        let dispatch_contract = serde_json::json!({
            "lane_sequence": ["implementer"],
            "execution_lane_sequence": ["implementer", "terminal_closure", "tester"]
        });

        assert_eq!(
            dispatch_contract_allowed_next_lane_sequence(&dispatch_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
    }

    #[test]
    fn dispatch_contract_appends_terminal_closure_only_as_final_immediate_successor() {
        let dispatch_contract = serde_json::json!({
            "lane_sequence": ["implementer", "tester"],
            "execution_lane_sequence": ["implementer", "tester", "terminal_closure"]
        });

        assert_eq!(
            dispatch_contract_allowed_next_lane_sequence(&dispatch_contract),
            vec!["blocked:dispatch_contract_lane_catalog_incomplete".to_string()]
        );
    }

    #[test]
    fn dispatch_contract_lane_resolves_aliases_to_canonical_catalog_entries() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "implementer": {
                            "activation": {
                                "activation_runtime_role": FIXTURE_RUNTIME_ROLE_B,
                                "selected_tier": FIXTURE_TIER_B
                            }
                        },
                        "specification": {
                            "activation": {
                                "activation_runtime_role": FIXTURE_RUNTIME_ROLE_A,
                                "selected_tier": FIXTURE_TIER_A
                            }
                        },
                        "verification": {
                            "activation": {
                                "activation_runtime_role": FIXTURE_RUNTIME_ROLE_B,
                                "selected_tier": FIXTURE_TIER_B
                            }
                        }
                    }
                }
            }
        });

        assert_eq!(
            dispatch_contract_lane_diagnostic(&execution_plan, "writer").and_then(|lane| {
                dispatch_contract_lane_activation(lane)["activation_runtime_role"].as_str()
            }),
            Some("worker")
        );
        let alias_target = execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"]
            .as_object()
            .and_then(|catalog| {
                catalog.iter().find_map(|(target, lane)| {
                    (dispatch_contract_lane_activation(lane)["activation_runtime_role"].as_str()
                        == Some(FIXTURE_RUNTIME_ROLE_A))
                    .then(|| target.clone())
                })
            })
            .expect("fixture alias target should exist");
        assert_eq!(
            dispatch_contract_lane_diagnostic(&execution_plan, &alias_target).and_then(|lane| {
                dispatch_contract_lane_activation(lane)["activation_runtime_role"].as_str()
            }),
            Some(FIXTURE_RUNTIME_ROLE_A)
        );
        let verifier_role_alias = crate::runtime_contract_vocab::RUNTIME_ROLE_PROVER;
        let canonical_verifier_target =
            crate::runtime_contract_vocab::canonical_dispatch_target_name(verifier_role_alias);
        let expected_verifier_runtime_role = execution_plan["development_flow"]
            ["dispatch_contract"]["lane_catalog"]
            .get(canonical_verifier_target.as_str())
            .and_then(|lane| {
                dispatch_contract_lane_activation(lane)["activation_runtime_role"].as_str()
            })
            .expect("canonical verifier target should be present in fixture catalog");
        assert_eq!(
            dispatch_contract_lane_diagnostic(&execution_plan, verifier_role_alias).and_then(
                |lane| dispatch_contract_lane_activation(lane)["activation_runtime_role"].as_str(),
            ),
            Some(expected_verifier_runtime_role)
        );
    }
}
