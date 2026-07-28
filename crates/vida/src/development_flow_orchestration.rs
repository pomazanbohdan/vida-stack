use crate::runtime_contract_vocab::{
    TASK_CLASS_ARCHITECTURE, TASK_CLASS_COACH, TASK_CLASS_IMPLEMENTATION, TASK_CLASS_SPECIFICATION,
    TASK_CLASS_VERIFICATION,
};
pub(crate) use crate::runtime_lane_summary::{
    RuntimeConsumptionLaneSelection, build_runtime_lane_selection_with_store,
};

fn canonicalize_moved_test_request(request: &str) -> String {
    const MOVED_TEST_MOVE_PREFIX: &str = "move ";
    const MOVED_TEST_MOVE_SUFFIX: &str =
        " from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs";
    const BARE_PROOF_TARGET_PREFIX: &str = "cargo test -p vida ";
    const BARE_PROOF_TARGET_SUFFIX: &str = " -- --nocapture";
    const MODULE_QUALIFIED_PREFIX: &str = "project_activator_surface::tests::";

    let Some(move_start) = request.find(MOVED_TEST_MOVE_PREFIX) else {
        return request.to_string();
    };
    let move_start = move_start + MOVED_TEST_MOVE_PREFIX.len();
    let Some(move_end) = request[move_start..].find(MOVED_TEST_MOVE_SUFFIX) else {
        return request.to_string();
    };
    let move_end = move_start + move_end;
    let test_name = request[move_start..move_end].trim();
    if test_name.is_empty() {
        return request.to_string();
    }

    let bare_proof_target =
        format!("{BARE_PROOF_TARGET_PREFIX}{test_name}{BARE_PROOF_TARGET_SUFFIX}");
    let canonical_proof_target = format!(
        "{BARE_PROOF_TARGET_PREFIX}{MODULE_QUALIFIED_PREFIX}{test_name} -- --exact --nocapture"
    );
    if request.contains(&bare_proof_target) {
        request.replace(&bare_proof_target, &canonical_proof_target)
    } else {
        request.to_string()
    }
}

