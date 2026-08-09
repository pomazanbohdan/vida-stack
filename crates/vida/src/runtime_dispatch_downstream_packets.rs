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
use crate::runtime_proof_scope::proof_scope_from_planner_metadata_and_text;
use crate::{
    derive_lane_status, downstream_activation_fields, json_string,
    validate_runtime_dispatch_packet_contract, RuntimeConsumptionLaneSelection,
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
) -> Result<Option<String>, String> {
    if explicit_lane_id.is_some() || downstream_target.trim().is_empty() {
        return Ok(explicit_lane_id);
    }
    let sequence = crate::runtime_dispatch_state::typed_lane_node_sequence(role_selection, false)?;
    let authority =
        crate::runtime_dispatch_state::require_team_flow_authority_for_selection(role_selection)
            .map_err(|blocker| blocker.code)?;
    let current_node = authority
        .resolve_target(None, current_dispatch_target)
        .map_err(|blocker| blocker.code)?;
    let downstream_node = authority
        .resolve_target(None, downstream_target)
        .map_err(|blocker| blocker.code)?;
    let current_index = sequence
        .iter()
        .position(|node| node.node_id == current_node.node_id);
    let candidate_lanes = current_index
        .map(|index| sequence.iter().skip(index + 1).collect::<Vec<_>>())
        .unwrap_or_else(|| sequence.iter().collect::<Vec<_>>());
    Ok(candidate_lanes
        .into_iter()
        .find(|node| node.node_id == downstream_node.node_id)
        .map(|node| {
            (node.lane_id != node.dispatch_target && node.lane_id != downstream_target)
                .then(|| node.lane_id.clone())
        })
        .flatten())
}

fn exact_downstream_successor_selection(
    role_selection: &RuntimeConsumptionLaneSelection,
    current_dispatch_target: &str,
    downstream_dispatch_target: &str,
) -> Result<RuntimeConsumptionLaneSelection, String> {
    let current_node_id = crate::runtime_dispatch_state::selected_flow_node_ref(role_selection)
        .ok_or_else(|| "team_flow_authority_selected_node_id_missing".to_string())?;
    let authority =
        crate::runtime_dispatch_state::require_team_flow_authority_for_selection(role_selection)
            .map_err(|blocker| blocker.code)?;
    let current_node = authority
        .resolve_target(None, current_node_id)
        .map_err(|blocker| blocker.code)?;
    let current_matches_receipt = [
        current_node.node_id.as_str(),
        current_node.dispatch_target.as_str(),
        current_node.dispatch_alias.as_str(),
    ]
    .into_iter()
    .any(|candidate| candidate == current_dispatch_target.trim())
        || crate::runtime_dispatch_state::resolve_team_flow_target_for_selection(
            &authority,
            Some(&role_selection.execution_plan),
            current_dispatch_target,
        )
        .is_ok_and(|resolved| resolved.node_id == current_node.node_id);
    if !current_matches_receipt {
        return Err(format!(
            "team_flow_selected_node_dispatch_target_mismatch:{}:{}",
            current_node.node_id, current_dispatch_target
        ));
    }
    let successor_node_id = current_node
        .next_node
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("team_flow_exact_successor_missing:{}", current_node.node_id))?;
    let successor = authority
        .resolve_target(None, successor_node_id)
        .map_err(|blocker| blocker.code)?;
    let downstream_matches_successor = [
        successor.node_id.as_str(),
        successor.dispatch_target.as_str(),
        successor.dispatch_alias.as_str(),
    ]
    .into_iter()
    .any(|candidate| candidate == downstream_dispatch_target.trim());
    if !downstream_matches_successor {
        return Err(format!(
            "team_flow_downstream_target_successor_mismatch:{}:{}:{}",
            current_node.node_id, successor.node_id, downstream_dispatch_target
        ));
    }
    let mut progressed = role_selection.clone();
    let plan = progressed
        .execution_plan
        .as_object_mut()
        .ok_or_else(|| "team_flow_authority_execution_plan_missing".to_string())?;
    plan.insert(
        "team_flow_authority_selected_node_id".to_string(),
        serde_json::Value::String(successor.node_id.clone()),
    );
    let contract = plan
        .get_mut("development_flow")
        .and_then(|flow| flow.get_mut("dispatch_contract"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "team_flow_authority_dispatch_contract_missing".to_string())?;
    contract.insert(
        "team_flow_authority_selected_node_id".to_string(),
        serde_json::Value::String(successor.node_id.clone()),
    );
    contract.insert(
        "selected_node_id".to_string(),
        serde_json::Value::String(successor.node_id.clone()),
    );
    if let Some(selected_flow_contract) = plan
        .get_mut("selected_flow_contract")
        .and_then(serde_json::Value::as_object_mut)
    {
        if selected_flow_contract.contains_key("selected_node_id") {
            selected_flow_contract.insert(
                "selected_node_id".to_string(),
                serde_json::Value::String(successor.node_id),
            );
        }
    }
    Ok(progressed)
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
) -> Result<Option<String>, String> {
    let authority =
        crate::runtime_dispatch_state::require_team_flow_authority_for_selection(role_selection)
            .map_err(|blocker| blocker.code)?;
    let node = authority
        .resolve_target(None, dispatch_target)
        .map_err(|blocker| blocker.code)?;
    let value = node
        .activation
        .get(field)
        .or_else(|| node.assignment.get(field))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            (field == "activation_runtime_role")
                .then(|| node.runtime_role.trim().to_string())
                .filter(|value| !value.is_empty())
        });
    Ok(value)
}

