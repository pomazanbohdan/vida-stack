use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use crate::runtime_dispatch_packet_text::{
    runtime_packet_prompt, runtime_packet_request_text, runtime_tracked_flow_packet,
};
use crate::runtime_dispatch_packets::{
    delivery_packet_task_class_requires_owned_paths, implementation_isolation_contract,
    runtime_coach_review_packet, runtime_delivery_task_packet_with_scope_context,
    runtime_escalation_packet, runtime_execution_block_packet, runtime_verifier_proof_packet,
};
use crate::{
    derive_lane_status, dispatch_contract_execution_lane_sequence, dispatch_contract_lane,
    downstream_activation_fields, json_string, validate_runtime_dispatch_packet_contract,
    RuntimeConsumptionLaneSelection,
};

fn neutral_downstream_activation_evidence() -> serde_json::Value {
    serde_json::json!({
        "activation_kind": "activation_view",
        "evidence_state": "activation_view_only",
        "execution_evidence_path": serde_json::Value::Null,
        "receipt_backed": false,
        "activation_semantics": {
            "activation_kind": "activation_view",
            "view_only": true,
            "executes_packet": false,
            "records_completion_receipt": false,
        },
        "execution_evidence": serde_json::Value::Null,
    })
}

fn infer_downstream_lane_id_for_dispatch_target(
    role_selection: &RuntimeConsumptionLaneSelection,
    current_dispatch_target: &str,
    downstream_target: &str,
    explicit_lane_id: Option<String>,
) -> Option<String> {
    if explicit_lane_id.is_some() || downstream_target.trim().is_empty() {
        return explicit_lane_id;
    }
    let dispatch_contract = &role_selection.execution_plan["development_flow"]["dispatch_contract"];
    let sequence = dispatch_contract_execution_lane_sequence(dispatch_contract);
    let current_index = sequence.iter().position(|lane_id| {
        lane_id == current_dispatch_target
            || crate::runtime_dispatch_state::resolve_runtime_dispatch_target(
                &role_selection.execution_plan,
                lane_id,
            )
            .is_some_and(|resolution| resolution.dispatch_target == current_dispatch_target)
    });
    let candidate_lanes = current_index
        .map(|index| sequence.iter().skip(index + 1).collect::<Vec<_>>())
        .unwrap_or_else(|| sequence.iter().collect::<Vec<_>>());
    candidate_lanes.into_iter().find_map(|lane_id| {
        crate::runtime_dispatch_state::resolve_runtime_dispatch_target(
            &role_selection.execution_plan,
            lane_id,
        )
        .filter(|resolution| {
            resolution.dispatch_target == downstream_target && lane_id.as_str() != downstream_target
        })
        .map(|_| lane_id.clone())
    })
}

fn push_unique_owned_path(paths: &mut Vec<String>, path: &str) {
    let trimmed = path.trim();
    if trimmed.is_empty() || paths.iter().any(|existing| existing == trimmed) {
        return;
    }
    paths.push(trimmed.to_string());
}

pub(crate) fn test_lane_requires_test_write_scope(
    downstream_target: &str,
    downstream_lane_id: Option<&str>,
    handoff_task_class: &str,
) -> bool {
    matches!(handoff_task_class, "test_authoring" | "regression_test")
        || [Some(downstream_target), downstream_lane_id]
            .into_iter()
            .flatten()
            .map(|value| value.to_ascii_lowercase())
            .any(|value| {
                value.contains("autotest")
                    || value.contains("test_author")
                    || value.contains("regression_test")
            })
}

pub(crate) fn project_test_write_scope_paths(project_root: &Path) -> Vec<String> {
    let candidates = ["test", "tests", "integration_test", "e2e"];
    let mut paths = candidates
        .iter()
        .filter(|candidate| project_root.join(candidate).exists())
        .map(|candidate| (*candidate).to_string())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        paths.push("test".to_string());
    }
    paths
}