pub(crate) fn build_design_first_tracked_flow_bootstrap(request: &str) -> serde_json::Value {
    // This helper is retained as a schema-level diagnostic for older callers.
    // Executable task ids, commands, and graph edges must come from the selected
    // TeamFlow/dev_team relation; request text alone is not an authority source.
    let canonical_request = canonicalize_moved_test_request(request);
    serde_json::json!({
        "required": true,
        "status": "blocked",
        "executable": false,
        "view_only": true,
        "activation_semantics": "configured_team_flow_relation_required",
        "blocker_codes": ["team_flow_authority_tracked_flow_relation_missing"],
        "request": canonical_request,
        "schema_vocabulary": ["epic", "spec_task", "work_pool_task", "dev_task"],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrePlanTeamFlowSelectionMode {
    Fresh,
    Persisted,
}

/// Resolve the TeamFlow authority before any derived routing or plan rebuild.
fn pre_plan_team_flow_authority(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    mode: PrePlanTeamFlowSelectionMode,
) -> Result<crate::team_flow_authority_adapter::TeamFlowExecutionAuthority, String> {
    let mut authority_selection = selection.clone();
    authority_selection.compiled_bundle = compiled_bundle.clone();
    match mode {
        PrePlanTeamFlowSelectionMode::Fresh => {
            crate::runtime_dispatch_state::require_team_flow_authority_for_selection(
                &authority_selection,
            )
        }
        PrePlanTeamFlowSelectionMode::Persisted => {
            crate::runtime_dispatch_state::require_persisted_team_flow_authority_for_selection(
                &authority_selection,
            )
        }
    }
    .map_err(|error| error.to_string())
}

fn task_class_for_selection(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> Result<String, String> {
    task_class_for_selection_with_mode(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Fresh,
    )
}

fn task_class_for_selection_with_mode(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    mode: PrePlanTeamFlowSelectionMode,
) -> Result<String, String> {
    let authority = pre_plan_team_flow_authority(compiled_bundle, selection, mode)?;
    let flow_ref = authority.snapshot.flow_ref.as_str();
    let selected_node_id = crate::runtime_dispatch_state::selected_flow_node_ref_for_mode(
        selection,
        &authority,
        mode == PrePlanTeamFlowSelectionMode::Fresh,
    )?;
    let selected_node = authority
        .resolve_target(None, &selected_node_id)
        .map_err(|error| error.to_string())?;
    if !selected_node.included {
        return Err(format!(
            "team_flow_authority_selected_node_id_excluded:{flow_ref}:{selected_node_id}"
        ));
    }

    let has_authoritative_plan_flow = mode == PrePlanTeamFlowSelectionMode::Persisted
        && selection.execution_plan["team_flow_authority_selected_flow_id"]
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
    if has_authoritative_plan_flow {
        if let Some(task_class) = crate::json_string(
            crate::runtime_assignment_from_execution_plan(&selection.execution_plan)
                .get("task_class"),
        ) {
            return Ok(task_class);
        }
    }

    let task_class = selected_node.task_class.trim();
    if task_class.is_empty() {
        return Err(format!(
            "team_flow_authority_selected_node_task_class_missing:{flow_ref}:{}",
            selected_node.node_id
        ));
    }
    Ok(task_class.to_string())
}

fn request_requires_execution_preparation(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    authority: &crate::team_flow_authority_adapter::TeamFlowExecutionAuthority,
    selected_node_id: &str,
) -> Result<bool, String> {
    let flow_id = authority.snapshot.flow_ref.as_str();
    let selected_flow = compiled_bundle["all_project_flow_catalog"]
        .get(flow_id)
        .or_else(|| compiled_bundle["project_flow_catalog"].get(flow_id));
    if let Some(policy) = selected_flow.and_then(|flow| flow.get("execution_preparation_policy")) {
        let mode = policy["mode"].as_str().unwrap_or_default();
        let gated_task_classes = policy["task_classes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        let selected_node = authority
            .resolve_target(None, selected_node_id)
            .map_err(|error| error.to_string())?;
        if !selected_node.included {
            return Err(format!(
                "team_flow_authority_selected_node_id_excluded:{flow_id}:{selected_node_id}"
            ));
        }
        let task_class = selected_node.task_class.trim().to_string();
        if task_class.is_empty() {
            return Err(format!(
                "team_flow_authority_selected_node_task_class_missing:{flow_id}:{}",
                selected_node.node_id
            ));
        }
        let validation_gate = if crate::json_bool(policy.get("honor_validation_gate"), false) {
            crate::json_bool(
                compiled_bundle["autonomous_execution"]
                    .get("validation_report_required_before_implementation"),
                false,
            )
        } else {
            false
        };
        match mode {
            "always" => return Ok(true),
            "never" => return Ok(false),
            "required_for_task_classes" => {
                return Ok(gated_task_classes.contains(&task_class.as_str()));
            }
            "required_for_code_shaped_work" => {
                if gated_task_classes.contains(&task_class.as_str()) {
                    return Ok(validation_gate || task_class == TASK_CLASS_IMPLEMENTATION);
                }
                return Ok(false);
            }
            _ => {}
        }
    }
    let normalized_request = selection.request.to_lowercase();
    let architecture_signals = crate::contains_keywords(
        &normalized_request,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "migration".to_string(),
            "refactor".to_string(),
            "topology".to_string(),
            "boundary".to_string(),
            "cross-scope".to_string(),
            "cross scope".to_string(),
        ],
    );
    let write_signals = crate::contains_keywords(
        &normalized_request,
        &[
            "implement".to_string(),
            "implementation".to_string(),
            "write code".to_string(),
            "write the code".to_string(),
            "patch".to_string(),
            "refactor".to_string(),
            "build".to_string(),
        ],
    );
    let selected_node = authority
        .resolve_target(None, selected_node_id)
        .map_err(|error| error.to_string())?;
    if !selected_node.included {
        return Err(format!(
            "team_flow_authority_selected_node_id_excluded:{flow_id}:{selected_node_id}"
        ));
    }
    let task_class = selected_node.task_class.trim().to_string();
    if task_class.is_empty() {
        return Err(format!(
            "team_flow_authority_selected_node_task_class_missing:{flow_id}:{selected_node_id}"
        ));
    }
    let validation_gate = crate::json_bool(
        compiled_bundle["autonomous_execution"]
            .get("validation_report_required_before_implementation"),
        false,
    );
    Ok(task_class == TASK_CLASS_IMPLEMENTATION
        && (validation_gate || !architecture_signals.is_empty() || !write_signals.is_empty()))
}

#[derive(Debug)]
struct ConfiguredDevelopmentFlow {
    flow_id: String,
    selected_node_id: String,
    lanes: Vec<crate::team_flow_authority_adapter::TeamFlowNodeResolution>,
    authority_id: String,
    config_authority_hash: String,
    registry_authority_hash: String,
}

fn team_flow_plan_identity_fields(
    authority: Option<&crate::team_flow_authority_adapter::TeamFlowExecutionAuthority>,
) -> serde_json::Value {
    let Some(authority) = authority else {
        return serde_json::json!({
            "team_flow_authority_id": serde_json::Value::Null,
            "team_flow_config_hash": serde_json::Value::Null,
            "team_flow_registry_hash": serde_json::Value::Null,
        });
    };
    let projection = authority.projection();
    serde_json::json!({
        "team_flow_authority_id": projection.authority_id.clone(),
        "team_flow_config_hash": projection.config_authority_hash.clone(),
        "team_flow_registry_hash": projection.registry_authority_hash.clone(),
    })
}

fn resolved_development_flow_templates(
    authority: &crate::team_flow_authority_adapter::TeamFlowExecutionAuthority,
    selected_node_id: Option<&str>,
) -> Result<ConfiguredDevelopmentFlow, Vec<String>> {
    configured_dev_team_flow_templates_from_authority(authority, selected_node_id)
        .map_err(|error| vec![error])
}

fn configured_dev_team_flow_templates(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> Result<ConfiguredDevelopmentFlow, String> {
    configured_dev_team_flow_templates_with_mode(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Fresh,
    )
}

fn configured_dev_team_flow_templates_with_mode(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    mode: PrePlanTeamFlowSelectionMode,
) -> Result<ConfiguredDevelopmentFlow, String> {
    let execution_authority = pre_plan_team_flow_authority(compiled_bundle, selection, mode)?;
    let selected_node_id = match mode {
        PrePlanTeamFlowSelectionMode::Fresh => Some(execution_authority.entry_node_id.clone()),
        PrePlanTeamFlowSelectionMode::Persisted => {
            crate::runtime_dispatch_state::validated_selected_flow_node_ref(selection)
                .map_err(|error| error.to_string())?
        }
    };
    configured_dev_team_flow_templates_from_authority(
        &execution_authority,
        selected_node_id.as_deref(),
    )
}

fn configured_dev_team_flow_templates_from_authority(
    execution_authority: &crate::team_flow_authority_adapter::TeamFlowExecutionAuthority,
    selected_node_id: Option<&str>,
) -> Result<ConfiguredDevelopmentFlow, String> {
    let projection = execution_authority.projection();
    let flow_id = projection.snapshot.flow_ref.clone();
    let authority_id = projection.authority_id.clone();
    let config_authority_hash = projection.config_authority_hash.clone();
    let registry_authority_hash = projection.registry_authority_hash.clone();
    let lanes = execution_authority
        .ordered_nodes()
        .filter(|node| node.node.included)
        .map(|node| {
            execution_authority
                .resolve_target(None, &node.node.node_id)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected_node_id = selected_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("team_flow_authority_selected_node_id_missing:{flow_id}"))?;
    let selected_node = execution_authority
        .resolve_target(None, selected_node_id)
        .map_err(|error| error.to_string())?;
    if !selected_node.included {
        return Err(format!(
            "team_flow_authority_selected_node_id_excluded:{flow_id}:{selected_node_id}"
        ));
    }
    Ok(ConfiguredDevelopmentFlow {
        flow_id,
        selected_node_id: selected_node_id.to_string(),
        lanes,
        authority_id,
        config_authority_hash,
        registry_authority_hash,
    })
}

pub(crate) fn normalize_selected_flow_for_execution_plan(
    selection: &mut crate::RuntimeConsumptionLaneSelection,
    compiled_bundle: &serde_json::Value,
    selected_flow_id: &str,
) -> Result<(), String> {
    let selected_node_id =
        crate::runtime_dispatch_state::validated_selected_flow_node_ref(selection)?;
    normalize_selected_flow_for_execution_plan_with_selected_node(
        selection,
        compiled_bundle,
        selected_flow_id,
        selected_node_id.as_deref(),
    )
}

pub(crate) fn normalize_selected_or_default_flow_for_execution_plan(
    selection: &mut crate::RuntimeConsumptionLaneSelection,
    compiled_bundle: &serde_json::Value,
) -> Result<(), String> {
    let authority = pre_plan_team_flow_authority(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Persisted,
    )?;
    let selected_flow_id = authority.snapshot.flow_ref.clone();
    let selected_node_id = crate::runtime_dispatch_state::selected_flow_node_ref(selection)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    normalize_selected_flow_for_execution_plan_with_selected_node(
        selection,
        compiled_bundle,
        &selected_flow_id,
        selected_node_id.as_deref(),
    )
}

/// Fresh consume-final selection has not crossed a persisted authority boundary.
/// Only a structured selector present in both the request and matched routing evidence is
/// authoritative; derived terms and compatibility plan copies are ignored by themselves.
pub(crate) fn normalize_fresh_selected_or_default_flow_for_execution_plan(
    selection: &mut crate::RuntimeConsumptionLaneSelection,
    compiled_bundle: &serde_json::Value,
) -> Result<(), String> {
    let authority = pre_plan_team_flow_authority(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Fresh,
    )?;
    let selected_flow_id = authority.snapshot.flow_ref.clone();
    let selected_node_id = Some(authority.entry_node_id.clone());
    normalize_selected_flow_for_execution_plan_with_selected_node(
        selection,
        compiled_bundle,
        &selected_flow_id,
        selected_node_id.as_deref(),
    )
}

pub(crate) fn normalize_selected_flow_for_execution_plan_with_selected_node(
    selection: &mut crate::RuntimeConsumptionLaneSelection,
    compiled_bundle: &serde_json::Value,
    selected_flow_id: &str,
    persisted_selected_node_id: Option<&str>,
) -> Result<(), String> {
    let selected_flow_id = selected_flow_id.trim();
    if selected_flow_id.is_empty() {
        return Err("team_flow_authority_selected_flow_missing".to_string());
    }
    let authority = crate::team_flow_authority_adapter::TeamFlowExecutionAuthority::require(
        compiled_bundle,
        Some(selected_flow_id),
        None,
    )
    .map_err(|error| error.to_string())?;
    let selected_node = if let Some(selected_node_id) = persisted_selected_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let node = authority
            .projection()
            .node(selected_node_id)
            .ok_or_else(|| {
                format!(
                    "team_flow_authority_selected_node_id_unknown:{selected_flow_id}:{selected_node_id}"
                )
            })?;
        if !node.node.included {
            return Err(format!(
                "team_flow_authority_selected_node_id_excluded:{selected_flow_id}:{selected_node_id}"
            ));
        }
        node.clone()
    } else {
        return Err(format!(
            "team_flow_authority_selected_node_id_missing:{selected_flow_id}"
        ));
    };
    let runtime_role = selected_node.node.runtime_role.trim();
    let selected_node_id = selected_node.node.node_id.trim().to_string();
    if selected_node_id.is_empty() {
        return Err(format!(
            "team_flow_authority_selected_flow_node_id_missing:{selected_flow_id}"
        ));
    }
    if runtime_role.is_empty() {
        return Err(format!(
            "team_flow_authority_selected_flow_runtime_role_missing:{selected_flow_id}:{}",
            selected_node.node.node_id
        ));
    }
    let explicit_task_id = selection
        .execution_plan
        .get("runtime_consumption_explicit_task_id")
        .cloned()
        .filter(|value| {
            value
                .as_str()
                .is_some_and(|task_id| !task_id.trim().is_empty())
        });
    selection.compiled_bundle = compiled_bundle.clone();
    selection.selected_role = runtime_role.to_string();
    selection
        .matched_terms
        .retain(|term| !term.starts_with("dev_team_flow_id:"));
    selection
        .matched_terms
        .push(format!("dev_team_flow_id:{selected_flow_id}"));
    selection.execution_plan = serde_json::json!({
        "team_flow_authority_selected_flow_id": selected_flow_id,
        "team_flow_authority_selected_node_id": selected_node_id,
    });
    selection.execution_plan = build_runtime_execution_plan_from_snapshot_with_mode(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Persisted,
    );
    let plan = selection
        .execution_plan
        .as_object_mut()
        .ok_or_else(|| "team_flow_authority_execution_plan_missing".to_string())?;
    if let Some(task_id) = explicit_task_id {
        plan.insert("runtime_consumption_explicit_task_id".to_string(), task_id);
    }
    plan.insert(
        "team_flow_authority_selected_flow_id".to_string(),
        serde_json::Value::String(selected_flow_id.to_string()),
    );
    plan.insert(
        "team_flow_authority_selected_node_id".to_string(),
        serde_json::Value::String(selected_node_id.clone()),
    );
    if let Some(contract) = plan
        .get_mut("development_flow")
        .and_then(|flow| flow.get_mut("dispatch_contract"))
        .and_then(serde_json::Value::as_object_mut)
    {
        contract.insert(
            "team_flow_authority_selected_node_id".to_string(),
            serde_json::Value::String(selected_node_id.clone()),
        );
        contract.insert(
            "selected_node_id".to_string(),
            serde_json::Value::String(selected_node_id),
        );
    }
    Ok(())
}

fn persisted_policy_diagnostics(
    compiled_bundle: &serde_json::Value,
    flow_id: &str,
    node_id: &str,
) -> serde_json::Value {
    compiled_bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
        .as_array()
        .and_then(|flows| {
            flows.iter().find(|flow| {
                flow["flow_id"].as_str() == Some(flow_id)
                    && flow["lanes"].as_array().is_some_and(|lanes| {
                        lanes
                            .iter()
                            .any(|lane| lane["node_id"].as_str() == Some(node_id))
                    })
            })
        })
        .and_then(|flow| flow["lanes"].as_array())
        .and_then(|lanes| {
            lanes
                .iter()
                .find(|lane| lane["node_id"].as_str() == Some(node_id))
        })
        .and_then(|lane| lane.get("policy_diagnostics"))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn packet_template_kind_for_dev_team_task_class(task_class: &str) -> &'static str {
    let _ = task_class;
    // Diagnostic vocabulary only; executable packet families come from TeamFlow.
    "team_flow_packet_template_kind_unresolved"
}

#[derive(Debug, Default)]
struct ConfiguredDispatchRelations {
    packet_families: Vec<String>,
    packet_family_by_task_class: serde_json::Map<String, serde_json::Value>,
    activation_by_task_class: serde_json::Map<String, serde_json::Value>,
    design_owner_runtime_role: Option<String>,
    blockers: Vec<String>,
}

fn derive_configured_dispatch_relations(
    resolved_lanes: &[serde_json::Value],
) -> ConfiguredDispatchRelations {
    let mut relations = ConfiguredDispatchRelations::default();
    let mut activation_by_key = serde_json::Map::new();
    let mut owner_roles = Vec::new();
    if resolved_lanes.is_empty() {
        relations
            .blockers
            .push("team_flow_authority_packet_relation_missing".to_string());
    }
    for lane in resolved_lanes {
        let node_id = lane["node_id"].as_str().unwrap_or("<unknown-node>");
        let task_class = lane["task_class"].as_str().map(str::trim).unwrap_or("");
        let runtime_role = lane["runtime_role"].as_str().map(str::trim).unwrap_or("");
        let packet_family = lane["packet_template_kind"]
            .as_str()
            .map(str::trim)
            .unwrap_or("");
        if task_class.is_empty() {
            relations
                .blockers
                .push(format!("team_flow_authority_task_class_missing:{node_id}"));
        }
        if packet_family.is_empty() {
            relations.blockers.push(format!(
                "team_flow_authority_packet_template_kind_missing:{node_id}"
            ));
        } else if !relations
            .packet_families
            .iter()
            .any(|value| value == packet_family)
        {
            relations.packet_families.push(packet_family.to_string());
        }
        if !task_class.is_empty() && !packet_family.is_empty() {
            if relations
                .packet_family_by_task_class
                .contains_key(task_class)
            {
                relations.blockers.push(format!(
                    "team_flow_authority_task_class_packet_template_ambiguous:{task_class}"
                ));
            } else {
                relations.packet_family_by_task_class.insert(
                    task_class.to_string(),
                    serde_json::Value::String(packet_family.to_string()),
                );
            }
        }
        let activation = lane
            .get("activation")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let activation_present = match &activation {
            serde_json::Value::Null => false,
            serde_json::Value::String(value) => !value.trim().is_empty(),
            serde_json::Value::Array(values) => !values.is_empty(),
            serde_json::Value::Object(values) => !values.is_empty(),
            _ => true,
        };
        if !activation_present {
            relations
                .blockers
                .push(format!("team_flow_authority_activation_missing:{node_id}"));
        } else if !task_class.is_empty() {
            if activation_by_key.contains_key(task_class) {
                relations.blockers.push(format!(
                    "team_flow_authority_task_class_activation_ambiguous:{task_class}"
                ));
            } else {
                activation_by_key.insert(task_class.to_string(), activation.clone());
                relations
                    .activation_by_task_class
                    .insert(task_class.to_string(), activation.clone());
            }
        }
        if runtime_role == "business_analyst" || task_class == TASK_CLASS_SPECIFICATION {
            if !runtime_role.is_empty() {
                owner_roles.push(runtime_role.to_string());
            }
        }
    }
    match owner_roles.as_slice() {
        [owner] => relations.design_owner_runtime_role = Some(owner.clone()),
        [] => {}
        _ => relations
            .blockers
            .push("team_flow_authority_design_owner_ambiguous".to_string()),
    }
    relations
}

fn configured_tracked_flow_sequence(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> Result<Vec<String>, Vec<String>> {
    if !requires_design_gate {
        return Ok(Vec::new());
    }
    let Some(modes) = compiled_bundle["role_selection"]["conversation_modes"].as_object() else {
        return Err(vec![
            "team_flow_authority_tracked_flow_binding_config_missing".to_string(),
        ]);
    };
    let mut sequence = Vec::new();
    let mut binding_modes = std::collections::BTreeMap::<String, Vec<String>>::new();
    for (mode_id, mode) in modes {
        if mode.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
            continue;
        }
        let Some(entry) = mode["tracked_flow_entry"].as_str().map(str::trim) else {
            continue;
        };
        if entry.is_empty() {
            continue;
        }
        let entry = entry.to_string();
        if !sequence.iter().any(|value| value == &entry) {
            sequence.push(entry.clone());
        }
        binding_modes
            .entry(entry)
            .or_default()
            .push(mode_id.clone());
    }
    let Some(selected) = selection
        .tracked_flow_entry
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(vec![
            "team_flow_authority_tracked_flow_binding_missing".to_string(),
        ]);
    };
    let Some(modes) = binding_modes.get(selected) else {
        return Err(vec![format!(
            "team_flow_authority_tracked_flow_binding_unknown:{selected}"
        )]);
    };
    if modes.len() > 1 {
        return Err(vec![format!(
            "team_flow_authority_tracked_flow_binding_ambiguous:{selected}"
        )]);
    }
    Ok(sequence)
}

fn copy_non_empty_route_value(
    target: &mut serde_json::Value,
    target_key: &str,
    source: &serde_json::Value,
    source_key: &str,
) {
    let Some(value) = source.get(source_key) else {
        return;
    };
    let configured = match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(raw) => !raw.trim().is_empty(),
        serde_json::Value::Array(rows) => !rows.is_empty(),
        serde_json::Value::Object(entries) => !entries.is_empty(),
        _ => true,
    };
    if configured {
        target[target_key] = value.clone();
    }
}

fn apply_implementation_analysis_route_overrides(
    analysis: &mut serde_json::Value,
    implementation: &serde_json::Value,
) {
    copy_non_empty_route_value(
        analysis,
        "executor_backend",
        implementation,
        "analysis_executor_backend",
    );
    copy_non_empty_route_value(
        analysis,
        "external_first_required",
        implementation,
        "analysis_external_first_required",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_executor_backends",
        implementation,
        "analysis_fanout_executor_backends",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_min_results",
        implementation,
        "analysis_fanout_min_results",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_subagents",
        implementation,
        "analysis_fanout_subagents",
    );
    copy_non_empty_route_value(
        analysis,
        "merge_policy",
        implementation,
        "analysis_merge_policy",
    );
}

fn build_resolved_development_dispatch_contract(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    build_resolved_development_dispatch_contract_with_mode(
        compiled_bundle,
        selection,
        requires_design_gate,
        PrePlanTeamFlowSelectionMode::Fresh,
    )
}

fn build_resolved_development_dispatch_contract_with_mode(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
    team_flow_mode: PrePlanTeamFlowSelectionMode,
) -> serde_json::Value {
    let authority = match pre_plan_team_flow_authority(compiled_bundle, selection, team_flow_mode) {
        Ok(authority) => authority,
        Err(blocker) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": [blocker],
                "selected_flow_set": serde_json::Value::Null,
                "team_flow_authority_id": serde_json::Value::Null,
                "team_flow_config_hash": serde_json::Value::Null,
                "team_flow_registry_hash": serde_json::Value::Null,
                "execution_preparation_required": false,
                "resolved_lanes": [],
                "lane_sequence": [],
                "execution_lane_sequence": [],
                "lane_catalog": {},
                "dispatch_target_index": {},
                "runtime_role_index": {},
            });
        }
    };
    build_resolved_development_dispatch_contract_using_authority(
        compiled_bundle,
        selection,
        requires_design_gate,
        &authority,
        team_flow_mode,
    )
}