pub(crate) fn configured_lane_runtime_role(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
) -> Result<Option<String>, String> {
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
    pub proof_artifact_paths: Vec<String>,
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
    ) -> Result<Self, String> {
        let authority = crate::runtime_dispatch_state::require_team_flow_authority_for_selection(
            role_selection,
        )
        .map_err(|blocker| blocker.code)?;
        let target_node = if let Some(selected_node_id) =
            crate::runtime_dispatch_state::selected_flow_node_ref(role_selection)
        {
            let selected = authority
                .resolve_target(None, selected_node_id)
                .map_err(|blocker| blocker.code)?;
            if ![
                selected.node_id.as_str(),
                selected.dispatch_target.as_str(),
                selected.dispatch_alias.as_str(),
            ]
            .into_iter()
            .any(|candidate| candidate == raw_dispatch_target.trim())
                && !crate::runtime_dispatch_state::resolve_team_flow_target_for_selection(
                    &authority,
                    Some(&role_selection.execution_plan),
                    raw_dispatch_target,
                )
                .is_ok_and(|resolved| resolved.node_id == selected.node_id)
            {
                return Err(format!(
                    "team_flow_selected_node_dispatch_target_mismatch:{}:{}",
                    selected.node_id, raw_dispatch_target
                ));
            }
            selected
        } else {
            authority
                .resolve_target(None, raw_dispatch_target)
                .map_err(|blocker| blocker.code)?
        };
        let dispatch_target = target_node.dispatch_target.clone();
        let resolved_lane_id =
            (target_node.lane_id != dispatch_target).then(|| target_node.lane_id.clone());
        let downstream_lane_id = infer_downstream_lane_id_for_dispatch_target(
            role_selection,
            current_dispatch_target,
            dispatch_target.as_str(),
            explicit_lane_id.or(resolved_lane_id),
        )?;
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
                )?
                .or_else(|| {
                    runtime_assignment_activation_field(role_selection, "activation_agent_type")
                });
            }
            if activation_runtime_role.is_none() {
                activation_runtime_role = configured_lane_runtime_role(
                    role_selection,
                    lookup_target.as_str(),
                )?
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
        let closure_class = if lookup_target.is_empty() {
            "implementation".to_string()
        } else {
            authority
                .resolve_target(None, lookup_target.as_str())
                .map_err(|blocker| blocker.code)?
                .closure_class
        };
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
        let proof_artifact_paths = proof_scope_from_planner_metadata_and_text(
            &role_selection.execution_plan,
            &role_selection.request,
        )
        .paths;
        let mut implementation_isolation =
            implementation_isolation_contract(handoff_task_class.as_str(), &owned_paths);
        if !proof_artifact_paths.is_empty() {
            if let Some(object) = implementation_isolation.as_object_mut() {
                object.insert(
                    "proof_artifact_paths".to_string(),
                    serde_json::json!(proof_artifact_paths.clone()),
                );
                object.insert(
                    "proof_artifact_scope".to_string(),
                    serde_json::json!(proof_artifact_paths.clone()),
                );
            }
        }
        Ok(Self {
            dispatch_target,
            downstream_lane_id,
            lookup_target,
            activation_agent_type,
            activation_runtime_role,
            handoff_runtime_role,
            handoff_task_class,
            closure_class,
            owned_paths,
            proof_artifact_paths,
            implementation_isolation,
        })
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
        if !self.proof_artifact_paths.is_empty() {
            repaired |= Self::set_field(
                object,
                "proof_artifact_paths",
                serde_json::json!(self.proof_artifact_paths.clone()),
            );
            repaired |= Self::set_field(
                object,
                "proof_artifact_scope",
                serde_json::json!(self.proof_artifact_paths.clone()),
            );
        }
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
            if !self.proof_artifact_paths.is_empty() {
                repaired |= Self::set_field(
                    object,
                    "proof_artifact_paths",
                    serde_json::json!(self.proof_artifact_paths.clone()),
                );
                repaired |= Self::set_field(
                    object,
                    "proof_artifact_scope",
                    serde_json::json!(self.proof_artifact_paths.clone()),
                );
            }
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
    match downstream_dispatch_packet_body_with_owned_paths_result(
        role_selection,
        run_graph_bootstrap,
        receipt,
        packet_path,
        implementation_owned_paths_override,
    ) {
        Ok(packet) => packet,
        Err(blocker_code) => serde_json::json!({
            "status": "blocked",
            "blocker_codes": [blocker_code],
            "dispatch_target": receipt.downstream_dispatch_target,
        }),
    }
}

