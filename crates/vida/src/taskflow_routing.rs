use crate::runtime_contract_vocab::{
    DISPATCH_TARGET_COACH, DISPATCH_TARGET_EXECUTION_PREPARATION, DISPATCH_TARGET_IMPLEMENTER,
    DISPATCH_TARGET_SPECIFICATION, DISPATCH_TARGET_VERIFICATION, RUNTIME_ROLE_BUSINESS_ANALYST,
    RUNTIME_ROLE_COACH, RUNTIME_ROLE_PM, RUNTIME_ROLE_PROVER, RUNTIME_ROLE_SOLUTION_ARCHITECT,
    RUNTIME_ROLE_VERIFIER, RUNTIME_ROLE_WORKER,
};
use crate::{json_string, json_string_list};

const REJECTED_NON_BEHAVIORAL_ROUTE_FIELDS: &[&str] = &[
    "coach_executor_backend",
    "deterministic_first",
    "external_first_required",
    "local_execution_allowed",
    "local_execution_preferred",
    "max_cli_subagent_calls",
    "max_coach_passes",
    "max_fallback_hops",
    "max_total_runtime_seconds",
    "max_verification_passes",
    "merge_policy",
    "min_output_bytes",
    "web_search_required",
];

const DIAGNOSTIC_ONLY_ROUTE_FIELDS: &[&str] = &[
    "dispatch_required",
    "graph_strategy",
    "internal_escalation_trigger",
    "write_scope",
];

fn legacy_dispatch_contract_lane<'a>(
    dispatch_contract: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    match dispatch_target {
        DISPATCH_TARGET_IMPLEMENTER => dispatch_contract.get("implementer_activation"),
        DISPATCH_TARGET_SPECIFICATION => dispatch_contract.get("specification_activation"),
        DISPATCH_TARGET_COACH => dispatch_contract.get("coach_activation"),
        DISPATCH_TARGET_VERIFICATION => dispatch_contract.get("verifier_activation"),
        "escalation" | DISPATCH_TARGET_EXECUTION_PREPARATION => {
            dispatch_contract.get("escalation_activation")
        }
        _ => None,
    }
}

fn legacy_dispatch_target_for_runtime_role(runtime_role: &str) -> Option<&'static str> {
    match runtime_role {
        RUNTIME_ROLE_BUSINESS_ANALYST | RUNTIME_ROLE_PM => Some(DISPATCH_TARGET_SPECIFICATION),
        RUNTIME_ROLE_WORKER => Some(DISPATCH_TARGET_IMPLEMENTER),
        RUNTIME_ROLE_COACH => Some(DISPATCH_TARGET_COACH),
        RUNTIME_ROLE_VERIFIER | RUNTIME_ROLE_PROVER => Some(DISPATCH_TARGET_VERIFICATION),
        RUNTIME_ROLE_SOLUTION_ARCHITECT => Some(DISPATCH_TARGET_EXECUTION_PREPARATION),
        _ => None,
    }
}

fn canonical_dispatch_target_name(dispatch_target: &str) -> String {
    legacy_dispatch_target_for_runtime_role(dispatch_target)
        .unwrap_or(dispatch_target)
        .to_string()
}

pub(crate) fn dispatch_contract_lane<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(lane) =
        execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"].get(dispatch_target)
    {
        return Some(lane);
    }
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    legacy_dispatch_contract_lane(dispatch_contract, dispatch_target)
}

pub(crate) fn dispatch_contract_lane_activation(lane: &serde_json::Value) -> &serde_json::Value {
    lane.get("activation").unwrap_or(lane)
}

pub(crate) fn dispatch_contract_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    let explicit = dispatch_contract["lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(canonical_dispatch_target_name)
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    let mut legacy = Vec::new();
    if dispatch_contract.get("specification_activation").is_some() {
        legacy.push("specification".to_string());
    }
    if dispatch_contract.get("escalation_activation").is_some() {
        legacy.push("execution_preparation".to_string());
    }
    if dispatch_contract.get("implementer_activation").is_some() {
        legacy.push("implementer".to_string());
    }
    if dispatch_contract.get("coach_activation").is_some() {
        legacy.push("coach".to_string());
    }
    if dispatch_contract.get("verifier_activation").is_some() {
        legacy.push("verification".to_string());
    }
    legacy
}

pub(crate) fn dispatch_contract_execution_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    let explicit = dispatch_contract["execution_lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    dispatch_contract_lane_sequence(dispatch_contract)
        .into_iter()
        .filter(|target| target != "specification")
        .collect()
}

pub(crate) fn dispatch_target_for_runtime_role(
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
    legacy_dispatch_target_for_runtime_role(runtime_role).map(str::to_string)
}

fn carrier_backend_from_assignment(assignment: &serde_json::Value) -> Option<String> {
    json_string(assignment.get("selected_tier"))
        .or_else(|| json_string(assignment.get("activation_agent_type")))
        .filter(|value| !value.is_empty())
}