fn build_resolved_development_dispatch_contract_using_authority(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    _requires_design_gate: bool,
    authority: &crate::team_flow_authority_adapter::TeamFlowExecutionAuthority,
    team_flow_mode: PrePlanTeamFlowSelectionMode,
) -> serde_json::Value {
    let identity = team_flow_plan_identity_fields(Some(authority));
    let selected_node_id = match crate::runtime_dispatch_state::selected_flow_node_ref_for_mode(
        selection,
        authority,
        team_flow_mode == PrePlanTeamFlowSelectionMode::Fresh,
    ) {
        Ok(node_id) => node_id,
        Err(blocker) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": [blocker],
                "selected_flow_set": serde_json::Value::Null,
                "team_flow_authority_id": identity["team_flow_authority_id"],
                "team_flow_config_hash": identity["team_flow_config_hash"],
                "team_flow_registry_hash": identity["team_flow_registry_hash"],
                "execution_preparation_required": false,
                "resolved_lanes": [],
                "lane_sequence": [],
                "execution_lane_sequence": [],
                "lane_catalog": {},
                "dispatch_target_index": {},
                "runtime_role_index": {},
            });
        }
    };
    let requires_execution_preparation = match request_requires_execution_preparation(
        compiled_bundle,
        selection,
        authority,
        &selected_node_id,
    ) {
        Ok(value) => value,
        Err(blocker) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": [blocker],
                "selected_flow_set": serde_json::Value::Null,
                "team_flow_authority_id": identity["team_flow_authority_id"],
                "team_flow_config_hash": identity["team_flow_config_hash"],
                "team_flow_registry_hash": identity["team_flow_registry_hash"],
                "execution_preparation_required": false,
                "resolved_lanes": [],
                "lane_sequence": [],
                "execution_lane_sequence": [],
                "lane_catalog": {},
                "dispatch_target_index": {},
                "runtime_role_index": {},
            });
        }
    };
    let configured_flow =
        match resolved_development_flow_templates(authority, Some(&selected_node_id)) {
            Ok(flow) => flow,
            Err(blocker_codes) => {
                return serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": blocker_codes,
                    "selected_flow_set": serde_json::Value::Null,
                    "team_flow_authority_id": identity["team_flow_authority_id"],
                    "team_flow_config_hash": identity["team_flow_config_hash"],
                    "team_flow_registry_hash": identity["team_flow_registry_hash"],
                    "execution_preparation_required": requires_execution_preparation,
                    "resolved_lanes": [],
                    "lane_sequence": [],
                    "execution_lane_sequence": [],
                    "lane_catalog": {},
                    "dispatch_target_index": {},
                    "runtime_role_index": {},
                });
            }
        };
    let ConfiguredDevelopmentFlow {
        flow_id,
        selected_node_id,
        lanes,
        authority_id,
        config_authority_hash,
        registry_authority_hash,
    } = configured_flow;
    let mut authority_blockers = Vec::new();
    let resolved_lanes = lanes
        .into_iter()
        .map(|lane| {
            let dispatch_alias = lane.dispatch_alias.clone();
            let policy_diagnostics =
                persisted_policy_diagnostics(compiled_bundle, &flow_id, &lane.node_id);
            if lane.runtime_role.trim().is_empty() {
                authority_blockers.push(format!(
                    "team_flow_authority_missing_runtime_role:{}",
                    lane.node_id
                ));
            }
            if lane.selected_model_profile.is_null() {
                authority_blockers.push(format!(
                    "team_flow_authority_missing_selected_model_profile:{}",
                    lane.node_id
                ));
            }
            if dispatch_alias.trim().is_empty() {
                authority_blockers.push(format!(
                    "team_flow_authority_missing_dispatch_alias:{}",
                    lane.node_id
                ));
            }
            serde_json::json!({
                "node_id": lane.node_id,
                "lane_id": lane.lane_id,
                "dispatch_target": lane.dispatch_target,
                "dispatch_alias": dispatch_alias,
                "task_class": lane.task_class,
                "runtime_role": lane.runtime_role,
                "packet_template_kind": lane.packet_template_kind,
                "closure_class": lane.closure_class,
                "stage": lane.stage,
                "inclusion_rule": lane.inclusion_rule,
                "included": lane.included,
                "required": lane.required,
                "next_node": lane.next_node,
                "completion_blocker": lane.completion_blocker,
                "evidence_requirements": lane.evidence_requirements,
                "proof_gates": lane.proof_gates,
                "command_ref": lane.command_ref,
                "command_mapping": lane.command_mapping,
                "rework": {"targets": lane.rework_targets},
                "terminal": lane.terminal,
                "profile_authority": lane.profile_authority,
                "selected_model_profile": lane.selected_model_profile,
                "requires_user_approval": lane.requires_user_approval,
                "approval_policy": lane.approval_policy,
                "lifecycle_hook_templates": lane.lifecycle_hook_templates,
                "resume_transitions": lane.resume_transitions,
                "policy_diagnostics": policy_diagnostics,
                "activation": lane.activation,
                "runtime_assignment": lane.assignment,
                "carrier_runtime_assignment": lane.assignment,
                "carrier_relation": lane.carrier_relation,
                "executor_backend_relation": lane.executor_backend_relation,
                "authority_identities": lane.authority_identities,
                "execution_identity": lane.execution_identity,
            })
        })
        .collect::<Vec<_>>();
    let lane_sequence = resolved_lanes
        .iter()
        .filter(|lane| lane["included"].as_bool() == Some(true))
        .filter_map(|lane| lane["node_id"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let execution_lane_sequence = resolved_lanes
        .iter()
        .filter(|lane| lane["included"].as_bool() == Some(true))
        .filter(|lane| lane["stage"].as_str() != Some("design_gate"))
        .filter_map(|lane| lane["node_id"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let mut lane_catalog = serde_json::Map::new();
    let mut dispatch_target_index = serde_json::Map::new();
    let mut runtime_role_index = serde_json::Map::new();
    let add_index = |index: &mut serde_json::Map<String, serde_json::Value>,
                     value: Option<&str>,
                     node_id: &str| {
        let Some(value) = value else {
            return;
        };
        let entries = index
            .entry(value.to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let Some(entries) = entries.as_array_mut() {
            entries.push(serde_json::Value::String(node_id.to_string()));
        }
    };
    for lane in &resolved_lanes {
        let Some(node_id) = lane["node_id"].as_str() else {
            continue;
        };
        lane_catalog.insert(node_id.to_string(), lane.clone());
        add_index(
            &mut dispatch_target_index,
            lane["dispatch_target"].as_str(),
            node_id,
        );
        add_index(
            &mut runtime_role_index,
            lane["runtime_role"].as_str(),
            node_id,
        );
    }
    let configured_relations = derive_configured_dispatch_relations(&resolved_lanes);
    authority_blockers.extend(configured_relations.blockers.clone());
    if _requires_design_gate {
        if configured_relations.design_owner_runtime_role.is_none() {
            authority_blockers.push("team_flow_authority_design_owner_missing".to_string());
        }
        if let Err(blockers) = configured_tracked_flow_sequence(compiled_bundle, selection, true) {
            authority_blockers.extend(blockers);
        }
    }
    let activation_for_persisted_class = |logical_class: &str| {
        configured_relations
            .activation_by_task_class
            .get(logical_class)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    serde_json::json!({
        "status": if authority_blockers.is_empty() { "ready" } else { "blocked" },
        "blocker_codes": authority_blockers,
        "selected_flow_set": flow_id,
        "team_flow_authority_id": authority_id,
        "team_flow_config_hash": config_authority_hash,
        "team_flow_registry_hash": registry_authority_hash,
        "team_flow_authority_selected_node_id": selected_node_id.clone(),
        "selected_node_id": selected_node_id,
        "execution_preparation_required": requires_execution_preparation,
        "root_session_must_remain_orchestrator": true,
        "packet_family_required": configured_relations.packet_families,
        "packet_family_by_task_class": configured_relations.packet_family_by_task_class,
        "activation_by_task_class": configured_relations.activation_by_task_class,
        "design_owner_runtime_role": configured_relations
            .design_owner_runtime_role
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "resolved_lanes": resolved_lanes,
        "lane_sequence": lane_sequence,
        "execution_lane_sequence": execution_lane_sequence,
        "lane_catalog": lane_catalog,
        "dispatch_target_index": dispatch_target_index,
        "runtime_role_index": runtime_role_index,
        "specification_activation": activation_for_persisted_class(TASK_CLASS_SPECIFICATION),
        "implementer_activation": activation_for_persisted_class(TASK_CLASS_IMPLEMENTATION),
        "coach_activation": activation_for_persisted_class(TASK_CLASS_COACH),
        "verifier_activation": activation_for_persisted_class(TASK_CLASS_VERIFICATION),
        "escalation_activation": activation_for_persisted_class(TASK_CLASS_ARCHITECTURE),
    })
}

fn dispatch_contract_lane<'a>(
    dispatch_contract: &'a serde_json::Value,
    dispatch_target: &str,
) -> Result<&'a serde_json::Value, String> {
    let dispatch_target = dispatch_target.trim();
    if dispatch_target.is_empty() {
        return Err("team_flow_authority_dispatch_target_missing".to_string());
    }
    let lane = dispatch_contract["lane_catalog"]
        .get(dispatch_target)
        .or_else(|| {
            let node_ids = dispatch_contract["dispatch_target_index"]
                .get(dispatch_target)?
                .as_array()?;
            match node_ids.as_slice() {
                [node_id] => dispatch_contract["lane_catalog"].get(node_id.as_str()?),
                _ => None,
            }
        })
        .ok_or_else(|| {
            format!("team_flow_authority_dispatch_target_ambiguous_or_unknown:{dispatch_target}")
        })?;
    Ok(lane)
}

fn orchestration_lane_step_label_for_contract(
    dispatch_contract: &serde_json::Value,
    dispatch_target: &str,
) -> Result<String, String> {
    let lane = dispatch_contract_lane(dispatch_contract, dispatch_target)?;
    let dispatch_alias = lane["dispatch_alias"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("team_flow_authority_dispatch_alias_missing:{dispatch_target}"))?;
    Ok(format!("delegate_{dispatch_alias}_lane"))
}

fn orchestration_checkpoint_label_for_contract(
    dispatch_contract: &serde_json::Value,
    dispatch_target: &str,
) -> Result<String, String> {
    let lane = dispatch_contract_lane(dispatch_contract, dispatch_target)?;
    let task_class = lane["task_class"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("team_flow_authority_task_class_missing:{dispatch_target}"))?;
    Ok(format!("after_{task_class}_evidence"))
}

fn build_runtime_orchestration_contract(
    requires_design_gate: bool,
    agent_only_development: bool,
    dispatch_contract: &serde_json::Value,
) -> serde_json::Value {
    if dispatch_contract["status"] != "ready" {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": dispatch_contract["blocker_codes"].clone(),
            "mode": "delegated_orchestration_cycle",
            "root_session_role": "orchestrator",
            "root_session_must_remain_orchestrator": true,
            "agent_only_development_required": agent_only_development,
            "active_cycle": [],
            "replanning": {"required": true, "checkpoints": []},
        });
    }
    let Some(execution_lane_sequence) = dispatch_contract["execution_lane_sequence"].as_array()
    else {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["team_flow_execution_lane_sequence_missing"],
            "mode": "delegated_orchestration_cycle",
            "root_session_role": "orchestrator",
            "active_cycle": [],
            "replanning": {"required": true, "checkpoints": []},
        });
    };
    let execution_lane_sequence = execution_lane_sequence
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let lane_step_labels = execution_lane_sequence
        .iter()
        .map(|lane| orchestration_lane_step_label_for_contract(dispatch_contract, lane))
        .collect::<Result<Vec<_>, _>>();
    let checkpoint_labels = execution_lane_sequence
        .iter()
        .map(|lane| orchestration_checkpoint_label_for_contract(dispatch_contract, lane))
        .collect::<Result<Vec<_>, _>>();
    let design_gate_label = if requires_design_gate {
        let design_gate_nodes = dispatch_contract["lane_sequence"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .filter_map(|node_id| dispatch_contract_lane(dispatch_contract, node_id).ok())
            .filter(|lane| lane["stage"].as_str() == Some("design_gate"))
            .collect::<Vec<_>>();
        match design_gate_nodes.as_slice() {
            [lane] => match lane["node_id"].as_str() {
                Some(node_id) => {
                    orchestration_lane_step_label_for_contract(dispatch_contract, node_id)
                }
                None => Err("team_flow_authority_design_gate_node_id_missing".to_string()),
            },
            [] => Err("team_flow_authority_design_gate_lane_missing".to_string()),
            _ => Err("team_flow_authority_design_gate_lane_ambiguous".to_string()),
        }
    } else {
        Ok(String::new())
    };
    let (lane_step_labels, checkpoint_labels, design_gate_label) =
        match (lane_step_labels, checkpoint_labels, design_gate_label) {
            (Ok(lane_step_labels), Ok(checkpoint_labels), Ok(design_gate_label)) => {
                (lane_step_labels, checkpoint_labels, design_gate_label)
            }
            (lane_step_labels, checkpoint_labels, design_gate_label) => {
                let blocker = [
                    lane_step_labels.err(),
                    checkpoint_labels.err(),
                    design_gate_label.err(),
                ]
                .into_iter()
                .flatten()
                .next()
                .unwrap_or_else(|| "team_flow_orchestration_label_resolution_failed".to_string());
                return serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": [blocker],
                    "mode": "delegated_orchestration_cycle",
                    "root_session_role": "orchestrator",
                    "root_session_must_remain_orchestrator": true,
                    "agent_only_development_required": agent_only_development,
                    "active_cycle": [],
                    "replanning": {"required": true, "checkpoints": []},
                });
            }
        };
    let active_cycle = if requires_design_gate {
        let mut cycle = vec![
            "publish_initial_execution_plan".to_string(),
            design_gate_label.clone(),
            "replan_after_design_gate".to_string(),
            "shape_work_pool_and_dev_packets".to_string(),
        ];
        cycle.extend(lane_step_labels.clone());
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    } else {
        let mut cycle = vec!["publish_initial_execution_plan".to_string()];
        cycle.extend(lane_step_labels);
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    };
    let replanning_checkpoints = if requires_design_gate {
        let mut checkpoints = vec![
            format!("after_{}_evidence", design_gate_label),
            "after_work_pool_shape".to_string(),
            "after_dev_packet_shape".to_string(),
        ];
        checkpoints.extend(checkpoint_labels.clone());
        serde_json::json!(checkpoints)
    } else {
        let mut checkpoints = vec!["after_packet_shape".to_string()];
        checkpoints.extend(checkpoint_labels);
        serde_json::json!(checkpoints)
    };

    serde_json::json!({
        "mode": "delegated_orchestration_cycle",
        "root_session_role": "orchestrator",
        "root_session_must_remain_orchestrator": true,
        "root_session_write_guard": build_root_session_write_guard(),
        "initial_response": {
            "plan_required_before_substantive_execution": true,
            "plan_scope": "one bounded active cycle",
            "must_happen_before": [
                "design_doc_mutation",
                "packet_dispatch",
                "implementation_work"
            ],
            "minimum_fields": [
                "active_bounded_unit",
                "next_steps",
                "delegation_targets",
                "proof_target"
            ],
            "operator_message": "publish a concise execution plan before mutating docs, dispatching work, or entering implementation"
        },
        "delegation_policy": {
            "normal_write_producing_work": "delegated_by_default",
            "agent_only_development_required": agent_only_development,
            "canonical_project_delegated_execution_surface": "vida agent-init",
            "host_subagent_apis_are_backend_details": true,
            "host_local_write_capability_is_not_authority": true,
            "generic_single_worker_dispatch_forbidden": true,
            "local_implementation_without_exception_path_forbidden": true,
            "required_lanes": dispatch_contract["lane_sequence"]
        },
        "replanning": {
            "required": true,
            "checkpoints": replanning_checkpoints,
            "trigger_rule": "replan after each bounded gate or delegated evidence return before the next write-producing step"
        },
        "continuation_binding": {
            "required_for_continue_development": true,
            "fail_closed_without_explicit_binding": true,
            "required_fields": [
                "active_bounded_unit",
                "why_this_unit",
                "primary_path",
                "sequential_vs_parallel_posture"
            ],
            "forbidden_fallbacks": [
                "ready_head[0]",
                "first_ready_backlog_candidate",
                "adjacent_sibling_slice"
            ]
        },
        "active_cycle": active_cycle
    })
}

