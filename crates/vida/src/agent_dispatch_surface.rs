use std::process::ExitCode;

use crate::{state_store, state_store::StateStore, AgentArgs, AgentCommand, AgentDispatchNextArgs};

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLaneSelectionTruth {
    selected_carrier: String,
    selected_backend: String,
    selected_model_profile: String,
    selected_model_ref: String,
    selected_reasoning_effort: String,
    rate: u64,
    estimated_task_price_units: u64,
    budget_verdict: String,
    runtime_role: String,
    task_class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLanePreview {
    lane_index: usize,
    task_id: String,
    title: String,
    role_label: String,
    runtime_role: String,
    task_class: String,
    dispatch_command: String,
    ready_parallel_safe: bool,
    selection_reason: String,
    selection_truth: AgentDispatchLaneSelectionTruth,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchBlockedCandidate {
    task_id: String,
    title: String,
    ready_now: bool,
    ready_parallel_safe: bool,
    reasons: Vec<String>,
    parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchNextPreview {
    status: String,
    mode: String,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    effective_max_parallel_agents: usize,
    lanes_selected: usize,
    selected_lanes: Vec<AgentDispatchLanePreview>,
    blocked_candidates: Vec<AgentDispatchBlockedCandidate>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    execute_supported: bool,
    execution_attempted: bool,
    source_surfaces: Vec<String>,
}

fn agent_dispatch_source_surfaces() -> Vec<String> {
    vec![
        "vida agent dispatch-next".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida taskflow graph-summary --json".to_string(),
        "vida taskflow scheduler dispatch --json".to_string(),
        "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            .to_string(),
        "vida agent-init --role worker <task-id> --json".to_string(),
        "vida agent-init --role <runtime-role> <task-id> --json".to_string(),
    ]
}

#[derive(Debug, Clone)]
struct DevTeamSequenceStep {
    role_label: String,
    runtime_role: String,
    task_class: String,
    requires_task: bool,
}

fn fallback_runtime_role_for_dispatch_target(dispatch_target: &str) -> String {
    match dispatch_target {
        "specification" | "analysis" => "business_analyst".to_string(),
        "implementer" => "worker".to_string(),
        "coach" => "coach".to_string(),
        "verification" => "verifier".to_string(),
        "execution_preparation" => "solution_architect".to_string(),
        _ => "worker".to_string(),
    }
}

fn role_label_from_runtime_role(runtime_role: &str, dispatch_target: &str) -> String {
    match runtime_role {
        "business_analyst" => "analyst".to_string(),
        "worker" => "developer".to_string(),
        "coach" => "coach".to_string(),
        "verifier" | "prover" => "tester/prover".to_string(),
        _ => match dispatch_target {
            "specification" | "analysis" => "analyst".to_string(),
            "implementer" => "developer".to_string(),
            "verification" => "tester/prover".to_string(),
            "execution_preparation" => "execution_preparation".to_string(),
            "release" | "closure" | "release/closure" => "release/closure".to_string(),
            _ => runtime_role.to_string(),
        },
    }
}

fn dev_team_sequence_from_readiness(readiness: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    let roles = readiness["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let role_id = role["role_id"].as_str()?;
            Some((role_id.to_string(), role))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let sequence = readiness["sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    if sequence.is_empty() || roles.is_empty() {
        return Vec::new();
    }
    sequence
        .into_iter()
        .filter_map(|role_id| {
            let role = roles.get(role_id)?;
            let runtime_role = role["runtime_role"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("worker")
                .to_string();
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .find(|value| !value.trim().is_empty())
                .unwrap_or("implementation")
                .to_string();
            Some(DevTeamSequenceStep {
                role_label: role_id.to_string(),
                runtime_role,
                task_class,
                requires_task: role_id != "release_closure" && role_id != "terminal_closure",
            })
        })
        .collect()
}

fn default_dev_team_sequence() -> Vec<DevTeamSequenceStep> {
    vec![
        DevTeamSequenceStep {
            role_label: "analyst".to_string(),
            runtime_role: "business_analyst".to_string(),
            task_class: "specification".to_string(),
            requires_task: true,
        },
        DevTeamSequenceStep {
            role_label: "developer".to_string(),
            runtime_role: "worker".to_string(),
            task_class: "implementation".to_string(),
            requires_task: true,
        },
        DevTeamSequenceStep {
            role_label: "coach".to_string(),
            runtime_role: "coach".to_string(),
            task_class: "coach".to_string(),
            requires_task: true,
        },
        DevTeamSequenceStep {
            role_label: "tester/prover".to_string(),
            runtime_role: "verifier".to_string(),
            task_class: "verification".to_string(),
            requires_task: true,
        },
        DevTeamSequenceStep {
            role_label: "release/closure".to_string(),
            runtime_role: "verifier".to_string(),
            task_class: "verification".to_string(),
            requires_task: false,
        },
    ]
}

fn dev_team_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    let readiness_sequence =
        dev_team_sequence_from_readiness(&activation_bundle["dev_team_readiness"]);
    if !readiness_sequence.is_empty() {
        return readiness_sequence;
    }
    let Some(development_flow) = activation_bundle.get("development_flow") else {
        return default_dev_team_sequence();
    };
    let Some(dispatch_contract) = development_flow.get("dispatch_contract") else {
        return default_dev_team_sequence();
    };
    let execution_lane_sequence =
        crate::dispatch_contract_execution_lane_sequence(dispatch_contract);
    if execution_lane_sequence.is_empty() {
        return default_dev_team_sequence();
    }

    let mut steps = execution_lane_sequence
        .into_iter()
        .map(|dispatch_target| {
            let lane = crate::dispatch_contract_lane(activation_bundle, &dispatch_target);
            let route = lane.unwrap_or(&serde_json::Value::Null);
            let activation = crate::dispatch_contract_lane_activation(route);
            let runtime_role = activation
                .get("activation_runtime_role")
                .or_else(|| route.get("runtime_role"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| fallback_runtime_role_for_dispatch_target(&dispatch_target));
            let task_class = activation
                .get("task_class")
                .or_else(|| route.get("task_class"))
                .and_then(|value| value.as_str())
                .unwrap_or("implementation")
                .to_string();
            let role_label = role_label_from_runtime_role(&runtime_role, &dispatch_target);
            DevTeamSequenceStep {
                role_label,
                runtime_role,
                task_class,
                requires_task: true,
            }
        })
        .collect::<Vec<_>>();
    steps.push(DevTeamSequenceStep {
        role_label: "release/closure".to_string(),
        runtime_role: "verifier".to_string(),
        task_class: "verification".to_string(),
        requires_task: false,
    });
    steps
}

fn configured_max_parallel_agents_from_activation_bundle(
    activation_bundle: &serde_json::Value,
) -> usize {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1)
}

fn agent_init_command(
    task_id: &str,
    state_dir: Option<&std::path::Path>,
    runtime_role: &str,
) -> String {
    let mut command = if runtime_role.trim().is_empty() {
        format!("vida agent-init --role worker {task_id} --json")
    } else {
        format!("vida agent-init --role {runtime_role} {task_id} --json")
    };
    if let Some(state_dir) = state_dir {
        command.push_str(" --state-dir ");
        command.push_str(&state_dir.display().to_string());
    }
    command
}

fn required_string_field(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload[key]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn selection_truth_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    selection_truth_for_task_with_role_and_class(activation_bundle, task, "worker", None, None)
}

fn selection_truth_for_task_with_role_and_class(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
    conversation_role: &str,
    runtime_role_override: Option<&str>,
    task_class_override: Option<&str>,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    let task_value = serde_json::to_value(task)
        .map_err(|error| format!("task_record_serialization_failed:{error}"))?;
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let task_class = task_class_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or(inferred_task_class);
    let runtime_role = runtime_role_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| crate::runtime_role_for_task_class(&task_class).to_string());
    let assignment = crate::build_runtime_assignment_from_resolved_constraints(
        activation_bundle,
        conversation_role,
        &task_class,
        &runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        let reason = required_string_field(&assignment, "reason")
            .unwrap_or_else(|| "runtime_assignment_disabled".to_string());
        return Err(reason);
    }

    let selected_carrier = required_string_field(&assignment, "selected_carrier_id")
        .ok_or_else(|| "selected_carrier_id_missing".to_string())?;
    let selected_backend = required_string_field(&assignment, "selected_backend_id")
        .ok_or_else(|| "selected_backend_id_missing".to_string())?;
    let selected_model_profile = required_string_field(&assignment, "selected_model_profile_id")
        .ok_or_else(|| "selected_model_profile_id_missing".to_string())?;
    let selected_model_ref = required_string_field(&assignment, "selected_model_ref")
        .ok_or_else(|| "selected_model_ref_missing".to_string())?;
    let selected_reasoning_effort = required_string_field(&assignment, "selected_reasoning_effort")
        .ok_or_else(|| "selected_reasoning_effort_missing".to_string())?;
    let budget_verdict = required_string_field(&assignment, "budget_verdict")
        .ok_or_else(|| "budget_verdict_missing".to_string())?;
    let rate = assignment["rate"]
        .as_u64()
        .ok_or_else(|| "rate_missing".to_string())?;
    let estimated_task_price_units = assignment["estimated_task_price_units"]
        .as_u64()
        .ok_or_else(|| "estimated_task_price_units_missing".to_string())?;

    Ok(AgentDispatchLaneSelectionTruth {
        selected_carrier,
        selected_backend,
        selected_model_profile,
        selected_model_ref,
        selected_reasoning_effort,
        rate,
        estimated_task_price_units,
        budget_verdict,
        runtime_role,
        task_class,
    })
}

fn blocked_candidate(
    candidate: &state_store::TaskSchedulingCandidate,
    reasons: Vec<String>,
) -> AgentDispatchBlockedCandidate {
    AgentDispatchBlockedCandidate {
        task_id: candidate.task.id.clone(),
        title: candidate.task.title.clone(),
        ready_now: candidate.ready_now,
        ready_parallel_safe: candidate.ready_parallel_safe,
        reasons,
        parallel_blockers: candidate.parallel_blockers.clone(),
    }
}

fn build_agent_dispatch_next_preview(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    dev_team: bool,
) -> AgentDispatchNextPreview {
    if dev_team {
        build_agent_dispatch_next_preview_dev_team(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    } else {
        build_agent_dispatch_next_preview_standard(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    }
}

fn build_agent_dispatch_next_preview_standard(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);

    let Some(primary) = projection.ready.first() else {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(
            "Inspect `vida task ready --json` and resolve blockers before previewing agent dispatch."
                .to_string(),
        );
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    };

    if effective_max_parallel_agents > 0 {
        match selection_truth_for_task(activation_bundle, &primary.task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: 1,
                task_id: primary.task.id.clone(),
                title: primary.task.title.clone(),
                role_label: "default".to_string(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &primary.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                ready_parallel_safe: primary.ready_parallel_safe,
                selection_reason: "primary_ready_task".to_string(),
                selection_truth,
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                primary.task.id, reason
            )),
        }
    }

    let mut remaining = effective_max_parallel_agents.saturating_sub(selected_lanes.len());
    for candidate in projection.ready.iter().skip(1) {
        if candidate.ready_parallel_safe && remaining > 0 {
            match selection_truth_for_task(activation_bundle, &candidate.task) {
                Ok(selection_truth) => {
                    selected_lanes.push(AgentDispatchLanePreview {
                        lane_index: selected_lanes.len() + 1,
                        task_id: candidate.task.id.clone(),
                        title: candidate.task.title.clone(),
                        role_label: "parallel".to_string(),
                        runtime_role: selection_truth.runtime_role.clone(),
                        task_class: selection_truth.task_class.clone(),
                        dispatch_command: agent_init_command(
                            &candidate.task.id,
                            explicit_state_dir,
                            &selection_truth.runtime_role,
                        ),
                        ready_parallel_safe: candidate.ready_parallel_safe,
                        selection_reason: "parallel_safe_ready_task".to_string(),
                        selection_truth,
                    });
                    remaining -= 1;
                }
                Err(reason) => blocker_codes.push(format!(
                    "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                    candidate.task.id, reason
                )),
            }
            continue;
        }

        let reasons = if candidate.ready_parallel_safe {
            vec!["effective_max_parallel_agents_cap_reached".to_string()]
        } else if candidate.parallel_blockers.is_empty() {
            vec!["parallel_safety_not_established".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        blocked_candidates.push(blocked_candidate(candidate, reasons));
    }

    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    let unsafe_ready_candidates = blocked_candidates
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    if effective_max_parallel_agents > 1 && unsafe_ready_candidates {
        blocker_codes.push("ambiguous_unsafe_parallel_candidates".to_string());
        next_actions.push(
            "Some ready candidates are not parallel-safe; reduce to `--lanes 1` or fix execution semantics/conflicts before multi-lane dispatch."
                .to_string(),
        );
    }
    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one chosen lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn build_agent_dispatch_next_preview_dev_team(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();
    let sequence = dev_team_sequence(activation_bundle);

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }

    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);
    let steps_to_preview = sequence
        .into_iter()
        .take(effective_max_parallel_agents)
        .collect::<Vec<_>>();
    if projection.ready.is_empty() {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(
            "Inspect `vida task ready --json` and resolve blockers before previewing dev-team dispatch."
                .to_string(),
        );
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview-dev-team".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    }

    let mut ready_index = 0;
    for (step_index, step) in steps_to_preview.into_iter().enumerate() {
        if !step.requires_task {
            next_actions.push(format!(
                "dev-team step [{}] {} is closure-oriented and does not emit a runtime launch command.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
            continue;
        }
        let Some(candidate) = projection.ready.get(ready_index) else {
            blocker_codes.push(format!(
                "dev_team_step_missing_ready_task:position={}:{}",
                step_index + 1,
                step.role_label
            ));
            break;
        };
        ready_index += 1;
        if !candidate.ready_now {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["task_not_ready_for_dev_team_step".to_string()],
            ));
            continue;
        }
        match selection_truth_for_task_with_role_and_class(
            activation_bundle,
            &candidate.task,
            &step.runtime_role,
            Some(&step.runtime_role),
            Some(&step.task_class),
        ) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: selected_lanes.len() + 1,
                task_id: candidate.task.id.clone(),
                title: candidate.task.title.clone(),
                role_label: step.role_label.clone(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &candidate.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                ready_parallel_safe: candidate.ready_parallel_safe,
                selection_reason: format!("dev_team_step_{}:{}", step_index + 1, step.role_label),
                selection_truth,
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                candidate.task.id, reason
            )),
        }
    }

    let blocked_ready_parallel = projection
        .ready
        .iter()
        .enumerate()
        .filter(|(index, candidate)| *index >= ready_index && !candidate.ready_parallel_safe)
        .collect::<Vec<_>>();
    for (_index, candidate) in blocked_ready_parallel {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["parallel_safety_not_established".to_string()],
        ));
    }
    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one configured dev-team step; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview-dev-team".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

pub(crate) async fn run_agent(args: AgentArgs) -> ExitCode {
    match args.command {
        AgentCommand::DispatchNext(command) => run_agent_dispatch_next(command).await,
    }
}

async fn run_agent_dispatch_next(command: AgentDispatchNextArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let explicit_state_dir = command.state_dir.as_deref();
    match StateStore::open_existing_read_only(state_dir).await {
        Ok(store) => {
            let configured_max_parallel_agents =
                match crate::build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => configured_max_parallel_agents_from_activation_bundle(
                        &payload.activation_bundle,
                    ),
                    Err(_) => 1,
                };
            let projection = match store
                .scheduling_projection_scoped(
                    command.scope.as_deref(),
                    command.current_task_id.as_deref(),
                )
                .await
            {
                Ok(projection) => projection,
                Err(error) => {
                    eprintln!("Failed to compute agent dispatch preview: {error}");
                    return ExitCode::from(1);
                }
            };
            let mut activation_bundle =
                match crate::build_taskflow_consume_bundle_payload(&store).await {
                    Ok(payload) => payload.activation_bundle,
                    Err(_) => serde_json::Value::Null,
                };
            if command.dev_team {
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
            }
            let preview = build_agent_dispatch_next_preview(
                &activation_bundle,
                &projection,
                command.lanes,
                configured_max_parallel_agents,
                explicit_state_dir,
                command.dev_team,
            );
            if command.json {
                crate::print_json_pretty(
                    &serde_json::to_value(&preview)
                        .expect("agent dispatch-next preview should serialize"),
                );
            } else {
                println!("agent dispatch-next: {}", preview.status);
                println!("lanes selected: {}", preview.lanes_selected);
                println!(
                        "preview only: review carrier/model/cost selection truth before launching any `vida agent-init` command"
                    );
                for lane in &preview.selected_lanes {
                    println!(
                        "lane {} [{}]: {} [{} / {} / rate={} / est_cost={}]",
                        lane.lane_index,
                        lane.role_label,
                        lane.task_id,
                        lane.selection_truth.selected_carrier,
                        lane.selection_truth.selected_model_ref,
                        lane.selection_truth.rate,
                        lane.selection_truth.estimated_task_price_units
                    );
                }
                if !preview.blocker_codes.is_empty() {
                    println!("blockers: {}", preview.blocker_codes.join(", "));
                }
            }
            if preview.status == "pass" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_agent_dispatch_next_preview, state_store};
    use crate::state_store::{
        CreateTaskRequest, TaskExecutionSemantics, TaskRecord, TaskSchedulingCandidate,
        TaskSchedulingProjection,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, EnvVarGuard};
    use std::process::ExitCode;

    fn task_with_labels(id: &str, title: &str, labels: &[&str]) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: title.to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: state_store::TaskPlannerMetadata::default(),
            dependencies: Vec::new(),
        }
    }

    fn candidate(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
    ) -> TaskSchedulingCandidate {
        candidate_with_labels(
            id,
            title,
            ready_now,
            ready_parallel_safe,
            parallel_blockers,
            &[],
        )
    }

    fn candidate_with_labels(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
        labels: &[&str],
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels(id, title, labels),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers,
        }
    }

    fn activation_bundle_with_worker_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation", "verification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "write_scope": "scoped_only",
                        "model_profiles": {
                            "gpt-5.4-low": {
                                "profile_id": "gpt-5.4-low",
                                "model_ref": "gpt-5.4",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation", "verification"],
                                "normalized_cost_units": 1
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "junior": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational"
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_dev_team_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "analyst-seat",
                        "tier": "senior",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["specification"],
                        "model_profiles": {
                            "analyst": {
                                "profile_id": "analyst-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["business_analyst"],
                                "task_classes": ["specification"],
                                "normalized_cost_units": 1
                            }
                        }
                    },
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.4",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    },
                    {
                        "role_id": "coach-seat",
                        "tier": "middle",
                        "default_runtime_role": "coach",
                        "runtime_roles": ["coach"],
                        "task_classes": ["coach"],
                        "normalized_cost_units": 3,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["coach"],
                        "model_profiles": {
                            "coach": {
                                "profile_id": "coach-profile",
                                "model_ref": "gpt-5.4-coach",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["coach"],
                                "task_classes": ["coach"],
                                "normalized_cost_units": 3
                            }
                        }
                    },
                    {
                        "role_id": "verifier-seat",
                        "tier": "middle",
                        "default_runtime_role": "verifier",
                        "runtime_roles": ["verifier", "prover"],
                        "task_classes": ["verification"],
                        "normalized_cost_units": 4,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["verification"],
                        "model_profiles": {
                            "prover": {
                                "profile_id": "verifier-profile",
                                "model_ref": "gpt-5.3",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["verifier", "prover"],
                                "task_classes": ["verification"],
                                "normalized_cost_units": 4
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "analyst-seat": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        },
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        },
                        "coach-seat": {
                            "effective_score": 74,
                            "lifecycle_state": "active"
                        },
                        "verifier-seat": {
                            "effective_score": 76,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_role_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.4",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_model_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "",
                                "model_ref": "",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_price_data_blocked() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.4",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": false
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn assertion_message_contains_actionable_blocker(blocker_codes: &[String], task_id: &str) {
        let expected_prefix =
            format!("selected_lane_runtime_assignment_truth_missing:task={task_id}:");
        assert!(blocker_codes
            .iter()
            .any(|code| code.starts_with(&expected_prefix)));
    }

    #[test]
    fn agent_dispatch_next_preview_selects_parallel_safe_lanes_with_commands() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            false,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 2);
        assert_eq!(preview.configured_max_parallel_agents, 4);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert_eq!(preview.selected_lanes[0].task_class, "implementation");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "junior"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.4"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert!(preview.selected_lanes[1]
            .dispatch_command
            .contains("--state-dir /tmp/vida-state"));
    }

    #[test]
    fn agent_dispatch_next_preview_blocks_no_ready_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: vec![candidate(
                "task-blocked",
                "Blocked",
                false,
                false,
                vec!["graph_blocked".to_string()],
            )],
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert_eq!(preview.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(preview.blocked_candidates[0].task_id, "task-blocked");
    }

    #[test]
    fn agent_dispatch_next_preview_blocks_unsafe_parallel_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate(
                    "task-b",
                    "Task B",
                    true,
                    false,
                    vec!["execution_mode_not_parallel_safe".to_string()],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(
            preview.blocker_codes,
            vec!["ambiguous_unsafe_parallel_candidates"]
        );
        assert_eq!(preview.blocked_candidates[0].task_id, "task-b");
    }

    #[test]
    fn agent_dispatch_next_preview_clamps_requested_lanes_to_configured_max() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
                candidate("task-d", "Task D", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            2,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 4);
        assert_eq!(preview.configured_max_parallel_agents, 2);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert!(preview.blocked_candidates.iter().any(
            |candidate| candidate.reasons == vec!["effective_max_parallel_agents_cap_reached"]
        ));
    }

    #[test]
    fn agent_dispatch_next_preview_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({}),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code
                .starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")));
    }

    #[test]
    fn agent_dispatch_next_preview_renders_configured_dev_team_sequence() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate_with_labels(
                    "task-analyst",
                    "Specification task",
                    true,
                    true,
                    Vec::new(),
                    &["documentation"],
                ),
                candidate_with_labels(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                    &[],
                ),
                candidate_with_labels(
                    "task-coach",
                    "Coach review task",
                    true,
                    true,
                    Vec::new(),
                    &["coach"],
                ),
                candidate_with_labels(
                    "task-tester",
                    "Tester verification",
                    true,
                    true,
                    Vec::new(),
                    &["tester"],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            4,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert_eq!(preview.selected_lanes[0].role_label, "analyst");
        assert_eq!(preview.selected_lanes[1].role_label, "developer");
        assert_eq!(preview.selected_lanes[2].role_label, "coach");
        assert_eq!(preview.selected_lanes[3].role_label, "tester/prover");
        assert_eq!(preview.selected_lanes[0].task_id, "task-analyst");
        assert_eq!(preview.selected_lanes[1].task_id, "task-developer");
        assert_eq!(preview.selected_lanes[2].task_id, "task-coach");
        assert_eq!(preview.selected_lanes[3].task_id, "task-tester");
        assert_eq!(preview.selected_lanes[0].runtime_role, "business_analyst");
        assert_eq!(preview.selected_lanes[1].runtime_role, "worker");
        assert_eq!(preview.selected_lanes[2].runtime_role, "coach");
        assert_eq!(preview.selected_lanes[3].runtime_role, "verifier");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.task_class,
            "specification"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.task_class,
            "implementation"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.task_class,
            "coach"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.task_class,
            "verification"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "analyst-seat"
        );
        assert_eq!(
            preview.selected_lanes[0]
                .selection_truth
                .selected_model_profile,
            "analyst-profile"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_carrier,
            "developer-seat"
        );
        assert_eq!(
            preview.selected_lanes[1]
                .selection_truth
                .selected_model_profile,
            "developer-profile"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_model_ref,
            "gpt-5.4"
        );
        assert_eq!(preview.selected_lanes[1].selection_truth.rate, 2);
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_carrier,
            "coach-seat"
        );
        assert_eq!(
            preview.selected_lanes[2]
                .selection_truth
                .selected_model_profile,
            "coach-profile"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_model_ref,
            "gpt-5.4-coach"
        );
        assert_eq!(preview.selected_lanes[2].selection_truth.rate, 3);
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_carrier,
            "verifier-seat"
        );
        assert_eq!(
            preview.selected_lanes[3]
                .selection_truth
                .selected_model_profile,
            "verifier-profile"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_model_ref,
            "gpt-5.3"
        );
        assert_eq!(preview.selected_lanes[3].selection_truth.rate, 4);
        assert!(
            preview.selected_lanes[0]
                .dispatch_command
                .contains("vida agent-init --role business_analyst task-analyst --json --state-dir /tmp/vida-state")
        );
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_reports_no_task_lane_for_release_closure_and_still_selects_roles(
    ) {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate("task-analyst", "Specification task", true, true, Vec::new()),
                candidate(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                ),
                candidate("task-coach", "Coach review task", true, true, Vec::new()),
                candidate("task-tester", "Tester verification", true, true, Vec::new()),
                candidate("task-unused", "Unused final task", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            5,
            5,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("closure-oriented")));
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code
                .starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")));
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_role_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_role_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_carrier_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_model_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_model_profile_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_price_policy() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_price_data_blocked(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code
                .starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")));
    }

    #[test]
    fn agent_dispatch_next_preview_exposes_dispatch_flow_discovery_surfaces() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert!(preview.source_surfaces.iter().any(|surface| {
            surface == "vida taskflow graph-summary --json"
                || surface == "vida taskflow scheduler dispatch --json"
        }));
        assert!(
            preview.source_surfaces.iter().any(
                |surface| surface
                    == "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            )
        );
        assert!(preview
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida agent-init --role worker <task-id> --json"));
    }

    #[test]
    fn agent_dispatch_next_command_fails_closed_without_runtime_selection_truth() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            store
                .create_task(CreateTaskRequest {
                    task_id: "task-ready",
                    title: "Ready task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 2,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: ".",
                })
                .await
                .expect("task should create");
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let code = runtime.block_on(crate::run(cli(&[
            "agent",
            "dispatch-next",
            "--lanes",
            "1",
            "--state-dir",
            harness.path().to_str().expect("state dir should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::from(1));
    }
}