pub(crate) fn activation_backend_from_route(route: &serde_json::Value) -> Option<String> {
    carrier_backend_from_assignment(dispatch_contract_lane_activation(route))
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
    explicit_executor_backend_from_route(route)
        .or_else(|| activation_backend_from_route(route))
        .or_else(|| route_backend_value(route, "carrier_backend_hint"))
        .or_else(|| route_backend_value(route, "subagents"))
}

pub(crate) fn runtime_assignment_backend_for_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    carrier_backend_from_assignment(runtime_assignment_from_route(route)).or_else(|| {
        carrier_backend_from_assignment(runtime_assignment_from_execution_plan(execution_plan))
    })
}

pub(crate) fn explicit_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "executor_backend")
}

pub(crate) fn fallback_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "fallback_executor_backend")
        .or_else(|| route_backend_value(route, "bridge_fallback_subagent"))
}

pub(crate) fn fanout_executor_backends_from_route(route: &serde_json::Value) -> Vec<String> {
    let values = json_string_list(route.get("fanout_executor_backends"));
    if !values.is_empty() {
        return values;
    }
    json_string_list(route.get("fanout_subagents"))
}

#[allow(dead_code)]
fn legacy_route_backend_hint(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "carrier_backend_hint")
        .or_else(|| route_backend_value(route, "subagents"))
        .or_else(|| route_backend_value(route, "bridge_fallback_subagent"))
        .or_else(|| route_backend_value(route, "fanout_subagents"))
}

pub(crate) fn selected_backend_from_execution_plan_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    runtime_assignment_backend_for_route(execution_plan, route)
        .or_else(|| activation_backend_from_route(route))
        .or_else(|| explicit_executor_backend_from_route(route))
        .or_else(|| route_backend_value(route, "fallback_executor_backend"))
        .or_else(|| route_backend_value(route, "fanout_executor_backends"))
        .or_else(|| legacy_route_backend_hint(route))
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
        .unwrap_or_default();
    let activation_agent_type = route.and_then(activation_backend_from_route);
    let selected_backend =
        route.and_then(|route| selected_backend_from_execution_plan_route(execution_plan, route));
    let non_behavioral_route_fields = route.map(route_non_behavioral_fields).unwrap_or_default();
    let diagnostic_only_route_fields = route.map(route_diagnostic_only_fields).unwrap_or_default();
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
        "route_profile_mapping_applied": assignment_field("route_profile_mapping_applied"),
        "selected_route_profile_mapping": assignment_field("selected_route_profile_mapping"),
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
    })
}

