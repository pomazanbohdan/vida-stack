use std::path::Path;

use time::format_description::well_known::Rfc3339;

use crate::release1_contracts::canonical_lane_status_str;
use crate::runtime_contract_vocab::{
    RUNTIME_ROLE_BUSINESS_ANALYST, RUNTIME_ROLE_COACH, RUNTIME_ROLE_PM,
    RUNTIME_ROLE_SOLUTION_ARCHITECT, RUNTIME_ROLE_VERIFIER, TASK_CLASS_ARCHITECTURE,
    TASK_CLASS_COACH, TASK_CLASS_IMPLEMENTATION, TASK_CLASS_SPECIFICATION, TASK_CLASS_VERIFICATION,
};
#[cfg(test)]
use crate::runtime_dispatch_downstream_packets::downstream_dispatch_packet_body;
use crate::runtime_dispatch_downstream_packets::{
    write_runtime_downstream_dispatch_packet, write_runtime_downstream_dispatch_packet_at,
};
use crate::runtime_dispatch_execution::{
    agent_lane_dispatch_result, execute_external_agent_lane_dispatch,
    execute_internal_agent_lane_dispatch,
};
use crate::runtime_dispatch_packet_text::{runtime_packet_prompt, runtime_tracked_flow_packet};
#[cfg(test)]
use crate::runtime_dispatch_packets::explicit_request_scope_paths;
use crate::runtime_dispatch_packets::{
    request_has_explicit_owned_scope, runtime_coach_review_packet, runtime_delivery_task_packet,
    runtime_escalation_packet, runtime_execution_block_packet, runtime_verifier_proof_packet,
    single_task_move_scope_paths,
};
use crate::taskflow_routing::{
    fallback_executor_backend_from_route, fanout_executor_backends_from_route,
    runtime_assignment_source_from_execution_plan,
};

use super::*;

const DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_DISPATCH_HANDOFF_EXECUTION_TIMEOUT_SECONDS: u64 = 10;

pub(crate) fn build_runtime_closure_admission(
    bundle_check: &TaskflowConsumeBundleCheck,
    docflow_verdict: &RuntimeConsumptionDocflowVerdict,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> RuntimeConsumptionClosureAdmission {
    let mut blockers = Vec::new();
    if !bundle_check.ok {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::MissingClosureProof,
        ) {
            blockers.push(code);
        }
        blockers.extend(bundle_check.blockers.iter().cloned());
    }
    if !docflow_verdict.ready {
        blockers.extend(docflow_verdict.blockers.iter().cloned());
    }
    if !docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"))
    {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::MissingClosureProof,
        ) {
            blockers.push(code);
        }
    }
    let has_readiness_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("readiness-check"));
    let has_proof_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"));
    if !(has_readiness_surface && has_proof_surface) {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::RestoreReconcileNotGreen,
        ) {
            blockers.push(code);
        }
    }
    if role_selection.execution_plan["status"] == "design_first" {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::PendingDesignPacket,
        ) {
            blockers.push(code);
        }
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::PendingDeveloperHandoffPacket,
        ) {
            blockers.push(code);
        }
    }
    blockers.sort();
    blockers.dedup();

    let mut proof_surfaces = vec!["vida taskflow consume bundle check".to_string()];
    proof_surfaces.extend(docflow_verdict.proof_surfaces.iter().cloned());

    RuntimeConsumptionClosureAdmission {
        status: if blockers.is_empty() {
            "admit".to_string()
        } else {
            "block".to_string()
        },
        admitted: blockers.is_empty(),
        blockers,
        proof_surfaces,
    }
}

pub(crate) fn build_taskflow_handoff_plan(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let execution_plan = &role_selection.execution_plan;
    let development_flow = &execution_plan["development_flow"];
    let dispatch_contract = &development_flow["dispatch_contract"];
    let lane_catalog = dispatch_contract["lane_catalog"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let activation_chain = lane_catalog
        .iter()
        .map(|(target, lane)| {
            (
                target.clone(),
                dispatch_contract_lane_activation(lane).clone(),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    if execution_plan["status"] == "design_first" {
        return serde_json::json!({
            "status": "spec_first_handoff_required",
            "orchestration_contract": execution_plan["orchestration_contract"],
            "tracked_flow_bootstrap": execution_plan["tracked_flow_bootstrap"],
            "design_packet_activation": runtime_assignment_from_execution_plan(execution_plan),
            "design_packet_activation_source": runtime_assignment_source_from_execution_plan(execution_plan),
            "post_design_activation_chain": activation_chain,
            "post_design_lane_contract": lane_catalog,
            "handoff_ready": true,
        });
    }

    serde_json::json!({
        "status": "execution_handoff_ready",
        "orchestration_contract": execution_plan["orchestration_contract"],
        "activation_chain": activation_chain,
        "lane_contract": lane_catalog,
        "runtime_assignment": runtime_assignment_from_execution_plan(execution_plan),
        "runtime_assignment_source": runtime_assignment_source_from_execution_plan(execution_plan),
        "lane_sequence": development_flow["lane_sequence"],
        "handoff_ready": true,
    })
}

pub(crate) fn runtime_consumption_run_id(
    role_selection: &RuntimeConsumptionLaneSelection,
) -> String {
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["spec_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    if let Some(task_id) = role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
        ["task_id"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        return task_id.to_string();
    }
    let slug = infer_feature_request_slug(&role_selection.request);
    if slug.trim().is_empty() {
        "runtime-consume-request".to_string()
    } else {
        format!("runtime-{slug}")
    }
}

fn missing_agent_lane_dispatch_packet_error(dispatch_target: &str) -> String {
    let _ = blocker_code_str(BlockerCode::MissingPacket);
    format!("Agent lane dispatch for `{dispatch_target}` is missing dispatch_packet_path")
}

pub(crate) fn downstream_activation_fields(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" => (
            "taskflow_pack".to_string(),
            match dispatch_target {
                "spec-pack" => Some("vida taskflow bootstrap-spec".to_string()),
                "work-pool-pack" => Some("vida task ensure".to_string()),
                "dev-pack" => Some("vida task ensure".to_string()),
                _ => None,
            },
            None,
            None,
        ),
        "closure" => ("closure".to_string(), None, None, None),
        _ => {
            let lane = dispatch_contract_lane(&role_selection.execution_plan, dispatch_target);
            (
                "agent_lane".to_string(),
                Some("vida agent-init".to_string()),
                lane.and_then(|row| {
                    json_string(dispatch_contract_lane_activation(row).get("activation_agent_type"))
                }),
                lane.and_then(|row| {
                    json_string(
                        dispatch_contract_lane_activation(row).get("activation_runtime_role"),
                    )
                }),
            )
        }
    }
}

pub(crate) fn execution_plan_route_for_dispatch_target<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    let development_flow = &execution_plan["development_flow"];
    if let Some(route) = development_flow
        .get(dispatch_target)
        .filter(|value| !value.is_null())
    {
        return Some(route);
    }
    let legacy_route_key = match dispatch_target {
        "implementer" => Some("implementation"),
        "execution_preparation" => Some("architecture"),
        _ => None,
    };
    if let Some(route_key) = legacy_route_key {
        if let Some(route) = development_flow
            .get(route_key)
            .filter(|value| !value.is_null())
        {
            return Some(route);
        }
    }
    dispatch_contract_lane(execution_plan, dispatch_target)
}

fn route_selected_backend(route: &serde_json::Value) -> Option<String> {
    selected_backend_from_execution_plan_route(&serde_json::Value::Null, route)
}

fn route_selected_backend_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> Option<String> {
    execution_plan_route_for_dispatch_target(execution_plan, dispatch_target)
        .and_then(route_selected_backend)
}

fn route_declares_backend(route: &serde_json::Value, candidate: &str) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    if route_selected_backend(route).as_deref() == Some(candidate) {
        return true;
    }
    if fallback_executor_backend_from_route(route).as_deref() == Some(candidate) {
        return true;
    }
    fanout_executor_backends_from_route(route)
        .iter()
        .any(|backend| backend == candidate)
}

fn route_has_backend_hints(route: &serde_json::Value) -> bool {
    route_selected_backend(route).is_some()
        || fallback_executor_backend_from_route(route).is_some()
        || !fanout_executor_backends_from_route(route).is_empty()
}

pub(crate) fn downstream_selected_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    activation_agent_type: Option<&str>,
    inherited_selected_backend: Option<&str>,
) -> Option<String> {
    match dispatch_target {
        "spec-pack" | "work-pool-pack" | "dev-pack" | "closure" => activation_agent_type
            .map(str::to_string)
            .or_else(|| inherited_selected_backend.map(str::to_string)),
        _ => {
            let route = execution_plan_route_for_dispatch_target(
                &role_selection.execution_plan,
                dispatch_target,
            )?;
            let route_is_backend_agnostic = !route_has_backend_hints(route);
            route_selected_backend(route)
                .or_else(|| {
                    inherited_selected_backend
                        .filter(|candidate| {
                            route_is_backend_agnostic || route_declares_backend(route, candidate)
                        })
                        .map(str::to_string)
                })
                .or_else(|| {
                    activation_agent_type
                        .filter(|_| route_is_backend_agnostic)
                        .map(str::to_string)
                })
        }
    }
}

fn backend_admissibility_row<'a>(
    execution_plan: &'a serde_json::Value,
    backend_id: &str,
) -> Option<&'a serde_json::Value> {
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["backend_id"].as_str() == Some(backend_id))
}

fn backend_class_for_execution_plan_backend(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> String {
    if backend_id.trim().is_empty() {
        return "unknown".to_string();
    }
    backend_admissibility_row(execution_plan, backend_id)
        .and_then(|row| row["backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if backend_id == "taskflow_state_store" {
                "taskflow_pack".to_string()
            } else {
                "unknown".to_string()
            }
        })
}

fn backend_execution_dimension(backend_class: &str) -> &'static str {
    match backend_class.trim() {
        "external_cli" | "external" => "external",
        "internal" | "internal_cli" => "internal",
        "taskflow_pack" => "taskflow_pack",
        _ => "unknown",
    }
}

pub(crate) fn effective_execution_posture_summary(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    selected_backend: Option<&str>,
    _activation_agent_type: Option<&str>,
    host_runtime: Option<&serde_json::Value>,
    receipt_backed_execution_evidence: bool,
) -> serde_json::Value {
    let route = execution_plan_route_for_dispatch_target(execution_plan, dispatch_target);
    let route_primary_backend = route.and_then(route_selected_backend);
    let fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let effective_selected_backend = selected_backend
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| route_primary_backend.clone());
    let selected_backend_source = if selected_backend.is_some_and(|value| !value.trim().is_empty())
    {
        "dispatch_receipt"
    } else if route_primary_backend.is_some() {
        "route_primary_backend"
    } else {
        "unknown"
    };
    let selected_backend_class = effective_selected_backend
        .as_deref()
        .map(|backend_id| backend_class_for_execution_plan_backend(execution_plan, backend_id));
    let route_primary_backend_class = route_primary_backend
        .as_deref()
        .map(|backend_id| backend_class_for_execution_plan_backend(execution_plan, backend_id));
    let fallback_backend_class = fallback_backend
        .as_deref()
        .map(|backend_id| backend_class_for_execution_plan_backend(execution_plan, backend_id));
    let fanout_backend_classes = fanout_backends
        .iter()
        .map(|backend_id| {
            (
                backend_id.clone(),
                serde_json::Value::String(backend_class_for_execution_plan_backend(
                    execution_plan,
                    backend_id,
                )),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();

    let route_dimensions = route_primary_backend_class
        .iter()
        .chain(fallback_backend_class.iter())
        .map(|value| backend_execution_dimension(value))
        .chain(
            fanout_backend_classes
                .values()
                .filter_map(serde_json::Value::as_str)
                .map(backend_execution_dimension),
        )
        .collect::<std::collections::BTreeSet<_>>();
    let route_contains_internal_backend = route_dimensions.contains("internal");
    let route_contains_external_backend = route_dimensions.contains("external");
    let mixed_route_backends = route_contains_internal_backend && route_contains_external_backend;

    let selected_execution_class = host_runtime
        .and_then(|value| value.get("selected_cli_execution_class"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_cli_system = host_runtime
        .and_then(|value| value.get("selected_cli_system"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let host_execution_dimension = selected_execution_class
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let selected_backend_dimension = selected_backend_class
        .as_deref()
        .map(backend_execution_dimension)
        .unwrap_or("unknown");
    let hybrid_host_backend_selection = matches!(
        (host_execution_dimension, selected_backend_dimension),
        ("internal", "external") | ("external", "internal")
    );
    let effective_posture_kind = if hybrid_host_backend_selection || mixed_route_backends {
        "mixed"
    } else if selected_backend_dimension == "external" || host_execution_dimension == "external" {
        "external"
    } else if selected_backend_dimension == "internal" || host_execution_dimension == "internal" {
        "internal"
    } else if selected_backend_dimension == "taskflow_pack" {
        "taskflow_pack"
    } else {
        "unknown"
    };

    serde_json::json!({
        "dispatch_target": dispatch_target,
        "selected_cli_system": selected_cli_system,
        "selected_execution_class": selected_execution_class,
        "selected_backend": effective_selected_backend,
        "selected_backend_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "route_primary_backend": route_primary_backend,
        "route_primary_backend_class": route_primary_backend_class,
        "fallback_backend": fallback_backend,
        "fallback_backend_class": fallback_backend_class,
        "fanout_backends": fanout_backends,
        "fanout_backend_classes": fanout_backend_classes,
        "route_contains_internal_backend": route_contains_internal_backend,
        "route_contains_external_backend": route_contains_external_backend,
        "mixed_route_backends": mixed_route_backends,
        "hybrid_host_backend_selection": hybrid_host_backend_selection,
        "effective_posture_kind": effective_posture_kind,
        "activation_evidence_state": if receipt_backed_execution_evidence {
            "execution_evidence"
        } else {
            "activation_view_only"
        },
        "receipt_backed_execution_evidence": receipt_backed_execution_evidence,
    })
}

pub(crate) fn canonical_selected_backend_for_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    downstream_selected_backend(
        role_selection,
        &receipt.dispatch_target,
        receipt.activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    )
}

fn backend_policy_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
    backend_id: &str,
) -> Option<&'a serde_json::Value> {
    execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
}

fn backend_class_from_execution_plan(
    execution_plan: &serde_json::Value,
    backend_id: &str,
) -> Option<String> {
    backend_policy_from_execution_plan(execution_plan, backend_id)
        .and_then(|entry| entry["backend_class"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn route_execution_posture_from_classes(classes: &[String]) -> &'static str {
    let has_internal = classes.iter().any(|value| value == "internal");
    let has_external = classes.iter().any(|value| value == "external_cli");
    match (has_internal, has_external) {
        (true, true) => "hybrid",
        (true, false) => "internal_only",
        (false, true) => "external_only",
        _ => "unknown",
    }
}

pub(crate) fn dispatch_execution_route_summary(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    selected_backend: Option<&str>,
) -> serde_json::Value {
    let route =
        execution_plan_route_for_dispatch_target(&role_selection.execution_plan, dispatch_target);
    let route_primary_backend = route.and_then(route_selected_backend);
    let route_fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let route_fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let effective_selected_backend = selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| route_primary_backend.clone());
    let selected_backend_source = match effective_selected_backend.as_deref() {
        Some(_backend_id) if selected_backend.is_some() => "dispatch_receipt",
        Some(backend_id) if route_primary_backend.as_deref() == Some(backend_id) => "route_primary",
        Some(backend_id) if route_fallback_backend.as_deref() == Some(backend_id) => {
            "route_fallback"
        }
        Some(backend_id)
            if route_fanout_backends
                .iter()
                .any(|candidate| candidate == backend_id) =>
        {
            "route_fanout"
        }
        Some(_) => "activation_or_inherited",
        None => "unknown",
    };

    let mut execution_classes = Vec::new();
    for backend_id in effective_selected_backend
        .iter()
        .chain(route_primary_backend.iter())
        .chain(route_fallback_backend.iter())
        .chain(route_fanout_backends.iter())
    {
        if let Some(class) =
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
        {
            if !execution_classes.iter().any(|existing| existing == &class) {
                execution_classes.push(class);
            }
        }
    }
    let effective_execution_posture = route_execution_posture_from_classes(&execution_classes);
    let selected_backend_class = effective_selected_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
        });
    let route_primary_backend_class = route_primary_backend.as_deref().and_then(|backend_id| {
        backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
    });
    let route_fallback_backend_class = route_fallback_backend.as_deref().and_then(|backend_id| {
        backend_class_from_execution_plan(&role_selection.execution_plan, backend_id)
    });
    let route_fanout_backend_classes = route_fanout_backends
        .iter()
        .map(|backend_id| {
            (
                backend_id.clone(),
                serde_json::json!(backend_class_from_execution_plan(
                    &role_selection.execution_plan,
                    backend_id
                )),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let selected_backend_policy = effective_selected_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_primary_backend_policy = route_primary_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fallback_backend_policy = route_fallback_backend
        .as_deref()
        .and_then(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fanout_backend_policies = route_fanout_backends
        .iter()
        .map(|backend_id| {
            backend_policy_from_execution_plan(&role_selection.execution_plan, backend_id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "backend_id": backend_id }))
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "dispatch_target": dispatch_target,
        "effective_selected_backend": effective_selected_backend,
        "selected_backend_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "route_primary_backend": route_primary_backend,
        "route_primary_backend_class": route_primary_backend_class,
        "route_fallback_backend": route_fallback_backend,
        "route_fallback_backend_class": route_fallback_backend_class,
        "route_fanout_backends": route_fanout_backends,
        "route_fanout_backend_classes": route_fanout_backend_classes,
        "selected_backend_policy": selected_backend_policy,
        "route_primary_backend_policy": route_primary_backend_policy,
        "route_fallback_backend_policy": route_fallback_backend_policy,
        "route_fanout_backend_policies": route_fanout_backend_policies,
        "selected_execution_class": selected_backend_class,
        "effective_execution_posture": effective_execution_posture,
        "mixed_posture": effective_execution_posture == "hybrid",
    })
}

fn activation_kind_from_dispatch_result_path(path: &str) -> Option<&'static str> {
    let result = crate::read_json_file_if_present(Path::new(path))?;
    if result["artifact_kind"].as_str() == Some("runtime_lane_completion_result")
        || result["execution_evidence"]["status"].as_str() == Some("recorded")
        || result["activation_semantics"]["activation_kind"].as_str() == Some("execution_evidence")
        || result["execution_state"].as_str() == Some("executed")
    {
        return Some("execution_evidence");
    }
    if result["artifact_kind"].as_str() == Some("runtime_dispatch_result")
        || result["activation_semantics"]["activation_kind"].as_str() == Some("activation_view")
        || result["execution_state"].as_str() == Some("blocked")
    {
        return Some("activation_view");
    }
    None
}

fn resolve_project_artifact_path(
    project_root: &Path,
    raw_path: Option<&str>,
) -> Option<std::path::PathBuf> {
    let raw_path = raw_path.map(str::trim).filter(|value| !value.is_empty())?;
    let path = Path::new(raw_path);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    })
}

pub(crate) fn dispatch_activation_evidence_summary(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> serde_json::Value {
    let dispatch_result_path = nonempty_result_path(receipt.dispatch_result_path.as_deref());
    let downstream_result_path =
        nonempty_result_path(receipt.downstream_dispatch_result_path.as_deref());
    let evidence_path = if dispatch_receipt_has_execution_evidence(receipt) {
        dispatch_result_path
            .clone()
            .or_else(|| downstream_result_path.clone())
    } else {
        dispatch_result_path
            .as_deref()
            .and_then(activation_kind_from_dispatch_result_path)
            .filter(|kind| *kind == "execution_evidence")
            .map(|_| {
                dispatch_result_path
                    .clone()
                    .expect("dispatch_result_path should exist")
            })
            .or_else(|| {
                downstream_result_path
                    .as_deref()
                    .and_then(activation_kind_from_dispatch_result_path)
                    .filter(|kind| *kind == "execution_evidence")
                    .map(|_| {
                        downstream_result_path
                            .clone()
                            .expect("downstream_result_path should exist")
                    })
            })
    };
    let activation_kind = if evidence_path.is_some() {
        "execution_evidence"
    } else {
        "activation_view"
    };
    let result_body = evidence_path
        .as_deref()
        .and_then(|path| crate::read_json_file_if_present(Path::new(path)));
    let activation_semantics = result_body
        .as_ref()
        .and_then(|value| value.get("activation_semantics"))
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "activation_kind": activation_kind,
                "view_only": activation_kind != "execution_evidence",
                "executes_packet": activation_kind == "execution_evidence",
                "records_completion_receipt": activation_kind == "execution_evidence",
            })
        });
    let execution_evidence = result_body
        .as_ref()
        .and_then(|value| value.get("execution_evidence"))
        .cloned()
        .unwrap_or_else(|| {
            if activation_kind == "execution_evidence" {
                serde_json::json!({
                    "status": "recorded",
                    "receipt_backed": true,
                    "result_path": evidence_path.clone(),
                })
            } else {
                serde_json::Value::Null
            }
        });

    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "execution_evidence_path": evidence_path,
        "receipt_backed": activation_kind == "execution_evidence",
        "activation_semantics": activation_semantics,
        "execution_evidence": execution_evidence,
    })
}

