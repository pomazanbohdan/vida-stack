use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use super::*;

pub(crate) fn build_runtime_closure_admission(
    bundle_check: &TaskflowConsumeBundleCheck,
    docflow_verdict: &RuntimeConsumptionDocflowVerdict,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> RuntimeConsumptionClosureAdmission {
    let mut blockers = Vec::new();
    if !bundle_check.ok {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
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
        if let Some(code) = crate::release1_contracts::blocker_code_value(
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
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::RestoreReconcileNotGreen,
        ) {
            blockers.push(code);
        }
    }
    if role_selection.execution_plan["status"] == "design_first" {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::PendingDesignPacket,
        ) {
            blockers.push(code);
        }
        if let Some(code) = crate::release1_contracts::blocker_code_value(
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
    let selected_backend = activation_agent_type
        .clone()
        .or_else(|| receipt.selected_backend.clone())
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

fn agent_init_command_for_packet_path(packet_path: &str) -> String {
    format!(
        "vida agent-init {} {} --json",
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

fn load_project_overlay_yaml_for_root(project_root: &Path) -> Result<serde_yaml::Value, String> {
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

fn selected_host_cli_system_for_runtime_dispatch(
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

fn selected_external_backend_for_system(
    overlay: &serde_yaml::Value,
    system: &str,
    preferred_backend: Option<&str>,
) -> Option<(String, serde_yaml::Value)> {
    let subagents = yaml_lookup(overlay, &["agent_system", "subagents"])?;
    let entries = subagents.as_mapping()?;
    let preferred_key = format!("{system}_cli");
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
                != Some("external_cli")
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
            != Some("external_cli")
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
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        parts.push(workdir_flag);
        parts.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    if prompt_mode == "positional" {
        parts.push(external_cli_activation_prompt(packet_path));
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

fn configured_external_activation_parts(
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
    if let Some(workdir_flag) = yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        args.push(workdir_flag);
        args.push(project_root.display().to_string());
    }
    let prompt_mode = yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => args.push(external_cli_activation_prompt(packet_path)),
        other => {
            return Err(format!(
                "Configured external backend uses unsupported prompt_mode `{other}`"
            ))
        }
    }
    Ok((command, args))
}

fn render_command_display(command: &str, args: &[String]) -> String {
    let mut rendered = Vec::with_capacity(args.len() + 1);
    rendered.push(shell_quote(command));
    rendered.extend(args.iter().map(|arg| shell_quote(arg)));
    rendered.join(" ")
}

fn runtime_agent_lane_dispatch_from_overlay(
    overlay: Option<&serde_yaml::Value>,
    selected_cli_system: &str,
    selected_execution_class: &str,
    project_root: &Path,
    packet_path: &str,
    preferred_backend: Option<&str>,
) -> RuntimeAgentLaneDispatch {
    let agent_init_command = agent_init_command_for_packet_path(packet_path);
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
    let activation_command =
        configured_external_activation_command(&backend_entry, project_root, packet_path)
            .unwrap_or_else(|| agent_init_command_for_packet_path(packet_path));
    RuntimeAgentLaneDispatch {
        surface: format!("external_cli:{backend_id}"),
        activation_command,
        backend_dispatch: serde_json::json!({
            "selected_cli_system": selected_cli_system,
            "selected_execution_class": selected_execution_class,
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

fn runtime_dispatch_packet_kind(
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

pub(crate) fn derive_downstream_dispatch_preview(
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
        execution_plan_agent_only_development_required(&role_selection.execution_plan);
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
                        .and_then(canonical_lane_target_for_runtime_role)
                        .map(str::to_string)
                });
            let current_index = current_index.or_else(|| {
                receipt
                    .activation_runtime_role
                    .as_deref()
                    .and_then(canonical_lane_target_for_runtime_role)
                    .and_then(|target| {
                        execution_lane_sequence
                            .iter()
                            .position(|candidate| candidate == target)
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
                let has_lane_evidence = receipt.dispatch_status == "executed"
                    || receipt
                        .dispatch_result_path
                        .as_deref()
                        .is_some_and(|path| !path.trim().is_empty());
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

pub(crate) fn refresh_downstream_dispatch_preview(
    state_root: &Path,
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
    ) = derive_downstream_dispatch_preview(role_selection, receipt);
    if let Some(error) = downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    receipt.downstream_dispatch_target = downstream_dispatch_target;
    receipt.downstream_dispatch_command = downstream_dispatch_command;
    receipt.downstream_dispatch_note = downstream_dispatch_note;
    receipt.downstream_dispatch_ready = downstream_dispatch_ready;
    receipt.downstream_dispatch_blockers = downstream_dispatch_blockers;
    receipt.downstream_dispatch_status = None;
    receipt.downstream_dispatch_result_path = None;
    receipt.downstream_dispatch_trace_path = None;
    receipt.downstream_dispatch_active_target = active_downstream_dispatch_target(receipt);
    receipt.downstream_dispatch_last_target = None;
    receipt.downstream_dispatch_executed_count = 0;
    receipt.downstream_dispatch_packet_path = write_runtime_downstream_dispatch_packet(
        state_root,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    Ok(())
}

fn runtime_packet_handoff_task_class(
    dispatch_target: &str,
    handoff_runtime_role: &str,
) -> &'static str {
    match dispatch_target {
        "specification" => "specification",
        "planning" => "planning",
        "coach" => "coach",
        "verification" => "verification",
        "escalation" => "architecture",
        "implementer" => "implementation",
        _ => match handoff_runtime_role {
            "business_analyst" => "specification",
            "pm" => "planning",
            "coach" => "coach",
            "verifier" => "verification",
            "solution_architect" => "architecture",
            _ => "implementation",
        },
    }
}

pub(crate) fn runtime_delivery_task_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
    request_text: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "backlog_id": run_id,
        "release_slice": "none",
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Execute bounded `{dispatch_target}` handoff for the active runtime request"),
        "non_goals": [
            "unbounded repository-wide rewrites",
            "out-of-scope taskflow state mutation"
        ],
        "scope_in": [
            format!("dispatch_target:{dispatch_target}"),
            format!("runtime_role:{handoff_runtime_role}")
        ],
        "scope_out": [
            "mutation outside bounded packet scope",
            "closure without recorded handoff evidence"
        ],
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "inputs": [
            "role_selection_full",
            "run_graph_bootstrap",
            "taskflow_handoff_plan"
        ],
        "outputs": [
            "dispatch_result_artifact",
            "updated_run_graph_dispatch_receipt"
        ],
        "definition_of_done": [
            format!("`{dispatch_target}` handoff produces a bounded runtime result artifact"),
            "dispatch receipt and downstream preview are refreshed consistently"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop after writing bounded dispatch result or explicit blocker",
            "do not widen scope beyond the active packet target"
        ],
        "blocking_question": format!("What is the next bounded action required for `{dispatch_target}`?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier",
        "request_excerpt": request_text.chars().take(240).collect::<String>(),
    })
}

pub(crate) fn runtime_execution_block_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::execution-block"),
        "parent_packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "backlog_id": run_id,
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Resolve bounded execution blocker for `{dispatch_target}`"),
        "scope_in": [
            format!("dispatch_target:{dispatch_target}")
        ],
        "scope_out": [
            "new feature scope without bounded packet update"
        ],
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "bounded blocker is resolved with receipt-backed evidence"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime receipt evidence that blocker is resolved or escalated",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop once blocker resolution evidence is recorded"
        ],
        "blocking_question": format!("Which explicit blocker prevents closing `{dispatch_target}` now?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier"
    })
}

pub(crate) fn runtime_coach_review_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::coach-review"),
        "source_packet_id": format!("{run_id}::implementer::delivery"),
        "review_goal": format!("Judge whether `{dispatch_target}` remains aligned with the approved bounded packet, acceptance criteria, and definition of done"),
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "coach review returns bounded approval-forward or bounded rework evidence"
        ],
        "proof_target": proof_target,
        "active_skills": "no_applicable_skill",
        "review_focus": [
            "spec_conformance",
            "acceptance_criteria_alignment",
            "bounded_scope_drift"
        ],
        "blocking_question": format!("Does `{dispatch_target}` match the approved bounded contract cleanly enough to proceed?"),
        "handoff_runtime_role": "coach",
        "handoff_task_class": "coach",
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_verifier_proof_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::verifier-proof"),
        "source_packet_id": format!("{run_id}::implementer::delivery"),
        "proof_goal": format!("Independently verify bounded closure readiness for `{dispatch_target}`"),
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": proof_target,
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("What proof is still missing before `{dispatch_target}` can close?"),
        "handoff_runtime_role": "verifier",
        "handoff_task_class": "verification",
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_escalation_packet(run_id: &str, dispatch_target: &str) -> serde_json::Value {
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::escalation"),
        "source_packet_id": format!("{run_id}::{dispatch_target}::delivery"),
        "conflict_type": "architecture",
        "decision_needed": format!("Resolve the bounded architecture-preparation or escalation decision for `{dispatch_target}`"),
        "options": [
            "approve current bounded route",
            "reshape bounded handoff",
            "block execution pending architectural clarification"
        ],
        "constraints": [
            "preserve one bounded packet owner",
            "do not widen scope without a new bounded packet"
        ],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("Which architectural decision is required before `{dispatch_target}` can proceed coherently?"),
        "handoff_runtime_role": "solution_architect",
        "handoff_task_class": "architecture",
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_tracked_flow_packet(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
    dispatch_target: &str,
) -> serde_json::Value {
    let tracked_packet_key = match dispatch_target {
        "spec-pack" => "spec_task",
        "work-pool-pack" => "work_pool_task",
        "dev-pack" => "dev_task",
        _ => "",
    };
    let tracked_flow_bootstrap = if role_selection.execution_plan["tracked_flow_bootstrap"]
        [tracked_packet_key]["task_id"]
        .as_str()
        .is_some()
    {
        role_selection.execution_plan["tracked_flow_bootstrap"].clone()
    } else {
        build_design_first_tracked_flow_bootstrap(&role_selection.request)
    };
    let tracked = tracked_flow_bootstrap
        .get(tracked_packet_key)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "packet_id": format!("{run_id}::{dispatch_target}::tracked-flow"),
        "dispatch_target": dispatch_target,
        "tracked_packet_key": tracked_packet_key,
        "activation_semantics": "tracked_flow_materialization_only",
        "view_only": true,
        "executes_packet": false,
        "transfers_root_session_write_authority": false,
        "task_id": tracked["task_id"],
        "title": tracked["title"],
        "runtime": tracked["runtime"],
        "inspect_command": tracked["inspect_command"],
        "ensure_command": tracked["ensure_command"],
        "next_command": tracked["ensure_command"],
        "create_command": tracked["create_command"],
        "close_command": tracked["close_command"],
        "required": tracked["required"],
        "request": role_selection.request,
    })
}

pub(crate) fn runtime_packet_prompt(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    request_text: &str,
    orchestration_contract: &serde_json::Value,
) -> String {
    let replan_points = orchestration_contract["replanning"]["checkpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Packet run_id={run_id}\nTarget={dispatch_target}\nRuntime role={handoff_runtime_role}\nRoot session role=orchestrator\nExecution mode=delegated_orchestration_cycle\nCanonical delegated execution surface=vida agent-init\nThis packet activation view is not an execution receipt and does not transfer root-session write authority.\nHost subagent APIs are backend details only; do not substitute them for the project runtime's delegated lane contract.\nHost-local shell/edit capability is not a write-authority receipt.\nFirst substantive response: publish a concise plan before edits or implementation.\nLocal orchestrator coding is forbidden without an explicit exception path.\nBefore any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`.\nUnder continued-development intent, stay in commentary/progress mode; final closure wording is forbidden unless the user explicitly asks to stop.\nDo not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\nIf closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting.\nAfter any bounded result, green test, successful build, or delegated handoff, immediately bind the next lawful continuation item instead of pausing at a summary.\nWhen recording task notes from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\nFinding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session fallback; wait, reroute, or record the exception path first.\nReplan checkpoints: {replan_points}\nGoal: execute only this bounded handoff and produce receipt-backed evidence.\nRequest: {request_text}"
    )
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
            ))
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
        "agent_lane" => Some(
            runtime_agent_lane_dispatch_for_root(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                packet_path,
                receipt.selected_backend.as_deref(),
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
    use crate::state_store::RunGraphDispatchReceipt;
    use serde_json::json;

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
            let host_runtime = runtime_host_execution_contract_for_root(&project_root);
            if json_string(host_runtime.get("selected_cli_execution_class")).as_deref()
                == Some("external")
            {
                return execute_external_agent_lane_dispatch(
                    store,
                    &project_root,
                    dispatch_packet_path,
                    receipt.selected_backend.as_deref(),
                    role_selection,
                    receipt,
                    host_runtime,
                )
                .await;
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
                receipt.selected_backend.as_deref(),
                role_selection,
                host_runtime,
            ))
        }
    }
}

fn agent_lane_dispatch_result(
    mut activation_view: serde_json::Value,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    host_runtime: serde_json::Value,
) -> serde_json::Value {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let lane_dispatch = runtime_agent_lane_dispatch_for_root(
        &project_root,
        dispatch_packet_path,
        preferred_backend,
    );
    let body = activation_view
        .as_object_mut()
        .expect("agent-init activation view should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(lane_dispatch.surface),
    );
    body.insert("status".to_string(), serde_json::json!("pass"));
    body.insert(
        "execution_state".to_string(),
        serde_json::json!("packet_ready"),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(lane_dispatch.activation_command),
    );
    body.insert(
        "dispatch_packet_path".to_string(),
        serde_json::json!(dispatch_packet_path),
    );
    body.insert("host_runtime".to_string(), host_runtime);
    body.insert(
        "backend_dispatch".to_string(),
        lane_dispatch.backend_dispatch,
    );
    body.insert(
        "role_selection".to_string(),
        serde_json::to_value(role_selection).expect("lane selection should serialize"),
    );
    activation_view
}

async fn execute_external_agent_lane_dispatch(
    store: &StateStore,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let overlay = load_project_overlay_yaml_for_root(project_root)?;
    let (selected_cli_system, _) = selected_host_cli_system_for_runtime_dispatch(&overlay);
    let (backend_id, backend_entry) =
        selected_external_backend_for_system(&overlay, &selected_cli_system, preferred_backend)
            .ok_or_else(|| {
                format!(
                    "Configured host CLI system `{selected_cli_system}` has no enabled external backend dispatch adapter"
                )
            })?;
    let (command, args) =
        configured_external_activation_parts(&backend_entry, project_root, dispatch_packet_path)?;
    let activation_command = render_command_display(&command, &args);

    let mut process = std::process::Command::new(&command);
    process.args(&args).current_dir(project_root);
    if let Some(serde_yaml::Value::Mapping(env_map)) =
        yaml_lookup(&backend_entry, &["dispatch", "env"])
    {
        for (key, value) in env_map {
            if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                process.env(key, value);
            }
        }
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }
    if let Some(selected_backend) = receipt.selected_backend.as_deref() {
        process.env("VIDA_SELECTED_BACKEND", selected_backend);
    }

    let output = process.output().map_err(|error| {
        format!(
            "Failed to execute configured external backend `{backend_id}` via `{command}`: {error}"
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let success = output.status.success();
    let exit_code = output.status.code();
    let activation_view = crate::init_surfaces::render_agent_init_packet_activation_with_store(
        store,
        project_root,
        dispatch_packet_path,
        false,
    )
    .await
    .unwrap_or_else(|_| {
        serde_json::json!({
            "selection": {
                "mode": "dispatch_packet",
                "selected_role": receipt
                    .activation_runtime_role
                    .as_deref()
                    .unwrap_or(&role_selection.selected_role),
            },
            "activation_semantics": {
                "activation_kind": "activation_view",
                "view_only": true,
            },
        })
    });
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        host_runtime,
    );
    let body = result
        .as_object_mut()
        .expect("agent lane dispatch result should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(format!("external_cli:{backend_id}")),
    );
    body.insert(
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if !success {
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
    }
    Ok(result)
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
    let encoded = serde_json::to_string_pretty(body)
        .map_err(|error| format!("Failed to encode dispatch result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write dispatch result: {error}"))?;
    Ok(result_path.display().to_string())
}

pub(crate) async fn execute_and_record_dispatch_receipt(
    state_root: &Path,
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
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
        } else {
            None
        };
    if let Some(dispatch_command) = json_string(execution_result.get("activation_command")) {
        receipt.dispatch_command = Some(dispatch_command);
    }
    refresh_downstream_dispatch_preview(state_root, role_selection, run_graph_bootstrap, receipt)?;
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
    let root_lane_has_execution_evidence = root_receipt.dispatch_status == "executed"
        || (root_receipt.dispatch_status == "packet_ready"
            && root_receipt
                .dispatch_result_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()));
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
            derive_downstream_dispatch_preview(role_selection, &downstream_receipt);
        if let Some(error) =
            downstream_dispatch_ready_blocker_parity_error(next_ready, &next_blockers)
        {
            return Err(error);
        }
        downstream_receipt.downstream_dispatch_target = next_target;
        downstream_receipt.downstream_dispatch_command = next_command;
        downstream_receipt.downstream_dispatch_note = next_note;
        downstream_receipt.downstream_dispatch_ready = next_ready;
        downstream_receipt.downstream_dispatch_blockers = next_blockers;
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
        downstream_receipt.downstream_dispatch_status =
            Some(downstream_receipt.dispatch_status.clone());
        downstream_receipt.downstream_dispatch_result_path =
            downstream_receipt.dispatch_result_path.clone();
        downstream_receipt.downstream_dispatch_active_target =
            active_downstream_dispatch_target(&downstream_receipt);
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

fn downstream_dispatch_packet_body(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: Option<&Path>,
) -> serde_json::Value {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let downstream_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .unwrap_or_default();
    let handoff_runtime_role = receipt
        .activation_runtime_role
        .as_deref()
        .unwrap_or(role_selection.selected_role.as_str());
    let packet_template_kind = if downstream_target.is_empty() {
        "delivery_task_packet".to_string()
    } else {
        runtime_dispatch_packet_kind(
            &role_selection.execution_plan,
            downstream_target,
            if matches!(
                downstream_target,
                "spec-pack" | "work-pool-pack" | "dev-pack"
            ) {
                "taskflow_pack"
            } else {
                "agent_lane"
            },
        )
    };
    let activation_command = packet_path
        .and_then(|path| path.to_str())
        .map(agent_init_command_for_packet_path);
    let handoff_task_class =
        runtime_packet_handoff_task_class(downstream_target, handoff_runtime_role);
    let closure_class = dispatch_contract_lane(&role_selection.execution_plan, downstream_target)
        .and_then(|lane| lane["closure_class"].as_str())
        .unwrap_or("implementation");
    let delivery_task_packet = runtime_delivery_task_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        &role_selection.request,
    );
    let execution_block_packet = runtime_execution_block_packet(
        &receipt.run_id,
        downstream_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
    );
    serde_json::json!({
        "packet_kind": "runtime_downstream_dispatch_packet",
        "packet_template_kind": packet_template_kind,
        "delivery_task_packet": if packet_template_kind == "delivery_task_packet" {
            delivery_task_packet
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
                &receipt.run_id,
                downstream_target,
                "bounded implementation result versus approved spec and definition of done",
            )
        } else {
            serde_json::Value::Null
        },
        "verifier_proof_packet": if packet_template_kind == "verifier_proof_packet" {
            runtime_verifier_proof_packet(
                &receipt.run_id,
                downstream_target,
                "independent bounded proof and closure readiness",
            )
        } else {
            serde_json::Value::Null
        },
        "escalation_packet": if packet_template_kind == "escalation_packet" {
            runtime_escalation_packet(&receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
        "tracked_flow_packet": if packet_template_kind == "tracked_flow_packet" {
            runtime_tracked_flow_packet(role_selection, &receipt.run_id, downstream_target)
        } else {
            serde_json::Value::Null
        },
        "prompt": runtime_packet_prompt(
            &receipt.run_id,
            downstream_target,
            handoff_runtime_role,
            &role_selection.request,
            &role_selection.execution_plan["orchestration_contract"],
        ),
        "recorded_at": receipt.recorded_at,
        "run_id": receipt.run_id,
        "source_dispatch_target": receipt.dispatch_target,
        "source_dispatch_status": receipt.dispatch_status,
        "source_lane_status": receipt.lane_status,
        "source_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "source_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "source_blocker_code": receipt.blocker_code,
        "downstream_dispatch_target": receipt.downstream_dispatch_target,
        "downstream_dispatch_command": activation_command.or_else(|| receipt.downstream_dispatch_command.clone()),
        "downstream_dispatch_note": receipt.downstream_dispatch_note,
        "downstream_dispatch_ready": receipt.downstream_dispatch_ready,
        "downstream_dispatch_blockers": receipt.downstream_dispatch_blockers,
        "downstream_dispatch_status": receipt.downstream_dispatch_status,
        "downstream_lane_status": receipt
            .downstream_dispatch_status
            .as_deref()
            .map(|status| {
                derive_lane_status(
                    status,
                    receipt.supersedes_receipt_id.as_deref(),
                    receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string()
            }),
        "downstream_supersedes_receipt_id": receipt.supersedes_receipt_id,
        "downstream_exception_path_receipt_id": receipt.exception_path_receipt_id,
        "downstream_dispatch_result_path": receipt.downstream_dispatch_result_path,
        "downstream_dispatch_active_target": receipt.downstream_dispatch_active_target,
        "activation_agent_type": receipt.activation_agent_type,
        "activation_runtime_role": receipt.activation_runtime_role,
        "selected_backend": receipt.selected_backend,
        "host_runtime": runtime_host_execution_contract_for_root(&project_root),
        "role_selection_full": role_selection,
        "run_graph_bootstrap": run_graph_bootstrap,
        "orchestration_contract": role_selection.execution_plan["orchestration_contract"],
    })
}

fn write_runtime_downstream_dispatch_packet_at(
    packet_path: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let body = downstream_dispatch_packet_body(
        role_selection,
        run_graph_bootstrap,
        receipt,
        Some(packet_path),
    );
    validate_runtime_dispatch_packet_contract(&body, "Runtime downstream dispatch packet")?;
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode downstream dispatch packet: {error}"))?;
    std::fs::write(packet_path, encoded)
        .map_err(|error| format!("Failed to write downstream dispatch packet: {error}"))?;
    Ok(())
}

fn write_runtime_downstream_dispatch_packet(
    state_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
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
    write_runtime_downstream_dispatch_packet_at(
        &packet_path,
        role_selection,
        run_graph_bootstrap,
        receipt,
    )?;
    let _ = target;
    Ok(Some(packet_path.display().to_string()))
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

pub(crate) fn fallback_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let conversational_mode = role_selection.conversational_mode.as_deref();
    let route_target = match conversational_mode {
        Some("scope_discussion") => "spec-pack".to_string(),
        Some("pbi_discussion") => "work-pool-pack".to_string(),
        _ if role_selection.execution_plan["status"] == "design_first" => "spec-pack".to_string(),
        _ => dispatch_contract_execution_lane_sequence(
            &role_selection.execution_plan["development_flow"]["dispatch_contract"],
        )
        .first()
        .map(|value| value.as_str())
        .unwrap_or(role_selection.selected_role.as_str())
        .to_string(),
    };
    let selected_route = if conversational_mode.is_some() {
        &role_selection.execution_plan["default_route"]
    } else {
        dispatch_contract_lane(&role_selection.execution_plan, &route_target).unwrap_or(
            runtime_assignment_from_execution_plan(&role_selection.execution_plan),
        )
    };
    let route_backend =
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, selected_route)
            .unwrap_or_else(|| "unknown".to_string());
    crate::state_store::RunGraphStatus {
        run_id: run_id.to_string(),
        task_id: run_id.to_string(),
        task_class: conversational_mode.unwrap_or("implementation").to_string(),
        active_node: if conversational_mode.is_some() {
            role_selection.selected_role.clone()
        } else {
            "planning".to_string()
        },
        next_node: Some(route_target.clone()),
        status: "ready".to_string(),
        route_task_class: if conversational_mode.is_some() {
            route_target.clone()
        } else {
            "implementation".to_string()
        },
        selected_backend: route_backend,
        lane_id: format!("{}_lane", role_selection.selected_role.replace('-', "_")),
        lifecycle_stage: if conversational_mode.is_some() {
            "conversation_active".to_string()
        } else {
            "implementation_dispatch_ready".to_string()
        },
        policy_gate: "not_required".to_string(),
        handoff_state: format!("awaiting_{route_target}"),
        context_state: "sealed".to_string(),
        checkpoint_kind: if conversational_mode.is_some() {
            "conversation_cursor".to_string()
        } else {
            "execution_cursor".to_string()
        },
        resume_target: format!("dispatch.{route_target}"),
        recovery_ready: true,
    }
}

fn blocking_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let mut status = fallback_runtime_consumption_run_graph_status(role_selection, run_id);
    status.status = "blocked".to_string();
    status.next_node = None;
    status.lifecycle_stage = "runtime_consumption_blocked".to_string();
    status.handoff_state = "none".to_string();
    status.context_state = "open".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}

pub(crate) async fn build_runtime_consumption_run_graph_bootstrap(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let run_id = runtime_consumption_run_id(role_selection);
    match crate::taskflow_run_graph::derive_seeded_run_graph_status(
        store,
        &run_id,
        &role_selection.request,
    )
    .await
    {
        Ok(seed_payload) => {
            let seed_payload_json =
                serde_json::to_value(&seed_payload).unwrap_or(serde_json::Value::Null);
            let seed_status_json =
                serde_json::to_value(&seed_payload.status).unwrap_or(serde_json::Value::Null);
            if let Err(error) = store.record_run_graph_status(&seed_payload.status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("record_seed_failed: {error}"),
                });
            }
            let mut latest_status = seed_status_json.clone();
            let mut advanced_payload = serde_json::Value::Null;

            if role_selection.conversational_mode.is_some() {
                match crate::taskflow_run_graph::derive_advanced_run_graph_status(
                    store,
                    seed_payload.status,
                )
                .await
                {
                    Ok(payload) => {
                        let advanced_status_json = serde_json::to_value(&payload.status)
                            .unwrap_or(serde_json::Value::Null);
                        if let Err(error) = store.record_run_graph_status(&payload.status).await {
                            let blocked_status = blocking_runtime_consumption_run_graph_status(
                                role_selection,
                                &run_id,
                            );
                            let blocked_status_json = serde_json::to_value(&blocked_status)
                                .unwrap_or(serde_json::Value::Null);
                            let blocked_write_error =
                                store.record_run_graph_status(&blocked_status).await.err();
                            return serde_json::json!({
                                "status": "blocked",
                                "handoff_ready": false,
                                "run_id": run_id,
                                "seed": seed_payload_json,
                                "latest_status": blocked_status_json,
                                "reason": if let Some(blocked_write_error) = blocked_write_error {
                                    format!(
                                        "record_advance_failed: {error}; compensating_blocked_record_failed: {blocked_write_error}"
                                    )
                                } else {
                                    format!("record_advance_failed: {error}")
                                },
                            });
                        }
                        advanced_payload =
                            serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
                        latest_status = advanced_status_json;
                    }
                    Err(error) => {
                        return serde_json::json!({
                            "status": "blocked",
                            "handoff_ready": false,
                            "run_id": run_id,
                            "seed": seed_payload_json,
                            "reason": format!("advance_failed: {error}"),
                        });
                    }
                }
            }

            serde_json::json!({
                "status": if advanced_payload.is_null() {
                    "seeded"
                } else {
                    "seeded_and_advanced"
                },
                "handoff_ready": true,
                "run_id": run_id,
                "seed": seed_payload_json,
                "advanced": advanced_payload,
                "latest_status": if advanced_payload.is_null() {
                    seed_status_json
                } else {
                    latest_status
                },
            })
        }
        Err(error) => {
            let status = blocking_runtime_consumption_run_graph_status(role_selection, &run_id);
            let latest_status = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
            if let Err(record_error) = store.record_run_graph_status(&status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("seed_failed: {error}; fallback_record_failed: {record_error}"),
                });
            }
            serde_json::json!({
                "status": "blocked",
                "handoff_ready": false,
                "run_id": run_id,
                "seed": serde_json::Value::Null,
                "advanced": serde_json::Value::Null,
                "latest_status": latest_status,
                "fallback_reason": format!("seed_failed: {error}"),
            })
        }
    }
}