pub(crate) fn route_explain_status(
    payload: &serde_json::Value,
    admissible: Option<bool>,
) -> String {
    if payload["route_present"].as_bool() != Some(true) {
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
        explicit_executor_backend_from_route, fallback_executor_backend_from_route,
        fanout_executor_backends_from_route, route_explain_blocker_codes, route_explain_payload,
        route_explain_status, selected_backend_from_execution_plan_route,
    };

    #[test]
    fn selected_backend_prefers_carrier_tier_over_internal_subagents() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "preferred_agent_tier": "junior",
                    "preferred_agent_type": "junior",
                    "subagents": "internal_subagents",
                    "runtime_assignment": {
                        "selected_tier": "junior",
                        "activation_agent_type": "junior",
                    }
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("junior")
        );
    }

    #[test]
    fn selected_backend_prefers_runtime_assignment_over_explicit_executor_backend() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "subagents": "hermes_cli",
                    "runtime_assignment": {
                        "selected_tier": "junior",
                        "activation_agent_type": "junior",
                    }
                }
            },
            "default_route": {
                "subagents": "hermes_cli"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("junior")
        );
    }

    #[test]
    fn runtime_assignment_source_ignores_legacy_execution_plan_alias() {
        let execution_plan = serde_json::json!({
            "codex_runtime_assignment": {
                "selected_tier": "senior",
                "activation_agent_type": "senior",
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
                "selected_tier": "architect",
                "activation_agent_type": "architect",
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
    fn selected_backend_uses_canonical_runtime_assignment_when_legacy_alias_is_present() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "codex_runtime_assignment": {
                "selected_tier": "senior",
                "activation_agent_type": "senior",
            },
            "development_flow": {
                "implementation": {
                    "subagents": "internal_subagents"
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("middle")
        );
        assert_eq!(
            super::runtime_assignment_source_from_execution_plan(&execution_plan),
            "runtime_assignment"
        );
    }

    #[test]
    fn selected_backend_prefers_carrier_backend_hint_over_legacy_subagents() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "carrier_backend_hint": "neutral_hint",
                    "subagents": "internal_subagents"
                }
            },
            "default_route": {
                "subagents": "internal_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];
        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("neutral_hint")
        );
    }

    #[test]
    fn runtime_assignment_wins_over_route_hints_and_legacy_hints() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "internal_review",
                    "fanout_executor_backends": ["internal_fast", "internal_arch"],
                    "preferred_agent_tier": "junior",
                    "preferred_agent_type": "junior",
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
            Some("middle")
        );
    }

    #[test]
    fn explicit_executor_helpers_preserve_fallback_and_fanout_fields() {
        let route = serde_json::json!({
            "executor_backend": "internal_subagents",
            "fallback_executor_backend": "internal_review",
            "fanout_executor_backends": ["internal_fast", "internal_arch"]
        });

        assert_eq!(
            explicit_executor_backend_from_route(&route).as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            fallback_executor_backend_from_route(&route).as_deref(),
            Some("internal_review")
        );
        assert_eq!(
            fanout_executor_backends_from_route(&route),
            vec!["internal_fast".to_string(), "internal_arch".to_string()]
        );
    }

    #[test]
    fn explicit_executor_helpers_fall_back_to_legacy_hints() {
        let route = serde_json::json!({
            "carrier_backend_hint": "legacy_hint",
            "subagents": "legacy_subagents",
            "bridge_fallback_subagent": "legacy_bridge",
            "fanout_subagents": "legacy_fanout"
        });

        assert_eq!(explicit_executor_backend_from_route(&route), None);
        assert_eq!(
            fallback_executor_backend_from_route(&route).as_deref(),
            Some("legacy_bridge")
        );
        assert_eq!(
            fanout_executor_backends_from_route(&route),
            vec!["legacy_fanout".to_string()]
        );
        assert_eq!(
            selected_backend_from_execution_plan_route(&serde_json::json!({}), &route).as_deref(),
            Some("legacy_hint")
        );
    }

    #[test]
    fn selected_backend_prefers_route_activation_agent_type_when_executor_hints_missing() {
        let execution_plan = serde_json::json!({});
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": "middle",
                "activation_runtime_role": "business_analyst"
            }
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            Some("middle")
        );
    }

    #[test]
    fn selected_backend_prefers_runtime_assignment_over_route_fallback_hint() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "senior",
            }
        });
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": "middle",
            },
            "fallback_executor_backend": "hermes_cli"
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            Some("senior")
        );
    }

    #[test]
    fn route_explain_payload_surfaces_hybrid_selection_sources() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "senior",
                "selected_carrier_tier": "senior",
                "selected_carrier_id": "senior",
                "selected_model_profile_id": "codex_spark_high_readonly",
                "selected_model_ref": "gpt-5.3-codex-spark",
                "selected_model_provider": "openai",
                "selected_reasoning_effort": "high",
                "selected_quality_tier": "high",
                "selected_speed_tier": "medium",
                "selected_model_profile_readiness_status": "ready",
                "budget_verdict": "in_budget",
                "budget_policy": "balanced",
                "selection_strategy": "balanced_cost_quality",
                "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                "model_selection_enabled": true,
                "candidate_scope": "unified_carrier_model_profiles",
                "rejected_candidates": [
                    {
                        "carrier_id": "junior",
                        "model_profile_id": "codex_gpt54_low_write",
                        "reason": "quality_floor_not_met",
                        "reasons": ["quality_floor_not_met"]
                    }
                ],
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "hermes_cli",
                    "fanout_executor_backends": ["middle", "senior"],
                    "activation": {
                        "activation_agent_type": "junior"
                    }
                }
            }
        });
        let route = &execution_plan["development_flow"]["implementation"];
        let payload = route_explain_payload(&execution_plan, "implementation", Some(route));

        assert_eq!(payload["route_present"].as_bool(), Some(true));
        assert_eq!(payload["selected_backend"].as_str(), Some("senior"));
        assert_eq!(payload["selected_carrier_id"].as_str(), Some("senior"));
        assert_eq!(
            payload["selected_model_profile_id"].as_str(),
            Some("codex_spark_high_readonly")
        );
        assert_eq!(payload["budget_verdict"].as_str(), Some("in_budget"));
        assert_eq!(
            payload["selection_inputs"]["selection_rule"].as_str(),
            Some("role_task_then_readiness_then_score_then_cost_quality")
        );
        assert_eq!(
            payload["selected_candidate"]["model_profile_id"].as_str(),
            Some("codex_spark_high_readonly")
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
                    && row["carrier_id"].as_str() == Some("junior")
            }));
        assert_eq!(
            payload["selection_source"].as_str(),
            Some("runtime_assignment")
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
            serde_json::json!(["max_cli_subagent_calls"])
        );
        assert_eq!(
            payload["rejected_route_fields"],
            serde_json::json!(["max_cli_subagent_calls"])
        );
        assert_eq!(
            payload["diagnostic_only_route_fields"],
            serde_json::json!(["dispatch_required", "graph_strategy", "write_scope"])
        );
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
                row["field"].as_str() == Some("dispatch_required")
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
}