fn activation_evidence_from_result_body(result: &serde_json::Value) -> serde_json::Value {
    let activation_kind = result["activation_semantics"]["activation_kind"]
        .as_str()
        .or_else(|| {
            if result["execution_evidence"]["status"].as_str() == Some("recorded")
                || result["execution_state"].as_str() == Some("executed")
            {
                Some("execution_evidence")
            } else if result["artifact_kind"].as_str() == Some("runtime_dispatch_result")
                || result["execution_state"].as_str() == Some("blocked")
            {
                Some("activation_view")
            } else {
                None
            }
        })
        .unwrap_or("activation_view");
    serde_json::json!({
        "activation_kind": activation_kind,
        "evidence_state": if activation_kind == "execution_evidence" {
            "execution_evidence_recorded"
        } else {
            "activation_view_only"
        },
        "activation_semantics": result["activation_semantics"].clone(),
        "execution_evidence": result["execution_evidence"].clone(),
        "receipt_backed": activation_kind == "execution_evidence",
    })
}

fn activation_evidence_from_receipt_result_paths(
    project_root: &Path,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<serde_json::Value> {
    for raw_path in [
        dispatch_receipt.dispatch_result_path.as_deref(),
        dispatch_receipt.downstream_dispatch_result_path.as_deref(),
    ] {
        let Some(resolved) = resolve_project_artifact_path(project_root, raw_path) else {
            continue;
        };
        let Some(result) = crate::read_json_file_if_present(&resolved) else {
            continue;
        };
        return Some(activation_evidence_from_result_body(&result));
    }
    None
}

pub(crate) fn dispatch_surface_truth_from_packet_path(
    project_root: &Path,
    packet_path: Option<&str>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Option<serde_json::Value> {
    let packet_path = resolve_project_artifact_path(project_root, packet_path)?;
    let packet = crate::read_json_file_if_present(&packet_path)?;
    let mut mixed_posture = packet
        .get("mixed_posture")
        .cloned()
        .or_else(|| packet.get("effective_execution_posture").cloned())
        .or_else(|| packet.get("execution_truth").cloned());
    if let Some(object) = mixed_posture
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        if object.get("effective_posture_kind").is_none() {
            if let Some(value) = object
                .get("effective_execution_posture")
                .cloned()
                .or_else(|| object.get("effective_posture_kind").cloned())
            {
                object.insert("effective_posture_kind".to_string(), value);
            }
        }
        if object.get("selected_backend").is_none() {
            if let Some(value) = object
                .get("effective_selected_backend")
                .cloned()
                .or_else(|| object.get("selected_backend").cloned())
            {
                object.insert("selected_backend".to_string(), value);
            }
        }
        if object.get("fanout_backends").is_none() {
            if let Some(value) = object
                .get("fanout_backends")
                .cloned()
                .or_else(|| object.get("fanout_executor_backends").cloned())
            {
                object.insert("fanout_backends".to_string(), value);
            }
        }
        if object.get("fallback_backend").is_none() {
            if let Some(value) = object
                .get("fallback_backend")
                .cloned()
                .or_else(|| object.get("fallback_executor_backend").cloned())
                .or_else(|| object.get("route_fallback_backend").cloned())
            {
                object.insert("fallback_backend".to_string(), value);
            }
        }
    }
    let activation_evidence = activation_evidence_from_receipt_result_paths(project_root, dispatch_receipt)
        .or_else(|| packet.get("activation_vs_execution_evidence").cloned())
        .or_else(|| packet.get("activation_evidence").cloned());
    Some(serde_json::json!({
        "mixed_posture": mixed_posture.unwrap_or(serde_json::Value::Null),
        "activation_vs_execution_evidence": activation_evidence.unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) fn fallback_backend_for_blocked_primary_dispatch_receipt(
    project_root: &std::path::Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return None;
    }
    let route = execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )?;
    let primary_backend =
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, route)?;
    let fallback_backend = fallback_executor_backend_from_route(route)?;
    if primary_backend == fallback_backend {
        return None;
    }
    let selected_backend = canonical_selected_backend_for_receipt(role_selection, dispatch_receipt)
        .or_else(|| dispatch_receipt.selected_backend.clone())?;
    if selected_backend != primary_backend {
        return None;
    }
    let overlay = load_project_overlay_yaml_for_root(project_root).ok()?;
    let (selected_cli_system, selected_cli_entry) =
        selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preflight = crate::status_surface_external_cli::external_cli_preflight_summary(
        &overlay,
        &selected_cli_system,
        selected_cli_entry.as_ref(),
    );
    let carrier_blocked = preflight["carrier_readiness"]["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|carrier| {
            carrier["backend_id"].as_str() == Some(primary_backend.as_str())
                && carrier["blocked"].as_bool() == Some(true)
        });
    carrier_blocked.then_some(fallback_backend)
}

pub(crate) fn build_downstream_dispatch_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<crate::state_store::RunGraphDispatchReceipt> {
    let dispatch_target = receipt.downstream_dispatch_target.clone()?;
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(role_selection, &dispatch_target);
    let selected_backend = downstream_selected_backend(
        role_selection,
        &dispatch_target,
        activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    )
    .filter(|value| !value.is_empty());
    let dispatch_status = if receipt.downstream_dispatch_ready {
        "routed".to_string()
    } else {
        "blocked".to_string()
    };
    Some(crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: dispatch_status.clone(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        lane_status: derive_lane_status(
            &dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: receipt.downstream_dispatch_command.clone(),
        dispatch_packet_path: receipt.downstream_dispatch_packet_path.clone(),
        dispatch_result_path: None,
        blocker_code: if dispatch_status == "blocked" && receipt.dispatch_status != "executed" {
            blocker_code_value(BlockerCode::MissingLaneReceipt)
        } else if dispatch_status == "blocked" && receipt.downstream_dispatch_packet_path.is_none()
        {
            blocker_code_value(BlockerCode::MissingPacket)
        } else {
            None
        },
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    })
}

fn root_receipt_fields_from_downstream_step(
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    step_receipt: &crate::state_store::RunGraphDispatchReceipt,
) {
    root_receipt.downstream_dispatch_target = step_receipt.downstream_dispatch_target.clone();
    root_receipt.downstream_dispatch_command = step_receipt.downstream_dispatch_command.clone();
    root_receipt.downstream_dispatch_note = step_receipt.downstream_dispatch_note.clone();
    root_receipt.downstream_dispatch_ready = step_receipt.downstream_dispatch_ready;
    root_receipt.downstream_dispatch_blockers = step_receipt.downstream_dispatch_blockers.clone();
    root_receipt.downstream_dispatch_packet_path =
        step_receipt.downstream_dispatch_packet_path.clone();
    root_receipt.downstream_dispatch_status = step_receipt.downstream_dispatch_status.clone();
    root_receipt.downstream_dispatch_result_path =
        step_receipt.downstream_dispatch_result_path.clone();
    root_receipt.downstream_dispatch_active_target =
        step_receipt.downstream_dispatch_active_target.clone();
    root_receipt.supersedes_receipt_id = step_receipt.supersedes_receipt_id.clone();
    root_receipt.exception_path_receipt_id = step_receipt.exception_path_receipt_id.clone();
    root_receipt.blocker_code = step_receipt.blocker_code.clone();
}

pub(crate) fn active_downstream_dispatch_target(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if receipt.dispatch_kind == "agent_lane" && receipt.dispatch_status != "executed" {
        Some(receipt.dispatch_target.clone())
    } else {
        None
    }
}

fn agent_init_packet_flag_for_path(packet_path: &str) -> &'static str {
    if packet_path.contains("/downstream-dispatch-packets/")
        || packet_path.contains("downstream-dispatch-packets")
    {
        "--downstream-packet"
    } else {
        "--dispatch-packet"
    }
}

pub(crate) fn agent_init_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --json",
        agent_init_packet_flag_for_path(packet_path),
        shell_quote(packet_path)
    )
}

pub(crate) fn agent_init_execute_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --execute-dispatch --json",
        agent_init_packet_flag_for_path(packet_path),
        shell_quote(packet_path)
    )
}

pub(crate) fn runtime_host_execution_contract_for_root(project_root: &Path) -> serde_json::Value {
    let project_activation_view =
        project_activator_surface::build_project_activator_view(project_root);
    let host_environment = &project_activation_view["host_environment"];
    serde_json::json!({
        "selected_cli_system": host_environment["selected_cli_system"],
        "selected_cli_execution_class": host_environment["selected_cli_execution_class"],
        "runtime_template_root": host_environment["runtime_template_root"],
        "template_materialized": host_environment["template_materialized"],
    })
}

pub(crate) fn load_project_overlay_yaml_for_root(
    project_root: &Path,
) -> Result<serde_yaml::Value, String> {
    let path = project_root.join("vida.config.yaml");
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeAgentLaneDispatch {
    pub(crate) surface: String,
    pub(crate) activation_command: String,
    pub(crate) backend_dispatch: serde_json::Value,
}

pub(crate) fn selected_host_cli_system_for_runtime_dispatch(
    overlay: &serde_yaml::Value,
) -> (String, Option<serde_yaml::Value>) {
    let registry = project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay));
    let candidate = yaml_lookup(overlay, &["host_environment", "cli_system"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "__HOST_CLI_SYSTEM__")
        .and_then(project_activator_surface::normalize_host_cli_system);
    let selected = candidate.unwrap_or_else(|| {
        let mut supported = registry
            .iter()
            .filter(|(_, entry)| yaml_bool(yaml_lookup(entry, &["enabled"]), true))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        supported.sort();
        supported
            .into_iter()
            .next()
            .or_else(|| {
                let mut fallback = registry.keys().cloned().collect::<Vec<_>>();
                fallback.sort();
                fallback.into_iter().next()
            })
            .unwrap_or_default()
    });
    let entry = registry.get(&selected).cloned();
    (selected, entry)
}

pub(crate) fn configured_dispatch_backend_class(
    overlay: &serde_yaml::Value,
    system: &str,
) -> String {
    project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay))
        .get(system)
        .and_then(|entry| {
            yaml_string(yaml_lookup(entry, &["dispatch_backend_class"]))
                .or_else(|| yaml_string(yaml_lookup(entry, &["backend_class"])))
        })
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "external_cli".to_string())
}

fn configured_subagent_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|entries| {
            entries.iter().find_map(|(key, value)| {
                let id = key.as_str()?.trim();
                if id == backend_id && yaml_bool(yaml_lookup(value, &["enabled"]), false) {
                    Some(value)
                } else {
                    None
                }
            })
        })
}

pub(crate) fn configured_external_backend_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    let entry = configured_subagent_entry(overlay, backend_id)?;
    (yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref()
        == Some("external_cli"))
    .then_some(entry)
}

pub(crate) fn selected_external_backend_for_system(
    overlay: &serde_yaml::Value,
    system: &str,
    preferred_backend: Option<&str>,
) -> Option<(String, serde_yaml::Value)> {
    let subagents = yaml_lookup(overlay, &["agent_system", "subagents"])?;
    let entries = subagents.as_mapping()?;
    let backend_class = configured_dispatch_backend_class(overlay, system);
    let configured_backend_id =
        project_activator_surface::host_cli_system_registry_with_fallback(Some(overlay))
            .get(system)
            .and_then(|entry| {
                yaml_string(yaml_lookup(entry, &["external_backend_id"]))
                    .or_else(|| yaml_string(yaml_lookup(entry, &["dispatch_backend_id"])))
            })
            .filter(|value| !value.trim().is_empty());
    if let Some(preferred_backend) = preferred_backend {
        for (key, value) in entries {
            let backend_id = key.as_str()?.trim();
            if backend_id != preferred_backend {
                continue;
            }
            if !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
                continue;
            }
            if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
                != Some(backend_class.as_str())
            {
                continue;
            }
            return Some((backend_id.to_string(), value.clone()));
        }
        return None;
    }
    if let Some(configured_backend_id) = configured_backend_id.as_deref() {
        for (key, value) in entries {
            let backend_id = key.as_str()?.trim();
            if backend_id != configured_backend_id {
                continue;
            }
            if !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
                continue;
            }
            if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
                != Some(backend_class.as_str())
            {
                continue;
            }
            return Some((backend_id.to_string(), value.clone()));
        }
    }
    let mut fallback = None;
    for (key, value) in entries {
        let backend_id = key.as_str()?.trim();
        if backend_id.is_empty() || !yaml_bool(yaml_lookup(value, &["enabled"]), false) {
            continue;
        }
        if yaml_string(yaml_lookup(value, &["subagent_backend_class"])).as_deref()
            != Some(backend_class.as_str())
        {
            continue;
        }
        let detect_command = yaml_string(yaml_lookup(value, &["detect_command"]));
        if detect_command.as_deref() == Some(system) {
            return Some((backend_id.to_string(), value.clone()));
        }
        if fallback.is_none() {
            fallback = Some((backend_id.to_string(), value.clone()));
        }
    }
    fallback
}

fn external_cli_activation_prompt(packet_path: &str) -> String {
    format!(
        "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
        packet_path
    )
}

fn configured_external_activation_prompt(
    backend_entry: &serde_yaml::Value,
    packet_path: &str,
) -> String {
    yaml_lookup(backend_entry, &["dispatch", "prompt_template"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|template| {
            template
                .replace("{packet_path}", packet_path)
                .replace("{dispatch_packet_path}", packet_path)
        })
        .unwrap_or_else(|| external_cli_activation_prompt(packet_path))
}

fn configured_external_dispatch_pin_args(backend_entry: &serde_yaml::Value) -> Vec<String> {
    let mut args = Vec::new();
    let dispatch = match yaml_lookup(backend_entry, &["dispatch"]) {
        Some(value) => value,
        None => return args,
    };

    if let Some(provider_flag) = yaml_string(yaml_lookup(dispatch, &["provider_flag"])) {
        let provider_value = yaml_string(yaml_lookup(dispatch, &["provider_value"]))
            .or_else(|| {
                yaml_string(yaml_lookup(backend_entry, &["default_model"])).and_then(|value| {
                    if value.contains("provider-configured") {
                        return None;
                    }
                    value
                        .split_once('/')
                        .map(|(provider, _)| provider.trim().to_string())
                })
            })
            .filter(|value| !value.is_empty() && !value.contains("provider-configured"));
        if let Some(provider_value) = provider_value {
            args.push(provider_flag);
            args.push(provider_value);
        }
    }

    if let Some(model_flag) = yaml_string(yaml_lookup(dispatch, &["model_flag"])) {
        let default_model = yaml_string(yaml_lookup(backend_entry, &["default_model"]))
            .filter(|value| !value.is_empty() && !value.contains("provider-configured"));
        if let Some(default_model) = default_model {
            args.push(model_flag);
            args.push(default_model);
        }
    }

    if let Some(variant_flag) = yaml_string(yaml_lookup(dispatch, &["variant_flag"])) {
        if let Some(variant_value) =
            yaml_string(yaml_lookup(dispatch, &["variant_value"])).filter(|value| !value.is_empty())
        {
            args.push(variant_flag);
            args.push(variant_value);
        }
    }

    args
}

fn configured_external_activation_command(
    backend_entry: &serde_yaml::Value,
    project_root: &Path,
    packet_path: &str,
) -> Option<String> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    let command = yaml_string(yaml_lookup(dispatch, &["command"]))?;
    let mut parts = Vec::new();
    if let Some(env_map) = yaml_lookup(dispatch, &["env"]).and_then(serde_yaml::Value::as_mapping) {
        let mut env_pairs = env_map
            .iter()
            .filter_map(|(key, value)| {
                Some(format!(
                    "{}={}",
                    key.as_str()?.trim(),
                    shell_quote(value.as_str()?.trim())
                ))
            })
            .collect::<Vec<_>>();
        env_pairs.sort();
        parts.extend(env_pairs);
    }
    parts.push(command);
    parts.extend(yaml_string_list(yaml_lookup(dispatch, &["static_args"])));
    parts.extend(configured_external_dispatch_pin_args(backend_entry));
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        parts.push(workdir_flag);
        parts.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    if prompt_mode == "positional" {
        parts.push(configured_external_activation_prompt(
            backend_entry,
            packet_path,
        ));
    }
    Some(
        parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 || (index > 0 && part.contains('=') && !part.starts_with('-')) {
                    part
                } else {
                    shell_quote(&part)
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn configured_external_activation_parts(
    backend_entry: &serde_yaml::Value,
    project_root: &Path,
    packet_path: &str,
) -> Result<(String, Vec<String>), String> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])
        .ok_or_else(|| "Configured external backend is missing `dispatch`".to_string())?;
    let command = yaml_string(yaml_lookup(dispatch, &["command"]))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Configured external backend is missing non-empty `dispatch.command`".to_string()
        })?;
    let mut args = yaml_string_list(yaml_lookup(dispatch, &["static_args"]));
    args.extend(configured_external_dispatch_pin_args(backend_entry));
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        args.push(workdir_flag);
        args.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => args.push(configured_external_activation_prompt(
            backend_entry,
            packet_path,
        )),
        other => {
            return Err(format!(
                "Configured external backend uses unsupported prompt_mode `{other}`"
            ));
        }
    }
    Ok((command, args))
}

