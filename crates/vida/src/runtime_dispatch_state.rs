use std::path::Path;

use time::format_description::well_known::Rfc3339;

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
use crate::runtime_dispatch_packets::{
    runtime_coach_review_packet, runtime_delivery_task_packet, runtime_escalation_packet,
    runtime_execution_block_packet, runtime_verifier_proof_packet,
};
use crate::taskflow_routing::{
    fallback_executor_backend_from_route, fanout_executor_backends_from_route,
    runtime_assignment_source_from_execution_plan,
};

use super::*;

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

fn route_declares_backend(route: &serde_json::Value, candidate: &str) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    if selected_backend_from_execution_plan_route(&serde_json::Value::Null, route).as_deref()
        == Some(candidate)
    {
        return true;
    }
    if fallback_executor_backend_from_route(route).as_deref() == Some(candidate) {
        return true;
    }
    fanout_executor_backends_from_route(route)
        .iter()
        .any(|backend| backend == candidate)
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
        _ => execution_plan_route_for_dispatch_target(
            &role_selection.execution_plan,
            dispatch_target,
        )
        .and_then(|route| {
            inherited_selected_backend
                .filter(|candidate| route_declares_backend(route, candidate))
                .map(str::to_string)
                .or_else(|| {
                    selected_backend_from_execution_plan_route(
                        &role_selection.execution_plan,
                        route,
                    )
                })
        })
        .or_else(|| activation_agent_type.map(str::to_string))
        .or_else(|| inherited_selected_backend.map(str::to_string)),
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
            } else if backend_id.ends_with("_cli") {
                "external_cli".to_string()
            } else {
                "internal".to_string()
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
    activation_agent_type: Option<&str>,
    host_runtime: Option<&serde_json::Value>,
    receipt_backed_execution_evidence: bool,
) -> serde_json::Value {
    let route = execution_plan_route_for_dispatch_target(execution_plan, dispatch_target);
    let route_primary_backend =
        route.and_then(|lane| selected_backend_from_execution_plan_route(execution_plan, lane));
    let fallback_backend = route.and_then(fallback_executor_backend_from_route);
    let fanout_backends = route
        .map(fanout_executor_backends_from_route)
        .unwrap_or_default();
    let effective_selected_backend = selected_backend
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            activation_agent_type
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
        })
        .or_else(|| route_primary_backend.clone());
    let selected_backend_source = if selected_backend.is_some_and(|value| !value.trim().is_empty())
    {
        "dispatch_receipt"
    } else if activation_agent_type.is_some_and(|value| !value.trim().is_empty()) {
        "activation_agent_type"
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
    let route_primary_backend = route.and_then(|entry| {
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, entry)
    });
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
    let activation_evidence = packet
        .get("activation_vs_execution_evidence")
        .cloned()
        .or_else(|| packet.get("activation_evidence").cloned())
        .or_else(|| {
            dispatch_receipt.dispatch_result_path.as_deref().and_then(|path| {
                let resolved = resolve_project_artifact_path(project_root, Some(path))?;
                let value = crate::read_json_file_if_present(&resolved)?;
                Some(serde_json::json!({
                    "activation_kind": value["activation_semantics"]["activation_kind"],
                    "evidence_state": if value["execution_evidence"]["status"].as_str() == Some("recorded") {
                        "execution_evidence_recorded"
                    } else {
                        "activation_view_only"
                    },
                    "activation_semantics": value["activation_semantics"].clone(),
                    "execution_evidence": value["execution_evidence"].clone(),
                }))
            })
        });
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