fn build_root_session_write_guard() -> serde_json::Value {
    serde_json::json!({
        "status": "blocked_by_default",
        "root_session_role": "orchestrator",
        "local_write_requires_exception_path": true,
        "lawful_write_surface": "vida agent-init",
        "explicit_user_ordered_agent_mode_is_sticky": true,
        "saturation_recovery_required_before_local_fallback": true,
        "local_fallback_without_lane_recovery_forbidden": true,
        "host_local_write_capability_is_not_authority": true,
        "required_exception_evidence": crate::status_surface_write_guard::root_session_write_guard_required_exception_evidence(),
        "pre_write_checkpoint_required": true,
    })
}

fn supported_autonomous_execution_settings(
    compiled_bundle: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "agent_only_development": crate::json_bool(
            compiled_bundle["autonomous_execution"].get("agent_only_development"),
            false,
        ),
        "validation_report_required_before_implementation": crate::json_bool(
            compiled_bundle["autonomous_execution"]
                .get("validation_report_required_before_implementation"),
            false,
        ),
    })
}

pub(crate) fn build_runtime_execution_plan_from_snapshot(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    build_runtime_execution_plan_from_snapshot_with_mode(
        compiled_bundle,
        selection,
        PrePlanTeamFlowSelectionMode::Fresh,
    )
}