pub(crate) fn render_command_display(command: &str, args: &[String]) -> String {
    let mut rendered = Vec::with_capacity(args.len() + 1);
    rendered.push(shell_quote(command));
    rendered.extend(args.iter().map(|arg| shell_quote(arg)));
    rendered.join(" ")
}

#[cfg(test)]
mod runtime_dispatch_external_backend_tests {
    use super::*;

    #[test]
    fn selected_external_backend_prefers_system_configured_backend_id() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      external_backend_id: qwen_dispatch
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
    qwen_dispatch:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");
        assert_eq!(backend_id, "qwen_dispatch");
    }

    #[test]
    fn configured_external_activation_parts_uses_prompt_template_when_present() {
        let backend_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: qwen
  static_args: ["run"]
  prompt_mode: positional
  prompt_template: "Process packet {packet_path} exactly once."
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "qwen");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "Process packet /tmp/project/.vida/dispatch.json exactly once.".to_string()
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_injects_provider_and_model_flags() {
        let backend_entry = serde_yaml::from_str(
            r#"
default_model: opencode/minimax-m2.5-free
dispatch:
  command: opencode
  static_args: ["run"]
  provider_flag: --provider
  provider_value: opencode
  model_flag: --model
  workdir_flag: --dir
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "opencode");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "--provider".to_string(),
                "opencode".to_string(),
                "--model".to_string(),
                "opencode/minimax-m2.5-free".to_string(),
                "--dir".to_string(),
                "/tmp/project".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
    }

    #[test]
    fn configured_external_activation_parts_skips_provider_configured_model_placeholders() {
        let backend_entry = serde_yaml::from_str(
            r#"
default_model: hermes/provider-configured
dispatch:
  command: hermes
  static_args: ["chat", "-Q", "-q"]
  model_flag: --model
  provider_flag: --provider
  prompt_mode: positional
"#,
        )
        .expect("backend entry should parse");

        let (command, args) = configured_external_activation_parts(
            &backend_entry,
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
        )
        .expect("dispatch parts should render");

        assert_eq!(command, "hermes");
        assert_eq!(
            args,
            vec![
                "chat".to_string(),
                "-Q".to_string(),
                "-q".to_string(),
                external_cli_activation_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
    }

    #[test]
    fn selected_external_backend_uses_configured_backend_class() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      dispatch_backend_class: remote_cli
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
    qwen_remote:
      enabled: true
      subagent_backend_class: remote_cli
"#,
        )
        .expect("overlay should parse");

        let backend_class = configured_dispatch_backend_class(&overlay, "qwen");
        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");

        assert_eq!(backend_class, "remote_cli");
        assert_eq!(backend_id, "qwen_remote");
    }

    #[test]
    fn selected_external_backend_fails_closed_when_preferred_backend_is_unavailable() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        assert!(
            selected_external_backend_for_system(&overlay, "qwen", Some("opencode_cli")).is_none()
        );
    }

    #[test]
    fn selected_external_backend_does_not_prefer_name_pattern_without_config_or_detect_signal() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
agent_system:
  subagents:
    alpha_external:
      enabled: true
      subagent_backend_class: external_cli
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let (backend_id, _) =
            selected_external_backend_for_system(&overlay, "qwen", None).expect("backend");

        assert_eq!(backend_id, "alpha_external");
    }

    #[test]
    fn configured_subagent_entry_resolves_enabled_internal_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let entry = configured_subagent_entry(&overlay, "internal_subagents")
            .expect("internal backend should resolve");

        assert_eq!(
            yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref(),
            Some("internal")
        );
    }

    #[test]
    fn internal_host_ignores_explicit_external_backend_and_stays_on_agent_init() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("qwen_cli"),
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn external_host_keeps_policy_selected_internal_backend_on_agent_init() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      runtime_root: .qwen
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "qwen",
            "external",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            Some("internal_subagents"),
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }

    #[test]
    fn internal_host_without_preferred_backend_stays_on_agent_init() {
        let overlay = serde_yaml::from_str(
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      runtime_root: .codex
agent_system:
  subagents:
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
"#,
        )
        .expect("overlay should parse");

        let dispatch = runtime_agent_lane_dispatch_from_overlay(
            Some(&overlay),
            "codex",
            "internal",
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
        );
    }
}

fn runtime_agent_lane_dispatch_from_overlay(
    overlay: Option<&serde_yaml::Value>,
    selected_cli_system: &str,
    selected_execution_class: &str,
    project_root: &Path,
    packet_path: &str,
    preferred_backend: Option<&str>,
) -> RuntimeAgentLaneDispatch {
    let agent_init_command = agent_init_execute_command_for_packet_path(packet_path);
    let internal_host_backend_hint = preferred_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|backend_id| {
            let configured_backend = overlay
                .and_then(|overlay| configured_subagent_entry(overlay, backend_id))
                .is_some();
            (!configured_backend).then(|| backend_id.to_string())
        });
    if let Some((backend_id, backend_class, backend_entry)) = overlay.and_then(|overlay| {
        preferred_backend.and_then(|backend_id| {
            configured_subagent_entry(overlay, backend_id).and_then(|entry| {
                yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
                    .map(|backend_class| (backend_id.to_string(), backend_class, entry))
            })
        })
    }) {
        if backend_class == "internal" {
            return RuntimeAgentLaneDispatch {
                surface: "vida agent-init".to_string(),
                activation_command: agent_init_command,
                backend_dispatch: serde_json::json!({
                    "selected_cli_system": selected_cli_system,
                    "selected_execution_class": selected_execution_class,
                    "backend_class": backend_class,
                    "backend_id": backend_id,
                    "policy_selected_internal_backend": true,
                }),
            };
        }
        return RuntimeAgentLaneDispatch {
            surface: format!("external_cli:{backend_id}"),
            activation_command: configured_external_activation_command(
                backend_entry,
                project_root,
                packet_path,
            )
            .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path)),
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_class": backend_class,
                "backend_id": backend_id,
                "policy_selected_external_backend": true,
            }),
        };
    }
    if selected_execution_class != "external" {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": internal_host_backend_hint,
            }),
        };
    }
    let Some(overlay) = overlay else {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": serde_json::Value::Null,
            }),
        };
    };
    let Some((backend_id, backend_entry)) =
        selected_external_backend_for_system(overlay, selected_cli_system, preferred_backend)
    else {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": serde_json::Value::Null,
            }),
        };
    };
    let backend_class = configured_dispatch_backend_class(overlay, selected_cli_system);
    let activation_command =
        configured_external_activation_command(&backend_entry, project_root, packet_path)
            .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path));
    RuntimeAgentLaneDispatch {
        surface: format!("{backend_class}:{backend_id}"),
        activation_command,
        backend_dispatch: serde_json::json!({
            "selected_cli_system": selected_cli_system,
            "selected_execution_class": selected_execution_class,
            "backend_class": backend_class,
            "backend_id": backend_id,
        }),
    }
}

pub(crate) fn runtime_agent_lane_dispatch_for_root(
    project_root: &Path,
    packet_path: &str,
    preferred_backend: Option<&str>,
) -> RuntimeAgentLaneDispatch {
    let host_runtime = runtime_host_execution_contract_for_root(project_root);
    let selected_cli_system = json_string(host_runtime.get("selected_cli_system"))
        .unwrap_or_else(|| "unknown".to_string());
    let selected_execution_class = json_string(host_runtime.get("selected_cli_execution_class"))
        .unwrap_or_else(|| "unknown".to_string());
    let overlay = load_project_overlay_yaml_for_root(project_root).ok();
    let effective_system = overlay
        .as_ref()
        .map(|config| selected_host_cli_system_for_runtime_dispatch(config).0)
        .unwrap_or_else(|| selected_cli_system.clone());
    runtime_agent_lane_dispatch_from_overlay(
        overlay.as_ref(),
        &effective_system,
        &selected_execution_class,
        project_root,
        packet_path,
        preferred_backend,
    )
}

pub(crate) fn dispatch_receipt_has_execution_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    match receipt.dispatch_status.as_str() {
        "executed" => true,
        "packet_ready" => {
            receipt.blocker_code.is_none()
                && receipt
                    .dispatch_result_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty())
        }
        _ => false,
    }
}

fn nonempty_result_path(path: Option<&str>) -> Option<String> {
    path.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn tracked_implementer_dev_task_id<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn tracked_specification_task_id<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["spec_task"]["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn tracked_design_doc_path<'a>(
    role_selection: &'a RuntimeConsumptionLaneSelection,
) -> Option<&'a str> {
    role_selection.execution_plan["tracked_flow_bootstrap"]["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn tracked_implementer_task_closed(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_target != "implementer" {
        return false;
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return false;
    };
    store
        .show_task(task_id)
        .await
        .map(|task| task.status == "closed")
        .unwrap_or(false)
}

fn tracked_design_doc_finalized(role_selection: &RuntimeConsumptionLaneSelection) -> bool {
    let Some(path) = tracked_design_doc_path(role_selection) else {
        return false;
    };
    std::fs::read_to_string(path)
        .map(|contents| contents.contains("Status: `approved`"))
        .unwrap_or(false)
}

async fn tracked_specification_task_closed(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.dispatch_target != "specification" {
        return false;
    }
    let Some(task_id) = tracked_specification_task_id(role_selection) else {
        return false;
    };
    store
        .show_task(task_id)
        .await
        .map(|task| task.status == "closed")
        .unwrap_or(false)
}

async fn tracked_implementer_task_close_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    let implementer_context = receipt.dispatch_target == "implementer"
        || receipt.downstream_dispatch_last_target.as_deref() == Some("implementer");
    if !implementer_context {
        return Ok(None);
    }
    if !tracked_implementer_task_closed(store, role_selection, receipt).await {
        return Ok(None);
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return Ok(None);
    };
    let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let completion_receipt_id = format!("task-close-{task_id}");
    write_runtime_lane_completion_result(
        store.root(),
        &receipt.run_id,
        "implementer",
        &completion_receipt_id,
        packet_path,
    )
    .map(Some)
}

async fn receipt_backed_execution_evidence_path(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    if let Some(path) =
        tracked_implementer_task_close_evidence_path(store, role_selection, receipt).await?
    {
        return Ok(Some(path));
    }
    if dispatch_receipt_has_execution_evidence(receipt) {
        return Ok(nonempty_result_path(
            receipt.dispatch_result_path.as_deref(),
        ));
    }
    Ok(nonempty_result_path(
        receipt.downstream_dispatch_result_path.as_deref(),
    ))
}

fn decode_receipt_packet_context(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(RuntimeConsumptionLaneSelection, serde_json::Value), String> {
    let packet_path = receipt.dispatch_packet_path.as_deref().ok_or_else(|| {
        format!(
            "Persisted dispatch receipt for `{}` is missing dispatch_packet_path",
            receipt.run_id
        )
    })?;
    let body = std::fs::read_to_string(packet_path).map_err(|error| {
        format!("Failed to read persisted dispatch packet `{packet_path}`: {error}")
    })?;
    let packet = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
        format!("Failed to parse persisted dispatch packet `{packet_path}`: {error}")
    })?;
    let role_selection = serde_json::from_value::<RuntimeConsumptionLaneSelection>(
        packet
            .get("role_selection_full")
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Persisted dispatch packet `{packet_path}` is missing role_selection_full"
                )
            })?,
    )
    .map_err(|error| {
        format!(
            "Failed to decode role_selection_full from persisted dispatch packet `{packet_path}`: {error}"
        )
    })?;
    let run_graph_bootstrap = packet.get("run_graph_bootstrap").cloned().ok_or_else(|| {
        format!("Persisted dispatch packet `{packet_path}` is missing run_graph_bootstrap")
    })?;
    Ok((role_selection, run_graph_bootstrap))
}

async fn maybe_bridge_closed_implementer_task_into_receipt_with_context(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    closed_task_id: Option<&str>,
) -> Result<bool, String> {
    if receipt.downstream_dispatch_last_target.as_deref() != Some("implementer") {
        return Ok(false);
    }
    if receipt
        .downstream_dispatch_target
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Ok(false);
    }
    let Some(task_id) = tracked_implementer_dev_task_id(role_selection) else {
        return Ok(false);
    };
    if closed_task_id.is_some_and(|value| value != task_id) {
        return Ok(false);
    }
    let implementer_receipt = crate::state_store::RunGraphDispatchReceipt {
        dispatch_target: "implementer".to_string(),
        ..receipt.clone()
    };
    if !tracked_implementer_task_closed(store, role_selection, &implementer_receipt).await {
        return Ok(false);
    }
    try_bridge_bounded_implementer_completion_to_downstream_receipt(
        store,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )
    .await
}

pub(crate) async fn maybe_bridge_closed_implementer_task_into_receipt(
    store: &StateStore,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    closed_task_id: Option<&str>,
) -> Result<bool, String> {
    let (role_selection, run_graph_bootstrap) = decode_receipt_packet_context(receipt)?;
    maybe_bridge_closed_implementer_task_into_receipt_with_context(
        store,
        &role_selection,
        &run_graph_bootstrap,
        receipt,
        closed_task_id,
    )
    .await
}

pub(crate) async fn maybe_bridge_closed_implementer_task_into_latest_receipt(
    store: &StateStore,
    closed_task_id: &str,
) -> Result<bool, String> {
    let Some(mut receipt) = store
        .latest_run_graph_dispatch_receipt()
        .await
        .map_err(|error| format!("Failed to load latest run-graph dispatch receipt: {error}"))?
    else {
        return Ok(false);
    };
    if !maybe_bridge_closed_implementer_task_into_receipt(store, &mut receipt, Some(closed_task_id))
        .await?
    {
        return Ok(false);
    }
    store
        .record_run_graph_dispatch_receipt(&receipt)
        .await
        .map_err(|error| {
            format!("Failed to persist bridged run-graph dispatch receipt: {error}")
        })?;
    Ok(true)
}

fn receipt_waiting_on_implementer_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.downstream_dispatch_last_target.as_deref() == Some("implementer")
        && receipt.downstream_dispatch_target.as_deref() == Some("coach")
        && !receipt.downstream_dispatch_ready
        && receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|value| value == blocker_code_str(BlockerCode::PendingImplementationEvidence))
}