fn normalized_owned_paths(owned_paths: &[String]) -> Vec<String> {
    let mut packet = serde_json::json!({});
    if !crate::runtime_dispatch_state::apply_owned_paths(&mut packet, owned_paths) {
        return Vec::new();
    }
    packet
        .get("owned_paths")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn configured_lane_contract_field(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    field: &str,
) -> Option<String> {
    let lane = dispatch_contract_lane(&role_selection.execution_plan, dispatch_target)?;
    let activation = lane.get("activation").unwrap_or(lane);
    json_string(activation.get(field))
        .or_else(|| {
            (field == "activation_runtime_role")
                .then(|| json_string(activation.get("runtime_role")))?
        })
        .or_else(|| {
            lane.get("runtime_assignment").and_then(|assignment| {
                json_string(assignment.get(field)).or_else(|| {
                    (field == "activation_runtime_role")
                        .then(|| json_string(assignment.get("runtime_role")))?
                })
            })
        })
        .or_else(|| {
            lane.get("carrier_runtime_assignment")
                .and_then(|assignment| {
                    json_string(assignment.get(field)).or_else(|| {
                        (field == "activation_runtime_role")
                            .then(|| json_string(assignment.get("runtime_role")))?
                    })
                })
        })
        .or_else(|| {
            (field == "activation_runtime_role").then(|| json_string(lane.get("runtime_role")))?
        })
}

pub(crate) fn configured_lane_runtime_role(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Option<String> {
    configured_lane_contract_field(role_selection, dispatch_target, "activation_runtime_role")
}

#[derive(Debug, Clone)]
pub(crate) struct DownstreamDispatchPacketContract {
    pub dispatch_target: String,
    pub downstream_lane_id: Option<String>,
    pub lookup_target: String,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub handoff_runtime_role: String,
    pub handoff_task_class: String,
    pub closure_class: String,
    pub owned_paths: Vec<String>,
    pub implementation_isolation: serde_json::Value,
}

impl DownstreamDispatchPacketContract {
    pub(crate) fn for_dispatch_target(
        role_selection: &RuntimeConsumptionLaneSelection,
        current_dispatch_target: &str,
        raw_dispatch_target: &str,
        explicit_lane_id: Option<String>,
        activation_agent_type_hint: Option<String>,
        activation_runtime_role_hint: Option<String>,
        implementation_owned_paths_override: &[String],
        project_root: &Path,
    ) -> Self {
        let target_resolution = crate::runtime_dispatch_state::resolve_runtime_dispatch_target(
            &role_selection.execution_plan,
            raw_dispatch_target,
        );
        let resolved_lane_id = target_resolution.as_ref().and_then(|resolution| {
            if raw_dispatch_target == resolution.dispatch_target {
                None
            } else {
                resolution.lane_id.clone()
            }
        });
        let dispatch_target = target_resolution
            .as_ref()
            .map(|resolution| resolution.dispatch_target.clone())
            .unwrap_or_else(|| raw_dispatch_target.to_string());
        let downstream_lane_id = infer_downstream_lane_id_for_dispatch_target(
            role_selection,
            current_dispatch_target,
            dispatch_target.as_str(),
            explicit_lane_id.or(resolved_lane_id),
        );
        let lookup_target = downstream_lane_id
            .clone()
            .unwrap_or_else(|| dispatch_target.clone());
        let (_, _, mut activation_agent_type, mut activation_runtime_role) =
            if dispatch_target.is_empty() {
                (
                    String::new(),
                    None,
                    activation_agent_type_hint.clone(),
                    activation_runtime_role_hint.clone(),
                )
            } else {
                downstream_activation_fields(role_selection, dispatch_target.as_str())
            };
        if !dispatch_target.is_empty() {
            if activation_agent_type.is_none() {
                activation_agent_type = configured_lane_contract_field(
                    role_selection,
                    lookup_target.as_str(),
                    "activation_agent_type",
                )
                .or_else(|| {
                    runtime_assignment_activation_field(role_selection, "activation_agent_type")
                });
            }
            if activation_runtime_role.is_none() {
                activation_runtime_role = configured_lane_runtime_role(
                    role_selection,
                    lookup_target.as_str(),
                )
                .or_else(|| {
                    runtime_assignment_activation_field(role_selection, "activation_runtime_role")
                });
            }
        }
        let handoff_runtime_role = activation_runtime_role
            .as_deref()
            .or(activation_runtime_role_hint.as_deref())
            .unwrap_or(role_selection.selected_role.as_str())
            .to_string();
        let handoff_task_class =
            crate::runtime_dispatch_state::runtime_packet_handoff_task_class_for_plan(
                &role_selection.execution_plan,
                lookup_target.as_str(),
                handoff_runtime_role.as_str(),
            );
        let closure_class =
            dispatch_contract_lane(&role_selection.execution_plan, lookup_target.as_str())
                .and_then(|lane| lane["closure_class"].as_str())
                .unwrap_or("implementation")
                .to_string();
        let owned_paths =
            if delivery_packet_task_class_requires_owned_paths(handoff_task_class.as_str()) {
                let mut owned_paths = if implementation_owned_paths_override.is_empty() {
                    crate::runtime_dispatch_state::owned_paths_for_required_delivery_task_class(
                        role_selection,
                        handoff_task_class.as_str(),
                    )
                } else {
                    implementation_owned_paths_override.to_vec()
                };
                if test_lane_requires_test_write_scope(
                    dispatch_target.as_str(),
                    downstream_lane_id.as_deref(),
                    handoff_task_class.as_str(),
                ) {
                    for test_path in project_test_write_scope_paths(project_root) {
                        push_unique_owned_path(&mut owned_paths, &test_path);
                    }
                }
                normalized_owned_paths(&owned_paths)
            } else {
                Vec::new()
            };
        let implementation_isolation =
            implementation_isolation_contract(handoff_task_class.as_str(), &owned_paths);
        Self {
            dispatch_target,
            downstream_lane_id,
            lookup_target,
            activation_agent_type,
            activation_runtime_role,
            handoff_runtime_role,
            handoff_task_class,
            closure_class,
            owned_paths,
            implementation_isolation,
        }
    }

    fn set_field(
        object: &mut serde_json::Map<String, serde_json::Value>,
        key: &str,
        value: serde_json::Value,
    ) -> bool {
        if object.get(key) == Some(&value) {
            return false;
        }
        object.insert(key.to_string(), value);
        true
    }

    pub(crate) fn apply_to_active_packet_body(&self, packet: &mut serde_json::Value) -> bool {
        let Some(object) = packet.as_object_mut() else {
            return false;
        };
        let mut repaired = false;
        repaired |= Self::set_field(
            object,
            "handoff_runtime_role",
            serde_json::json!(self.handoff_runtime_role),
        );
        repaired |= Self::set_field(
            object,
            "handoff_task_class",
            serde_json::json!(self.handoff_task_class),
        );
        if !self.owned_paths.is_empty() {
            repaired |= crate::runtime_dispatch_state::apply_owned_paths(packet, &self.owned_paths);
        }
        let Some(object) = packet.as_object_mut() else {
            return repaired;
        };
        repaired |= Self::set_field(
            object,
            "implementation_isolation",
            self.implementation_isolation.clone(),
        );
        repaired
    }

    pub(crate) fn apply_to_packet(&self, packet: &mut serde_json::Value) -> bool {
        let packet_template_kind = packet
            .get("packet_template_kind")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "delivery_task_packet".to_string());
        let mut repaired = false;
        if let Some(object) = packet.as_object_mut() {
            repaired |= Self::set_field(
                object,
                "activation_agent_type",
                serde_json::json!(self.activation_agent_type),
            );
            repaired |= Self::set_field(
                object,
                "activation_runtime_role",
                serde_json::json!(self.activation_runtime_role),
            );
            repaired |= Self::set_field(
                object,
                "handoff_runtime_role",
                serde_json::json!(self.handoff_runtime_role),
            );
            repaired |= Self::set_field(
                object,
                "handoff_task_class",
                serde_json::json!(self.handoff_task_class),
            );
        }
        if !self.owned_paths.is_empty() {
            repaired |= crate::runtime_dispatch_state::apply_owned_paths(packet, &self.owned_paths);
        }
        if let Some(object) = packet.as_object_mut() {
            repaired |= Self::set_field(
                object,
                "implementation_isolation",
                self.implementation_isolation.clone(),
            );
        }
        if let Some(active_packet) = packet.get_mut(&packet_template_kind) {
            repaired |= self.apply_to_active_packet_body(active_packet);
        }
        repaired
    }
}

#[cfg(test)]
pub(crate) fn downstream_dispatch_packet_body(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: Option<&Path>,
) -> serde_json::Value {
    downstream_dispatch_packet_body_with_owned_paths(
        role_selection,
        run_graph_bootstrap,
        receipt,
        packet_path,
        &[],
    )
}

pub(crate) fn downstream_dispatch_packet_body_with_owned_paths(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: Option<&Path>,
    implementation_owned_paths_override: &[String],
) -> serde_json::Value {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let raw_downstream_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .unwrap_or_default();
    let contract = DownstreamDispatchPacketContract::for_dispatch_target(
        role_selection,
        receipt.dispatch_target.as_str(),
        raw_downstream_target,
        None,
        receipt.activation_agent_type.clone(),
        receipt.activation_runtime_role.clone(),
        implementation_owned_paths_override,
        &project_root,
    );
    let downstream_target = contract.dispatch_target.as_str();
    let downstream_contract_lookup_target = contract.lookup_target.as_str();
    let (downstream_dispatch_kind, _downstream_dispatch_surface, _, _) =
        if downstream_target.is_empty() {
            (
                receipt.dispatch_kind.clone(),
                receipt.dispatch_surface.clone(),
                contract.activation_agent_type.clone(),
                contract.activation_runtime_role.clone(),
            )
        } else {
            downstream_activation_fields(role_selection, downstream_target)
        };
    let handoff_runtime_role = contract.handoff_runtime_role.as_str();
    let packet_template_kind = if downstream_target.is_empty() {
        "delivery_task_packet".to_string()
    } else {
        crate::runtime_dispatch_state::runtime_dispatch_packet_kind(
            &role_selection.execution_plan,
            downstream_contract_lookup_target,
            &downstream_dispatch_kind,
        )
    };
    let activation_command = packet_path
        .and_then(|path| path.to_str())
        .map(crate::runtime_dispatch_state::agent_init_execute_command_for_packet_path);
    let handoff_task_class = contract.handoff_task_class.as_str();
    let closure_class = contract.closure_class.as_str();
    let selected_backend = crate::runtime_dispatch_state::downstream_selected_backend(
        role_selection,
        downstream_target,
        contract.activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    );
    let mut delivery_task_packet = runtime_delivery_task_packet_with_scope_context(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &role_selection.request,
        crate::runtime_dispatch_state::resolved_tracked_design_doc_path(role_selection).as_deref(),
    );
    contract.apply_to_active_packet_body(&mut delivery_task_packet);
    let execution_block_packet = runtime_execution_block_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    let host_runtime =
        crate::runtime_dispatch_state::runtime_host_execution_contract_for_root(&project_root);
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            downstream_target,
            selected_backend.as_deref(),
            contract.activation_agent_type.as_deref(),
            Some(&host_runtime),
            downstream_target.is_empty()
                && crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(receipt),
            None,
        );
    let execution_truth = crate::runtime_dispatch_state::dispatch_execution_route_summary(
        role_selection,
        downstream_target,
        selected_backend.as_deref(),
        None,
    );
    let activation_evidence = if downstream_target.is_empty() {
        crate::runtime_dispatch_state::dispatch_activation_evidence_summary(receipt)
    } else {
        neutral_downstream_activation_evidence()
    };
    let mut body = serde_json::Map::new();
    body.insert(
        "packet_kind".to_string(),
        serde_json::json!("runtime_downstream_dispatch_packet"),
    );
    body.insert(
        "packet_template_kind".to_string(),
        serde_json::json!(packet_template_kind),
    );
    body.insert(
        "delivery_task_packet".to_string(),
        if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "execution_block_packet".to_string(),
        if packet_template_kind == "execution_block_packet" {
            execution_block_packet
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "coach_review_packet".to_string(),
        if packet_template_kind == "coach_review_packet" {
            runtime_coach_review_packet(
                &receipt.run_id,
                downstream_target,
                Some(&receipt.dispatch_target),
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "verifier_proof_packet".to_string(),
        if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &receipt.run_id,
                downstream_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "escalation_packet".to_string(),
        if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
    );
    body.insert(
        "tracked_flow_packet".to_string(),
        if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(role_selection, &receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
    );
    let packet_for_request = serde_json::Value::Object(body.clone());
    let request_text = runtime_packet_request_text(&packet_template_kind, &packet_for_request)
        .unwrap_or_else(|| role_selection.request.trim().to_string());
    body.insert(
        "request_text".to_string(),
        serde_json::json!(request_text.clone()),
    );
    body.insert(
        "prompt".to_string(),
        serde_json::json!(runtime_packet_prompt(
            &receipt.run_id,
            downstream_target,
            handoff_runtime_role,
            &request_text,
            &role_selection.execution_plan["orchestration_contract"],
        )),
    );
    body.insert(
        "recorded_at".to_string(),
        serde_json::json!(receipt.recorded_at),
    );
    body.insert("run_id".to_string(), serde_json::json!(receipt.run_id));
    body.insert(
        "dispatch_target".to_string(),
        serde_json::json!(if downstream_target.is_empty() {
            receipt.dispatch_target.as_str()
        } else {
            downstream_target
        }),
    );
    body.insert(
        "source_dispatch_target".to_string(),
        serde_json::json!(receipt.dispatch_target),
    );
    body.insert(
        "source_dispatch_status".to_string(),
        serde_json::json!(receipt.dispatch_status),
    );
    body.insert(
        "source_lane_status".to_string(),
        serde_json::json!(receipt.lane_status),
    );
    body.insert(
        "source_supersedes_receipt_id".to_string(),
        serde_json::json!(receipt.supersedes_receipt_id),
    );
    body.insert(
        "source_exception_path_receipt_id".to_string(),
        serde_json::json!(receipt.exception_path_receipt_id),
    );
    body.insert(
        "source_blocker_code".to_string(),
        serde_json::json!(receipt.blocker_code),
    );
    body.insert(
        "downstream_dispatch_target".to_string(),
        serde_json::json!((!downstream_target.is_empty()).then(|| downstream_target.to_string())),
    );
    body.insert(
        "source_downstream_dispatch_target".to_string(),
        serde_json::json!(receipt.downstream_dispatch_target),
    );
    body.insert(
        "downstream_lane_id".to_string(),
        serde_json::json!(contract.downstream_lane_id),
    );
    body.insert(
        "downstream_dispatch_command".to_string(),
        serde_json::json!(
            activation_command.or_else(|| receipt.downstream_dispatch_command.clone())
        ),
    );
    body.insert(
        "downstream_dispatch_note".to_string(),
        serde_json::json!(receipt.downstream_dispatch_note),
    );
    body.insert(
        "downstream_dispatch_ready".to_string(),
        serde_json::json!(receipt.downstream_dispatch_ready),
    );
    body.insert(
        "downstream_dispatch_blockers".to_string(),
        serde_json::json!(receipt.downstream_dispatch_blockers),
    );
    body.insert(
        "downstream_dispatch_status".to_string(),
        serde_json::json!(receipt.downstream_dispatch_status),
    );
    body.insert(
        "downstream_lane_status".to_string(),
        serde_json::json!(receipt
            .downstream_dispatch_status
            .as_deref()
            .map(|status| { derive_lane_status(status, None, None).as_str().to_string() })),
    );
    body.insert(
        "downstream_supersedes_receipt_id".to_string(),
        serde_json::Value::Null,
    );
    body.insert(
        "downstream_exception_path_receipt_id".to_string(),
        serde_json::Value::Null,
    );
    body.insert(
        "downstream_dispatch_result_path".to_string(),
        serde_json::json!(receipt.downstream_dispatch_result_path),
    );
    body.insert(
        "downstream_dispatch_active_target".to_string(),
        serde_json::json!(receipt.downstream_dispatch_active_target),
    );
    body.insert(
        "activation_agent_type".to_string(),
        serde_json::json!(contract.activation_agent_type),
    );
    body.insert(
        "activation_runtime_role".to_string(),
        serde_json::json!(contract.activation_runtime_role),
    );
    body.insert(
        "handoff_runtime_role".to_string(),
        serde_json::json!(contract.handoff_runtime_role),
    );
    body.insert(
        "handoff_task_class".to_string(),
        serde_json::json!(contract.handoff_task_class),
    );
    body.insert(
        "selected_backend".to_string(),
        serde_json::json!(selected_backend),
    );
    body.insert(
        "effective_execution_posture".to_string(),
        effective_execution_posture.clone(),
    );
    body.insert("mixed_posture".to_string(), effective_execution_posture);
    body.insert("route_policy".to_string(), execution_truth.clone());
    body.insert(
        "activation_vs_execution_evidence".to_string(),
        activation_evidence.clone(),
    );
    body.insert(
        "activation_semantics".to_string(),
        activation_evidence["activation_semantics"].clone(),
    );
    body.insert(
        "execution_evidence".to_string(),
        activation_evidence["execution_evidence"].clone(),
    );
    body.insert("execution_truth".to_string(), execution_truth);
    body.insert("activation_evidence".to_string(), activation_evidence);
    body.insert("host_runtime".to_string(), host_runtime);
    body.insert(
        "role_selection_full".to_string(),
        serde_json::to_value(role_selection).expect("role selection should serialize"),
    );
    body.insert(
        "run_graph_bootstrap".to_string(),
        run_graph_bootstrap.clone(),
    );
    body.insert(
        "orchestration_contract".to_string(),
        role_selection.execution_plan["orchestration_contract"].clone(),
    );
    let mut packet = serde_json::Value::Object(body);
    contract.apply_to_packet(&mut packet);
    packet
}

fn runtime_assignment_activation_field(
    role_selection: &RuntimeConsumptionLaneSelection,
    field: &str,
) -> Option<String> {
    json_string(role_selection.execution_plan["runtime_assignment"].get(field)).or_else(|| {
        json_string(
            role_selection.execution_plan["runtime_assignment"]["role_selection"].get(field),
        )
    })
}

#[cfg(test)]
pub(crate) fn write_runtime_downstream_dispatch_packet_at(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    write_runtime_downstream_dispatch_packet_at_with_owned_paths(
        packet_path,
        role_selection,
        run_graph_bootstrap,
        receipt,
        &[],
    )
}

pub(crate) fn write_runtime_downstream_dispatch_packet_at_with_owned_paths(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    implementation_owned_paths_override: &[String],
) -> Result<(), String> {
    let body = downstream_dispatch_packet_body_with_owned_paths(
        role_selection,
        run_graph_bootstrap,
        receipt,
        Some(packet_path),
        implementation_owned_paths_override,
    );
    validate_runtime_dispatch_packet_contract(&body, "Runtime downstream dispatch packet")?;
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch packet: {error}"))?;
    std::fs::write(packet_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch packet: {error}"))?;
    Ok(())
}

pub(crate) fn write_runtime_downstream_dispatch_packet(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<String>, String> {
    write_runtime_downstream_dispatch_packet_with_owned_paths(
        state_root,
        role_selection,
        run_graph_bootstrap,
        receipt,
        &[],
    )
}

pub(crate) fn write_runtime_downstream_dispatch_packet_with_owned_paths(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    implementation_owned_paths_override: &[String],
) -> Result<Option<String>, String> {
    let Some(target) = receipt.downstream_dispatch_target.as_deref() else {
        return Ok(None);
    };
    let packet_dir = state_root
        .join("runtime-consumption")
        .join("downstream-dispatch-packets");
    std::fs::create_dir_all(&packet_dir).map_err(|error| {
        format!("Failed to create downstream-dispatch-packets directory: {error}")
    })?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let packet_path = packet_dir.join(format!("{}-{ts}.json", receipt.run_id));
    write_runtime_downstream_dispatch_packet_at_with_owned_paths(
        &packet_path,
        role_selection,
        run_graph_bootstrap,
        receipt,
        implementation_owned_paths_override,
    )
    .map_err(|error| {
        format!(
            "{error}; run_id `{}`; dispatch packet `{}`",
            receipt.run_id,
            packet_path.display()
        )
    })?;
    let _ = target;
    Ok(Some(packet_path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::downstream_dispatch_packet_body_with_owned_paths;
    use crate::RuntimeConsumptionLaneSelection;
    use std::path::PathBuf;

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn enter(path: &std::path::Path) -> Self {
            let previous = std::env::current_dir().expect("current dir should resolve");
            std::env::set_current_dir(path).expect("test current dir should switch");
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
        }
    }

    fn role_selection_with_empty_request() -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: String::new(),
            selected_role: "test_author".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "replanning": {
                        "checkpoints": ["after_review"]
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "coach": {
                                "packet_template_kind": "coach_review_packet",
                                "task_class": "coach",
                                "runtime_role": "coach",
                                "closure_class": "review"
                            }
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "vibe_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "coach": true
                        }
                    }
                ],
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "activation_runtime_role": "coach",
                    "selected_backend_id": "vibe_cli"
                }
            }),
            reason: "test".to_string(),
        }
    }

    fn receipt_with_coach_downstream() -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "feature-test".to_string(),
            dispatch_target: "test_author".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("dispatch.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after test author evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("test_author".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("vibe_cli".to_string()),
            recorded_at: "2026-06-07T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn coach_downstream_packet_synthesizes_non_empty_request_text_from_packet_body() {
        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection_with_empty_request(),
            &serde_json::json!({ "run_id": "feature-test" }),
            &receipt_with_coach_downstream(),
            None,
            &[],
        );

        let request_text = packet["request_text"]
            .as_str()
            .expect("downstream packet should expose request_text");
        let prompt = packet["prompt"]
            .as_str()
            .expect("downstream packet should expose prompt");

        assert!(!request_text.trim().is_empty());
        assert!(request_text.contains("review_goal:"));
        assert!(request_text.contains("blocking_question:"));
        assert!(prompt.contains("Request: review_goal:"));
        assert!(!prompt.trim_end().ends_with("Request:"));
    }

    #[test]
    fn downstream_packet_canonicalizes_lane_id_to_top_level_dispatch_target() {
        let mut role_selection = role_selection_with_empty_request();
        role_selection.execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"]
            ["coach_test_gate"] = serde_json::json!({
            "dispatch_target": "coach",
            "task_class": "coach",
            "runtime_role": "coach",
            "closure_class": "review",
            "packet_template_kind": "coach_review_packet"
        });
        let mut receipt = receipt_with_coach_downstream();
        receipt.downstream_dispatch_target = Some("coach_test_gate".to_string());

        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection,
            &serde_json::json!({ "run_id": "feature-test" }),
            &receipt,
            None,
            &[],
        );

        assert_eq!(packet["dispatch_target"], "coach");
        assert_eq!(packet["downstream_dispatch_target"], "coach");
        assert_eq!(
            packet["source_downstream_dispatch_target"],
            "coach_test_gate"
        );
        assert_eq!(packet["downstream_lane_id"], "coach_test_gate");
        assert_eq!(packet["packet_template_kind"], "coach_review_packet");
    }

    #[test]
    fn autotester_downstream_packet_uses_lane_assignment_and_test_write_scope() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-autotester-downstream-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(project_root.join("test")).expect("test dir should exist");
        let _cwd = CurrentDirGuard::enter(&project_root);
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Use meeting-specific event fields when scheduling Meeting activities"
                .to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {},
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "activation_runtime_role": "business_analyst",
                    "runtime_role": "business_analyst",
                    "task_class": "specification",
                    "selected_backend_id": "internal_subagents"
                },
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "planner_metadata": {
                            "owned_paths": [
                                "src/lib/features/list_view/presentation/stac/widgets/record_detail_view.dart"
                            ],
                            "proof_targets": [
                                "RecordScheduleActivityDialog widget tests cover switching to Meeting"
                            ]
                        }
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_sequence": ["designer", "autotester", "developer"],
                        "execution_lane_sequence": ["autotester", "developer"],
                        "lane_catalog": {
                            "autotester": {
                                "dispatch_target": "autotester",
                                "task_class": "implementation_medium",
                                "runtime_role": "worker",
                                "closure_class": "implementation",
                                "packet_template_kind": "delivery_task_packet"
                            }
                        }
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let mut receipt = receipt_with_coach_downstream();
        receipt.dispatch_target = "designer".to_string();
        receipt.downstream_dispatch_target = Some("autotester".to_string());
        receipt.activation_runtime_role = Some("designer".to_string());
        receipt.activation_agent_type = Some("middle".to_string());

        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection,
            &serde_json::json!({ "run_id": "activity-meeting-event-form-fields" }),
            &receipt,
            None,
            &[],
        );

        assert_eq!(packet["activation_runtime_role"], "worker");
        assert_eq!(packet["activation_agent_type"], "middle");
        assert_eq!(
            packet["delivery_task_packet"]["handoff_task_class"],
            "implementation_medium"
        );
        assert_eq!(
            packet["delivery_task_packet"]["handoff_runtime_role"],
            "worker"
        );
        assert_eq!(
            packet["delivery_task_packet"]["implementation_isolation"]
                ["canonical_worktree_writes_allowed"],
            false
        );
        let owned_paths = packet["delivery_task_packet"]["owned_paths"]
            .as_array()
            .expect("owned paths should render");
        assert!(
            owned_paths.iter().any(|path| path == "test"),
            "autotester owned scope should include a test write root: {owned_paths:?}"
        );
        assert!(
            owned_paths
                .iter()
                .any(|path| path == "src/lib/features/list_view/presentation/stac/widgets/record_detail_view.dart"),
            "autotester should keep production paths as read/write context when inherited from planner metadata"
        );

        let _ = std::fs::remove_dir_all(project_root);
    }
}