pub(crate) fn selected_external_backend_for_system(
    overlay: &serde_yaml::Value,
    system: &str,
    preferred_backend: Option<&str>,
) -> Option<(String, serde_yaml::Value)> {
    let subagents = yaml_lookup(overlay, &["agent_system", "subagents"])?;
    let entries = subagents.as_mapping()?;
    let preferred_key = format!("{system}_cli");
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
        if backend_id == preferred_key
            || detect_command.as_deref() == Some(system)
            || backend_id.starts_with(system)
        {
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
    if let Some((backend_id, backend_class)) = overlay.and_then(|overlay| {
        preferred_backend.and_then(|backend_id| {
            configured_subagent_entry(overlay, backend_id).and_then(|entry| {
                yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
                    .map(|backend_class| (backend_id.to_string(), backend_class))
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
    }
    if selected_execution_class != "external" {
        return RuntimeAgentLaneDispatch {
            surface: "vida agent-init".to_string(),
            activation_command: agent_init_command,
            backend_dispatch: serde_json::json!({
                "selected_cli_system": selected_cli_system,
                "selected_execution_class": selected_execution_class,
                "backend_id": serde_json::Value::Null,
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
                "pending_design_finalize".to_string(),
                "pending_spec_task_close".to_string(),
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
        "dev-pack" => (
            Some(
                execution_lane_sequence
                    .first()
                    .map(|value| value.as_str())
                    .unwrap_or("implementer")
                    .to_string(),
            ),
            Some("vida agent-init".to_string()),
            Some(
                "after the dev packet is created, activate the selected implementer lane for bounded execution"
                    .to_string(),
            ),
            true,
            Vec::new(),
        ),
        _ if receipt.dispatch_kind == "agent_lane" => {
            let current_lane =
                dispatch_contract_lane(&role_selection.execution_plan, &receipt.dispatch_target);
            if current_lane.and_then(|lane| lane["stage"].as_str()) == Some("design_gate")
                || (receipt.dispatch_target == "specification"
                    && current_lane.and_then(|lane| lane["stage"].as_str()).is_none()
                    && dispatch_contract.get("specification_activation").is_some())
            {
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
                        if receipt.dispatch_status == "executed" {
                            "after specification/planning evidence is recorded, finalize the design doc and close spec-pack before work-pool shaping via tracked work-pool ensure/reuse"
                        } else {
                            "specification/planning lane is active; wait for bounded evidence return before design finalization, spec-pack closure, and tracked work-pool ensure/reuse"
                        }
                        .to_string(),
                    ),
                    false,
                    vec![
                        evidence_blocker.to_string(),
                        "pending_design_finalize".to_string(),
                        "pending_spec_task_close".to_string(),
                    ],
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
                (
                    Some(next_target.clone()),
                    Some("vida agent-init".to_string()),
                    Some(format!(
                        "after `{}` evidence is recorded, activate `{}` for the next bounded lane",
                        receipt.dispatch_target, next_target
                    )),
                    has_lane_evidence,
                    if has_lane_evidence {
                        Vec::new()
                    } else {
                        vec![blocker]
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
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        store.root(),
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
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
    let missing = match packet_template_kind {
        "delivery_task_packet" | "execution_block_packet" => {
            let mut missing = Vec::new();
            if !packet_nonempty_string(active_packet.get("goal")) {
                missing.push("goal");
            }
            if !packet_nonempty_string_array(active_packet, "scope_in") {
                missing.push("scope_in");
            }
            if !packet_has_owned_or_read_only_paths(active_packet) {
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
    let canonical_backend = canonical_selected_backend_for_receipt(role_selection, receipt);
    match receipt.dispatch_kind.as_str() {
        "taskflow_pack" => {
            runtime_dispatch_command_for_target(role_selection, &receipt.dispatch_target)
        }
        "agent_lane" => Some(
            runtime_agent_lane_dispatch_for_root(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                packet_path,
                canonical_backend.as_deref(),
            )
            .activation_command,
        ),
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
mod runtime_dispatch_packet_context_tests {
    use super::*;
    use crate::state_store::CreateTaskRequest;
    use crate::state_store::RunGraphDispatchReceipt;
    use crate::temp_state::TempStateHarness;
    use serde_json::json;
    use std::fs;

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
        assert!(packet["mixed_posture"]["route_primary_backend"].is_null());
        assert!(packet["route_policy"]["route_primary_backend"].is_null());
        assert_eq!(
            packet["activation_vs_execution_evidence"]["evidence_state"],
            "execution_evidence_recorded"
        );
        assert_eq!(
            packet["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(packet["execution_evidence"]["status"], "recorded");
        assert_eq!(
            packet["effective_execution_posture"]["route_primary_backend"],
            serde_json::Value::Null
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
            dispatch_surface: Some("internal_cli:codex".to_string()),
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
                "surface": "internal_cli:codex",
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
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let project_root = taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        .unwrap_or(std::env::current_dir().map_err(|error| {
            format!("Failed to resolve current directory for dispatch execution: {error}")
        })?);
    match receipt.dispatch_target.as_str() {
        "spec-pack" => execute_taskflow_bootstrap_spec_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
        ),
        "work-pool-pack" => execute_work_packet_create_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
            "work_pool_task",
        ),
        "dev-pack" => execute_work_packet_create_with_store(
            &project_root,
            store,
            &role_selection.request,
            &role_selection.execution_plan["tracked_flow_bootstrap"],
            "dev_task",
        ),
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
            let host_runtime = runtime_host_execution_contract_for_root(&project_root);
            let lane_dispatch = runtime_agent_lane_dispatch_for_root(
                &project_root,
                dispatch_packet_path,
                canonical_backend.as_deref(),
            );
            if lane_dispatch.surface != "vida agent-init" {
                return execute_external_agent_lane_dispatch(
                    store,
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
                store,
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
            let activation_view =
                crate::init_surfaces::render_agent_init_packet_activation_with_store(
                    store,
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
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if receipt.dispatch_kind == "agent_lane" {
        receipt.selected_backend = canonical_selected_backend_for_receipt(role_selection, receipt);
    }
    let execution_result =
        execute_runtime_dispatch_handoff(state_root, store, role_selection, receipt).await?;
    let dispatch_result_path =
        write_runtime_dispatch_result(state_root, receipt, &execution_result)?;
    receipt.dispatch_result_path = Some(dispatch_result_path);
    let execution_state = json_string(execution_result.get("execution_state"))
        .unwrap_or_else(|| "executed".to_string());
    receipt.dispatch_status = execution_state;
    let closure_completed = receipt.dispatch_target == "closure"
        && receipt.dispatch_status == "executed"
        && json_bool(execution_result.get("closure_ready"), false);
    let mut lane_status = derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    );
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
    refresh_downstream_dispatch_preview(store, role_selection, run_graph_bootstrap, receipt)
        .await?;
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
                    store,
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
    Ok(())
}

pub(crate) async fn execute_downstream_dispatch_chain(
    state_root: &Path,
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    root_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let root_lane_has_execution_evidence = dispatch_receipt_has_execution_evidence(root_receipt)
        || tracked_implementer_task_closed(store, role_selection, root_receipt).await;
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
            store,
            role_selection,
            run_graph_bootstrap,
            &mut downstream_receipt,
        )
        .await
        .map_err(|error| {
            format!("Failed to execute downstream runtime dispatch handoff: {error}")
        })?;

        let (next_target, next_command, next_note, next_ready, next_blockers) =
            derive_downstream_dispatch_preview(store, role_selection, &downstream_receipt).await;
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

fn apply_first_handoff_execution_to_run_graph_status(
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> crate::state_store::RunGraphStatus {
    if receipt.dispatch_target == "closure" {
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
    let mut updated = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: receipt.dispatch_target.clone(),
        next_node: None,
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
        handoff_state: "none".to_string(),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: "none".to_string(),
        recovery_ready: true,
    };
    if receipt.dispatch_kind == "taskflow_pack" {
        updated.selected_backend = "taskflow_state_store".to_string();
    }
    updated
}