fn blocked_implementer_step_receipt(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphDispatchReceipt {
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        downstream_activation_fields(role_selection, "implementer");
    crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: "implementer".to_string(),
        dispatch_status: receipt
            .downstream_dispatch_status
            .clone()
            .unwrap_or_else(|| "blocked".to_string()),
        lane_status: derive_lane_status(
            receipt
                .downstream_dispatch_status
                .as_deref()
                .unwrap_or("blocked"),
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: Some("vida agent-init".to_string()),
        dispatch_packet_path: None,
        dispatch_result_path: None,
        blocker_code: receipt.blocker_code.clone(),
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend: receipt.selected_backend.clone(),
        recorded_at,
    }
}

pub(crate) async fn try_bridge_bounded_implementer_completion_to_downstream_receipt(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if !receipt_waiting_on_implementer_evidence(receipt) {
        return Ok(false);
    }

    let implementer_receipt = blocked_implementer_step_receipt(role_selection, receipt);
    let (next_target, next_command, next_note, next_ready, next_blockers) =
        derive_downstream_dispatch_preview(store, role_selection, &implementer_receipt).await;
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
    {
        return Err(error);
    }
    if !next_ready {
        return Ok(false);
    }

    let preview_result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        next_target,
        next_command,
        next_note,
        next_ready,
        next_blockers,
        preview_result_path,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        store.root(),
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    receipt.blocker_code = None;
    Ok(true)
}

fn receipt_backed_downstream_preview_result_path(
    preview_result_path: Option<String>,
) -> Option<String> {
    preview_result_path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn apply_downstream_dispatch_preview_to_receipt(
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    downstream_dispatch_target: Option<String>,
    downstream_dispatch_command: Option<String>,
    downstream_dispatch_note: Option<String>,
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: Vec<String>,
    preview_result_path: Option<String>,
) {
    let preview_result_path = receipt_backed_downstream_preview_result_path(preview_result_path);
    let packet_ready = downstream_dispatch_ready
        && downstream_dispatch_blockers.is_empty()
        && preview_result_path.is_some();
    receipt.downstream_dispatch_target = downstream_dispatch_target;
    receipt.downstream_dispatch_command = downstream_dispatch_command;
    receipt.downstream_dispatch_note = downstream_dispatch_note;
    receipt.downstream_dispatch_ready = downstream_dispatch_ready;
    receipt.downstream_dispatch_blockers = downstream_dispatch_blockers;
    receipt.downstream_dispatch_status = packet_ready.then(|| "packet_ready".to_string());
    receipt.downstream_dispatch_result_path = preview_result_path;
    receipt.downstream_dispatch_active_target = active_downstream_dispatch_target(receipt);
}

fn write_runtime_downstream_dispatch_trace(
    state_root: &Path,
    run_id: &str,
    trace: &[serde_json::Value],
) -> Result<String, String> {
    let trace_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-traces");
    std::fs::create_dir_all(&trace_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-traces directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let trace_path = trace_dir.join(format!("{run_id}-{ts}.json"));
    let body = serde_json::json!({
        "artifact_kind": "runtime_downstream_dispatch_trace",
        "run_id": run_id,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "step_count": trace.len(),
        "steps": trace,
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch trace: {error}"))?;
    std::fs::write(&trace_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch trace: {error}"))?;
    Ok(trace_path.display().to_string())
}

pub(crate) fn runtime_dispatch_command_for_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Option<String> {
    match dispatch_target {
        "spec-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"].get("bootstrap_command"),
        ),
        "work-pool-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                .get("ensure_command"),
        ),
        "dev-pack" => json_string(
            role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                .get("ensure_command"),
        ),
        _ => Some("vida agent-init".to_string()),
    }
}

pub(crate) fn runtime_dispatch_packet_kind(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    dispatch_kind: &str,
) -> String {
    if dispatch_kind == "taskflow_pack" {
        return "tracked_flow_packet".to_string();
    }
    dispatch_contract_lane(execution_plan, dispatch_target)
        .and_then(|lane| json_string(lane.get("packet_template_kind")))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "delivery_task_packet".to_string())
}

pub(crate) async fn derive_downstream_dispatch_preview(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Vec<String>,
) {
    let agent_only_development =
        super::execution_plan_agent_only_development_required(&role_selection.execution_plan);
    let dispatch_contract = &role_selection.execution_plan["development_flow"]["dispatch_contract"];
    let lane_sequence = dispatch_contract_lane_sequence(dispatch_contract);
    let execution_lane_sequence = dispatch_contract_execution_lane_sequence(dispatch_contract);
    match receipt.dispatch_target.as_str() {
        "spec-pack" if agent_only_development => (
            Some(
                lane_sequence
                    .first()
                    .map(|value| value.as_str())
                    .unwrap_or("specification")
                    .to_string(),
            ),
            Some("vida agent-init".to_string()),
            Some(
                "after spec-pack task materialization, dispatch the business-analyst lane for bounded research/specification/planning before work-pool shaping"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
        "spec-pack" => {
            let blockers = vec![
                blocker_code_value(BlockerCode::PendingDesignFinalize)
                    .expect("pending design finalize should stay registry-backed"),
                blocker_code_value(BlockerCode::PendingSpecTaskClose)
                    .expect("pending spec task close should stay registry-backed"),
            ];
            (
                Some("work-pool-pack".to_string()),
                json_string(
                    role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                        .get("ensure_command"),
                ),
                Some(
                    "after the design document is finalized and the spec task is closed, ensure or reuse the tracked work-pool packet"
                        .to_string(),
                ),
                false,
                blockers,
            )
        }
        "work-pool-pack" => (
            Some("dev-pack".to_string()),
            json_string(
                role_selection.execution_plan["tracked_flow_bootstrap"]["dev_task"]
                    .get("ensure_command"),
            ),
            Some(
                "after the work-pool packet is shaped, ensure or reuse the bounded dev packet for delegated implementation"
                    .to_string(),
            ),
            receipt.dispatch_status == "executed",
            if receipt.dispatch_status == "executed" {
                Vec::new()
            } else {
                vec!["pending_work_pool_shape".to_string()]
            },
        ),
        "dev-pack" => {
            let next_target = execution_lane_sequence
                .first()
                .map(|value| value.as_str())
                .unwrap_or("implementer")
                .to_string();
            let missing_owned_scope =
                request_missing_owned_write_scope_for_dispatch_target(role_selection, &next_target);
            (
                Some(next_target),
                Some("vida agent-init".to_string()),
                Some(
                    "after the dev packet is created, activate the selected implementer lane for bounded execution"
                        .to_string(),
                ),
                !missing_owned_scope,
                if missing_owned_scope {
                    vec![missing_owned_write_scope_blocker()]
                } else {
                    Vec::new()
                },
            )
        }
        _ if receipt.dispatch_kind == "agent_lane" => {
            let current_lane =
                dispatch_contract_lane(&role_selection.execution_plan, &receipt.dispatch_target);
            if current_lane.and_then(|lane| lane["stage"].as_str()) == Some("design_gate")
                || (receipt.dispatch_target == "specification"
                    && current_lane.and_then(|lane| lane["stage"].as_str()).is_none()
                    && dispatch_contract.get("specification_activation").is_some())
            {
                let has_specification_evidence = dispatch_receipt_has_execution_evidence(receipt);
                let spec_task_closed =
                    tracked_specification_task_closed(store, role_selection, receipt).await;
                let design_doc_finalized = tracked_design_doc_finalized(role_selection);
                let evidence_blocker = current_lane
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingSpecificationEvidence));
                return (
                    Some("work-pool-pack".to_string()),
                    json_string(
                        role_selection.execution_plan["tracked_flow_bootstrap"]["work_pool_task"]
                        .get("ensure_command"),
                    ),
                    Some(
                        if receipt.dispatch_status == "executed"
                            && has_specification_evidence
                            && spec_task_closed
                            && design_doc_finalized
                        {
                            "specification/planning evidence is recorded and the spec-pack is closed; ensure or reuse the tracked work-pool packet"
                        } else if receipt.dispatch_status == "executed" {
                            "after specification/planning evidence is recorded, finalize the design doc and close spec-pack before work-pool shaping via tracked work-pool ensure/reuse"
                        } else {
                            "specification/planning lane is active; wait for bounded evidence return before design finalization, spec-pack closure, and tracked work-pool ensure/reuse"
                        }
                        .to_string(),
                    ),
                    has_specification_evidence && spec_task_closed && design_doc_finalized,
                    {
                        let mut blockers = Vec::new();
                        if !has_specification_evidence {
                            blockers.push(evidence_blocker.to_string());
                        }
                        if !design_doc_finalized {
                            blockers.push(
                                blocker_code_value(BlockerCode::PendingDesignFinalize)
                                    .expect("pending design finalize should stay registry-backed"),
                            );
                        }
                        if !spec_task_closed {
                            blockers.push(
                                blocker_code_value(BlockerCode::PendingSpecTaskClose)
                                    .expect("pending spec task close should stay registry-backed"),
                            );
                        }
                        blockers
                    },
                );
            }
            let current_index = execution_lane_sequence
                .iter()
                .position(|target| target == &receipt.dispatch_target);
            let effective_current_target = current_index
                .map(|_| receipt.dispatch_target.clone())
                .or_else(|| {
                    receipt
                        .activation_runtime_role
                        .as_deref()
                        .and_then(|runtime_role| {
                            dispatch_target_for_runtime_role(
                                &role_selection.execution_plan,
                                runtime_role,
                            )
                        })
                });
            let current_index = current_index.or_else(|| {
                receipt
                    .activation_runtime_role
                    .as_deref()
                    .and_then(|runtime_role| {
                        dispatch_target_for_runtime_role(
                            &role_selection.execution_plan,
                            runtime_role,
                        )
                    })
                    .and_then(|target| {
                        execution_lane_sequence
                            .iter()
                            .position(|candidate| candidate == &target)
                    })
            });
            let Some(current_index) = current_index else {
                return (None, None, None, false, Vec::new());
            };
            let next_target = execution_lane_sequence.get(current_index + 1);
            if let Some(next_target) = next_target {
                let blocker = effective_current_target
                    .as_deref()
                    .and_then(|target| dispatch_contract_lane(&role_selection.execution_plan, target))
                    .and_then(|lane| lane["completion_blocker"].as_str())
                    .unwrap_or(blocker_code_str(BlockerCode::PendingLaneEvidence))
                    .to_string();
                let has_lane_evidence = dispatch_receipt_has_execution_evidence(receipt)
                    || tracked_implementer_task_closed(store, role_selection, receipt).await;
                let missing_owned_scope = request_missing_owned_write_scope_for_dispatch_target(
                    role_selection,
                    next_target,
                );
                (
                    Some(next_target.clone()),
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "after `{}` evidence is recorded, activate `{}` for the next bounded lane",
                        receipt.dispatch_target, next_target
                    )),
                    has_lane_evidence && !missing_owned_scope,
                    {
                        let mut blockers = Vec::new();
                        if !has_lane_evidence {
                            blockers.push(blocker);
                        }
                        if missing_owned_scope {
                            blockers.push(missing_owned_write_scope_blocker());
                        }
                        blockers
                    },
                )
            } else {
                (
                    Some("closure".to_string()),
                    None,
                    Some(
                        "no additional downstream lane is required by the current execution plan after this handoff"
                            .to_string(),
                    ),
                    true,
                    Vec::new(),
                )
            }
        }
        _ => (None, None, None, false, Vec::new()),
    }
}

pub(crate) fn downstream_dispatch_ready_blocker_parity_error(
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: &[String],
) -> Option<String> {
    if downstream_dispatch_ready && !downstream_dispatch_blockers.is_empty() {
        return Some(
            "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                .to_string(),
        );
    }
    None
}

pub(crate) async fn refresh_downstream_dispatch_preview(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let (
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
    ) = derive_downstream_dispatch_preview(store, role_selection, receipt).await;
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let preview_result_path =
        receipt_backed_execution_evidence_path(store, role_selection, receipt).await?;
    apply_downstream_dispatch_preview_to_receipt(
        receipt,
        downstream_dispatch_target,
        downstream_dispatch_command,
        downstream_dispatch_note,
        downstream_dispatch_ready,
        downstream_dispatch_blockers,
        preview_result_path,
    );
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_last_target = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_packet_path =
        if receipt.downstream_dispatch_status.as_deref() == Some("packet_ready") {
            write_runtime_downstream_dispatch_packet(
                store.root(),
                role_selection,
                run_graph_bootstrap,
                receipt,
            )?
        } else {
            None
        };
    Ok(())
}

pub(crate) fn runtime_packet_handoff_task_class(
    dispatch_target: &str,
    handoff_runtime_role: &str,
) -> &'static str {
    match dispatch_target {
        "specification" => TASK_CLASS_SPECIFICATION,
        "planning" => "planning",
        "coach" => TASK_CLASS_COACH,
        "verification" => TASK_CLASS_VERIFICATION,
        "escalation" => TASK_CLASS_ARCHITECTURE,
        "implementer" => TASK_CLASS_IMPLEMENTATION,
        _ => match handoff_runtime_role {
            RUNTIME_ROLE_BUSINESS_ANALYST => TASK_CLASS_SPECIFICATION,
            RUNTIME_ROLE_PM => "planning",
            RUNTIME_ROLE_COACH => TASK_CLASS_COACH,
            RUNTIME_ROLE_VERIFIER => TASK_CLASS_VERIFICATION,
            RUNTIME_ROLE_SOLUTION_ARCHITECT => TASK_CLASS_ARCHITECTURE,
            _ => TASK_CLASS_IMPLEMENTATION,
        },
    }
}

fn packet_nonempty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

fn packet_nonempty_string_array(packet: &serde_json::Value, key: &str) -> bool {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                })
        })
}

fn packet_has_owned_or_read_only_paths(packet: &serde_json::Value) -> bool {
    packet_nonempty_string_array(packet, "owned_paths")
        || packet_nonempty_string_array(packet, "read_only_paths")
}

fn packet_requires_owned_write_scope(
    packet_template_kind: &str,
    active_packet: &serde_json::Value,
) -> bool {
    if packet_template_kind != "delivery_task_packet" {
        return false;
    }
    active_packet
        .get("handoff_task_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some("implementation")
}

fn dispatch_target_requires_owned_write_scope(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> bool {
    let (_, _, _, activation_runtime_role) =
        downstream_activation_fields(role_selection, dispatch_target);
    let handoff_runtime_role = activation_runtime_role
        .as_deref()
        .unwrap_or(role_selection.selected_role.as_str());
    runtime_packet_handoff_task_class(dispatch_target, handoff_runtime_role) == "implementation"
}

fn missing_owned_write_scope_blocker() -> String {
    "missing_owned_write_scope".to_string()
}

fn request_missing_owned_write_scope_for_dispatch_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> bool {
    dispatch_target_requires_owned_write_scope(role_selection, dispatch_target)
        && !request_has_explicit_owned_scope(&role_selection.request)
}

fn single_task_move_scope_owned_paths(packet: &serde_json::Value) -> Option<Vec<String>> {
    let single_task_only = packet
        .get("role_selection_full")
        .and_then(|value| value.get("single_task_only"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !single_task_only {
        return None;
    }
    let request_text = packet
        .get("request_text")
        .and_then(serde_json::Value::as_str)?;
    single_task_move_scope_paths(request_text)
}

fn active_runtime_packet<'a>(
    packet: &'a serde_json::Value,
) -> Result<(&'a str, &'a serde_json::Value), String> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted dispatch packet is missing packet_template_kind".to_string())?;
    let packet_value = packet.get(packet_template_kind).ok_or_else(|| {
        format!("Persisted dispatch packet is missing active packet body `{packet_template_kind}`")
    })?;
    if packet_value.is_null() {
        return Err(format!(
            "Persisted dispatch packet has null active packet body `{packet_template_kind}`"
        ));
    }
    Ok((packet_template_kind, packet_value))
}

pub(crate) fn validate_runtime_dispatch_packet_contract(
    packet: &serde_json::Value,
    packet_label: &str,
) -> Result<(), String> {
    let (packet_template_kind, active_packet) = active_runtime_packet(packet)?;
    if let Some(expected_owned_paths) = single_task_move_scope_owned_paths(packet) {
        let actual_owned_paths = active_packet
            .get("owned_paths")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                format!(
                    "{packet_label} `{packet_template_kind}` is missing owned_paths for a single-task move request"
                )
            })?;
        let actual_owned_paths = actual_owned_paths
            .iter()
            .map(|value| {
                value.as_str().map(str::trim).filter(|value| !value.is_empty()).map(|value| {
                    value.to_string()
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                format!(
                    "{packet_label} `{packet_template_kind}` contains non-string owned_paths entries for a single-task move request"
                )
            })?;
        if actual_owned_paths != expected_owned_paths {
            return Err(format!(
                "{packet_label} `{packet_template_kind}` single-task move packet owned_paths must match the declared source/destination pair exactly; expected {:?}, got {:?}",
                expected_owned_paths, actual_owned_paths
            ));
        }
    }
    let missing = match packet_template_kind {
        "delivery_task_packet" | "execution_block_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("goal")) {
                missing.push("goal");
            }
            if !packet_nonempty_string_array(active_packet, "scope_in") {
                missing.push("scope_in");
            }
            if packet_requires_owned_write_scope(packet_template_kind, active_packet) {
                if !packet_nonempty_string_array(active_packet, "owned_paths") {
                    missing.push("owned_paths");
                }
            } else if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string_array(active_packet, "definition_of_done") {
                missing.push("definition_of_done");
            }
            if !packet_nonempty_string(active_packet.get("verification_command")) {
                missing.push("verification_command");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_nonempty_string_array(active_packet, "stop_rules") {
                missing.push("stop_rules");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "coach_review_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("review_goal")) {
                missing.push("review_goal");
            }
            if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string_array(active_packet, "definition_of_done") {
                missing.push("definition_of_done");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "verifier_proof_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("proof_goal")) {
                missing.push("proof_goal");
            }
            if !packet_nonempty_string(active_packet.get("verification_command")) {
                missing.push("verification_command");
            }
            if !packet_nonempty_string(active_packet.get("proof_target")) {
                missing.push("proof_target");
            }
            if !packet_has_owned_or_read_only_paths(active_packet) {
                missing.push("owned_paths|read_only_paths");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "escalation_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("decision_needed")) {
                missing.push("decision_needed");
            }
            if !packet_nonempty_string_array(active_packet, "options") {
                missing.push("options");
            }
            if !packet_nonempty_string_array(active_packet, "constraints") {
                missing.push("constraints");
            }
            if !packet_nonempty_string(active_packet.get("blocking_question")) {
                missing.push("blocking_question");
            }
            missing
        }
        "tracked_flow_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("dispatch_target")) {
                missing.push("dispatch_target");
            }
            if !packet_nonempty_string(active_packet.get("tracked_packet_key")) {
                missing.push("tracked_packet_key");
            }
            if !packet_nonempty_string(active_packet.get("task_id")) {
                missing.push("task_id");
            }
            if !packet_nonempty_string(active_packet.get("title")) {
                missing.push("title");
            }
            if !packet_nonempty_string(active_packet.get("runtime")) {
                missing.push("runtime");
            }
            if !packet_nonempty_string(active_packet.get("create_command")) {
                missing.push("create_command");
            }
            if !packet_nonempty_string(active_packet.get("ensure_command")) {
                missing.push("ensure_command");
            }
            if !packet_nonempty_string(active_packet.get("next_command")) {
                missing.push("next_command");
            }
            missing
        }
        other => {
            return Err(format!(
                "Persisted dispatch packet has unsupported packet_template_kind `{other}`"
            ));
        }
    };
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{packet_label} `{packet_template_kind}` is missing required packet fields: {}",
        missing.join(", ")
    ))
}

fn runtime_dispatch_command_for_packet_path(
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: &str,
) -> Option<String> {
    match receipt.dispatch_kind.as_str() {
        "taskflow_pack" => {
            runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target)
        }
        "agent_lane" => receipt
            .dispatch_command
            .clone()
            .or_else(|| runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target))
            .or_else(|| {
                Some(
                    runtime_agent_lane_dispatch_for_root(
                        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                        packet_path,
                        None,
                    )
                    .activation_command,
                )
            }),
        _ => runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target),
    }
}

pub(crate) struct RuntimeDispatchPacketContext<'a> {
    pub(crate) state_root: &'a Path,
    pub(crate) role_selection: &'a RuntimeConsumptionLaneSelection,
    pub(crate) receipt: &'a crate::state_store::RunGraphDispatchReceipt,
    pub(crate) taskflow_handoff_plan: &'a serde_json::Value,
    pub(crate) run_graph_bootstrap: &'a serde_json::Value,
}

