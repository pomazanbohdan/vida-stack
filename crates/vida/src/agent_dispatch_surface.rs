use std::process::ExitCode;

use crate::{AgentArgs, AgentCommand, AgentDispatchNextArgs, state_store, state_store::StateStore};

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLanePreview {
    lane_index: usize,
    task_id: String,
    title: String,
    role: String,
    dispatch_command: String,
    ready_parallel_safe: bool,
    selection_reason: String,
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
    ]
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

fn agent_init_command(task_id: &str, state_dir: Option<&std::path::Path>) -> String {
    let mut command = format!("vida agent-init --role worker {task_id} --json");
    if let Some(state_dir) = state_dir {
        command.push_str(" --state-dir ");
        command.push_str(&state_dir.display().to_string());
    }
    command
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
        selected_lanes.push(AgentDispatchLanePreview {
            lane_index: 1,
            task_id: primary.task.id.clone(),
            title: primary.task.title.clone(),
            role: "worker".to_string(),
            dispatch_command: agent_init_command(&primary.task.id, explicit_state_dir),
            ready_parallel_safe: primary.ready_parallel_safe,
            selection_reason: "primary_ready_task".to_string(),
        });
    }

    let mut remaining = effective_max_parallel_agents.saturating_sub(selected_lanes.len());
    for candidate in projection.ready.iter().skip(1) {
        if candidate.ready_parallel_safe && remaining > 0 {
            selected_lanes.push(AgentDispatchLanePreview {
                lane_index: selected_lanes.len() + 1,
                task_id: candidate.task.id.clone(),
                title: candidate.task.title.clone(),
                role: "worker".to_string(),
                dispatch_command: agent_init_command(&candidate.task.id, explicit_state_dir),
                ready_parallel_safe: candidate.ready_parallel_safe,
                selection_reason: "parallel_safe_ready_task".to_string(),
            });
            remaining -= 1;
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
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: run the selected `vida agent-init` commands manually/canonically after operator review."
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
            let preview = build_agent_dispatch_next_preview(
                &projection,
                command.lanes,
                configured_max_parallel_agents,
                explicit_state_dir,
            );
            if command.json {
                crate::print_json_pretty(
                    &serde_json::to_value(&preview)
                        .expect("agent dispatch-next preview should serialize"),
                );
            } else {
                println!("agent dispatch-next: {}", preview.status);
                println!("lanes selected: {}", preview.lanes_selected);
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
    use crate::test_cli_support::{EnvVarGuard, cli};
    use std::process::ExitCode;

    fn task(id: &str, title: &str) -> TaskRecord {
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
            labels: Vec::new(),
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
        TaskSchedulingCandidate {
            task: task(id, title),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers,
        }
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
            &projection,
            2,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
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
        assert!(
            preview.selected_lanes[1]
                .dispatch_command
                .contains("--state-dir /tmp/vida-state")
        );
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

        let preview = build_agent_dispatch_next_preview(&projection, 4, 4, None);

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

        let preview = build_agent_dispatch_next_preview(&projection, 4, 4, None);

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

        let preview = build_agent_dispatch_next_preview(&projection, 4, 2, None);

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
    fn agent_dispatch_next_command_passes_with_explicit_state_dir_single_ready_task() {
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

        assert_eq!(code, ExitCode::SUCCESS);
    }
}