fn build_runtime_execution_plan_from_snapshot_with_mode(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    team_flow_mode: PrePlanTeamFlowSelectionMode,
) -> serde_json::Value {
    let agent_system = &compiled_bundle["agent_system"];
    let authority = pre_plan_team_flow_authority(compiled_bundle, selection, team_flow_mode).ok();
    let implementation = authority
        .as_ref()
        .map(|authority| {
            crate::runtime_lane_summary::summarize_agent_route_from_snapshot_with_authority(
                compiled_bundle,
                agent_system,
                "implementation",
                authority,
            )
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["team_flow_selected_flow_authority_missing"],
                "route_id": "implementation",
            })
        });
    let analysis_route_id = implementation["analysis_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let mut analysis = authority
        .as_ref()
        .map(|authority| {
            crate::runtime_lane_summary::summarize_agent_route_from_snapshot_with_authority(
                compiled_bundle,
                agent_system,
                analysis_route_id,
                authority,
            )
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["team_flow_selected_flow_authority_missing"],
                "route_id": analysis_route_id,
            })
        });
    apply_implementation_analysis_route_overrides(&mut analysis, &implementation);
    let coach_route_id = implementation["coach_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let verification_route_id = implementation["verification_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let feature_design_terms =
        crate::feature_delivery_design_terms(&selection.request.to_lowercase());
    let suppress_fresh_design_gate =
        selection.reason == "auto_existing_design_backed_implementation_request_override";
    let requires_design_gate = !suppress_fresh_design_gate
        && (selection
            .tracked_flow_entry
            .as_deref()
            .is_some_and(|entry| !entry.trim().is_empty())
            || !feature_design_terms.is_empty());
    let tracked_flow_sequence =
        configured_tracked_flow_sequence(compiled_bundle, selection, requires_design_gate);
    let tracked_flow_bootstrap = if requires_design_gate {
        serde_json::json!({
            "required": true,
            "status": "blocked",
            "executable": false,
            "view_only": true,
            "activation_semantics": "configured_team_flow_relation_required",
            "blocker_codes": ["team_flow_authority_tracked_flow_relation_missing"],
            "schema_vocabulary": ["epic", "spec_task", "work_pool_task", "dev_task"],
        })
    } else {
        serde_json::Value::Null
    };
    let autonomous_execution = supported_autonomous_execution_settings(compiled_bundle);
    let agent_only_development =
        crate::json_bool(autonomous_execution.get("agent_only_development"), false);
    let dispatch_contract = authority
        .as_ref()
        .map(|authority| {
            build_resolved_development_dispatch_contract_using_authority(
                compiled_bundle,
                selection,
                requires_design_gate,
                authority,
                team_flow_mode,
            )
        })
        .unwrap_or_else(|| {
            build_resolved_development_dispatch_contract_with_mode(
                compiled_bundle,
                selection,
                requires_design_gate,
                team_flow_mode,
            )
        });
    let design_owner_runtime_role = dispatch_contract["design_owner_runtime_role"].clone();
    let tracked_flow_binding_blocked = tracked_flow_sequence.is_err();
    let tracked_flow_sequence_value = tracked_flow_sequence
        .clone()
        .map(|sequence| {
            serde_json::Value::Array(
                sequence
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            )
        })
        .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
    let orchestration_contract = build_runtime_orchestration_contract(
        requires_design_gate,
        agent_only_development,
        &dispatch_contract,
    );
    let policy_bundle_ref = crate::runtime_lane_summary::resolve_policy_pin(compiled_bundle);
    let mut runtime_assignment =
        crate::build_runtime_assignment(compiled_bundle, selection, requires_design_gate);
    crate::runtime_assignment_builder::attach_policy_bundle_ref(
        &mut runtime_assignment,
        &policy_bundle_ref,
    );
    let dispatch_ready = dispatch_contract["status"] == "ready";
    let selected_node_id = dispatch_contract["selected_node_id"]
        .as_str()
        .map(str::to_string);
    let lane_sequence = dispatch_contract["lane_sequence"].as_array().cloned();
    let lane_sequence_value = lane_sequence
        .clone()
        .map(serde_json::Value::Array)
        .unwrap_or(serde_json::Value::Null);
    let backend_admissibility_matrix =
        crate::runtime_lane_summary::build_executor_backend_admissibility_matrix(agent_system);
    let mut execution_plan = serde_json::json!({
        "status": if !dispatch_ready || tracked_flow_binding_blocked {
            "blocked_team_flow_authority"
        } else if requires_design_gate {
            "design_first"
        } else {
            "ready_for_runtime_routing"
        },
        "policy_bundle_ref": policy_bundle_ref,
        "system_mode": crate::json_string(crate::json_lookup(agent_system, &["mode"])).unwrap_or_default(),
        "state_owner": crate::json_string(crate::json_lookup(agent_system, &["state_owner"])).unwrap_or_default(),
        "max_parallel_agents": crate::json_lookup(agent_system, &["max_parallel_agents"]).cloned().unwrap_or(serde_json::Value::Null),
        "autonomous_execution": autonomous_execution,
        "backend_admissibility_matrix": backend_admissibility_matrix,
        "orchestration_contract": orchestration_contract,
        "default_route": serde_json::Value::Null,
        "conversation_stage": {
            "selected_role": selection.selected_role,
            "conversational_mode": selection.conversational_mode,
            "tracked_flow_entry": selection.tracked_flow_entry,
            "allow_freeform_chat": selection.allow_freeform_chat,
            "single_task_only": selection.single_task_only,
        },
        "pre_execution_design_gate": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "not_required"
            },
            "developer_handoff_packet_required": requires_design_gate,
            "developer_handoff_packet_status": if requires_design_gate {
                "blocked_pending_developer_handoff_packet"
            } else {
                "not_required"
            },
            "design_runtime": "vida docflow",
            "design_template": crate::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            "intake_runtime": if requires_design_gate {
                serde_json::Value::String("vida taskflow consume final <request> --json".to_string())
            } else {
                serde_json::Value::Null
            },
            "tracked_handoff": if requires_design_gate {
                selection
                    .tracked_flow_entry
                    .clone()
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            },
            "todo_sequence": if requires_design_gate {
                serde_json::json!([
                    "capture research, specification scope, and implementation plan in one bounded design document",
                    "create one epic and one configured specification task in vida taskflow before code execution",
                    "keep the design artifact canonical through vida docflow init/finalize-edit/check",
                    "close the configured specification task and shape one bounded execution packet from the approved design before delegated development"
                ])
            } else {
                serde_json::json!([])
            },
            "taskflow_sequence": if requires_design_gate {
                tracked_flow_sequence_value.clone()
            } else {
                serde_json::json!([])
            }
        },
        "pre_execution_todo": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "open"
            } else {
                "not_required"
            },
            "items": if requires_design_gate {
                serde_json::json!([
                    {
                        "id": "taskflow_epic_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one epic that will own the feature-level tracked flow before documentation or implementation begins"
                    },
                    {
                        "id": "taskflow_spec_task_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one configured specification task under the epic before authoring the design artifact"
                    },
                    {
                        "id": "design_doc_scope",
                        "owner": design_owner_runtime_role.clone(),
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "capture research, specification scope, and implementation plan in one bounded design document"
                    },
                    {
                        "id": "design_doc_finalize",
                        "owner": "orchestrator",
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "finalize and validate the bounded design artifact canonically"
                    },
                    {
                        "id": "taskflow_spec_task_close",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "close the configured specification task only after the design artifact is finalized and validated"
                    },
                    {
                        "id": "taskflow_packet_shape",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "shape the configured tracked-flow handoff before delegated implementation dispatch"
                    }
                ])
            } else {
                serde_json::json!([])
            }
        },
        "tracked_flow_bootstrap": tracked_flow_bootstrap,
        "development_flow": {
            "activation_status": if !dispatch_ready {
                "blocked_team_flow_authority"
            } else if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "eligible_after_runtime_routing"
            },
            "lane_sequence": lane_sequence_value.clone(),
            "generic_single_worker_dispatch_forbidden": true,
            "dispatch_contract": dispatch_contract,
            "timeout_policy": {
                "worker_wait_timeout_is_not_root_write_permission": true,
                "generic_internal_worker_fallback_forbidden": true,
                "root_session_takeover_requires_exception_receipt": true,
                "next_actions": [
                    "continue_lawful_waiting_or_polling",
                    "inspect_open_delegated_lane_state",
                    "reuse_or_reclaim_eligible_lane_if_lawful",
                    "dispatch_coach_or_verifier_or_escalation_when_route_requires_it",
                    "record_explicit_blocker_or_exception_path_before_any_root_session_write"
                ]
            },
            "implementation": implementation,
            "analysis": analysis,
            "coach": authority
                .as_ref()
                .map(|authority| {
                    crate::runtime_lane_summary::summarize_agent_route_from_snapshot_with_authority(
                        compiled_bundle,
                        agent_system,
                        coach_route_id,
                        authority,
                    )
                })
                .unwrap_or_else(|| serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": ["team_flow_selected_flow_authority_missing"],
                    "route_id": coach_route_id,
                })),
            "verification": authority
                .as_ref()
                .map(|authority| {
                    crate::runtime_lane_summary::summarize_agent_route_from_snapshot_with_authority(
                        compiled_bundle,
                        agent_system,
                        verification_route_id,
                        authority,
                    )
                })
                .unwrap_or_else(|| serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": ["team_flow_selected_flow_authority_missing"],
                    "route_id": verification_route_id,
                })),
        },
    });
    if let Some(selected_node_id) = selected_node_id {
        if let Some(plan) = execution_plan.as_object_mut() {
            plan.insert(
                "team_flow_authority_selected_node_id".to_string(),
                serde_json::Value::String(selected_node_id),
            );
        }
    }
    if let Some(plan) = execution_plan.as_object_mut() {
        plan.insert(
            "root_session_write_guard".to_string(),
            build_root_session_write_guard(),
        );
        plan.extend(crate::runtime_assignment_alias_fields(&runtime_assignment));
    }
    execution_plan
}

#[cfg(test)]
mod tests {
    use super::{
        PrePlanTeamFlowSelectionMode, apply_implementation_analysis_route_overrides,
        build_design_first_tracked_flow_bootstrap, build_resolved_development_dispatch_contract,
        configured_dev_team_flow_templates, configured_tracked_flow_sequence,
        derive_configured_dispatch_relations,
        normalize_fresh_selected_or_default_flow_for_execution_plan,
        normalize_selected_flow_for_execution_plan,
        normalize_selected_or_default_flow_for_execution_plan, pre_plan_team_flow_authority,
        request_requires_execution_preparation, supported_autonomous_execution_settings,
        task_class_for_selection,
    };
    use crate::RuntimeConsumptionLaneSelection;
    use crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle;
    use serde_json::json;

    fn strict_team_flow_bundle() -> serde_json::Value {
        canonical_compiled_bundle()
    }