impl<'a> RuntimeDispatchPacketContext<'a> {
    pub(crate) fn new(
        state_root: &'a Path,
        role_selection: &'a RuntimeConsumptionLaneSelection,
        receipt: &'a crate::state_store::RunGraphDispatchReceipt,
        taskflow_handoff_plan: &'a serde_json::Value,
        run_graph_bootstrap: &'a serde_json::Value,
    ) -> Self {
        Self {
            state_root,
            role_selection,
            receipt,
            taskflow_handoff_plan,
            run_graph_bootstrap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::CreateTaskRequest;
    use crate::state_store::RunGraphDispatchReceipt;
    use crate::temp_state::TempStateHarness;
    use crate::{run, Cli};
    use clap::Parser;
    use serde_json::json;
    use std::cell::Cell;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

    fn harness_state_root(harness: &TempStateHarness) -> PathBuf {
        harness.path().join(crate::state_store::default_state_dir())
    }

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn set(path: PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    struct HarnessStateRootGuards {
        _proxy_override: ProxyStateDirOverrideGuard,
        _env_guard: EnvVarGuard,
    }

    impl HarnessStateRootGuards {
        fn set(path: PathBuf) -> Self {
            let env_value = path.display().to_string();
            Self {
                _proxy_override: ProxyStateDirOverrideGuard::set(path),
                _env_guard: EnvVarGuard::set("VIDA_STATE_DIR", &env_value),
            }
        }
    }

    struct EnvVarGuard {
        lock: Option<MutexGuard<'static, ()>>,
        key: &'static str,
        original: Option<String>,
    }

    fn env_var_lock() -> &'static RecoveringMutex {
        static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
        LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
    }

    thread_local! {
        static ENV_VAR_GUARD_DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let lock = ENV_VAR_GUARD_DEPTH.with(|depth| {
                let current = depth.get();
                depth.set(current + 1);
                (current == 0).then(|| env_var_lock().lock())
            });
            let original = env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                lock,
                key,
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = self.original.as_deref() {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
            ENV_VAR_GUARD_DEPTH.with(|depth| {
                let current = depth.get();
                depth.set(current.saturating_sub(1));
            });
            let _ = self.lock.take();
        }
    }

    fn cli(args: &[&str]) -> Cli {
        let mut argv = vec!["vida"];
        argv.extend(args.iter().copied());
        Cli::parse_from(argv)
    }

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn install_external_cli_test_subagents(config_path: &Path) {
        let config = fs::read_to_string(config_path).expect("config should exist");
        let updated = config.replace(
            "agent_system:\n  init_on_boot: true\n  mode: native\n  state_owner: orchestrator_only\n  max_parallel_agents: 4\n  routing: {}\n  scoring: {}\n",
            concat!(
                "agent_system:\n",
                "  init_on_boot: true\n",
                "  mode: native\n",
                "  state_owner: orchestrator_only\n",
                "  max_parallel_agents: 4\n",
                "  subagents:\n",
                "    internal_subagents:\n",
                "      enabled: true\n",
                "      subagent_backend_class: internal\n",
                "    qwen_cli:\n",
                "      enabled: true\n",
                "      subagent_backend_class: external_cli\n",
                "      detect_command: qwen\n",
                "      dispatch:\n",
                "        command: qwen\n",
                "        static_args:\n",
                "          - -y\n",
                "          - -o\n",
                "          - text\n",
                "        model_flag: --model\n",
                "        prompt_mode: positional\n",
                "    hermes_cli:\n",
                "      enabled: true\n",
                "      subagent_backend_class: external_cli\n",
                "      detect_command: hermes\n",
                "      dispatch:\n",
                "        command: hermes\n",
                "        static_args:\n",
                "          - chat\n",
                "          - -Q\n",
                "          - -q\n",
                "        model_flag: --model\n",
                "        provider_flag: --provider\n",
                "        prompt_mode: positional\n",
                "  routing: {}\n",
                "  scoring: {}\n",
            ),
        );
        assert_ne!(
            updated, config,
            "expected agent_system scaffold replacement"
        );
        fs::write(config_path, updated).expect("config should update");
    }

    fn bridge_test_role_selection(dev_task_id: &str) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": dev_task_id,
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
    }

    fn agent_lane_test_execution_plan(executor_backend: &str) -> serde_json::Value {
        json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "junior",
                    "backend_class": "internal"
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal"
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli"
                }
            ],
            "development_flow": {
                "implementer": {
                    "executor_backend": executor_backend
                }
            }
        })
    }

    fn agent_lane_test_request() -> &'static str {
        "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
    }

    fn specification_test_role_selection(
        spec_task_id: &str,
        design_doc_path: &str,
    ) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue specification".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "spec_task": {
                        "task_id": spec_task_id
                    },
                    "design_doc_path": design_doc_path,
                    "work_pool_task": {
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "completion_blocker": "pending_specification_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
    }

    fn bridge_waiting_root_receipt(run_id: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: Some("vida task ensure".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        }
    }

    async fn create_and_close_task(store: &crate::StateStore, task_id: &str) {
        let labels = vec!["dev-pack".to_string()];
        store
            .create_task(CreateTaskRequest {
                task_id,
                title: "Dev pack",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &labels,
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");
        store
            .close_task(task_id, "implemented and proven")
            .await
            .expect("task should close");
    }

    fn write_approved_design_doc(path: &Path) {
        fs::write(path, "# Test Design\n\nStatus: `approved`\n").expect("design doc should write");
    }

    fn read_json(project_root: &Path, path: &str) -> serde_json::Value {
        let resolved = if Path::new(path).is_absolute() {
            Path::new(path).to_path_buf()
        } else {
            project_root.join(path)
        };
        serde_json::from_str(
            &fs::read_to_string(&resolved).expect("json artifact should be readable"),
        )
        .expect("json artifact should decode")
    }

    #[test]
    fn runtime_dispatch_packet_contract_accepts_template_specific_minimums() {
        let delivery = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": runtime_delivery_task_packet(
                "run-1",
                "implementer",
                "worker",
                "implementation",
                "implementation",
                "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&delivery, "test packet").is_ok());

        let coach = serde_json::json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": runtime_coach_review_packet(
                "run-1",
                "coach",
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&coach, "test packet").is_ok());

        let verifier = serde_json::json!({
            "packet_template_kind": "verifier_proof_packet",
            "verifier_proof_packet": runtime_verifier_proof_packet(
                "run-1",
                "verification",
                "bounded proof target",
            ),
        });
        assert!(validate_runtime_dispatch_packet_contract(&verifier, "test packet").is_ok());
    }

    #[test]
    fn runtime_delivery_task_packet_collects_explicit_owned_paths_from_request_text() {
        let packet = runtime_delivery_task_packet(
            "run-1",
            "implementer",
            "worker",
            "implementation",
            "implementation",
            "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
        );

        assert_eq!(
            packet["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
    }

    #[test]
    fn explicit_request_scope_paths_stay_empty_without_file_scope_in_request_text() {
        assert!(explicit_request_scope_paths("continue development").is_empty());
    }

    #[test]
    fn runtime_dispatch_packet_contract_declares_and_enforces_single_task_move_scope() {
        let request_text = "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation.";
        let delivery_packet = runtime_delivery_task_packet(
            "run-1",
            "implementer",
            "worker",
            "implementation",
            "implementation",
            request_text,
        );
        assert_eq!(
            delivery_packet["owned_paths"],
            serde_json::json!([
                "crates/vida/src/main.rs",
                "crates/vida/src/project_activator_surface.rs"
            ])
        );

        let packet = serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": delivery_packet.clone(),
            "request_text": request_text,
            "role_selection_full": {
                "single_task_only": true
            }
        });
        assert!(validate_runtime_dispatch_packet_contract(&packet, "test packet").is_ok());

        let mut widened_packet = packet.clone();
        widened_packet["delivery_task_packet"]["owned_paths"] = serde_json::json!([
            "crates/vida/src/main.rs",
            "crates/vida/src/project_activator_surface.rs",
            "crates/vida/src/runtime_dispatch_state.rs"
        ]);
        let error = validate_runtime_dispatch_packet_contract(&widened_packet, "test packet")
            .expect_err("widened single-task move packet should fail closed");
        assert!(error.contains("single-task move packet owned_paths"));
        assert!(error.contains("expected"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_rejects_implementation_delivery_without_owned_paths() {
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer"],
                "owned_paths": [],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?",
                "handoff_task_class": "implementation"
            }
        });

        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("implementation delivery packet without owned scope should fail closed");
        assert!(error.contains("owned_paths"));
    }

    #[test]
    fn runtime_dispatch_packet_contract_fails_closed_for_missing_required_fields() {
        let malformed = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "scope_in": ["dispatch_target:implementer"],
                "read_only_paths": ["docs/process"],
                "definition_of_done": ["done"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "proof",
                "stop_rules": ["stop"],
                "blocking_question": "what next?"
            }
        });
        let error = validate_runtime_dispatch_packet_contract(&malformed, "test packet")
            .expect_err("packet without goal should fail closed");
        assert!(error.contains("missing required packet fields"));
        assert!(error.contains("goal"));
    }

    #[test]
    fn execute_runtime_dispatch_handoff_executes_internal_codex_carrier() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    "continue development"
                ),
                "dispatch_target": "implementer",
                "request_text": "continue development",
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-agent-dispatch"
        });
        let status = crate::state_store::RunGraphStatus {
            run_id: "run-agent-dispatch".to_string(),
            task_id: "run-agent-dispatch".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("worker".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "worker_lane".to_string(),
            lifecycle_stage: "dispatch_ready".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_worker".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.worker_lane".to_string(),
            recovery_ready: true,
        };
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should record");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let dispatch_packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let mut persisted_receipt = receipt.clone();
        persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
        runtime
            .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
            .expect("dispatch receipt should record");
        drop(store);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-init",
                "--dispatch-packet",
                dispatch_packet_path.as_str(),
                "--execute-dispatch",
                "--json",
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should reopen");
        let recorded_receipt = runtime
            .block_on(store.latest_run_graph_dispatch_receipt())
            .expect("latest dispatch receipt should load")
            .expect("latest dispatch receipt should exist");
        let dispatch_result_path = recorded_receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
        assert_eq!(parsed["execution_state"], "executed");
        assert_eq!(parsed["status"], "pass");
        assert_eq!(
            parsed["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(parsed["activation_semantics"]["view_only"], false);
        assert_eq!(parsed["activation_semantics"]["executes_packet"], true);
        assert_eq!(parsed["execution_evidence"]["status"], "recorded");
        assert_eq!(
            parsed["execution_evidence"]["evidence_kind"],
            "internal_carrier_completion"
        );
        assert_eq!(parsed["provider_result"], "internal-dispatch-ok");
        assert_eq!(parsed["backend_dispatch"]["backend_id"], "junior");
    }

    #[test]
    fn execute_runtime_dispatch_handoff_sets_writable_runtime_env_for_internal_codex_carrier() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let original_home = env::var("HOME").unwrap_or_default();
        let original_xdg_data_home = env::var("XDG_DATA_HOME").unwrap_or_default();
        let original_xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or_default();

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let env_capture = harness.path().join("internal-host-env.txt");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$HOME\" > \"{capture}\"\nprintf '%s\\n' \"$XDG_CONFIG_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_DATA_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_STATE_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$XDG_CACHE_HOME\" >> \"{capture}\"\nprintf '%s\\n' \"$TMPDIR\" >> \"{capture}\"\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}}'\nprintf '%s\\n' '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}}}'\n",
                capture = env_capture.display()
            ),
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch-env.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch-env",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    "continue development"
                ),
                "dispatch_target": "implementer",
                "request_text": "continue development",
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch-env".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("internal host should execute with writable runtime env");

        assert!(result["surface"]
            .as_str()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert_eq!(result["execution_state"], "executed");
        let captured = fs::read_to_string(&env_capture).expect("env capture should exist");
        let rows: Vec<_> = captured.lines().collect();
        assert_eq!(
            rows.len(),
            6,
            "expected HOME, XDG config/data, state/cache, and TMPDIR"
        );
        assert_eq!(
            rows[0], original_home,
            "HOME should stay intact for auth/config discovery"
        );
        assert_ne!(
            rows[1], original_xdg_config_home,
            "XDG_CONFIG_HOME should move into the writable project runtime root"
        );
        assert_ne!(
            rows[2], original_xdg_data_home,
            "XDG_DATA_HOME should move into the writable project runtime root"
        );
        for row in &rows[1..] {
            let path = Path::new(row);
            assert!(
                path.starts_with(harness.path().join(".vida/data/internal-host/codex/junior")),
                "runtime env path should stay inside writable project runtime root: {}",
                row
            );
            assert!(path.is_dir(), "runtime env dir should exist: {}", row);
        }
    }

    #[test]
    fn agent_init_execute_dispatch_executes_internal_codex_carrier() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_state.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/runtime_dispatch_state.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-agent-init-execute-dispatch"
        });
        let status = crate::state_store::RunGraphStatus {
            run_id: "run-agent-init-execute-dispatch".to_string(),
            task_id: "run-agent-init-execute-dispatch".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("worker".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "worker_lane".to_string(),
            lifecycle_stage: "dispatch_ready".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_worker".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.worker_lane".to_string(),
            recovery_ready: true,
        };
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should record");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-init-execute-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let dispatch_packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let mut persisted_receipt = receipt.clone();
        persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
        runtime
            .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
            .expect("dispatch receipt should record");
        drop(store);
        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-init",
                "--dispatch-packet",
                dispatch_packet_path.as_str(),
                "--execute-dispatch",
                "--json",
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should reopen");
        let recorded_receipt = runtime
            .block_on(store.latest_run_graph_dispatch_receipt())
            .expect("latest dispatch receipt should load")
            .expect("latest dispatch receipt should exist");
        let dispatch_result_path = recorded_receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
        assert_eq!(parsed["execution_state"], "executed");
        assert_eq!(parsed["status"], "pass");
        assert_eq!(
            parsed["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(parsed["activation_semantics"]["view_only"], false);
        assert_eq!(parsed["activation_semantics"]["executes_packet"], true);
        assert_eq!(parsed["execution_evidence"]["status"], "recorded");
        assert_eq!(
            parsed["execution_evidence"]["evidence_kind"],
            "internal_carrier_completion"
        );
        assert_eq!(parsed["provider_result"], "internal-dispatch-ok");
        assert_eq!(parsed["backend_dispatch"]["backend_id"], "junior");
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_internal_host_on_codex_when_receipt_backend_is_external(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-internal-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-internal-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-internal-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("internal host should ignore external receipt backend and execute on codex");

        assert!(result["surface"]
            .as_str()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_evidence"]["backend_id"], "junior");
        assert_eq!(result["backend_dispatch"]["backend_class"], "internal");
        assert_eq!(result["backend_dispatch"]["backend_id"], "junior");
        assert_eq!(result["provider_result"], "internal-dispatch-ok");
    }

    #[test]
    fn execute_and_record_dispatch_receipt_updates_surface_from_internal_execution_result() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("agent-dispatch-record.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-agent-dispatch-record",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-agent-dispatch-record"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-agent-dispatch-record".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("dispatch receipt should record execution evidence");

        assert_eq!(receipt.dispatch_status, "executed");
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert!(receipt
            .dispatch_command
            .as_deref()
            .is_some_and(|value| value.contains("exec")));
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        let store = runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store should reopen");
        let persisted_receipt = runtime
            .block_on(store.run_graph_dispatch_receipt("run-agent-dispatch-record"))
            .expect("persisted dispatch receipt should load")
            .expect("persisted dispatch receipt should exist");
        assert_eq!(persisted_receipt.dispatch_status, "executed");
        assert_eq!(persisted_receipt.dispatch_target, "implementer");
        assert!(persisted_receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
    }

    #[test]
    fn execute_and_record_dispatch_receipt_persists_blocked_result_for_internal_codex_timeout() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 1\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\ntrap '' TERM\nsleep 30\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("internal-host-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-internal-host-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-internal-host-timeout"
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        runtime
            .block_on(execute_and_record_dispatch_receipt(
                &state_root,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("dispatch receipt should persist blocked timeout result");
        let elapsed = started.elapsed();

        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        let dispatch_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("dispatch result json should parse");
        assert!(
            elapsed < Duration::from_secs(4),
            "expected timeout wrapper to return promptly, got {:?}",
            elapsed
        );
        assert_eq!(parsed["status"], "blocked");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(parsed["blocker_code"], "internal_activation_view_only");
        assert!(
            parsed["exit_code"].is_null() || parsed["exit_code"].as_i64().is_some(),
            "expected timeout path to record an exit code value or null signal exit, got {:?}",
            parsed["exit_code"]
        );
        assert!(parsed["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));
        assert_eq!(parsed["timeout_wrapper"]["timeout_seconds"], 1);
        assert_eq!(parsed["timeout_wrapper"]["kill_after_grace_seconds"], 1);
        assert_eq!(parsed["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn taskflow_consume_continue_returns_prompt_blocked_receipt_for_internal_coach_timeout() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 1\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\ntrap '' TERM\nsleep 30\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let run_id = "run-coach-timeout-continue";
        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = Some("verification".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_coach".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.coach_lane".to_string();
        status.recovery_ready = true;
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should record");
        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("runtime-consumption dir should exist");
        let snapshot_path = snapshot_dir.join("final-2026-04-16T00-00-00Z.json");
        let snapshot_path_string = snapshot_path.display().to_string();
        let operator_contracts = crate::build_release1_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path_string.clone(),
                "latest_run_graph_dispatch_receipt_id": run_id,
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        let failure_control_evidence =
            crate::taskflow_consume_resume::build_failure_control_evidence(
                run_id,
                &snapshot_path_string,
            );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": run_id
                    },
                    "release_admission": {},
                    "failure_control_evidence": failure_control_evidence.clone()
                },
                "failure_control_evidence": failure_control_evidence
            })
            .to_string(),
        )
        .expect("final snapshot should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        runtime
            .block_on(store.record_run_graph_dispatch_context(
                &crate::state_store::RunGraphDispatchContext {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    request_text: "continue coach review".to_string(),
                    role_selection:
                        serde_json::to_value(&role_selection).expect("encode role selection"),
                    recorded_at: "2026-04-16T00:00:00Z".to_string(),
                },
            ))
            .expect("dispatch context should record");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida taskflow consume continue".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({ "run_id": run_id });
        let handoff_plan = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let dispatch_packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let mut persisted_receipt = receipt.clone();
        persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path);
        runtime
            .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
            .expect("dispatch receipt should record");
        drop(store);

        let started = Instant::now();
        assert_eq!(
            runtime.block_on(run(cli(&[
                "taskflow", "consume", "continue", "--run-id", run_id, "--json",
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        let elapsed = started.elapsed();

        let store = runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store should reopen");
        let persisted = runtime
            .block_on(store.run_graph_dispatch_receipt(run_id))
            .expect("dispatch receipt should load")
            .expect("dispatch receipt should exist");
        assert!(
            elapsed < Duration::from_secs(6),
            "expected consume continue to return promptly on coach timeout, got {:?}",
            elapsed
        );
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(persisted.lane_status, "lane_blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        let dispatch_result_path = persisted
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("dispatch result json should parse");
        assert_eq!(parsed["status"], "blocked");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(parsed["blocker_code"], "internal_activation_view_only");
        assert_eq!(parsed["timeout_wrapper"]["timed_out"], true);
        assert!(parsed["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));
    }

    #[test]
    fn execute_and_record_dispatch_receipt_releases_authoritative_lock_while_internal_codex_runs() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 5\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nsleep 2\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness
            .path()
            .join("internal-host-lock-release-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-internal-host-lock-release",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-internal-host-lock-release"
        });
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-host-lock-release".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let state_root_dispatch = state_root.clone();
        let run_graph_bootstrap_dispatch = run_graph_bootstrap.clone();
        let dispatch = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
            let mut receipt = receipt;
            runtime
                .block_on(execute_and_record_dispatch_receipt(
                    &state_root_dispatch,
                    &role_selection,
                    &run_graph_bootstrap_dispatch,
                    &mut receipt,
                ))
                .expect("dispatch receipt should execute without holding authoritative lock");
            receipt
        });

        thread::sleep(Duration::from_millis(250));
        let probe_runtime =
            tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let probe_started = Instant::now();
        let probe_store = probe_runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store reopen should succeed while dispatch is in flight");
        drop(probe_store);
        let probe_elapsed = probe_started.elapsed();

        let receipt = dispatch
            .join()
            .expect("dispatch thread should join successfully");
        assert!(
            probe_elapsed < Duration::from_secs(1),
            "expected concurrent store reopen during dispatch to finish quickly, got {:?}",
            probe_elapsed
        );
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_running");
        assert!(receipt
            .dispatch_surface
            .as_deref()
            .is_some_and(|value| value.starts_with("internal_cli:")));
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
    }

    #[test]
    fn execute_and_record_dispatch_receipt_persists_in_flight_runtime_truth_while_internal_codex_runs(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 5\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"test-thread\"}'\nsleep 2\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"internal-dispatch-ok\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let patched_path = if original_path.is_empty() {
            fake_bin.display().to_string()
        } else {
            format!("{}:{}", fake_bin.display(), original_path)
        };
        let _path_guard = EnvVarGuard::set("PATH", &patched_path);

        let state_root = harness_state_root(&harness);
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let run_graph_status = crate::state_store::RunGraphStatus {
            run_id: "run-in-flight-dispatch".to_string(),
            task_id: "task-in-flight-dispatch".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("implementer".to_string()),
            status: "running".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "implementation_dispatch_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_implementer".to_string(),
            context_state: "open".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: true,
        };
        runtime
            .block_on(store.record_run_graph_status(&run_graph_status))
            .expect("run graph status should persist");
        drop(store);
        let dispatch_packet_path = harness.path().join("in-flight-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-in-flight-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-in-flight-dispatch"
        });
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-in-flight-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let state_root_dispatch = state_root.clone();
        let run_graph_bootstrap_dispatch = run_graph_bootstrap.clone();
        let dispatch = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
            let mut receipt = receipt;
            runtime
                .block_on(execute_and_record_dispatch_receipt(
                    &state_root_dispatch,
                    &role_selection,
                    &run_graph_bootstrap_dispatch,
                    &mut receipt,
                ))
                .expect("dispatch receipt should execute");
            receipt
        });

        thread::sleep(Duration::from_millis(250));
        let probe_runtime =
            tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let probe_store = probe_runtime
            .block_on(StateStore::open_existing(state_root.clone()))
            .expect("state store reopen should succeed while dispatch is in flight");
        let in_flight_receipt = probe_runtime
            .block_on(probe_store.run_graph_dispatch_receipt("run-in-flight-dispatch"))
            .expect("in-flight receipt should load")
            .expect("in-flight receipt should exist");
        let in_flight_status = probe_runtime
            .block_on(probe_store.run_graph_status("run-in-flight-dispatch"))
            .expect("in-flight run graph status should load");
        drop(probe_store);

        assert_eq!(in_flight_receipt.dispatch_status, "executing");
        assert_eq!(in_flight_receipt.lane_status, "lane_running");
        assert!(in_flight_receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        assert_eq!(in_flight_status.active_node, "implementer");
        assert_eq!(in_flight_status.lifecycle_stage, "implementer_active");
        assert_eq!(in_flight_status.handoff_state, "none");
        assert_eq!(in_flight_status.status, "running");
        assert!(!in_flight_status.recovery_ready);

        let receipt = dispatch
            .join()
            .expect("dispatch thread should join successfully");
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_running");
    }

    #[test]
    fn execute_runtime_dispatch_handoff_times_out_configured_external_backend() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config
            .replace(
                "command: qwen\n        static_args:\n          - -y\n          - -o\n          - text",
                "command: sh\n        static_args:\n          - -lc\n          - \"sleep 30\"\n          - vida-dispatch",
            )
            .replace(
                "        prompt_mode: positional\n",
                "        prompt_mode: positional\n        no_output_timeout_seconds: 1\n      max_runtime_seconds: 1\n",
            );
        fs::write(&config_path, updated).expect("config should update");

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-timeout-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-dispatch-timeout",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("qwen_cli"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-dispatch-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let started = Instant::now();
        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external timeout dispatch should render");
        let elapsed = started.elapsed();

        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "timeout_without_takeover_authority");
        assert!(
            elapsed < Duration::from_secs(4),
            "expected external timeout wrapper to return promptly, got {:?}",
            elapsed
        );
        assert!(result["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));
        assert!(
            result["exit_code"].is_null() || result["exit_code"].as_i64().is_some(),
            "expected timeout path to record an exit code value or null signal exit, got {:?}",
            result["exit_code"]
        );
        assert_eq!(result["timeout_wrapper"]["timeout_seconds"], 1);
        assert_eq!(result["timeout_wrapper"]["kill_after_grace_seconds"], 1);
        assert_eq!(result["timeout_wrapper"]["timed_out"], true);
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_external_host_internal_backend_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-internal-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-internal-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("internal_subagents"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-internal-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("hybrid external-host internal-backend dispatch should stay on agent-init");

        assert_eq!(result["surface"], "vida agent-init");
        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["host_runtime"]["selected_cli_system"], "qwen");
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(result["backend_dispatch"]["backend_class"], "internal");
        assert_eq!(
            result["backend_dispatch"]["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            result["backend_dispatch"]["policy_selected_internal_backend"],
            true
        );
        assert_eq!(result["blocker_code"], "internal_activation_view_only");
    }

    #[test]
    fn runtime_agent_lane_dispatch_prefers_receipt_selected_backend_for_external_hosts() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("hermes_cli"),
        );

        assert_eq!(dispatch.surface, "external_cli:hermes_cli");
        assert!(
            dispatch.activation_command.contains("hermes"),
            "expected hermes command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_id"], "hermes_cli");
    }

    #[test]
    fn runtime_agent_lane_dispatch_keeps_internal_hosts_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            None,
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert!(
            dispatch.activation_command.contains("vida agent-init"),
            "expected canonical internal activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "codex");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "internal"
        );
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn runtime_agent_lane_dispatch_keeps_policy_selected_internal_backend_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        install_external_cli_test_subagents(&harness.path().join("vida.config.yaml"));

        let dispatch_packet_path = harness.path().join("runtime-dispatch-packet.json");
        let dispatch = runtime_agent_lane_dispatch_for_root(
            harness.path(),
            dispatch_packet_path.to_string_lossy().as_ref(),
            Some("internal_subagents"),
        );

        assert_eq!(dispatch.surface, "vida agent-init");
        assert!(
            dispatch.activation_command.contains("vida agent-init"),
            "expected canonical internal activation command, got {}",
            dispatch.activation_command
        );
        assert_eq!(dispatch.backend_dispatch["selected_cli_system"], "qwen");
        assert_eq!(
            dispatch.backend_dispatch["selected_execution_class"],
            "external"
        );
        assert_eq!(dispatch.backend_dispatch["backend_class"], "internal");
        assert_eq!(
            dispatch.backend_dispatch["backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            dispatch.backend_dispatch["policy_selected_internal_backend"],
            true
        );
    }

    #[test]
    fn execute_runtime_dispatch_handoff_keeps_internal_host_external_backend_hint_on_agent_init() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "command: qwen\n        static_args:\n          - -y\n          - -o\n          - text",
            "command: sh\n        static_args:\n          - -lc\n          - \"printf 'external-dispatch:%s' \\\"$1\\\"\"\n          - vida-dispatch",
        );
        fs::write(&config_path, updated).expect("config should update");

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("hybrid-external-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-hybrid-external-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("qwen_cli"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-hybrid-external-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external backend hint should execute through the hinted lawful backend");

        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["host_runtime"]["selected_cli_system"], "codex");
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "internal"
        );
        assert_eq!(result["backend_dispatch"]["backend_id"], "qwen_cli");
        assert!(result["activation_command"]
            .as_str()
            .expect("activation command should render")
            .contains("sh"));
        assert!(result["blocker_code"].is_null());
    }

    #[test]
    fn execute_runtime_dispatch_handoff_executes_configured_external_backend() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _vida_root_guard = EnvVarGuard::set("VIDA_ROOT", &harness.path().display().to_string());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        install_external_cli_test_subagents(&config_path);
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "command: qwen\n        static_args:\n          - -y\n          - -o\n          - text",
            "command: sh\n        static_args:\n          - -lc\n          - \"printf 'external-dispatch:%s' \\\"$1\\\"\"\n          - vida-dispatch",
        );
        fs::write(&config_path, updated).expect("config should update");

        let state_root = harness_state_root(&harness);
        runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let dispatch_packet_path = harness.path().join("external-agent-dispatch.json");
        fs::write(
            &dispatch_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": runtime_delivery_task_packet(
                    "run-external-dispatch",
                    "implementer",
                    "worker",
                    "implementation",
                    "implementation",
                    agent_lane_test_request()
                ),
                "dispatch_target": "implementer",
                "request_text": agent_lane_test_request(),
                "activation_runtime_role": "worker",
                "role_selection": {
                    "selected_role": "worker"
                }
            }))
            .expect("dispatch packet json should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: agent_lane_test_request().to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("qwen_cli"),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(dispatch_packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let result = runtime
            .block_on(execute_runtime_dispatch_handoff(
                &state_root,
                &role_selection,
                &receipt,
            ))
            .expect("external agent-lane dispatch handoff should execute");

        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert!(result["blocker_code"].is_null());
        assert_eq!(
            result["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(result["backend_dispatch"]["backend_id"], "qwen_cli");
        assert!(result["activation_command"]
            .as_str()
            .expect("activation command should render")
            .contains("sh"));
        assert!(result["provider_output"]
            .as_str()
            .expect("provider output should render")
            .contains("external-dispatch:Read and execute the VIDA dispatch packet"));
        assert_eq!(result["role_selection"]["selected_role"], "worker");
    }

    #[test]
    fn runtime_dispatch_packet_carries_external_host_runtime_contract() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_root_guards = HarnessStateRootGuards::set(harness_state_root(&harness));

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
                "qwen",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );

        let state_root = harness.path().join(".vida/data/state");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement backend execution".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "qwen-primary",
                            "activation_runtime_role": "worker",
                            "closure_class": "implementation",
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-qwen-dispatch".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("qwen-primary".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen-primary".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let packet = crate::read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");
        assert_eq!(packet["host_runtime"]["selected_cli_system"], "qwen");
        assert_eq!(
            packet["host_runtime"]["selected_cli_execution_class"],
            "external"
        );
        assert_eq!(packet["host_runtime"]["runtime_template_root"], ".qwen");
        assert_eq!(packet["selected_backend"], "qwen-primary");
        assert_eq!(
            packet["effective_execution_posture"]["selected_cli_system"],
            "qwen"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_execution_class"],
            "external"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_backend"],
            "qwen-primary"
        );
        assert_eq!(
            packet["effective_execution_posture"]["route_primary_backend"],
            serde_json::Value::Null
        );
        assert_eq!(
            packet["effective_execution_posture"]["activation_evidence_state"],
            "activation_view_only"
        );
    }

    #[test]
    fn downstream_receipt_backend_prefers_activation_agent_type() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let root_receipt = RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let downstream = build_downstream_dispatch_receipt(&role_selection, &root_receipt)
            .expect("downstream receipt should build");
        assert_eq!(downstream.activation_agent_type.as_deref(), Some("junior"));
        assert_eq!(downstream.selected_backend.as_deref(), Some("junior"));
    }

    #[test]
    fn spec_pack_downstream_routes_to_specification_lane_when_agent_only_enabled() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("specification"));
        assert_eq!(command.as_deref(), Some("vida agent-init"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn spec_pack_downstream_canonicalizes_explicit_business_analyst_lane_alias() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_sequence": ["business_analyst", "implementer"],
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );

        assert_eq!(target.as_deref(), Some("specification"));
        assert_eq!(command.as_deref(), Some("vida agent-init"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn packet_ready_specification_lane_stays_active_while_work_pool_handoff_remains_blocked() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "work_pool_task": {
                        "create_command": "vida task create feature-x-work-pool \"Work-pool pack\" --type task --status open --json",
                        "ensure_command": "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "implementation": {
                        "coach_required": false,
                        "independent_verification_required": false
                    },
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-spec".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, command, note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("work-pool-pack"));
        assert_eq!(
            command.as_deref(),
            Some(
                "vida task ensure feature-x-work-pool \"Work-pool pack\" --type task --status open --json"
            )
        );
        assert!(!ready);
        assert!(blockers.contains(&"pending_specification_evidence".to_string()));
        assert!(blockers.contains(&"pending_design_finalize".to_string()));
        assert!(blockers.contains(&"pending_spec_task_close".to_string()));
        assert_eq!(
            active_downstream_dispatch_target(&receipt).as_deref(),
            Some("specification")
        );
        assert!(note
            .as_deref()
            .unwrap_or_default()
            .contains("wait for bounded evidence return"));
    }

    #[test]
    fn specification_downstream_activation_uses_specification_contract() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        },
                        "escalation_activation": {
                            "activation_agent_type": "architect",
                            "activation_runtime_role": "solution_architect"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let (_kind, surface, agent_type, runtime_role) =
            downstream_activation_fields(&role_selection, "specification");
        assert_eq!(surface.as_deref(), Some("vida agent-init"));
        assert_eq!(agent_type.as_deref(), Some("middle"));
        assert_eq!(runtime_role.as_deref(), Some("business_analyst"));
    }

    #[test]
    fn route_selected_backend_for_specification_prefers_contract_activation_tier() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "specification_activation": {
                        "activation_agent_type": "middle",
                    },
                }
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "specification");

        assert_eq!(backend.as_deref(), Some("middle"));
    }

    #[test]
    fn route_selected_backend_for_implementer_keeps_explicit_route_hint() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "implementation": {
                    "executor_backend": "qwen_cli",
                    "activation": {
                        "activation_agent_type": "middle",
                    },
                },
            }
        });

        let backend = route_selected_backend_for_dispatch_target(&execution_plan, "implementer");

        assert_eq!(backend.as_deref(), Some("qwen_cli"));
    }

    #[test]
    fn effective_execution_posture_keeps_backend_class_unknown_without_matrix_row() {
        let summary = effective_execution_posture_summary(
            &serde_json::json!({}),
            "coach",
            Some("qwen_cli"),
            None,
            None,
            false,
        );

        assert_eq!(summary["selected_backend"], "qwen_cli");
        assert_eq!(summary["selected_backend_class"], "unknown");
    }

    #[test]
    fn executed_worker_lane_sets_downstream_ready_without_evidence_blocker() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("dispatch-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-17T00:00:00Z".to_string(),
        };

        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("coach"));
        assert!(ready);
        assert!(blockers.is_empty());
    }

    #[test]
    fn activation_view_only_dispatch_result_does_not_unlock_the_next_lane() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev-blocked".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: Some("dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-08T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_has_execution_evidence(&receipt));
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                harness.path().join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        let (target, _command, _note, ready, blockers) = runtime.block_on(
            derive_downstream_dispatch_preview(&store, &role_selection, &receipt),
        );
        assert_eq!(target.as_deref(), Some("coach"));
        assert!(!ready);
        assert_eq!(
            blockers,
            vec!["pending_implementation_evidence".to_string()]
        );
    }

    #[test]
    fn refresh_downstream_dispatch_preview_unblocks_dev_handoff_after_work_pool_execution() {
        let root = std::env::temp_dir().join(format!(
            "vida-refresh-downstream-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("work-pool-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-work-pool",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-work-pool".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("dev-pack".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_work_pool_shape".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let store = runtime
            .block_on(crate::StateStore::open(
                root.join(crate::state_store::default_state_dir()),
            ))
            .expect("state store should initialize");
        runtime
            .block_on(refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            ))
            .expect("refresh should succeed");

        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("dev-pack")
        );
        assert_eq!(
            receipt.downstream_dispatch_command.as_deref(),
            Some("vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json")
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bounded_implementer_task_close_bridges_downstream_receipt_to_coach_ready() {
        let root = std::env::temp_dir().join(format!(
            "vida-implementer-bridge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("state store should open");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "feature-bridge-dev",
                title: "Bridge dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");
        store
            .close_task("feature-bridge-dev", "implemented and proven")
            .await
            .expect("task should close");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-bridge-dev",
                        "ensure_command": "vida task ensure feature-bridge-dev \"Bridge dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-bridge",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(
            try_bridge_bounded_implementer_completion_to_downstream_receipt(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("bridge should succeed")
        );
        assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
        assert!(receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt.blocker_code.is_none());
        assert!(receipt
            .downstream_dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bounded_implementer_bridge_stays_blocked_while_dev_task_is_open() {
        let root = std::env::temp_dir().join(format!(
            "vida-implementer-bridge-open-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("state store should open");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "feature-bridge-open-dev",
                title: "Open bridge dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-bridge-open-dev",
                        "ensure_command": "vida task ensure feature-bridge-open-dev \"Open bridge dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-bridge-open",
        });
        let mut receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge-open".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/work-pool-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/work-pool-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `implementer` evidence is recorded, activate `coach` for the next bounded lane"
                    .to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(
            !try_bridge_bounded_implementer_completion_to_downstream_receipt(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("bridge should evaluate cleanly")
        );
        assert!(!receipt.downstream_dispatch_ready);
        assert_eq!(
            receipt.downstream_dispatch_blockers,
            vec!["pending_implementation_evidence".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn downstream_preview_ready_blocker_parity_guard_detects_inconsistency() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            super::downstream_dispatch_ready_blocker_parity_error(true, &blockers),
            Some(
                "Derived downstream dispatch preview indicates downstream_dispatch_ready while blocker evidence remains"
                    .to_string()
            )
        );
        assert!(super::downstream_dispatch_ready_blocker_parity_error(false, &blockers).is_none());
    }

    #[test]
    fn context_preserves_inputs() {
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test-mode".to_string(),
            fallback_role: "junior".to_string(),
            request: "req".to_string(),
            selected_role: "junior".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: json!({}),
            execution_plan: json!({ "orchestration_contract": {}, "tracked_flow_bootstrap": {} }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-test".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let execution_plan_value = json!({"plan": "value"});
        let bootstrap_value = json!({"bootstrap": "value"});
        let ctx = RuntimeDispatchPacketContext::new(
            Path::new("/tmp"),
            &selection,
            &receipt,
            &execution_plan_value,
            &bootstrap_value,
        );
        assert_eq!(ctx.receipt.run_id, "run-test");
        assert_eq!(ctx.role_selection.request, "req");
    }

    #[test]
    fn downstream_packet_uses_next_lane_activation_for_dev_pack_handoff() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue bounded Release-1 work for task r1-04-a".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("pbi_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["task".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "tracked_flow_bootstrap": {},
                "orchestration_contract": {},
                "development_flow": {
                    "dispatch_contract": {
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-dev-pack".to_string(),
            dispatch_target: "dev-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
            dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("implementer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after the dev packet is created, activate the selected implementer lane for bounded execution"
                    .to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("executed".to_string()),
            downstream_dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("dev-pack".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-06T06:47:13Z".to_string(),
        };

        let packet = downstream_dispatch_packet_body(
            &role_selection,
            &json!({ "run_id": "run-dev-pack" }),
            &receipt,
            None,
        );

        assert_eq!(packet["packet_template_kind"], "delivery_task_packet");
        assert_eq!(packet["activation_agent_type"], "junior");
        assert_eq!(packet["activation_runtime_role"], "worker");
        assert_eq!(packet["selected_backend"], "junior");
        assert_eq!(packet["mixed_posture"]["route_primary_backend"], "junior");
        assert_eq!(packet["route_policy"]["route_primary_backend"], "junior");
        assert_eq!(
            packet["activation_vs_execution_evidence"]["evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            packet["activation_semantics"]["activation_kind"],
            "activation_view"
        );
        assert!(packet["execution_evidence"].is_null());
        assert_eq!(
            packet["effective_execution_posture"]["route_primary_backend"],
            "junior"
        );
        assert_eq!(
            packet["effective_execution_posture"]["selected_backend"],
            "junior"
        );
        assert_eq!(
            packet["effective_execution_posture"]["mixed_route_backends"],
            false
        );
        assert_eq!(
            packet["effective_execution_posture"]["activation_evidence_state"],
            "execution_evidence"
        );
        assert_eq!(
            packet["delivery_task_packet"]["handoff_runtime_role"],
            "worker"
        );
    }

    #[test]
    fn dispatch_surface_truth_prefers_receipt_result_evidence_over_packet_activation_view() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-surface-truth-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("temp root should exist");
        let packet_path = root.join("dispatch-packet.json");
        let result_path = root.join("dispatch-result.json");

        fs::write(
            &packet_path,
            json!({
                "activation_vs_execution_evidence": {
                    "activation_kind": "activation_view",
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                },
                "mixed_posture": {
                    "effective_posture_kind": "external_only"
                }
            })
            .to_string(),
        )
        .expect("packet should write");
        fs::write(
            &result_path,
            json!({
                "artifact_kind": "runtime_dispatch_result",
                "activation_semantics": {
                    "activation_kind": "execution_evidence"
                },
                "execution_evidence": {
                    "status": "recorded",
                    "backend_id": "internal_subagents"
                },
                "execution_state": "executed"
            })
            .to_string(),
        )
        .expect("result should write");

        let receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-status-truth".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("implementer".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let truth = dispatch_surface_truth_from_packet_path(
            &root,
            Some(packet_path.to_str().expect("packet path should be utf8")),
            &receipt,
        )
        .expect("surface truth should resolve");

        assert_eq!(
            truth["activation_vs_execution_evidence"]["evidence_state"],
            "execution_evidence_recorded"
        );
        assert_eq!(
            truth["activation_vs_execution_evidence"]["receipt_backed"],
            true
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn closed_tracked_dev_task_counts_as_implementer_evidence_for_preview() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let receipt = RunGraphDispatchReceipt {
                run_id: "run-bridge".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: None,
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            let (target, command, _note, ready, blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;
            assert_eq!(target.as_deref(), Some("coach"));
            assert_eq!(command.as_deref(), Some("vida agent-init"));
            assert!(ready);
            assert!(blockers.is_empty());
        });
    }

    #[test]
    fn latest_receipt_bridge_persists_ready_coach_handoff_after_task_close() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-bridge" });
            let taskflow_handoff_plan = build_taskflow_handoff_plan(&role_selection);
            let mut receipt = bridge_waiting_root_receipt("run-bridge");
            let ctx = RuntimeDispatchPacketContext::new(
                &state_root,
                &role_selection,
                &receipt,
                &taskflow_handoff_plan,
                &run_graph_bootstrap,
            );
            let dispatch_packet_path =
                write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
            receipt.dispatch_packet_path = Some(dispatch_packet_path);
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "run-bridge".to_string(),
                    task_id: "run-bridge".to_string(),
                    task_class: "pbi_discussion".to_string(),
                    active_node: "dev-pack".to_string(),
                    next_node: None,
                    status: "ready".to_string(),
                    route_task_class: "work-pool-pack".to_string(),
                    selected_backend: "taskflow_state_store".to_string(),
                    lane_id: "dev_pack_direct".to_string(),
                    lifecycle_stage: "dev_pack_active".to_string(),
                    policy_gate: "single_task_scope_required".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "conversation_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: true,
                })
                .await
                .expect("run-graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&receipt)
                .await
                .expect("receipt should persist");
            let bridged = maybe_bridge_closed_implementer_task_into_receipt(
                &store,
                &mut receipt,
                Some("feature-x-dev"),
            )
            .await
            .expect("bridge should succeed");
            assert!(bridged);
            store
                .record_run_graph_dispatch_receipt(&receipt)
                .await
                .expect("bridged receipt should persist");

            let persisted = store
                .run_graph_dispatch_receipt("run-bridge")
                .await
                .expect("receipt should load")
                .expect("receipt should exist");
            assert_eq!(
                persisted.downstream_dispatch_target.as_deref(),
                Some("coach")
            );
            assert!(persisted.downstream_dispatch_ready);
            assert!(persisted.downstream_dispatch_blockers.is_empty());
            assert_eq!(
                persisted.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = persisted
                .downstream_dispatch_result_path
                .as_deref()
                .expect("bridged downstream evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "implementer");
            assert_eq!(
                evidence["completion_receipt_id"],
                "task-close-feature-x-dev"
            );
            assert!(persisted
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()));
            let packet_path = persisted
                .downstream_dispatch_packet_path
                .as_deref()
                .expect("downstream packet path should exist");
            let packet: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(packet_path).expect("downstream packet should be readable"),
            )
            .expect("downstream packet should decode");
            let prompt = packet["prompt"]
                .as_str()
                .expect("downstream packet prompt should be a string");
            assert!(prompt.contains("Runtime role=coach"));
            assert!(prompt.contains("Do not run root-only orchestration commands"));
            assert!(!prompt.contains("vida taskflow consume continue --json"));
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_marks_ready_packets_as_packet_ready_with_result_path() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-preview" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-preview".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
            assert!(receipt.downstream_dispatch_ready);
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            assert_eq!(
                receipt.downstream_dispatch_result_path.as_deref(),
                Some("/tmp/implementer-result.json")
            );
            assert!(receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()));
        });
    }

    #[test]
    fn dev_pack_handoff_stays_blocked_without_owned_write_scope_for_implementer() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-missing-scope".to_string(),
                dispatch_target: "dev-pack".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "taskflow_pack".to_string(),
                dispatch_surface: Some("vida task ensure".to_string()),
                dispatch_command: Some("vida task ensure".to_string()),
                dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
                dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("dev-pack".to_string()),
                downstream_dispatch_last_target: Some("dev-pack".to_string()),
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("taskflow_state_store".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            };

            let (next_target, _, _, next_ready, next_blockers) =
                derive_downstream_dispatch_preview(&store, &role_selection, &receipt).await;

            assert_eq!(next_target.as_deref(), Some("implementer"));
            assert!(!next_ready);
            assert_eq!(next_blockers, vec!["missing_owned_write_scope".to_string()]);
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_does_not_mark_implementer_packet_ready_without_owned_scope(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-missing-scope" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-missing-scope".to_string(),
                dispatch_target: "dev-pack".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "taskflow_pack".to_string(),
                dispatch_surface: Some("vida task ensure".to_string()),
                dispatch_command: Some("vida task ensure".to_string()),
                dispatch_packet_path: Some("/tmp/dev-pack.json".to_string()),
                dispatch_result_path: Some("/tmp/dev-pack-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("dev-pack".to_string()),
                downstream_dispatch_last_target: Some("dev-pack".to_string()),
                activation_agent_type: None,
                activation_runtime_role: None,
                selected_backend: Some("taskflow_state_store".to_string()),
                recorded_at: "2026-04-14T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should fail closed into a blocked state, not an error");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("implementer")
            );
            assert!(!receipt.downstream_dispatch_ready);
            assert_eq!(receipt.downstream_dispatch_status, None);
            assert_eq!(
                receipt.downstream_dispatch_blockers,
                vec!["missing_owned_write_scope".to_string()]
            );
            assert!(receipt.downstream_dispatch_packet_path.is_none());
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_uses_task_close_completion_evidence_for_blocked_implementer(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-dev").await;

            let role_selection = bridge_test_role_selection("feature-x-dev");
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-closed-task" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-closed-task".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/activation-view.json".to_string()),
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
            assert!(receipt.downstream_dispatch_ready);
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            let evidence_path = receipt
                .downstream_dispatch_result_path
                .as_deref()
                .expect("task-close bridge evidence path should exist");
            let evidence = read_json(harness.path(), evidence_path);
            assert_eq!(evidence["artifact_kind"], "runtime_lane_completion_result");
            assert_eq!(evidence["completed_target"], "implementer");
            assert_eq!(
                evidence["completion_receipt_id"],
                "task-close-feature-x-dev"
            );
        });
    }

    #[test]
    fn refresh_downstream_dispatch_preview_unblocks_work_pool_handoff_after_spec_task_closure() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(state_root.clone())
                .await
                .expect("state store should open");
            create_and_close_task(&store, "feature-x-spec").await;
            let design_doc_path = harness.path().join("feature-x-spec-design.md");
            write_approved_design_doc(&design_doc_path);

            let role_selection = specification_test_role_selection(
                "feature-x-spec",
                design_doc_path
                    .to_str()
                    .expect("design doc path should be utf-8"),
            );
            let run_graph_bootstrap = json!({ "run_id": "run-refresh-spec-closed-task" });
            let mut receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-refresh-spec-closed-task".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_complete".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_specification_evidence".to_string(),
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-10T00:00:00Z".to_string(),
            };

            refresh_downstream_dispatch_preview(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut receipt,
            )
            .await
            .expect("preview should refresh");

            assert_eq!(
                receipt.downstream_dispatch_target.as_deref(),
                Some("work-pool-pack")
            );
            assert!(receipt.downstream_dispatch_ready);
            assert!(receipt.downstream_dispatch_blockers.is_empty());
            assert_eq!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready")
            );
            assert_eq!(
                receipt.downstream_dispatch_result_path.as_deref(),
                Some("/tmp/specification-result.json")
            );
            assert!(receipt
                .downstream_dispatch_note
                .as_deref()
                .unwrap_or_default()
                .contains("spec-pack is closed"));
        });
    }

    #[test]
    fn downstream_receipt_prefers_route_executor_backend_over_activation_tier() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "subagents": "legacy_hint_should_not_win"
                    },
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-bridge".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_complete".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after `coach` evidence is recorded, activate `verification`".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        let downstream = build_downstream_dispatch_receipt(&role_selection, &receipt)
            .expect("downstream receipt should build");

        assert_eq!(downstream.activation_agent_type.as_deref(), Some("middle"));
        assert_eq!(downstream.activation_runtime_role.as_deref(), Some("coach"));
        assert_eq!(downstream.selected_backend.as_deref(), Some("qwen_cli"));
    }

    #[test]
    fn backend_agnostic_route_keeps_inherited_selected_backend() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };

        assert_eq!(
            downstream_selected_backend(
                &role_selection,
                "implementer",
                Some("junior"),
                Some("junior")
            ),
            Some("junior".to_string())
        );
    }

    #[test]
    fn apply_first_handoff_execution_advances_executed_implementer_into_downstream_handoff() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-advance-implementer",
            "implementation",
            "implementation",
        );
        status.task_id = "run-advance-implementer".to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-advance-implementer".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after implementer evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/coach-preview.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };

        let advanced = apply_first_handoff_execution_to_run_graph_status(&status, &receipt);

        assert_eq!(advanced.active_node, "implementer");
        assert_eq!(advanced.next_node.as_deref(), Some("coach"));
        assert_eq!(advanced.handoff_state, "awaiting_coach");
        assert_eq!(advanced.resume_target, "dispatch.coach");
        assert_eq!(advanced.lifecycle_stage, "implementer_active");
    }

    #[test]
    fn apply_first_handoff_execution_does_not_complete_exception_recorded_closure() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure-exception",
            "closure",
            "delivery",
        );
        status.task_id = "run-closure-exception".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "closure_active".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-closure-exception".to_string(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-1".to_string()),
            dispatch_kind: "closure".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            dispatch_packet_path: Some("/tmp/closure-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/closure-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let advanced = apply_first_handoff_execution_to_run_graph_status(&status, &receipt);

        assert_eq!(advanced.active_node, "closure");
        assert_eq!(advanced.status, "blocked");
        assert_eq!(advanced.lifecycle_stage, "closure_active");
        assert_eq!(advanced.resume_target, "none");
        assert!(!advanced.recovery_ready);
    }

    #[test]
    fn write_runtime_dispatch_result_records_completion_evidence_for_executed_agent_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-completion-evidence".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:qwen".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "internal_cli:qwen",
                "status": "pass",
                "execution_state": "executed",
                "provider_result": "implemented"
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(artifact["artifact_kind"], "runtime_lane_completion_result");
        assert_eq!(artifact["run_id"], "run-completion-evidence");
        assert_eq!(artifact["completed_target"], "implementer");
        assert_eq!(
            artifact["source_dispatch_packet_path"],
            "/tmp/implementer-packet.json"
        );
        assert_eq!(artifact["provider_result"], "implemented");
        assert!(artifact["completion_receipt_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("dispatch-completion-")));
        assert!(artifact["recorded_at"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty()));
    }

    #[test]
    fn write_runtime_dispatch_result_keeps_blocked_agent_lane_as_dispatch_artifact() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-blocked-dispatch".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let path = write_runtime_dispatch_result(
            harness.path(),
            &receipt,
            &serde_json::json!({
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only"
            }),
        )
        .expect("dispatch result should write");

        let artifact: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("dispatch result should be readable"),
        )
        .expect("dispatch result should decode");

        assert_eq!(artifact["artifact_kind"], "runtime_dispatch_result");
        assert_eq!(artifact["run_id"], "run-blocked-dispatch");
        assert_eq!(artifact["dispatch_target"], "coach");
        assert_eq!(artifact["blocker_code"], "internal_activation_view_only");
        assert!(artifact.get("completion_receipt_id").is_none());
    }

    #[test]
    fn write_runtime_dispatch_packet_keeps_agent_init_command_for_mixed_implementer_route_before_execution(
    ) {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let state_root = harness.path().join(crate::state_store::default_state_dir());
        fs::create_dir_all(state_root.join("runtime-consumption"))
            .expect("runtime-consumption dir should exist");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue implementation in crates/vida/src/runtime_dispatch_state.rs"
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "qwen_cli",
                        "backend_class": "external_cli",
                        "write_scope": "none",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "write_scope": "repo",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "runtime_assignment": {
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-mixed-command".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("qwen_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let handoff_plan = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );

        let packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let packet = crate::read_json_file_if_present(Path::new(&packet_path))
            .expect("dispatch packet json should load");

        assert_eq!(packet["dispatch_surface"], "vida agent-init");
        assert_eq!(packet["dispatch_command"], "vida agent-init");
        assert_eq!(packet["selected_backend"], "qwen_cli");
        assert_eq!(
            packet["route_policy"]["route_primary_backend"],
            "qwen_cli"
        );
    }
}