fn downstream_dispatch_packet_body_with_owned_paths_result(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: Option<&Path>,
    implementation_owned_paths_override: &[String],
) -> Result<serde_json::Value, String> {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let raw_downstream_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .unwrap_or_default();
    let progressed_role_selection = exact_downstream_successor_selection(
        role_selection,
        receipt.dispatch_target.as_str(),
        raw_downstream_target,
    )?;
    let role_selection = &progressed_role_selection;
    let contract = DownstreamDispatchPacketContract::for_dispatch_target(
        role_selection,
        receipt.dispatch_target.as_str(),
        raw_downstream_target,
        None,
        receipt.activation_agent_type.clone(),
        receipt.activation_runtime_role.clone(),
        implementation_owned_paths_override,
        &project_root,
    )?;
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
        crate::runtime_dispatch_state::runtime_dispatch_packet_kind_for_role_selection(
            role_selection,
            downstream_contract_lookup_target,
            &downstream_dispatch_kind,
        )?
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
    let receipt_has_execution_evidence =
        crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(receipt);
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            downstream_target,
            selected_backend.as_deref(),
            contract.activation_agent_type.as_deref(),
            Some(&host_runtime),
            receipt_has_execution_evidence,
            None,
        );
    let execution_truth = crate::runtime_dispatch_state::dispatch_execution_route_summary(
        role_selection,
        downstream_target,
        selected_backend.as_deref(),
        None,
    );
    let activation_evidence = if receipt_has_execution_evidence {
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
    let persisted_role_selection =
        crate::runtime_dispatch_state::persisted_role_selection_projection(role_selection)?;
    body.insert("role_selection_full".to_string(), persisted_role_selection);
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
    Ok(packet)
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
    let mut body = downstream_dispatch_packet_body_with_owned_paths_result(
        role_selection,
        run_graph_bootstrap,
        receipt,
        Some(packet_path),
        implementation_owned_paths_override,
    )?;
    crate::runtime_dispatch_state::normalize_persisted_dispatch_packet_role_selection(&mut body)?;
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
    use super::{
        downstream_dispatch_packet_body_with_owned_paths,
        downstream_dispatch_packet_body_with_owned_paths_result,
        exact_downstream_successor_selection, DownstreamDispatchPacketContract,
    };
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

    fn select_current_node(
        selection: &mut RuntimeConsumptionLaneSelection,
        current_target: &str,
    ) -> String {
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &selection.compiled_bundle,
            None,
            None,
        )
        .expect("test TeamFlow authority should compile");
        let node = authority
            .resolve_target(None, current_target)
            .expect("test current target should resolve");
        let flow_id = authority.projection().snapshot.flow_ref.clone();
        let node_id = node.node_id.clone();
        let authority_id = authority.projection().authority_id.clone();
        let config_hash = authority.projection().config_authority_hash.clone();
        let registry_hash = authority.projection().registry_authority_hash.clone();
        selection.execution_plan["team_flow_authority_selected_flow_id"] =
            serde_json::json!(flow_id);
        selection.execution_plan["team_flow_authority_selected_node_id"] =
            serde_json::json!(node_id);
        let contract = &mut selection.execution_plan["development_flow"]["dispatch_contract"];
        contract["selected_flow_set"] = serde_json::json!(flow_id);
        contract["selected_node_id"] = serde_json::json!(node_id);
        contract["team_flow_authority_selected_node_id"] = serde_json::json!(node_id);
        contract["team_flow_authority_id"] = serde_json::json!(authority_id);
        contract["team_flow_config_hash"] = serde_json::json!(config_hash);
        contract["team_flow_registry_hash"] = serde_json::json!(registry_hash);
        node_id
    }

    fn role_selection_with_empty_request() -> RuntimeConsumptionLaneSelection {
        let mut selection = RuntimeConsumptionLaneSelection {
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
            compiled_bundle:
                crate::runtime_dispatch_state::repository_team_flow_test_bundle_for_flow(Some(
                    "default_delivery",
                )),
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "replanning": {
                        "checkpoints": ["after_review"]
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "coach_test_gate": {
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
        };
        select_current_node(&mut selection, "test_author");
        selection
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
            downstream_dispatch_target: Some("coach_test_gate".to_string()),
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
            policy_bundle_ref: None,
            recorded_at: "2026-06-07T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn selected_non_default_flow_replays_canonical_node_and_dispatch_target_through_alias() {
        let mut role_selection = role_selection_with_empty_request();
        let (flow_id, current_node_id) =
            crate::runtime_dispatch_state::select_repository_non_default_flow(&mut role_selection)
                .expect("repository should expose an enabled non-default flow");
        let authority = crate::runtime_dispatch_state::require_team_flow_authority_for_selection(
            &role_selection,
        )
        .expect("selected flow authority should compile");
        let current = authority
            .resolve_target(None, &current_node_id)
            .expect("selected node should resolve");
        let successor_id = current
            .next_node
            .as_deref()
            .expect("selected flow fixture should expose a successor");
        let successor = authority
            .resolve_target(None, successor_id)
            .expect("successor should resolve");
        let current_replay = if current.dispatch_alias.trim().is_empty() {
            current.dispatch_target.clone()
        } else {
            current.dispatch_alias.clone()
        };
        let successor_replay = if successor.dispatch_alias.trim().is_empty() {
            successor.dispatch_target.clone()
        } else {
            successor.dispatch_alias.clone()
        };
        let progressed = exact_downstream_successor_selection(
            &role_selection,
            &current_replay,
            &successor_replay,
        )
        .expect("alias replay should follow the selected flow successor");
        assert_eq!(
            crate::runtime_dispatch_state::selected_flow_ref(&progressed),
            Some(flow_id.as_str())
        );
        let progressed_node_id = crate::runtime_dispatch_state::selected_flow_node_ref(&progressed)
            .expect("successor node id should remain persisted");
        assert_eq!(progressed_node_id, successor.node_id);
        let canonical_successor =
            crate::runtime_dispatch_state::require_team_flow_authority_for_selection(&progressed)
                .expect("replayed selected flow authority should compile")
                .resolve_target(None, progressed_node_id)
                .expect("canonical successor node should resolve");
        assert_eq!(canonical_successor.node_id, successor.node_id);
        assert_eq!(
            canonical_successor.dispatch_target,
            successor.dispatch_target
        );

        let mut unknown = role_selection.clone();
        unknown.execution_plan["team_flow_authority_selected_node_id"] =
            serde_json::json!("unknown-persisted-node");
        let unknown_node = crate::runtime_dispatch_state::selected_flow_node_ref(&unknown)
            .expect("unknown node id should remain observable");
        assert!(
            crate::runtime_dispatch_state::require_team_flow_authority_for_selection(&unknown)
                .expect("selected flow authority should still compile")
                .resolve_target(None, unknown_node)
                .is_err(),
            "unknown persisted node must fail closed"
        );

        let contract = DownstreamDispatchPacketContract::for_dispatch_target(
            &progressed,
            current.dispatch_target.as_str(),
            successor_replay.as_str(),
            None,
            None,
            None,
            &[],
            std::path::Path::new("."),
        )
        .expect("downstream contract should canonicalize replayed alias");
        assert_eq!(contract.dispatch_target, successor.dispatch_target);
    }

    #[test]
    fn coach_downstream_packet_synthesizes_non_empty_request_text_from_packet_body() {
        let mut role_selection = role_selection_with_empty_request();
        role_selection.execution_plan["team_flow_authority_selected_flow_id"] =
            serde_json::json!("default_delivery");
        role_selection.execution_plan["development_flow"]["dispatch_contract"] = serde_json::json!({
            "selected_flow_set": "default_delivery",
            "team_flow_authority_id": "authority-test",
            "team_flow_config_hash": "config-hash-test",
            "team_flow_registry_hash": "registry-hash-test"
        });
        let current_node_id =
            role_selection.execution_plan["team_flow_authority_selected_node_id"].clone();
        role_selection.execution_plan["development_flow"]["dispatch_contract"]
            ["selected_node_id"] = current_node_id.clone();
        role_selection.execution_plan["development_flow"]["dispatch_contract"]
            ["team_flow_authority_selected_node_id"] = current_node_id;
        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection,
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
        let persisted_selection = &packet["role_selection_full"];
        assert!(persisted_selection["compiled_bundle"].is_null());
        assert_eq!(
            persisted_selection["execution_plan"]["team_flow_authority_selected_flow_id"],
            "default_delivery"
        );
        assert_eq!(
            persisted_selection["execution_plan"]["development_flow"]["dispatch_contract"]
                ["team_flow_authority_id"],
            "authority-test"
        );
        assert_eq!(
            persisted_selection["execution_plan"]["development_flow"]["dispatch_contract"]
                ["team_flow_config_hash"],
            "config-hash-test"
        );
        assert_eq!(
            persisted_selection["execution_plan"]["development_flow"]["dispatch_contract"]
                ["team_flow_registry_hash"],
            "registry-hash-test"
        );
        assert!(
            serde_json::to_vec(&packet)
                .expect("downstream packet should serialize")
                .len()
                < 1024 * 1024
        );
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
        let expected_target =
            crate::team_flow_authority_adapter::require_team_flow_execution_authority(
                &role_selection.compiled_bundle,
                Some("default_delivery"),
                None,
            )
            .expect("default delivery authority should compile")
            .resolve_target(None, "coach_test_gate")
            .expect("coach test gate should resolve")
            .dispatch_target;

        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection,
            &serde_json::json!({ "run_id": "feature-test" }),
            &receipt,
            None,
            &[],
        );

        assert_eq!(packet["dispatch_target"], expected_target);
        assert_eq!(packet["downstream_dispatch_target"], expected_target);
        assert_eq!(
            packet["source_downstream_dispatch_target"],
            "coach_test_gate"
        );
        assert_eq!(packet["downstream_lane_id"], "coach_test_gate");
        assert_eq!(packet["packet_template_kind"], "coach_review_packet");
    }

    #[test]
    fn receipt_backed_execution_evidence_propagates_to_non_empty_downstream_packet() {
        let result_path = std::env::temp_dir().join(format!(
            "vida-downstream-host-bridge-result-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "pass",
                "execution_state": "executed",
                "run_id": "feature-test",
                "dispatch_target": "test_author",
                "completed_target": "test_author",
                "execution_evidence": {
                    "status": "recorded",
                    "receipt_backed": true,
                    "receipt_id": "receipt-feature-test"
                },
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                }
            })
            .to_string(),
        )
        .expect("host bridge result should write");
        let mut receipt = receipt_with_coach_downstream();
        receipt.dispatch_result_path = Some(result_path.display().to_string());

        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection_with_empty_request(),
            &serde_json::json!({ "run_id": "feature-test" }),
            &receipt,
            None,
            &[],
        );

        assert_eq!(
            packet["activation_vs_execution_evidence"]["evidence_state"],
            "execution_evidence_recorded"
        );
        assert_eq!(
            packet["activation_vs_execution_evidence"]["receipt_backed"],
            true
        );
        assert_eq!(
            packet["execution_evidence"]["receipt_id"],
            "receipt-feature-test"
        );

        let _ = std::fs::remove_file(result_path);
    }

    #[test]
    fn downstream_packet_keeps_activation_view_only_without_receipt_evidence() {
        let packet = downstream_dispatch_packet_body_with_owned_paths(
            &role_selection_with_empty_request(),
            &serde_json::json!({ "run_id": "feature-test" }),
            &receipt_with_coach_downstream(),
            None,
            &[],
        );

        assert_eq!(
            packet["activation_vs_execution_evidence"]["evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            packet["activation_vs_execution_evidence"]["receipt_backed"],
            false
        );
        assert!(packet["execution_evidence"].is_null());
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
        let mut role_selection = RuntimeConsumptionLaneSelection {
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
            compiled_bundle:
                crate::runtime_dispatch_state::repository_team_flow_test_bundle_for_flow(Some(
                    "default_delivery",
                )),
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
                                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart covers switching to Meeting"
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
        let (current_target, current_runtime_role, downstream_target, downstream_runtime_role) = {
            let authority =
                crate::team_flow_authority_adapter::require_team_flow_execution_authority(
                    &role_selection.compiled_bundle,
                    None,
                    None,
                )
                .expect("test TeamFlow authority should compile");
            let nodes = authority.ordered_nodes().collect::<Vec<_>>();
            let (current, downstream) = nodes
                .windows(2)
                .find_map(|pair| {
                    (pair[1].node.task_class == "test_authoring").then_some((&pair[0], &pair[1]))
                })
                .expect("fixture should expose a test-authoring successor");
            (
                current.node.node_id.clone(),
                current.node.runtime_role.clone(),
                downstream.node.node_id.clone(),
                downstream.node.runtime_role.clone(),
            )
        };
        select_current_node(&mut role_selection, &current_target);
        role_selection.execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"]
            [&downstream_target] = serde_json::json!({
            "dispatch_target": downstream_target.clone(),
            "task_class": "test_authoring",
            "runtime_role": downstream_runtime_role,
            "closure_class": "implementation",
            "packet_template_kind": "delivery_task_packet"
        });
        let mut receipt = receipt_with_coach_downstream();
        receipt.dispatch_target = current_target;
        receipt.downstream_dispatch_target = Some(downstream_target.clone());
        receipt.activation_runtime_role = Some(current_runtime_role);
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
            "test_authoring"
        );
        assert_eq!(
            packet["delivery_task_packet"]["handoff_runtime_role"],
            "worker"
        );
        let owned_paths = packet["delivery_task_packet"]["owned_paths"]
            .as_array()
            .expect("owned paths should render");
        assert!(
            owned_paths.iter().any(|path| path == "test"),
            "test-author owned scope should include a test write root: {owned_paths:?}"
        );
        assert!(
            owned_paths.iter().any(|path| path
                == "src/lib/features/list_view/presentation/stac/widgets/record_detail_view.dart"),
            "autotester should keep production paths as read/write context when inherited from planner metadata"
        );
        assert_eq!(
            packet["delivery_task_packet"]["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );
        assert_eq!(
            packet["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );

        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn ambiguous_alias_resolves_only_through_exact_successor() {
        let mut role_selection = role_selection_with_empty_request();
        select_current_node(&mut role_selection, "duplication_reviewer");
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &role_selection.compiled_bundle,
            Some("default_delivery"),
            None,
        )
        .expect("default delivery authority should compile");
        let successor = authority
            .resolve_target(None, "tester")
            .expect("exact successor should resolve");
        assert!(
            authority
                .resolve_target(None, successor.dispatch_alias.as_str())
                .is_err(),
            "successor alias must remain ambiguous in this fixture"
        );
        let mut receipt = receipt_with_coach_downstream();
        receipt.dispatch_target = "duplication_reviewer".to_string();
        receipt.downstream_dispatch_target = Some(successor.dispatch_alias.clone());

        let packet = downstream_dispatch_packet_body_with_owned_paths_result(
            &role_selection,
            &serde_json::json!({ "run_id": "ambiguous-successor" }),
            &receipt,
            None,
            &[],
        )
        .expect("ambiguous alias should resolve through current exact successor");

        assert_eq!(
            packet["role_selection_full"]["execution_plan"]["team_flow_authority_selected_node_id"],
            successor.node_id
        );
    }

    #[test]
    fn unknown_downstream_target_fails_closed_before_packet_write() {
        let role_selection = role_selection_with_empty_request();
        let mut receipt = receipt_with_coach_downstream();
        receipt.downstream_dispatch_target = Some("tampered_missing_target".to_string());

        let blocker = downstream_dispatch_packet_body_with_owned_paths_result(
            &role_selection,
            &serde_json::json!({ "run_id": "tamper" }),
            &receipt,
            None,
            &[],
        )
        .expect_err("unknown downstream target must fail closed");

        assert_eq!(
            blocker,
            "team_flow_downstream_target_successor_mismatch:test_author:coach_test_gate:tampered_missing_target"
        );
    }
}