    fn strict_team_flow_selection(bundle: &serde_json::Value) -> RuntimeConsumptionLaneSelection {
        let persisted_lane = &bundle["team_flow_authority"]["lanes"][0];
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "configured".to_string(),
            fallback_role: bundle["role_selection"]["fallback_role"]
                .as_str()
                .expect("master config fallback role")
                .to_string(),
            request: format!(
                "strict flow dev_team_flow_id:{}",
                bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["flow_id"]
                    .as_str()
                    .expect("canonical flow id")
            ),
            selected_role: persisted_lane["runtime_role"]
                .as_str()
                .expect("master config selected runtime role")
                .to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec![format!(
                "dev_team_flow_id:{}",
                bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["flow_id"]
                    .as_str()
                    .expect("canonical flow id")
            )],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn task_class_for_selection_missing_selected_node_fails_closed() {
        let mut bundle = strict_team_flow_bundle();
        bundle["role_selection"]["selected_task_class"] = serde_json::json!("legacy-task-class");
        bundle["default_flow_set"] = serde_json::json!("minimal");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.selected_role = "orchestrator".to_string();
        selection.matched_terms.clear();
        selection.execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": bundle["team_flow_authority"]
                ["resolved_all_flow_payload"]["flows"][0]["flow_id"]
        });
        let error = super::task_class_for_selection_with_mode(
            &bundle,
            &selection,
            PrePlanTeamFlowSelectionMode::Persisted,
        )
        .expect_err("missing selected node must fail closed");
        assert!(
            error.contains("team_flow_authority_selected_node_id_missing"),
            "unexpected blocker: {error}"
        );
    }

    #[test]
    fn task_class_for_selection_explicit_unknown_flow_fails_closed() {
        let mut bundle = strict_team_flow_bundle();
        bundle["role_selection"]["selected_task_class"] = serde_json::Value::Null;
        let mut selection = strict_team_flow_selection(&bundle);
        selection.selected_role = "orchestrator".to_string();
        selection.matched_terms = vec!["dev_team_flow_id:unknown-explicit-flow".to_string()];
        selection.request = "dev_team_flow_id:unknown-explicit-flow".to_string();
        selection.execution_plan = serde_json::Value::Null;
        let error = task_class_for_selection(&bundle, &selection)
            .expect_err("unknown explicit flow must fail closed");
        assert!(error.contains("team_flow_authority_unknown_flow"));
    }

    #[test]
    fn pre_plan_team_flow_authority_selection_matrix_is_strict_and_persisted_first() {
        let mut bundle = strict_team_flow_bundle();
        let default_flow = bundle["team_flow_authority"]["selected_config"]["authority_selection"]
            ["default_flow_id"]
            .as_str()
            .expect("authority default flow")
            .to_string();
        let alternate_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() != Some(default_flow.as_str()))
            .and_then(|flow| flow["flow_id"].as_str())
            .expect("alternate authority flow")
            .to_string();
        bundle["default_flow_set"] = serde_json::json!("legacy-compatibility-flow");

        let cases = vec![
            (
                "legacy_default",
                super::PrePlanTeamFlowSelectionMode::Fresh,
                "plain request".to_string(),
                Vec::<String>::new(),
                serde_json::Value::Null,
                Some(default_flow.clone()),
            ),
            (
                "matched_term_selects_authority_flow",
                super::PrePlanTeamFlowSelectionMode::Fresh,
                "plain request".to_string(),
                vec![format!("dev_team_flow_id:{alternate_flow}")],
                serde_json::Value::Null,
                Some(alternate_flow.clone()),
            ),
            (
                "explicit_known_selector",
                super::PrePlanTeamFlowSelectionMode::Fresh,
                format!("dev_team_flow_id:{alternate_flow}"),
                vec![format!("dev_team_flow_id:{alternate_flow}")],
                serde_json::Value::Null,
                Some(alternate_flow.clone()),
            ),
            (
                "persisted_flow_wins",
                super::PrePlanTeamFlowSelectionMode::Persisted,
                "plain request".to_string(),
                vec![format!("dev_team_flow_id:{alternate_flow}")],
                serde_json::json!({
                    "team_flow_authority_selected_flow_id": alternate_flow
                }),
                Some(alternate_flow.clone()),
            ),
            (
                "explicit_unknown_selector",
                super::PrePlanTeamFlowSelectionMode::Fresh,
                "dev_team_flow_id:unknown-explicit-flow".to_string(),
                vec!["dev_team_flow_id:unknown-explicit-flow".to_string()],
                serde_json::Value::Null,
                None,
            ),
        ];

        for (name, mode, request, matched_terms, execution_plan, expected_flow) in cases {
            let mut selection = strict_team_flow_selection(&bundle);
            selection.request = request;
            selection.matched_terms = matched_terms;
            selection.execution_plan = execution_plan;
            let authority = super::pre_plan_team_flow_authority(&bundle, &selection, mode);
            match expected_flow {
                Some(expected_flow) => assert_eq!(
                    authority.expect(name).snapshot.flow_ref,
                    expected_flow,
                    "{name} should resolve through TeamFlow authority"
                ),
                None => assert!(
                    authority
                        .expect_err(name)
                        .to_string()
                        .contains("team_flow_authority_unknown_flow"),
                    "{name} must fail closed"
                ),
            }
        }
    }

    #[test]
    fn normalize_selected_flow_rehydrates_empty_plan_from_matched_term() {
        let bundle = strict_team_flow_bundle();
        let default_flow = bundle["default_flow_set"]
            .as_str()
            .expect("configured default flow id");
        let alternate_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|flow| {
                flow["flow_id"].as_str() != Some(default_flow)
                    && flow["flow_policy"]["enabled"].as_bool() == Some(true)
                    && flow["lanes"].as_array().is_some_and(|lanes| {
                        lanes
                            .iter()
                            .any(|lane| lane["included"].as_bool() == Some(true))
                    })
            })
            .find(|flow| flow["flow_id"].as_str().is_some())
            .and_then(|flow| flow["flow_id"].as_str())
            .expect("canonical config should expose an enabled alternate flow");
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle,
            Some(alternate_flow),
            None,
        )
        .expect("alternate flow authority must compile");
        let first_node = authority
            .ordered_nodes()
            .find(|node| node.node.included)
            .expect("alternate flow must expose an included node");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.execution_plan = serde_json::json!({
            "runtime_consumption_explicit_task_id": "explicit-alternate-task",
            "team_flow_authority_selected_node_id": first_node.node.node_id
        });
        selection.matched_terms = vec![format!("dev_team_flow_id:{alternate_flow}")];
        normalize_selected_flow_for_execution_plan(&mut selection, &bundle, alternate_flow)
            .expect("alternate flow should rebuild from the matched flow term");

        let contract = &selection.execution_plan["development_flow"]["dispatch_contract"];
        assert_eq!(contract["selected_flow_set"].as_str(), Some(alternate_flow));
        assert_eq!(
            contract["team_flow_authority_id"].as_str(),
            Some(authority.projection().authority_id.as_str())
        );
        assert_eq!(
            contract["team_flow_config_hash"].as_str(),
            Some(authority.projection().config_authority_hash.as_str())
        );
        assert_eq!(
            contract["team_flow_registry_hash"].as_str(),
            Some(authority.projection().registry_authority_hash.as_str())
        );
        assert_eq!(
            contract["selected_node_id"].as_str(),
            Some(first_node.node.node_id.as_str())
        );
        assert_eq!(
            selection.execution_plan["team_flow_authority_selected_node_id"].as_str(),
            Some(first_node.node.node_id.as_str())
        );
        assert_eq!(
            selection.selected_role, first_node.node.runtime_role,
            "selected role must be derived from the selected flow authority"
        );
        assert_eq!(
            selection.execution_plan["runtime_consumption_explicit_task_id"].as_str(),
            Some("explicit-alternate-task")
        );
        assert_eq!(
            selection.execution_plan["team_flow_authority_selected_flow_id"].as_str(),
            Some(alternate_flow)
        );
    }

    #[test]
    fn normalize_selected_or_default_flow_materializes_exact_node_copies() {
        let bundle = strict_team_flow_bundle();
        let default_flow = bundle["default_flow_set"]
            .as_str()
            .expect("configured default flow id");
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle,
            Some(default_flow),
            None,
        )
        .expect("default flow authority must compile");
        let first_node = authority
            .ordered_nodes()
            .find(|node| node.node.included)
            .expect("default flow must expose an included node");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.matched_terms.clear();
        selection.execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": default_flow,
            "team_flow_authority_selected_node_id": first_node.node.node_id
        });

        normalize_selected_or_default_flow_for_execution_plan(&mut selection, &bundle)
            .expect("persisted selected node should normalize");

        assert_eq!(
            selection.execution_plan["team_flow_authority_selected_flow_id"].as_str(),
            Some(default_flow)
        );
        assert_eq!(
            selection.execution_plan["team_flow_authority_selected_node_id"].as_str(),
            Some(first_node.node.node_id.as_str())
        );
        let contract = &selection.execution_plan["development_flow"]["dispatch_contract"];
        assert_eq!(
            contract["team_flow_authority_selected_node_id"].as_str(),
            Some(first_node.node.node_id.as_str())
        );
        assert_eq!(
            contract["selected_node_id"].as_str(),
            Some(first_node.node.node_id.as_str())
        );
    }

    #[test]
    fn normalize_fresh_selected_flow_rejects_unknown_compatibility_candidate() {
        let mut bundle = strict_team_flow_bundle();
        bundle["default_flow_set"] = serde_json::json!("minimal");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.matched_terms = vec!["dev_team_flow_id:minimal".to_string()];
        selection.request = "fix dispatch packet validation".to_string();
        selection.conversational_mode = Some("scope_discussion".to_string());
        selection.execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": "minimal",
            "team_flow_authority_selected_node_id": "compat-node",
            "development_flow": {
                "dispatch_contract": {
                    "selected_flow_set": "minimal",
                    "selected_node_id": "compat-node",
                    "team_flow_authority_selected_node_id": "compat-node",
                    "team_flow_authority_id": "compat-authority",
                    "team_flow_config_hash": "compat-config-hash",
                    "team_flow_registry_hash": "compat-registry-hash"
                }
            }
        });

        let error =
            normalize_fresh_selected_or_default_flow_for_execution_plan(&mut selection, &bundle)
                .expect_err("fresh compatibility candidate must fail closed");
        assert!(error.contains("team_flow_authority_unknown_flow"));
        assert!(error.contains("minimal"));
    }

    #[test]
    fn normalize_selected_or_default_flow_rejects_explicit_unknown_flow() {
        let bundle = strict_team_flow_bundle();
        let mut selection = strict_team_flow_selection(&bundle);
        selection.matched_terms = vec!["dev_team_flow_id:explicit-unknown-flow".to_string()];
        selection.request =
            "fix runtime dev_team_flow_id:explicit-unknown-flow validation".to_string();
        selection.execution_plan = serde_json::json!({
            "runtime_consumption_explicit_task_id": "unknown-flow-task"
        });

        let error =
            normalize_fresh_selected_or_default_flow_for_execution_plan(&mut selection, &bundle)
                .expect_err("explicit unknown flow must fail closed");

        assert!(error.contains("team_flow_authority_unknown_flow"));
        assert!(error.contains("explicit-unknown-flow"));
    }

    #[test]
    fn execution_preparation_policy_uses_selected_flow_over_poisoned_default() {
        let mut bundle = strict_team_flow_bundle();
        let default_flow = bundle["default_flow_set"]
            .as_str()
            .expect("configured default flow id")
            .to_string();
        let alternate_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() != Some(default_flow.as_str()))
            .and_then(|flow| flow["flow_id"].as_str())
            .expect("alternate authority flow")
            .to_string();
        bundle["all_project_flow_catalog"][&default_flow]["execution_preparation_policy"] =
            json!({"mode": "always"});
        bundle["all_project_flow_catalog"][&alternate_flow]["execution_preparation_policy"] =
            json!({"mode": "never"});
        let mut selection = strict_team_flow_selection(&bundle);
        selection.request = format!("dev_team_flow_id:{alternate_flow}");
        selection.matched_terms = vec![format!("dev_team_flow_id:{alternate_flow}")];
        let authority =
            pre_plan_team_flow_authority(&bundle, &selection, PrePlanTeamFlowSelectionMode::Fresh)
                .expect("selected alternate authority must compile");
        assert!(
            !request_requires_execution_preparation(
                &bundle,
                &selection,
                &authority,
                &authority.entry_node_id,
            )
            .expect("selected flow policy should resolve")
        );
    }

    #[test]
    fn persisted_selected_flow_identity_conflict_fails_closed() {
        let bundle = strict_team_flow_bundle();
        let mut selection = strict_team_flow_selection(&bundle);
        selection.execution_plan = json!({
            "team_flow_authority_selected_flow_id": "persisted-a",
            "development_flow": {"dispatch_contract": {"selected_flow_set": "persisted-b"}}
        });
        let error = pre_plan_team_flow_authority(
            &bundle,
            &selection,
            PrePlanTeamFlowSelectionMode::Persisted,
        )
        .expect_err("conflicting persisted identities must fail closed");
        assert!(error.contains("team_flow_selected_flow_identity_conflict"));
    }

    #[test]
    fn selected_flow_dispatch_contract_contains_only_selected_flow_lanes() {
        let bundle = strict_team_flow_bundle();
        let alternate_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() != bundle["default_flow_set"].as_str())
            .and_then(|flow| flow["flow_id"].as_str())
            .expect("alternate authority flow");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.request = format!("dev_team_flow_id:{alternate_flow}");
        selection.matched_terms = vec![format!("dev_team_flow_id:{alternate_flow}")];
        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);
        assert_eq!(contract["selected_flow_set"].as_str(), Some(alternate_flow));
        assert!(contract["lane_catalog"].as_object().is_some_and(|catalog| {
            !catalog.is_empty()
                && catalog.values().all(|lane| {
                    lane["node_id"].as_str().is_some() && lane["included"].as_bool() == Some(true)
                })
        }));
    }

    #[test]
    fn design_first_bootstrap_canonicalizes_moved_project_activator_test_proof_target() {
        let request = "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation. Proof target: cargo test -p vida project_activator_command_accepts_json_output -- --nocapture. After a green bounded result, continue with the normal commit, push, release build, and system binary update cycle.";

        let bootstrap = build_design_first_tracked_flow_bootstrap(request);
        assert_eq!(bootstrap["status"], "blocked");
        assert_eq!(bootstrap["executable"], false);
        assert_eq!(bootstrap["view_only"], true);
        assert!(bootstrap["request"]
            .as_str()
            .is_some_and(|value| value.contains(
                "cargo test -p vida project_activator_surface::tests::project_activator_command_accepts_json_output -- --exact --nocapture"
            )));
        assert!(bootstrap.get("bootstrap_command").is_none());
        assert!(bootstrap["epic"]["task_id"].is_null());
    }

    #[test]
    fn design_first_work_packet_bootstrap_has_no_executable_task_graph() {
        let bootstrap = build_design_first_tracked_flow_bootstrap(
            "Research the feature, write detailed specifications, create a plan, and implement runtime flow",
        );
        assert_eq!(bootstrap["status"], "blocked");
        assert_eq!(
            bootstrap["activation_semantics"],
            "configured_team_flow_relation_required"
        );
        assert_eq!(
            bootstrap["schema_vocabulary"],
            json!(["epic", "spec_task", "work_pool_task", "dev_task"])
        );
        assert!(bootstrap.get("work_pool_task").is_none());
        assert!(bootstrap.get("dev_task").is_none());
    }

    #[test]
    fn implementation_analysis_route_overrides_materialize_analysis_policy_knobs() {
        let implementation = json!({
            "analysis_executor_backend": "opencode_cli",
            "analysis_external_first_required": "yes",
            "analysis_fanout_executor_backends": ["hermes_cli", "opencode_cli"],
            "analysis_fanout_min_results": 2,
            "analysis_fanout_subagents": "hermes_cli,opencode_cli",
            "analysis_merge_policy": "consensus_with_conflict_flag",
        });
        let mut analysis = json!({
            "executor_backend": "internal_subagents",
            "fanout_min_results": 1,
            "merge_policy": "first_success",
        });

        apply_implementation_analysis_route_overrides(&mut analysis, &implementation);

        assert_eq!(analysis["executor_backend"], "opencode_cli");
        assert_eq!(analysis["external_first_required"], "yes");
        assert_eq!(
            analysis["fanout_executor_backends"],
            json!(["hermes_cli", "opencode_cli"])
        );
        assert_eq!(analysis["fanout_min_results"], 2);
        assert_eq!(analysis["fanout_subagents"], "hermes_cli,opencode_cli");
        assert_eq!(analysis["merge_policy"], "consensus_with_conflict_flag");
    }

    #[test]
    fn dispatch_contract_blocks_readiness_only_legacy_shape() {
        let bundle = json!({
            "dev_team_readiness": {
                "roles": [
                    {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "task_classes": ["implementation"],
                        "packet_template_kind": "delivery_task_packet",
                        "closure_class": "implementation",
                        "stage": "execution",
                        "completion_blocker": "configured_implementation_blocker",
                        "inclusion_rule": "always"
                    },
                    {"role_id": "reviewer", "runtime_role": "verifier", "task_classes": ["review"]}
                ],
                "flows": [
                    {
                        "flow_id": "configured_flow",
                        "enabled": true,
                        "ordered_steps": [
                            {"role_id": "analyst"},
                            {"role_id": "developer"},
                            {
                                "role_id": "reviewer",
                                "packet_template_kind": "verifier_proof_packet",
                                "closure_class": "proof",
                                "stage": "execution",
                                "completion_blocker": "configured_review_blocker",
                                "inclusion_rule": "when_flow_requires_verification"
                            }
                        ]
                    }
                ]
            },
            "carrier_runtime": {
                "dispatch_aliases": []
            }
        });
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "configured".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "configured flow".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec!["dev_team_flow_id:configured_flow".to_string()],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, true);

        assert_eq!(contract["status"], "blocked");
        assert_eq!(contract["lane_sequence"], json!([]));
        assert!(
            contract["blocker_codes"][0]
                .as_str()
                .is_some_and(|code| code.contains("team_flow_authority_missing"))
        );
    }

    #[test]
    fn dispatch_contract_does_not_execute_partial_conditional_readiness_shape() {
        let bundle = json!({
            "dev_team_readiness": {
                "roles": [
                    {"role_id": "coder", "runtime_role": "worker", "task_classes": ["implementation"]},
                    {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                    {"role_id": "coach_validator", "runtime_role": "coach", "task_classes": ["review"]},
                    {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
                ],
                "flows": [
                    {
                        "flow_id": "adaptive-task-flow",
                        "enabled": true,
                        "ordered_steps": [
                            {"role_id": "coder", "inclusion_rule": "always"},
                            {"role_id": "tester", "inclusion_rule": "when_proof_required"},
                            {"role_id": "coach_validator", "inclusion_rule": "when_review_triggered"},
                            {"role_id": "architect", "inclusion_rule": "when_architecture_triggered"}
                        ]
                    }
                ]
            },
            "carrier_runtime": {
                "dispatch_aliases": []
            }
        });
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "configured".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "routine adaptive flow".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec!["dev_team_flow_id:adaptive-task-flow".to_string()],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);

        assert_eq!(contract["status"], "blocked");
        assert_eq!(contract["resolved_lanes"], json!([]));
    }

    #[test]
    fn dispatch_contract_does_not_infer_authority_from_carrier_runtime() {
        let bundle = json!({
            "agent_system": {
                "routing": {
                    "default": {
                        "executor_backend": "internal_subagents"
                    }
                }
            },
            "dev_team_readiness": {
                "roles": [
                    {"role_id": "designer", "runtime_role": "designer", "task_classes": ["design"]},
                    {"role_id": "autotester", "runtime_role": "worker", "task_classes": ["implementation_medium"]}
                ],
                "flows": [
                    {
                        "flow_id": "configured_autotester_flow",
                        "enabled": true,
                        "ordered_steps": [
                            {"role_id": "designer"},
                            {"role_id": "autotester"}
                        ]
                    }
                ]
            },
            "carrier_runtime": {
                "model_selection": {
                    "enabled": true,
                    "candidate_scope": "unified_carrier_model_profiles",
                    "default_strategy": "balanced_cost_quality"
                },
                "roles": [
                    {
                        "role_id": "middle",
                        "tier": "middle",
                        "rate": 4,
                        "normalized_cost_units": 4,
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation_medium"],
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
                                "task_classes": ["implementation_medium"],
                                "readiness": { "required": true, "ready": true }
                            }
                        }
                    }
                ]
            }
        });
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "configured".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "configured flow".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec!["dev_team_flow_id:configured_autotester_flow".to_string()],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);
        assert_eq!(contract["status"], "blocked");
        assert_eq!(contract["lane_catalog"], json!({}));
    }

    #[test]
    fn dispatch_contract_fails_closed_when_team_flow_authority_is_absent() {
        let bundle = json!({});
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "legacy".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "legacy flow".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec![],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);

        assert_eq!(contract["status"], "blocked");
        assert_eq!(contract["lane_sequence"], json!([]));
    }

    #[test]
    fn strict_team_flow_projection_preserves_typed_authority_fields() {
        let bundle = strict_team_flow_bundle();
        let selected_node_id =
            crate::team_flow_authority_adapter::TeamFlowExecutionAuthority::require(
                &bundle, None, None,
            )
            .expect("strict authority")
            .ordered_nodes()
            .find(|node| node.node.included)
            .map(|node| node.node.node_id.clone())
            .expect("strict authority selected node");
        let mut selection = strict_team_flow_selection(&bundle);
        selection.execution_plan = json!({
            "team_flow_authority_selected_node_id": selected_node_id
        });

        let flow = configured_dev_team_flow_templates(&bundle, &selection)
            .expect("strict configured flow should resolve");

        let persisted_flow =
            &bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0];
        let persisted_lanes = persisted_flow["lanes"]
            .as_array()
            .expect("persisted lanes must be an array")
            .iter()
            .filter(|lane| lane["included"].as_bool() == Some(true))
            .collect::<Vec<_>>();
        assert_eq!(flow.flow_id, persisted_flow["flow_id"]);
        assert_eq!(flow.lanes.len(), persisted_lanes.len());
        let worker = &flow.lanes[0];
        let persisted_lane = &persisted_lanes[0];
        assert_eq!(worker.node_id, persisted_lane["node_id"]);
        assert_eq!(worker.lane_id, persisted_lane["lane_id"]);
        assert_eq!(worker.dispatch_target, persisted_lane["dispatch_target"]);
        assert_eq!(worker.dispatch_alias, persisted_lane["dispatch_alias"]);
        assert_eq!(worker.runtime_role, persisted_lane["runtime_role"]);
        assert_eq!(worker.task_class, persisted_lane["task_class"]);
        let persisted_rework_targets = persisted_lane["rework"]["targets"]
            .as_array()
            .expect("persisted rework targets must be an array")
            .iter()
            .map(|target| {
                target
                    .as_str()
                    .expect("persisted rework target")
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(worker.rework_targets, persisted_rework_targets);
        assert_eq!(
            worker.profile_authority,
            persisted_lane["profile_authority"]
        );
        assert_eq!(
            worker.selected_model_profile,
            persisted_lane["selected_model_profile"]
        );
        assert_eq!(worker.carrier_relation, persisted_lane["carrier_relation"]);
        assert_eq!(
            worker.executor_backend_relation,
            persisted_lane["executor_backend_relation"]
        );
        assert_eq!(worker.assignment, persisted_lane["runtime_assignment"]);
    }

    #[test]
    fn resolved_lane_fields_match_the_closed_authority_schema() {
        let bundle = strict_team_flow_bundle();
        let selection = strict_team_flow_selection(&bundle);
        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                repository_root.join("vida/config/schemas/team-flow-authority.schema.json"),
            )
            .expect("TeamFlow schema should be readable"),
        )
        .expect("TeamFlow schema should parse");
        let lane_schema = &schema["$defs"]["lane"];
        assert_eq!(lane_schema["additionalProperties"], false);
        let schema_fields = lane_schema["properties"]
            .as_object()
            .expect("lane schema properties must be an object")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let required_fields = lane_schema["required"]
            .as_array()
            .expect("lane schema required fields must be an array")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("lane required fields must be strings")
                    .to_string()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(schema_fields, required_fields);
        let persisted_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flows")
            .iter()
            .find(|flow| flow["flow_id"] == contract["selected_flow_set"])
            .expect("selected persisted flow");
        let persisted_lanes = persisted_flow["lanes"].as_array().expect("persisted lanes");
        let included_node_ids = persisted_lanes
            .iter()
            .filter(|lane| lane["included"].as_bool() == Some(true))
            .filter_map(|lane| lane["node_id"].as_str().map(str::to_string))
            .collect::<std::collections::BTreeSet<_>>();
        let excluded_node_ids = persisted_lanes
            .iter()
            .filter(|lane| lane["included"].as_bool() == Some(false))
            .filter_map(|lane| lane["node_id"].as_str().map(str::to_string))
            .collect::<std::collections::BTreeSet<_>>();
        let emitted_lanes = contract["resolved_lanes"]
            .as_array()
            .expect("resolved lanes must be an array");
        let emitted_node_ids = emitted_lanes
            .iter()
            .filter_map(|lane| lane["node_id"].as_str().map(str::to_string))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(emitted_node_ids, included_node_ids);
        assert!(emitted_node_ids.is_disjoint(&excluded_node_ids));

        for lane in emitted_lanes {
            let emitted_fields = lane
                .as_object()
                .expect("resolved lane must be an object")
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(emitted_fields, schema_fields);
            let persisted_lane = persisted_lanes
                .iter()
                .find(|candidate| candidate["node_id"] == lane["node_id"])
                .expect("emitted lane must retain persisted identity");
            assert_eq!(lane["carrier_relation"], persisted_lane["carrier_relation"]);
            assert_eq!(
                lane["executor_backend_relation"],
                persisted_lane["executor_backend_relation"]
            );
            assert_eq!(lane["dispatch_alias"], persisted_lane["dispatch_alias"]);
            assert_eq!(
                lane["runtime_assignment"],
                persisted_lane["runtime_assignment"]
            );
            assert_eq!(
                lane["carrier_runtime_assignment"],
                persisted_lane["carrier_runtime_assignment"]
            );
        }
    }

    #[test]
    fn dispatch_contract_preserves_persisted_policy_diagnostics() {
        let bundle = strict_team_flow_bundle();
        let selection = strict_team_flow_selection(&bundle);
        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);
        let persisted_flow = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flow payload")
            .iter()
            .find(|flow| flow["flow_id"] == contract["selected_flow_set"])
            .expect("selected persisted flow");
        for lane in contract["resolved_lanes"]
            .as_array()
            .expect("resolved lanes")
        {
            let persisted_lane = persisted_flow["lanes"]
                .as_array()
                .expect("persisted lanes")
                .iter()
                .find(|candidate| candidate["node_id"] == lane["node_id"])
                .expect("persisted lane");
            assert_eq!(
                lane["policy_diagnostics"],
                persisted_lane["policy_diagnostics"]
            );
        }
    }

    #[test]
    fn supported_autonomous_execution_settings_excludes_unwired_overlay_toggles() {
        let settings = supported_autonomous_execution_settings(&json!({
            "autonomous_execution": {
                "agent_only_development": true,
                "validation_report_required_before_implementation": true,
                "spec_ready_auto_development": true,
                "resume_after_validation_gate": true
            }
        }));

        assert_eq!(
            settings,
            json!({
                "agent_only_development": true,
                "validation_report_required_before_implementation": true
            })
        );
    }

    #[test]
    fn configured_dispatch_relations_follow_configured_order_and_identity() {
        let lanes = vec![
            json!({
                "node_id": "proof-node",
                "task_class": "verification",
                "runtime_role": "verifier",
                "packet_template_kind": "configured-proof",
                "activation": {"runtime_role": "verifier"}
            }),
            json!({
                "node_id": "write-node",
                "task_class": "implementation",
                "runtime_role": "worker",
                "packet_template_kind": "configured-write",
                "activation": {"runtime_role": "worker"}
            }),
            json!({
                "node_id": "spec-node",
                "task_class": "specification",
                "runtime_role": "business_analyst",
                "packet_template_kind": "configured-spec",
                "activation": {"runtime_role": "business_analyst"}
            }),
        ];
        let relations = derive_configured_dispatch_relations(&lanes);
        assert!(relations.blockers.is_empty());
        assert_eq!(
            relations.packet_families,
            vec!["configured-proof", "configured-write", "configured-spec"]
        );
        assert_eq!(
            relations.packet_family_by_task_class["implementation"],
            "configured-write"
        );
        assert_eq!(
            relations.activation_by_task_class["verification"],
            json!({"runtime_role": "verifier"})
        );
        assert_eq!(
            relations.design_owner_runtime_role.as_deref(),
            Some("business_analyst")
        );
    }

    #[test]
    fn configured_dispatch_relations_missing_and_ambiguous_bindings_fail_closed() {
        let lanes = vec![
            json!({
                "node_id": "missing-node",
                "task_class": "implementation",
                "runtime_role": "worker",
                "packet_template_kind": "",
                "activation": null
            }),
            json!({
                "node_id": "first-proof",
                "task_class": "verification",
                "runtime_role": "verifier",
                "packet_template_kind": "proof-a",
                "activation": {"runtime_role": "verifier"}
            }),
            json!({
                "node_id": "second-proof",
                "task_class": "verification",
                "runtime_role": "verifier",
                "packet_template_kind": "proof-b",
                "activation": {"runtime_role": "prover"}
            }),
        ];
        let blockers = derive_configured_dispatch_relations(&lanes).blockers;
        assert!(blockers.iter().any(|code| {
            code == "team_flow_authority_packet_template_kind_missing:missing-node"
        }));
        assert!(
            blockers
                .iter()
                .any(|code| code == "team_flow_authority_activation_missing:missing-node")
        );
        assert!(blockers.iter().any(|code| {
            code == "team_flow_authority_task_class_packet_template_ambiguous:verification"
        }));
        assert!(blockers.iter().any(|code| {
            code == "team_flow_authority_task_class_activation_ambiguous:verification"
        }));
    }

    #[test]
    fn configured_tracked_flow_binding_order_variance_and_missing_binding_fail_closed() {
        let bundle = strict_team_flow_bundle();
        let mut selection = strict_team_flow_selection(&bundle);
        selection.tracked_flow_entry = Some("work-pool-pack".to_string());
        let sequence = configured_tracked_flow_sequence(&bundle, &selection, true)
            .expect("configured tracked-flow entry should resolve");
        assert!(sequence.contains(&"spec-pack".to_string()));
        assert!(sequence.contains(&"work-pool-pack".to_string()));

        selection.tracked_flow_entry = None;
        let missing = configured_tracked_flow_sequence(&bundle, &selection, true)
            .expect_err("missing tracked-flow binding must fail closed");
        assert_eq!(
            missing,
            vec!["team_flow_authority_tracked_flow_binding_missing"]
        );

        selection.tracked_flow_entry = Some("unknown-pack".to_string());
        let unknown = configured_tracked_flow_sequence(&bundle, &selection, true)
            .expect_err("unknown tracked-flow binding must fail closed");
        assert_eq!(
            unknown,
            vec!["team_flow_authority_tracked_flow_binding_unknown:unknown-pack"]
        );
    }

    #[test]
    fn configured_dispatch_relations_table_is_order_invariant_and_fail_closed() {
        let cases = vec![
            (
                "entry_not_first",
                vec![
                    json!({"node_id":"proof","task_class":"verification","runtime_role":"verifier","packet_template_kind":"proof","activation":{"runtime_role":"verifier"}}),
                    json!({"node_id":"entry","task_class":"implementation","runtime_role":"worker","packet_template_kind":"write","activation":{"runtime_role":"worker"}}),
                ],
                None,
                vec!["proof", "write"],
            ),
            (
                "zero",
                Vec::new(),
                Some("team_flow_authority_packet_relation_missing"),
                vec![],
            ),
            (
                "missing",
                vec![
                    json!({"node_id":"missing","task_class":"implementation","runtime_role":"worker","packet_template_kind":"","activation":null}),
                ],
                Some("team_flow_authority_packet_template_kind_missing:missing"),
                vec![],
            ),
            (
                "ambiguous",
                vec![
                    json!({"node_id":"a","task_class":"verification","runtime_role":"verifier","packet_template_kind":"proof-a","activation":{"runtime_role":"verifier"}}),
                    json!({"node_id":"b","task_class":"verification","runtime_role":"verifier","packet_template_kind":"proof-b","activation":{"runtime_role":"prover"}}),
                ],
                Some("team_flow_authority_task_class_packet_template_ambiguous:verification"),
                vec!["proof-a", "proof-b"],
            ),
        ];

        for (name, lanes, blocker, expected_families) in cases {
            let relations = derive_configured_dispatch_relations(&lanes);
            assert_eq!(relations.packet_families, expected_families, "{name}");
            match blocker {
                Some(blocker) => assert!(
                    relations.blockers.iter().any(|value| value == blocker),
                    "{name} missing blocker {blocker}: {:?}",
                    relations.blockers
                ),
                None => assert!(
                    relations.blockers.is_empty(),
                    "{name}: {:?}",
                    relations.blockers
                ),
            }
        }
    }
}