pub(crate) fn write_runtime_dispatch_packet(
    ctx: &RuntimeDispatchPacketContext<'_>,
) -> Result<String, String> {
    let packet_dir = ctx
        .state_root
        .join("runtime-consumption")
        .join("dispatch-packets");
    std::fs::create_dir_all(&packet_dir)
        .map_err(|error| format!("Failed to create dispatch-packets directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", ctx.receipt.run_id));
    let packet_path_display = packet_path.display().to_string();
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(ctx.state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve project root for dispatch packet rendering: {error}")
        })?);
    let host_runtime = runtime_host_execution_contract_for_root(&project_root);
    let effective_execution_posture = effective_execution_posture_summary(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
        ctx.receipt.selected_backend.as_deref(),
        ctx.receipt.activation_agent_type.as_deref(),
        Some(&host_runtime),
        false,
    );
    let packet_template_kind = runtime_dispatch_packet_kind(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
        &ctx.receipt.dispatch_kind,
    );
    let handoff_runtime_role = ctx
        .receipt
        .activation_runtime_role
        .as_deref()
        .unwrap_or(ctx.role_selection.selected_role.as_str());
    let handoff_task_class =
        runtime_packet_handoff_task_class(&ctx.receipt.dispatch_target, handoff_runtime_role);
    let closure_class = dispatch_contract_lane(
        &ctx.role_selection.execution_plan,
        &ctx.receipt.dispatch_target,
    )
    .and_then(|lane| lane["closure_class"].as_str())
    .unwrap_or("implementation");
    let activation_command = runtime_dispatch_command_for_packet_path(
        ctx.role_selection,
        ctx.receipt,
        &packet_path_display,
    );
    let execution_truth = dispatch_execution_route_summary(
        ctx.role_selection,
        &ctx.receipt.dispatch_target,
        ctx.receipt.selected_backend.as_deref(),
    );
    let activation_evidence = dispatch_activation_evidence_summary(ctx.receipt);
    let delivery_task_packet = runtime_delivery_task_packet(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &ctx.role_selection.request,
    );
    let execution_block_packet = runtime_execution_block_packet(
        &ctx.receipt.run_id,
        &ctx.receipt.dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    let body = serde_json::json!({
        "packet_kind": "runtime_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet.clone()
        } else {
            serde_json::Value::Null
        },
        "execution_block_packet": if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
        "coach_review_packet": if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&ctx.receipt.run_id, &ctx.receipt.dispatch_target)
        } else {
            serde_json::Value::Null
        },
        "tracked_flow_packet": if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(
                ctx.role_selection,
                &ctx.receipt.run_id,
                &ctx.receipt.dispatch_target,
            )
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &ctx.receipt.run_id,
            &ctx.receipt.dispatch_target,
            handoff_runtime_role,
            &ctx.role_selection.request,
            &ctx.role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": ctx.receipt.recorded_at,
        "run_id": ctx.receipt.run_id,
        "dispatch_target": ctx.receipt.dispatch_target,
        "dispatch_status": ctx.receipt.dispatch_status,
        "lane_status": ctx.receipt.lane_status,
        "blocker_code": ctx.receipt.blocker_code,
        "supersedes_receipt_id": ctx.receipt.supersedes_receipt_id,
        "exception_path_receipt_id": ctx.receipt.exception_path_receipt_id,
        "dispatch_kind": ctx.receipt.dispatch_kind,
        "dispatch_surface": ctx.receipt.dispatch_surface,
        "dispatch_command": activation_command,
        "activation_agent_type": ctx.receipt.activation_agent_type,
        "activation_runtime_role": ctx.receipt.activation_runtime_role,
        "selected_backend": ctx.receipt.selected_backend,
        "mixed_posture": effective_execution_posture.clone(),
        "route_policy": execution_truth.clone(),
        "activation_vs_execution_evidence": activation_evidence.clone(),
        "activation_semantics": activation_evidence["activation_semantics"].clone(),
        "execution_evidence": activation_evidence["execution_evidence"].clone(),
        "effective_execution_posture": effective_execution_posture,
        "execution_truth": execution_truth,
        "activation_evidence": activation_evidence,
        "host_runtime": host_runtime,
        "request_text": ctx.role_selection.request,
        "role_selection": {
            "selected_role": ctx.role_selection.selected_role,
            "conversational_mode": ctx.role_selection.conversational_mode,
            "tracked_flow_entry": ctx.role_selection.tracked_flow_entry,
            "confidence": ctx.role_selection.confidence,
        },
        "role_selection_full": ctx.role_selection,
        "taskflow_handoff_plan": ctx.taskflow_handoff_plan,
        "run_graph_bootstrap": ctx.run_graph_bootstrap,
        "orchestration_contract": ctx.role_selection.execution_plan["orchestration_contract"],
    });
    validate_runtime_dispatch_packet_contract(&body, "Runtime dispatch packet")?;
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode dispatch packet: {error}"))?;
    std::fs::write(&packet_path, encoded)
        .map_err(|error| format!("Failed to write dispatch packet: {error}"))?;
    Ok(packet_path.display().to_string())
}

pub(crate) async fn execute_runtime_dispatch_handoff(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve current directory for dispatch execution: {error}")
        })?);
    match receipt.dispatch_target.as_str() {
        "spec-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for spec-pack dispatch: {error}"
                    )
                })?;
            execute_taskflow_bootstrap_spec_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
            )
        }
        "work-pool-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for work-pool dispatch: {error}"
                    )
                })?;
            execute_work_packet_create_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
                "work_pool_task",
            )
        }
        "dev-pack" => {
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for dev-pack dispatch: {error}"
                    )
                })?;
            execute_work_packet_create_with_store(
                &project_root,
                &store,
                &role_selection.request,
                &role_selection.execution_plan["tracked_flow_bootstrap"],
                "dev_task",
            )
        }
        "closure" => Ok(serde_json::json!({
            "surface": "vida taskflow closure-preview",
            "status": "pass",
            "closure_ready": true,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
            "note": "runtime downstream scheduler reached closure without additional lane activation",
        })),
        _ => {
            let dispatch_packet_path =
                receipt.dispatch_packet_path.as_deref().ok_or_else(|| {
                    missing_agent_lane_dispatch_packet_error(&receipt.dispatch_target)
                })?;
            let canonical_backend = canonical_selected_backend_for_receipt(role_selection, receipt);
            if canonical_backend.is_none() {
                return Err(format!(
                    "Dispatch target `{}` is routed to an agent lane but no lawful backend could be resolved from the execution route",
                    receipt.dispatch_target
                ));
            }
            let host_runtime = runtime_host_execution_contract_for_root(&project_root);
            let lane_dispatch = runtime_agent_lane_dispatch_for_root(
                &project_root,
                dispatch_packet_path,
                canonical_backend.as_deref(),
            );
            if lane_dispatch.surface != "vida agent-init" {
                return execute_external_agent_lane_dispatch(
                    state_root,
                    &project_root,
                    dispatch_packet_path,
                    canonical_backend.as_deref(),
                    role_selection,
                    receipt,
                    host_runtime,
                )
                .await;
            }
            if let Some(result) = execute_internal_agent_lane_dispatch(
                state_root,
                &project_root,
                dispatch_packet_path,
                canonical_backend.as_deref(),
                role_selection,
                receipt,
                host_runtime.clone(),
            )
            .await?
            {
                return Ok(result);
            }
            let store = StateStore::open_existing(state_root.to_path_buf())
                .await
                .map_err(|error| {
                    format!(
                        "Failed to reopen authoritative state store for activation view: {error}"
                    )
                })?;
            let activation_view =
                crate::init_surfaces::render_agent_init_packet_activation_with_store(
                    &store,
                    &project_root,
                    dispatch_packet_path,
                    false,
                )
                .await?;
            Ok(agent_lane_dispatch_result(
                activation_view,
                dispatch_packet_path,
                canonical_backend.as_deref(),
                role_selection,
                receipt,
                host_runtime,
            ))
        }
    }
}

fn write_runtime_dispatch_result(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    body: &serde_json::Value,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let result_path = result_dir.join(format!("{}-{ts}.json", receipt.run_id));
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let mut artifact_body = body.clone();
    if let Some(object) = artifact_body.as_object_mut() {
        object.insert("run_id".to_string(), serde_json::json!(receipt.run_id));
        object.insert(
            "recorded_at".to_string(),
            serde_json::json!(recorded_at.clone()),
        );
        if let Some(packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            object.insert(
                "source_dispatch_packet_path".to_string(),
                serde_json::json!(packet_path),
            );
        }
        let executed_agent_lane = receipt.dispatch_kind == "agent_lane"
            && json_string(body.get("execution_state")).as_deref() == Some("executed")
            && receipt
                .dispatch_packet_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty());
        if executed_agent_lane {
            let completion_receipt_id = format!(
                "dispatch-completion-{}",
                time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            );
            object.insert(
                "artifact_kind".to_string(),
                serde_json::json!("runtime_lane_completion_result"),
            );
            object.insert(
                "completed_target".to_string(),
                serde_json::json!(receipt.dispatch_target),
            );
            object.insert(
                "completion_receipt_id".to_string(),
                serde_json::json!(completion_receipt_id),
            );
        } else {
            object.insert(
                "artifact_kind".to_string(),
                serde_json::json!("runtime_dispatch_result"),
            );
            object.insert(
                "dispatch_target".to_string(),
                serde_json::json!(receipt.dispatch_target),
            );
        }
    }
    let encoded = serde_json::to_string_pretty(&artifact_body)
        .map_err(|error| format!("Failed to encode dispatch result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write dispatch result: {error}"))?;
    Ok(result_path.display().to_string())
}

fn runtime_dispatch_execution_started_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> serde_json::Value {
    serde_json::json!({
        "surface": receipt.dispatch_surface,
        "activation_command": receipt.dispatch_command,
        "status": "pass",
        "execution_state": "executing",
        "dispatch_target": receipt.dispatch_target,
        "selected_backend": receipt.selected_backend,
        "note": "runtime dispatch handoff started; terminal completion is still pending",
    })
}

pub(crate) fn write_runtime_lane_completion_result(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let result_path = result_dir.join(format!("{run_id}-{ts}.json"));
    let body = serde_json::json!({
        "artifact_kind": "runtime_lane_completion_result",
        "status": "pass",
        "execution_state": "executed",
        "run_id": run_id,
        "completed_target": completed_target,
        "completion_receipt_id": receipt_id,
        "source_dispatch_packet_path": source_dispatch_packet_path,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    });
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode lane completion result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write lane completion result: {error}"))?;
    Ok(result_path.display().to_string())
}

pub(crate) async fn execute_and_record_dispatch_receipt(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if receipt.dispatch_kind == "agent_lane" {
        receipt.selected_backend = canonical_selected_backend_for_receipt(role_selection, receipt);
    }
    let in_flight_dispatch_result_path = write_runtime_dispatch_result(
        state_root,
        receipt,
        &runtime_dispatch_execution_started_result(receipt),
    )?;
    receipt.dispatch_result_path = Some(in_flight_dispatch_result_path);
    receipt.dispatch_status = "executing".to_string();
    receipt.lane_status = LaneStatus::LaneRunning.as_str().to_string();
    receipt.blocker_code = None;
    let store = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out reopening authoritative state store before dispatch execution after {}s",
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })?
    .map_err(|error| {
        format!("Failed to reopen authoritative state store before dispatch execution: {error}")
    })?;
    if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
        if let Ok(status) = store.run_graph_status(&run_id).await {
            let executing_status =
                apply_dispatch_execution_started_to_run_graph_status(&status, receipt);
            store
                .record_run_graph_status(&executing_status)
                .await
                .map_err(|error| {
                    format!("Failed to record in-flight run-graph status before dispatch execution: {error}")
                })?;
            crate::taskflow_continuation::sync_run_graph_continuation_binding(
                &store,
                &executing_status,
                "dispatch_execution_started",
            )
            .await
            .map_err(|error| {
                format!(
                    "Failed to synchronize continuation binding before dispatch execution: {error}"
                )
            })?;
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| {
            format!("Failed to persist in-flight dispatch receipt before execution: {error}")
        })?;
    drop(store);
    let execution_result = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_HANDOFF_EXECUTION_TIMEOUT_SECONDS),
        execute_runtime_dispatch_handoff(state_root, role_selection, receipt),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out executing runtime dispatch handoff after {}s",
            DEFAULT_DISPATCH_HANDOFF_EXECUTION_TIMEOUT_SECONDS
        )
    })??;
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    let execution_state = json_string(execution_result.get("execution_state"))
        .unwrap_or_else(|| "executed".to_string());
    receipt.dispatch_status = execution_state;
    let mut lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    );
    let closure_completed = receipt.dispatch_target == "closure"
        && receipt.dispatch_status == "executed"
        && json_bool(execution_result.get("closure_ready"), false)
        && lane_status == LaneStatus::LaneRunning;
    if closure_completed {
        lane_status = LaneStatus::LaneCompleted;
    }
    receipt.lane_status = lane_status.as_str().to_string();
    receipt.blocker_code =
        if receipt.dispatch_status == "blocked" && receipt.dispatch_packet_path.is_none() {
            blocker_code_value(BlockerCode::MissingPacket)
        } else if receipt.dispatch_status == "blocked" {
            json_string(execution_result.get("blocker_code"))
        } else {
            None
        };
    if let Some(dispatch_surface) = json_string(execution_result.get("surface")) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    let store = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out reopening authoritative state store after dispatch execution after {}s",
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })?
    .map_err(|error| {
        format!("Failed to reopen authoritative state store after dispatch execution: {error}")
    })?;
    tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS),
        refresh_downstream_dispatch_preview(&store, role_selection, run_graph_bootstrap, receipt),
    )
    .await
    .map_err(|_| {
        format!(
            "Timed out refreshing downstream dispatch preview after dispatch execution after {}s",
            DEFAULT_DISPATCH_STATE_COORDINATION_TIMEOUT_SECONDS
        )
    })??;
    if receipt.dispatch_status == "executed" {
        if let Some(run_id) = json_string(run_graph_bootstrap.get("run_id")) {
            if let Ok(status) = store.run_graph_status(&run_id).await {
                let executed_status =
                    apply_first_handoff_execution_to_run_graph_status(&status, receipt);
                store
                    .record_run_graph_status(&executed_status)
                    .await
                    .map_err(|error| {
                        format!("Failed to record executed run-graph status: {error}")
                    })?;
                crate::taskflow_continuation::sync_run_graph_continuation_binding(
                    &store,
                    &executed_status,
                    "dispatch_execution",
                )
                .await
                .map_err(|error| {
                    format!(
                        "Failed to synchronize continuation binding after dispatch execution: {error}"
                    )
                })?;
            }
        }
    }
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .map_err(|error| format!("Failed to persist dispatch receipt after execution: {error}"))?;
    Ok(())
}

pub(crate) async fn execute_downstream_dispatch_chain(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let root_lane_has_execution_evidence = if dispatch_receipt_has_execution_evidence(root_receipt)
    {
        true
    } else {
        let store = StateStore::open_existing(state_root.to_path_buf())
            .await
            .map_err(|error| {
                format!(
                    "Failed to reopen authoritative state store for downstream execution evidence: {error}"
                )
            })?;
        tracked_implementer_task_closed(&store, role_selection, root_receipt).await
    };
    if !root_lane_has_execution_evidence || !root_receipt.downstream_dispatch_ready {
        return Ok(());
    }

    let mut downstream_source = root_receipt.clone();
    let mut downstream_trace = Vec::new();
    for _ in 0..4 {
        let Some(mut downstream_receipt) =
            build_downstream_dispatch_receipt(role_selection, &downstream_source)
        else {
            break;
        };
        if downstream_receipt.dispatch_status != "routed"
            || (downstream_receipt.dispatch_kind == "taskflow_pack"
                && taskflow_task_bridge::infer_project_root_from_state_root(state_root).is_none())
        {
            root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
            break;
        }

        execute_and_record_dispatch_receipt(
            state_root,
            role_selection,
            run_graph_bootstrap,
            &mut downstream_receipt,
        )
        .await
        .map_err(|error| {
            format!("Failed to execute downstream runtime dispatch handoff: {error}")
        })?;

        let store = StateStore::open_existing(state_root.to_path_buf())
            .await
            .map_err(|error| {
                format!(
                    "Failed to reopen authoritative state store for downstream preview refresh: {error}"
                )
            })?;
        let (next_target, next_command, next_note, next_ready, next_blockers) =
            derive_downstream_dispatch_preview(&store, role_selection, &downstream_receipt).await;
        if let Some(error) =
            downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
        {
            return Err(error);
        }
        let preview_result_path = downstream_receipt.dispatch_result_path.clone();
        apply_downstream_dispatch_preview_to_receipt(
            &mut downstream_receipt,
            next_target,
            next_command,
            next_note,
            next_ready,
            next_blockers,
            preview_result_path,
        );
        downstream_receipt.downstream_dispatch_packet_path =
            write_runtime_downstream_dispatch_packet(
                state_root,
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
            )
            .map_err(|error| {
                format!("Failed to write chained downstream runtime dispatch packet: {error}")
            })?;
        if let Some(packet_path) = downstream_receipt
            .downstream_dispatch_packet_path
            .as_deref()
        {
            write_runtime_downstream_dispatch_packet_at(
                Path::new(packet_path),
                role_selection,
                run_graph_bootstrap,
                &downstream_receipt,
            )
            .map_err(|error| {
                format!("Failed to refresh chained downstream runtime dispatch packet: {error}")
            })?;
        }

        downstream_trace
            .push(serde_json::to_value(&downstream_receipt).unwrap_or(serde_json::Value::Null));
        if downstream_receipt.dispatch_status == "executed" {
            root_receipt.downstream_dispatch_executed_count += 1;
        }
        root_receipt.downstream_dispatch_last_target =
            Some(downstream_receipt.dispatch_target.clone());
        root_receipt_fields_from_downstream_step(root_receipt, &downstream_receipt);
        if !downstream_receipt.downstream_dispatch_ready {
            break;
        }
        downstream_source = downstream_receipt;
    }

    if !downstream_trace.is_empty() {
        let trace_path = write_runtime_downstream_dispatch_trace(
            state_root,
            &root_receipt.run_id,
            &downstream_trace,
        )
        .map_err(|error| format!("Failed to write downstream runtime dispatch trace: {error}"))?;
        root_receipt.downstream_dispatch_trace_path = Some(trace_path);
    }
    Ok(())
}

pub(crate) fn apply_first_handoff_execution_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    if receipt.dispatch_target == "closure" {
        if canonical_lane_status_str(&receipt.lane_status) != Some("lane_completed") {
            return crate::state_store::RunGraphStatus {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_class: status.task_class.clone(),
                active_node: "closure".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: status.route_task_class.clone(),
                selected_backend: receipt
                    .selected_backend
                    .clone()
                    .unwrap_or_else(|| status.selected_backend.clone()),
                lane_id: "closure_direct".to_string(),
                lifecycle_stage: "closure_active".to_string(),
                policy_gate: status.policy_gate.clone(),
                handoff_state: "none".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: status.checkpoint_kind.clone(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
        }
        return crate::state_store::RunGraphStatus {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            task_class: status.task_class.clone(),
            active_node: "closure".to_string(),
            next_node: None,
            status: "completed".to_string(),
            route_task_class: status.route_task_class.clone(),
            selected_backend: receipt
                .selected_backend
                .clone()
                .unwrap_or_else(|| status.selected_backend.clone()),
            lane_id: "closure_direct".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            policy_gate: status.policy_gate.clone(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: status.checkpoint_kind.clone(),
            resume_target: "none".to_string(),
            recovery_ready: true,
        };
    }
    let dispatch_target = receipt.dispatch_target.replace('-', "_");
    let next_node =
        if receipt.downstream_dispatch_ready && receipt.downstream_dispatch_blockers.is_empty() {
            receipt
                .downstream_dispatch_target
                .as_deref()
                .map(|target| target.replace('-', "_"))
        } else {
            None
        };
    let (handoff_state, resume_target) = if let Some(next_target) = next_node.as_deref() {
        (
            format!("awaiting_{next_target}"),
            format!("dispatch.{next_target}"),
        )
    } else {
        ("none".to_string(), "none".to_string())
    };
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node,
        status: "ready".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: if receipt.dispatch_kind == "taskflow_pack" {
            format!("{dispatch_target}_direct")
        } else {
            format!("{dispatch_target}_lane")
        },
        lifecycle_stage: format!("{dispatch_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state,
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target,
        recovery_ready: true,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}

fn apply_dispatch_execution_started_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    let dispatch_target = receipt.dispatch_target.replace('-', "_");
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node: None,
        status: "running".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: if receipt.dispatch_kind == "taskflow_pack" {
            format!("{dispatch_target}_direct")
        } else {
            format!("{dispatch_target}_lane")
        },
        lifecycle_stage: format!("{dispatch_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state: "none".to_string(),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}
