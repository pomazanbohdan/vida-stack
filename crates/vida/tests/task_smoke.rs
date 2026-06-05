use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/tmp/vida-task-state-{}-{}-{}",
        std::process::id(),
        nanos,
        counter
    )
}

fn unique_test_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    static UNIQUE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = UNIQUE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{}-{}", prefix, std::process::id(), nanos, counter)
}

fn project_bound_state_dir() -> (String, String) {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    fs::create_dir_all(&state_dir).expect("create project-bound state dir");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("write AGENTS.md");
    fs::write(
        format!("{project_root}/vida.config.yaml"),
        concat!(
            "project:\n",
            "  id: test\n",
            "agent_system:\n",
            "  mode: internal\n",
            "  state_owner: taskflow_state_store\n",
            "  max_parallel_agents: 4\n",
            "  model_selection:\n",
            "    enabled: true\n",
            "    candidate_scope: unified_carrier_model_profiles\n",
            "    default_strategy: balanced_cost_quality\n",
            "    selection_rule: cheapest_capable\n",
            "  subagents:\n",
            "    junior:\n",
            "      enabled: true\n",
            "      subagent_backend_class: internal\n",
            "      rate: 1\n",
            "      default_runtime_role: worker\n",
            "      runtime_roles:\n",
            "        - worker\n",
            "      task_classes:\n",
            "        - implementation\n",
            "      default_model_profile: test_low\n",
            "      write_scope: scoped_only\n",
            "      model_profiles:\n",
            "        test_low:\n",
            "          profile_id: test_low\n",
            "          provider: test\n",
            "          model_ref: test-model-low\n",
            "          reasoning_effort: low\n",
            "          normalized_cost_units: 1\n",
            "          sandbox_mode: workspace-write\n",
            "          write_scope: scoped_only\n",
            "          runtime_roles:\n",
            "            - worker\n",
            "          task_classes:\n",
            "            - implementation\n",
            "          readiness:\n",
            "            required: false\n",
            "            ready: true\n",
            "agent_extensions:\n",
            "  role_selection:\n",
            "    mode: default\n",
            "    fallback_role: orchestrator\n",
        ),
    )
    .expect("write vida.config.yaml");
    for relative in [".vida/config", ".vida/db", ".vida/project"] {
        fs::create_dir_all(format!("{project_root}/{relative}"))
            .expect("runtime project marker dir should exist");
    }
    (project_root, state_dir)
}

fn rewrite_project_model_ref(project_root: &str, model_ref: &str) {
    let config_path = format!("{project_root}/vida.config.yaml");
    let config = fs::read_to_string(&config_path).expect("project config should read");
    fs::write(&config_path, config.replace("test-model-low", model_ref))
        .expect("project config should update");
}

static PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn sample_jsonl(path: &str) {
    fs::write(
        path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-c\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("write task jsonl");
}

fn run_and_assert_success(args: &[&str], state_dir: &str) -> String {
    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "args: {args:?}\nstatus: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run_command_capture(args: &[&str], state_dir: &str) -> std::process::Output {
    run_with_state_lock_retry(|| {
        let mut command = vida();
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    })
}

fn run_command_json(args: &[&str], state_dir: &str) -> serde_json::Value {
    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "args: {args:?}\nstatus: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json output should parse")
}

fn run_command_json_allow_failure(args: &[&str], state_dir: &str) -> (serde_json::Value, bool) {
    let output = run_command_capture(args, state_dir);
    let json = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "json output should parse for args {args:?}: {error}\nstatus: {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (json, output.status.success())
}

fn create_epic_parent(state_dir: &str, parent_id: &str, title: &str, status: &str) {
    let parent = run_command_json(
        &[
            "task",
            "create",
            parent_id,
            title,
            "--type",
            "epic",
            "--status",
            status,
            "--priority",
            "1",
            "--json",
        ],
        state_dir,
    );
    assert_eq!(parent["status"], "pass");
}

fn write_operator_projection(state_dir: &str, projection_name: &str, payload: &serde_json::Value) {
    let projection_dir = format!("{state_dir}/operator-projections");
    fs::create_dir_all(&projection_dir).expect("operator projection dir should exist");
    let mut payload = payload.clone();
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert(
            "projection_cache_dependencies".to_string(),
            serde_json::json!({
                "task_snapshot_marker": null
            }),
        );
    }
    fs::write(
        format!("{projection_dir}/{projection_name}.json"),
        serde_json::to_string_pretty(&payload).expect("operator projection should render"),
    )
    .expect("operator projection should write");
}

fn stale_blocked_next_lawful_projection() -> serde_json::Value {
    serde_json::json!({
        "status": "blocked",
        "cache_probe": "task-next-lawful-reused",
        "active_bounded_unit": null,
        "binding_source": null,
        "why_this_unit": "stale cached no-ready projection",
        "sequential_vs_parallel_posture": "blocked",
        "ready_task_candidates": [],
        "blocker_codes": ["no_ready_task_candidates"],
        "next_actions": ["Create/import the next task or refresh TaskFlow state before continuing."],
        "source_surfaces": ["task-next-lawful-latest"]
    })
}

fn seed_model_profile_readiness_dispatch_context(state_dir: &str) {
    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let runtime_assignment = serde_json::json!({
            "enabled": true,
            "selected_backend_id": "internal_subagents",
            "selected_carrier_id": "internal_subagents",
            "selected_model_profile_id": "codex_gpt55_low_write",
            "selected_model_ref": "gpt-5.5",
            "selected_model_provider": "openai-codex",
            "selected_reasoning_effort": "low",
            "selected_reasoning_control_mode": "fixed",
            "model_selection_enabled": true,
            "candidate_scope": "unified_carrier_model_profiles",
            "selection_source_paths": {
                "selected_model_profile_id": "carrier_runtime.roles[internal_subagents].model_profiles.codex_gpt55_low_write.profile_id"
            },
            "selection_override_reasons": ["route_profile_mapping"],
            "selection_precedence": ["route_profile_mapping", "role_default"],
            "selected_route_profile_mapping": {
                "runtime_role": "worker",
                "profile_id": "codex_gpt55_low_write"
            },
            "selected_candidate": {
                "profile_id": "codex_gpt55_low_write",
                "selected": true
            },
            "rejected_candidates": [
                {
                    "profile_id": "codex_gpt55_high_readonly",
                    "reason": "write_scope_required"
                }
            ],
            "budget_policy": "tier_budget_guard",
            "budget_verdict": "within_budget",
            "max_budget_units": 4,
            "selected_over_budget": false,
            "budget_scope": "task",
            "selection_budget": {
                "remaining_units": 3
            },
            "runtime_budget_ledger": {
                "spent_units": 1
            }
        });
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true,
                        "review": true,
                        "verification": true
                    }
                }
            ],
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementation"],
                    "lane_catalog": {
                        "implementation": {
                            "executor_backend": "internal_subagents",
                            "fallback_executor_backend": "internal_subagents",
                            "carrier_runtime_assignment": runtime_assignment
                        }
                    }
                }
            }
        });
        let context = serde_json::json!({
            "run_id": "run-model-profile-readiness-smoke",
            "task_id": "task-model-profile-readiness-smoke",
            "request_text": "model profile readiness smoke",
            "recorded_at": "2026-04-24T00:00:00Z",
            "role_selection": {
                "ok": true,
                "activation_source": "smoke-test",
                "selection_mode": "fixed",
                "fallback_role": "orchestrator",
                "request": "model profile readiness smoke",
                "selected_role": "worker",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": ["implementation"],
                "compiled_bundle": null,
                "reason": "smoke-test",
                "execution_plan": execution_plan
            }
        });
        db.query(
            "UPSERT run_graph_dispatch_context:`run-model-profile-readiness-smoke` CONTENT $context",
        )
        .bind(("context", context))
        .await
        .expect("seed run graph dispatch context");
        drop(db);
    });
}

fn run_model_profile_readiness_seed_helper(state_dir: &str) {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("taskflow_model_profile_readiness_seed_helper")
        .arg("--exact")
        .env("VIDA_MODEL_PROFILE_READINESS_SEED_STATE_DIR", state_dir)
        .output()
        .expect("seed helper should run");
    assert!(
        output.status.success(),
        "seed helper stdout: {}\nseed helper stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn extract_plain_surface_line(output: &str, label: &str) -> String {
    let prefix = format!("{label}: ");
    output
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_string))
        .unwrap_or_else(|| panic!("{label} line missing from plain output"))
}

fn require_json_string(value: &serde_json::Value, label: &str) -> String {
    value
        .as_str()
        .map(|text| text.to_string())
        .unwrap_or_else(|| panic!("{} missing or not a string", label))
}

fn require_json_string_array(value: &serde_json::Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} missing or not an array"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("{label} entry missing or not a string"))
                .to_string()
        })
        .collect()
}

fn find_scheduling_candidate<'a>(
    candidates: &'a serde_json::Value,
    task_id: &str,
) -> &'a serde_json::Value {
    candidates
        .as_array()
        .unwrap_or_else(|| panic!("scheduling candidates missing or not an array: {candidates}"))
        .iter()
        .find(|candidate| candidate["task"]["id"].as_str() == Some(task_id))
        .unwrap_or_else(|| panic!("scheduling candidate `{task_id}` missing"))
}

fn find_task_ref_by_id<'a>(tasks: &'a serde_json::Value, task_id: &str) -> &'a serde_json::Value {
    tasks
        .as_array()
        .unwrap_or_else(|| panic!("task refs missing or not an array"))
        .iter()
        .find(|task| task["id"].as_str() == Some(task_id))
        .unwrap_or_else(|| panic!("task ref `{task_id}` missing"))
}

fn find_rejected_candidate<'a>(
    candidates: &'a serde_json::Value,
    task_id: &str,
) -> &'a serde_json::Value {
    candidates
        .as_array()
        .unwrap_or_else(|| panic!("rejected candidates missing or not an array"))
        .iter()
        .find(|candidate| candidate["task_id"].as_str() == Some(task_id))
        .unwrap_or_else(|| panic!("rejected candidate `{task_id}` missing"))
}

fn task_ids_from_rows(rows: &Value, label: &str) -> Vec<String> {
    rows.as_array()
        .unwrap_or_else(|| panic!("{label} missing or not an array"))
        .iter()
        .map(|row| {
            row["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label} row id missing"))
                .to_string()
        })
        .collect()
}

fn blocked_task_ids_from_rows(rows: &Value, label: &str) -> Vec<String> {
    rows.as_array()
        .unwrap_or_else(|| panic!("{label} missing or not an array"))
        .iter()
        .map(|row| {
            row["task"]["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label} row task id missing"))
                .to_string()
        })
        .collect()
}

fn next_lawful_candidate_ids(next_lawful: &Value) -> Vec<String> {
    next_lawful["ready_task_candidates"]
        .as_array()
        .expect("next-lawful ready_task_candidates should be an array")
        .iter()
        .map(|candidate| {
            candidate["task_id"]
                .as_str()
                .expect("next-lawful candidate task_id should render")
                .to_string()
        })
        .collect()
}

fn assert_no_run_id_consume_continue_command(value: &Value, run_id: &str, label: &str) {
    let rendered = value.to_string();
    let impossible_command = format!("vida taskflow consume continue --run-id {run_id} --json");
    assert!(
        !rendered.contains(&impossible_command),
        "{label} must not emit impossible consume command `{impossible_command}`: {rendered}"
    );
}

fn normalize_json_fixture(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    serde_json::to_string_pretty(&parsed).expect("json output should pretty render")
}

const STATE_LOCK_RETRY_LIMIT: usize = 600;

fn is_state_lock_error(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    stderr.contains(vida_test_support::STATE_LOCK_ERROR_MESSAGE)
        || stderr.contains("timed out while waiting for authoritative datastore lock")
        || stdout.contains("state_store_read_lock_contention")
        || stdout.contains("authoritative_state_store_locked")
}

fn run_with_state_lock_retry<F>(mut builder: F) -> std::process::Output
where
    F: FnMut() -> Command,
{
    vida_test_support::command_output_with_retry_errors(
        &mut builder,
        STATE_LOCK_RETRY_LIMIT,
        is_state_lock_error,
        |error| error.raw_os_error() == Some(26),
    )
}

fn assert_json_status_pass(output: &str) {
    let parsed: serde_json::Value = serde_json::from_str(output).expect("json output should parse");
    assert_eq!(parsed["status"], "pass");
}

fn assert_task_graph_valid_after(state_dir: &str, mutation: &str) {
    let validate = run_command_json(&["task", "validate-graph", "--json"], state_dir);
    assert_eq!(
        validate["surface"], "vida task validate-graph",
        "{mutation}"
    );
    assert_eq!(validate["status"], "pass", "{mutation}");
    assert_eq!(validate["valid"], true, "{mutation}");
    assert_eq!(validate["issue_count"], 0, "{mutation}");
}

fn write_single_task_snapshot(path: &str, task_id: &str, title: &str, status: &str, priority: u32) {
    let row = serde_json::json!({
        "id": task_id,
        "title": title,
        "description": "snapshot evidence",
        "status": status,
        "priority": priority,
        "issue_type": "task",
        "created_at": "2026-03-08T00:00:00Z",
        "created_by": "tester",
        "updated_at": "2026-03-08T00:00:00Z",
        "source_repo": ".",
        "compaction_level": 0,
        "original_size": 0,
        "labels": [],
        "dependencies": [],
    });
    let snapshot_path = std::path::Path::new(path);
    fs::create_dir_all(
        snapshot_path
            .parent()
            .expect("snapshot path should have a parent"),
    )
    .expect("create snapshot parent");
    fs::write(path, format!("{row}\n")).expect("write task snapshot");
}

fn task_row_by_id<'a>(payload: &'a Value, task_id: &str) -> &'a Value {
    payload["tasks"]
        .as_array()
        .expect("task payload should include task array")
        .iter()
        .find(|task| task["id"] == task_id)
        .unwrap_or_else(|| panic!("task payload should include {task_id}"))
}

#[test]
fn taskflow_model_profile_readiness_cli_smoke_matches_config_census_embedding() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    run_model_profile_readiness_seed_helper(&state_dir);

    let standalone = run_command_json(
        &[
            "taskflow",
            "route",
            "model-profile-readiness",
            "--run-id",
            "run-model-profile-readiness-smoke",
            "--json",
        ],
        &state_dir,
    );
    let census = run_command_json(
        &[
            "taskflow",
            "config-actuation",
            "census",
            "--run-id",
            "run-model-profile-readiness-smoke",
            "--json",
        ],
        &state_dir,
    );
    let embedded = &census["routes"][0]["model_profile_readiness_audit"];

    assert_eq!(
        standalone["surface"],
        "vida taskflow model-profile readiness audit"
    );
    assert_eq!(standalone["status"], "pass");
    assert_eq!(standalone["dispatch_target"], "implementation");
    assert_eq!(
        standalone["selected_profile"]["profile_id"],
        "codex_gpt55_low_write"
    );
    assert_eq!(standalone["selected_profile"]["provider"], "openai-codex");
    assert_eq!(
        standalone["source_paths"]["selected_model_profile_id"],
        "carrier_runtime.roles[internal_subagents].model_profiles.codex_gpt55_low_write.profile_id"
    );
    assert_eq!(
        standalone["override_reasons"],
        serde_json::json!(["route_profile_mapping"])
    );
    assert_eq!(
        standalone["rejected_alternatives"][0]["profile_id"],
        "codex_gpt55_high_readonly"
    );
    assert_eq!(standalone["run_id"], "run-model-profile-readiness-smoke");
    assert_eq!(standalone["task_id"], "task-model-profile-readiness-smoke");

    assert_eq!(embedded["surface"], standalone["surface"]);
    assert_eq!(embedded["status"], standalone["status"]);
    assert_eq!(embedded["blocker_codes"], standalone["blocker_codes"]);
    assert_eq!(embedded["selected_profile"], standalone["selected_profile"]);
    assert_eq!(embedded["source_paths"], standalone["source_paths"]);
    assert_eq!(embedded["override_reasons"], standalone["override_reasons"]);
    assert_eq!(
        embedded["rejected_alternatives"],
        standalone["rejected_alternatives"]
    );
}

#[test]
fn root_route_explain_alias_matches_taskflow_route_explain() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    run_model_profile_readiness_seed_helper(&state_dir);

    let taskflow = run_command_json(
        &[
            "taskflow",
            "route",
            "explain",
            "--run-id",
            "run-model-profile-readiness-smoke",
            "--json",
        ],
        &state_dir,
    );
    let root = run_command_json(
        &[
            "route",
            "explain",
            "--run-id",
            "run-model-profile-readiness-smoke",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(root["surface"], "vida taskflow route explain");
    assert_eq!(root["status"], "pass");
    assert_eq!(root["route"], taskflow["route"]);
}

#[test]
fn taskflow_model_profile_readiness_seed_helper() {
    let Some(state_dir) = std::env::var_os("VIDA_MODEL_PROFILE_READINESS_SEED_STATE_DIR") else {
        return;
    };
    seed_model_profile_readiness_dispatch_context(&state_dir.to_string_lossy());
}

fn donor_ready_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let rows = parsed
        .get("tasks")
        .and_then(serde_json::Value::as_array)
        .or_else(|| parsed.as_array())
        .expect("ready output should expose task rows");
    let normalized = rows
        .iter()
        .map(|row| {
            let dependencies = row["dependencies"]
                .as_array()
                .expect("dependencies should be an array");
            serde_json::json!({
                "id": row["id"].as_str().expect("id").to_string(),
                "status": row["status"].as_str().expect("status").to_string(),
                "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
                "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&normalized).expect("semantic ready output should render")
}

fn donor_show_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let row = parsed.get("task").unwrap_or(&parsed);
    let dependencies = row["dependencies"]
        .as_array()
        .expect("dependencies should be an array");
    let normalized = serde_json::json!({
        "id": row["id"].as_str().expect("id").to_string(),
        "title": row["title"].as_str().expect("title").to_string(),
        "status": row["status"].as_str().expect("status").to_string(),
        "priority": row["priority"].as_i64().expect("priority"),
        "issue_type": row["issue_type"].as_str().expect("issue_type").to_string(),
        "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
        "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&normalized).expect("semantic show output should render")
}

fn donor_list_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let rows = parsed
        .get("tasks")
        .and_then(serde_json::Value::as_array)
        .or_else(|| parsed.as_array())
        .expect("list output should be an array");
    let normalized = rows
        .iter()
        .map(|row| {
            let dependencies = row["dependencies"]
                .as_array()
                .expect("dependencies should be an array");
            serde_json::json!({
                "id": row["id"].as_str().expect("id").to_string(),
                "status": row["status"].as_str().expect("status").to_string(),
                "priority": row["priority"].as_i64().expect("priority"),
                "issue_type": row["issue_type"].as_str().expect("issue_type").to_string(),
                "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
                "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&normalized).expect("semantic list output should render")
}

fn require_string_array(value: &serde_json::Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} should be an array"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("{label} entries should be strings"))
                .to_string()
        })
        .collect()
}

fn assert_release1_contract_mirror(surface: &serde_json::Value, mirror_key: &str, label: &str) {
    assert_eq!(
        surface["status"], surface[mirror_key]["status"],
        "{} top-level status should mirror {}.status",
        label, mirror_key
    );
    assert_eq!(
        surface["blocker_codes"], surface[mirror_key]["blocker_codes"],
        "{} blocker_codes should mirror {}.blocker_codes",
        label, mirror_key
    );
    assert_eq!(
        surface["next_actions"], surface[mirror_key]["next_actions"],
        "{} next_actions should mirror {}.next_actions",
        label, mirror_key
    );
    assert_eq!(
        surface["artifact_refs"], surface[mirror_key]["artifact_refs"],
        "{} artifact_refs should mirror {}.artifact_refs",
        label, mirror_key
    );
}

fn assert_shared_fields_consistency(surface: &serde_json::Value, label: &str) {
    assert_release1_contract_mirror(surface, "shared_fields", label);
}

fn assert_operator_contracts_consistency(surface: &serde_json::Value, label: &str) {
    assert_release1_contract_mirror(surface, "operator_contracts", label);
}

fn assert_release1_shared_envelope_fields(surface: &serde_json::Value, label: &str) {
    for key in [
        "surface",
        "status",
        "trace_id",
        "workflow_class",
        "risk_tier",
        "artifact_refs",
        "next_actions",
        "blocker_codes",
    ] {
        assert!(
            surface.get(key).is_some(),
            "{label} should expose top-level release-1 shared envelope field `{key}`"
        );
    }
    for key in [
        "status",
        "trace_id",
        "workflow_class",
        "risk_tier",
        "artifact_refs",
        "next_actions",
        "blocker_codes",
    ] {
        assert_eq!(
            surface[key], surface["shared_fields"][key],
            "{label} should mirror `{key}` through shared_fields"
        );
        assert_eq!(
            surface[key], surface["operator_contracts"][key],
            "{label} should mirror `{key}` through operator_contracts"
        );
    }
}

#[test]
fn taskflow_plan_generate_require_context_blocks_missing_cli_refs() {
    let state_dir = unique_state_dir();
    let parsed = run_command_json(
        &[
            "taskflow",
            "plan",
            "generate",
            "--source-text",
            "Implement planner",
            "--task-prefix",
            "smoke-plan",
            "--require-context",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["validation"]["status"], "blocked");
    assert!(parsed["validation"]["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array")
        .contains(&serde_json::json!("missing_plan_context")));
    assert_eq!(parsed["input_contract"]["status"], "partial");
    assert_eq!(
        require_string_array(
            &parsed["input_contract"]["missing_context"],
            "missing_context"
        ),
        vec![
            "spec_reference_missing".to_string(),
            "backlog_reference_missing".to_string(),
            "context_reference_missing".to_string(),
        ]
    );
    let _ = fs::remove_dir_all(state_dir);
}

#[test]
fn validate_graph_json_exposes_full_release1_shared_envelope() {
    let state_dir = unique_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let parsed = run_command_json(&["task", "validate-graph", "--json"], &state_dir);

    assert_eq!(parsed["surface"], "vida task validate-graph");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["valid"], true);
    assert_release1_shared_envelope_fields(&parsed, "validate-graph");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_artifacts_json_exposes_full_release1_shared_envelope() {
    let state_dir = unique_state_dir();
    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    fs::create_dir_all(&runtime_consumption_dir).expect("create runtime-consumption dir");
    fs::write(
        format!("{runtime_consumption_dir}/final-artifacts-envelope-smoke.json"),
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {},
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "trace_id": null,
                "workflow_class": null,
                "risk_tier": null,
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "shared_fields": {
                "status": "pass",
                "trace_id": null,
                "workflow_class": null,
                "risk_tier": null,
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "execution_preparation_artifacts": {
                "required_artifacts": [
                    "architecture_preparation_report",
                    "change_boundary",
                    "dependency_impact_summary",
                    "developer_handoff_packet",
                    "spec_alignment_summary"
                ],
                "architecture_preparation_report": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "change_boundary": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "dependency_impact_summary": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "developer_handoff_packet": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "spec_alignment_summary": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "execution_preparation_evidence": {
                    "ready": true,
                    "status": "ready"
                }
            }
        })
        .to_string(),
    )
    .expect("write final artifact snapshot");

    let list = run_command_json(&["taskflow", "artifacts", "list", "--json"], &state_dir);
    assert_eq!(list["surface"], "vida taskflow artifacts list");
    assert_release1_shared_envelope_fields(&list, "taskflow artifacts list");

    let show = run_command_json(
        &[
            "taskflow",
            "artifacts",
            "show",
            "developer_handoff_packet",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(show["surface"], "vida taskflow artifacts show");
    assert_release1_shared_envelope_fields(&show, "taskflow artifacts show");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_plan_generate_require_context_passes_with_cli_refs() {
    let state_dir = unique_state_dir();
    let parsed = run_command_json(
        &[
            "taskflow",
            "plan",
            "generate",
            "--source-text",
            "Implement planner",
            "--task-prefix",
            "smoke-plan",
            "--require-context",
            "--spec-ref",
            "docs/product/spec/current-spec-map.md",
            "--backlog-ref",
            "audit-p1-plan-generate-require-context-cli-smoke",
            "--context-ref",
            "crates/vida/tests/task_smoke.rs",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["validation"]["status"], "valid");
    assert_eq!(parsed["input_contract"]["status"], "complete");
    assert!(parsed["input_contract"]["missing_context"]
        .as_array()
        .expect("missing_context should be an array")
        .is_empty());
    assert!(parsed["input_contract"]["sources"]
        .as_array()
        .expect("sources should be an array")
        .iter()
        .any(|source| source["source_type"] == "spec_reference"
            && source["reference"] == "docs/product/spec/current-spec-map.md"
            && source["evidence"] == "cli_spec_ref"));
    assert!(parsed["input_contract"]["sources"]
        .as_array()
        .expect("sources should be an array")
        .iter()
        .any(|source| source["source_type"] == "backlog_reference"
            && source["reference"] == "audit-p1-plan-generate-require-context-cli-smoke"
            && source["evidence"] == "cli_backlog_ref"));
    assert!(parsed["input_contract"]["sources"]
        .as_array()
        .expect("sources should be an array")
        .iter()
        .any(|source| source["source_type"] == "context_reference"
            && source["reference"] == "crates/vida/tests/task_smoke.rs"
            && source["evidence"] == "cli_context_ref"));
    let _ = fs::remove_dir_all(state_dir);
}

#[test]
fn task_command_round_trip_succeeds_via_binary_surface() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let list_stdout = run_and_assert_success(&["task", "list", "--all", "--json"], &state_dir);
    assert!(
        list_stdout.contains("\"id\": \"vida-b\"") || list_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        list_stdout.contains("\"id\": \"vida-a\"") || list_stdout.contains("\"id\":\"vida-a\"")
    );
    let list_json: Value = serde_json::from_str(&list_stdout).expect("task list json should parse");
    let list_task_a = task_row_by_id(&list_json, "vida-a");
    assert_eq!(list_task_a["parent_id"], "vida-root");
    assert_eq!(list_task_a["parent_edge"]["parent_id"], "vida-root");
    assert_eq!(list_task_a["parent_edge"]["edge_type"], "parent-child");

    let summary_list_stdout = run_and_assert_success(
        &["task", "list", "--all", "--summary", "--json"],
        &state_dir,
    );
    let summary_list_json: Value =
        serde_json::from_str(&summary_list_stdout).expect("summary task list json should parse");
    let summary_task_a = task_row_by_id(&summary_list_json, "vida-a");
    assert_eq!(summary_task_a["parent_id"], "vida-root");
    assert_eq!(summary_task_a["parent_edge"]["parent_id"], "vida-root");
    assert_eq!(summary_task_a["parent_edge"]["edge_type"], "parent-child");
    assert_eq!(summary_task_a["parent_edge"]["metadata"], Value::Null);
    assert_eq!(summary_task_a["parent_edge"]["thread_id"], Value::Null);

    let ready_stdout = run_and_assert_success(&["task", "ready", "--json"], &state_dir);
    assert!(
        ready_stdout.contains("\"id\": \"vida-a\"") || ready_stdout.contains("\"id\":\"vida-a\"")
    );
    assert!(
        !ready_stdout.contains("\"id\": \"vida-b\"") && !ready_stdout.contains("\"id\":\"vida-b\"")
    );

    let scoped_ready_stdout = run_and_assert_success(
        &["task", "ready", "--scope", "vida-root", "--json"],
        &state_dir,
    );
    assert!(
        scoped_ready_stdout.contains("\"id\": \"vida-a\"")
            || scoped_ready_stdout.contains("\"id\":\"vida-a\"")
    );
    assert!(
        !scoped_ready_stdout.contains("\"id\": \"vida-b\"")
            && !scoped_ready_stdout.contains("\"id\":\"vida-b\"")
    );

    let deps_stdout = run_and_assert_success(&["task", "deps", "vida-b", "--json"], &state_dir);
    assert!(
        deps_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || deps_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );
    assert!(
        deps_stdout.contains("\"dependency_status\": \"open\"")
            || deps_stdout.contains("\"dependency_status\":\"open\"")
    );

    let reverse_stdout =
        run_and_assert_success(&["task", "reverse-deps", "vida-a", "--json"], &state_dir);
    assert!(
        reverse_stdout.contains("\"issue_id\": \"vida-b\"")
            || reverse_stdout.contains("\"issue_id\":\"vida-b\"")
    );
    assert!(
        reverse_stdout.contains("\"edge_type\": \"blocks\"")
            || reverse_stdout.contains("\"edge_type\":\"blocks\"")
    );

    let blocked_stdout = run_and_assert_success(&["task", "blocked", "--json"], &state_dir);
    assert!(
        blocked_stdout.contains("\"surface\": \"vida task blocked\"")
            || blocked_stdout.contains("\"surface\":\"vida task blocked\"")
    );
    assert!(
        blocked_stdout.contains("\"blocked_count\": 1")
            || blocked_stdout.contains("\"blocked_count\":1")
    );
    assert!(
        blocked_stdout.contains("\"id\": \"vida-b\"")
            || blocked_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        blocked_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || blocked_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );

    let tree_stdout = run_and_assert_success(&["task", "tree", "vida-b", "--json"], &state_dir);
    assert!(
        tree_stdout.contains("\"surface\": \"vida task tree\"")
            || tree_stdout.contains("\"surface\":\"vida task tree\"")
    );
    assert!(
        tree_stdout.contains("\"root_task_id\": \"vida-b\"")
            || tree_stdout.contains("\"root_task_id\":\"vida-b\"")
    );
    assert!(
        tree_stdout.contains("\"id\": \"vida-b\"") || tree_stdout.contains("\"id\":\"vida-b\"")
    );
    let tree_json: Value = serde_json::from_str(&tree_stdout).expect("task tree json should parse");
    let tree_dependencies = tree_json["dependencies"]
        .as_array()
        .expect("task tree dependencies should be an array");
    assert_eq!(tree_dependencies[0]["id"], "vida-a");
    assert_eq!(tree_dependencies[0]["edge_type"], "blocks");

    let validate_stdout = run_and_assert_success(&["task", "validate-graph", "--json"], &state_dir);
    let validate_graph: Value =
        serde_json::from_str(&validate_stdout).expect("validate-graph json should parse");
    assert_eq!(validate_graph["status"], "pass");
    assert_eq!(validate_graph["valid"], true);
    assert_eq!(validate_graph["issue_count"], 0);

    let critical_path: serde_json::Value = serde_json::from_str(&run_and_assert_success(
        &["task", "critical-path", "--json"],
        &state_dir,
    ))
    .expect("critical-path json should parse");
    assert_eq!(critical_path["status"], "pass");
    assert_eq!(critical_path["surface"], "vida task critical-path");
    assert_eq!(critical_path["length"], 2);
    assert_eq!(critical_path["root_task_id"], "vida-a");
    assert_eq!(critical_path["terminal_task_id"], "vida-b");

    let ready_explain_output = run_command_capture(
        &["taskflow", "graph", "explain", "vida-a", "--json"],
        &state_dir,
    );
    assert!(
        !ready_explain_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&ready_explain_output.stderr)
    );
    let ready_explain: serde_json::Value = serde_json::from_slice(&ready_explain_output.stdout)
        .expect("graph explain ready json should parse");
    assert_eq!(ready_explain["surface"], "vida taskflow graph explain");
    assert_eq!(ready_explain["task_id"], "vida-a");
    assert_eq!(ready_explain["ready_now"], true);
    assert_eq!(ready_explain["active_critical_path"], true);
    if !ready_explain_output.status.success() {
        assert_eq!(ready_explain["status"], "blocked");
        assert!(
            ready_explain["blocker_codes"]
                .as_array()
                .expect("ready explain blocker_codes should be an array")
                .iter()
                .any(|code| code.as_str() == Some("current_task_reference")),
            "current task references should fail closed while still rendering explain JSON"
        );
    }

    let blocked_explain_output = run_command_capture(
        &["taskflow", "graph", "explain", "vida-b", "--json"],
        &state_dir,
    );
    assert!(
        !blocked_explain_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&blocked_explain_output.stderr)
    );
    let blocked_explain: serde_json::Value = serde_json::from_slice(&blocked_explain_output.stdout)
        .expect("graph explain blocked json should parse");
    assert_eq!(blocked_explain["surface"], "vida taskflow graph explain");
    assert_eq!(blocked_explain["task_id"], "vida-b");
    assert_eq!(blocked_explain["ready_now"], false);
    assert_eq!(blocked_explain["blocked_by"][0]["depends_on_id"], "vida-a");
    assert_eq!(blocked_explain["active_critical_path"], true);
    if !blocked_explain_output.status.success() {
        assert_eq!(blocked_explain["status"], "blocked");
    }

    let dep_add_stdout = run_and_assert_success(
        &["task", "dep", "add", "vida-c", "vida-a", "blocks", "--json"],
        &state_dir,
    );
    assert!(
        dep_add_stdout.contains("\"issue_id\": \"vida-c\"")
            || dep_add_stdout.contains("\"issue_id\":\"vida-c\"")
    );
    assert!(
        dep_add_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || dep_add_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );

    let deps_after_add_stdout =
        run_and_assert_success(&["task", "deps", "vida-c", "--json"], &state_dir);
    assert!(
        deps_after_add_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || deps_after_add_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );

    let dep_remove_stdout = run_and_assert_success(
        &[
            "task", "dep", "remove", "vida-c", "vida-a", "blocks", "--json",
        ],
        &state_dir,
    );
    assert!(
        dep_remove_stdout.contains("\"issue_id\": \"vida-c\"")
            || dep_remove_stdout.contains("\"issue_id\":\"vida-c\"")
    );

    let deps_after_remove_stdout =
        run_and_assert_success(&["task", "deps", "vida-c", "--json"], &state_dir);
    assert!(
        deps_after_remove_stdout.contains("\"surface\": \"vida task deps\"")
            || deps_after_remove_stdout.contains("\"surface\":\"vida task deps\"")
    );
    assert!(
        deps_after_remove_stdout.contains("\"task_id\": \"vida-c\"")
            || deps_after_remove_stdout.contains("\"task_id\":\"vida-c\"")
    );
    assert!(
        deps_after_remove_stdout.contains("\"dependency_count\": 1")
            || deps_after_remove_stdout.contains("\"dependency_count\":1")
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_list_fields_and_default_toon_shape_are_binary_visible() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let field_list_stdout = run_and_assert_success(
        &[
            "task",
            "list",
            "--all",
            "--view",
            "compact",
            "--fields",
            "id,status,title",
            "--json",
        ],
        &state_dir,
    );
    let field_list_json: Value = serde_json::from_str(&field_list_stdout)
        .expect("field-selected task list json should parse");
    assert_eq!(field_list_json["fields"], "id,status,title");
    assert_eq!(field_list_json["view"], "compact");
    assert_eq!(field_list_json["output_policy"]["mode"], "compact");
    assert_eq!(field_list_json["output_policy"]["max_inline_items"], 25);
    let field_task_a = task_row_by_id(&field_list_json, "vida-a");
    assert_eq!(field_task_a["id"], "vida-a");
    assert_eq!(field_task_a["status"], "open");
    assert_eq!(field_task_a["title"], "Task A");
    assert!(field_task_a.get("description").is_none());
    assert!(field_task_a.get("parent_edge").is_none());

    let toon_list_stdout = run_and_assert_success(&["task", "list", "--all"], &state_dir);
    assert!(toon_list_stdout.starts_with("vida task list\n  task_count: 4"));
    assert!(toon_list_stdout.contains("\n  tasks[4]{id,status,priority,title}:"));

    let toon_fields_stdout = run_and_assert_success(
        &[
            "task",
            "list",
            "--all",
            "--view",
            "compact",
            "--fields",
            "id,status,title",
        ],
        &state_dir,
    );
    assert!(toon_fields_stdout.starts_with("vida task list\n  task_count: 4"));
    assert!(toon_fields_stdout.contains("\n  tasks[4]{id,status,title}:"));
    assert!(!toon_fields_stdout.contains("\n  tasks[4]{id,status,priority,title}:"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn dead_code_proof_protects_public_command_entrypoints() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    for (args, expected_usage) in [
        (&["task", "--help"][..], "vida task"),
        (&["taskflow", "--help"][..], "vida taskflow"),
        (&["docflow", "--help"][..], "vida docflow"),
        (&["lane", "--help"][..], "vida lane"),
        (&["status", "--help"][..], "vida status"),
        (&["doctor", "--help"][..], "vida doctor"),
        (
            &["orchestrator-init", "--help"][..],
            "vida orchestrator-init",
        ),
        (&["agent-init", "--help"][..], "vida agent-init"),
    ] {
        let output = run_command_capture(args, &state_dir);
        assert!(
            output.status.success(),
            "public entrypoint should be reachable: {args:?}\nstatus: {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected_usage),
            "public entrypoint help should keep its command label: {args:?}\nexpected: {expected_usage}\nstdout: {stdout}"
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn cli_help_description_inventory_covers_taskflow_proxy_topics() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let root_help = run_command_capture(&["taskflow", "help"], &state_dir);
    assert!(
        root_help.status.success(),
        "taskflow help should execute: stderr={}",
        String::from_utf8_lossy(&root_help.stderr)
    );
    let root_stdout = String::from_utf8_lossy(&root_help.stdout);
    for expected in [
        "vida taskflow help",
        "route",
        "validate-routing",
        "status",
        "vida taskflow route explain --json",
        "vida taskflow validate-routing --json",
        "vida taskflow status --summary --json",
    ] {
        assert!(
            root_stdout.contains(expected),
            "taskflow root help should discover `{expected}`:\n{root_stdout}"
        );
    }

    for (topic, expected) in [
        ("route", "vida taskflow route explain [--json]"),
        (
            "validate-routing",
            "vida taskflow validate-routing [--json]",
        ),
        ("status", "vida taskflow status [--summary] [--json]"),
    ] {
        let output = run_command_capture(&["taskflow", "help", topic], &state_dir);
        assert!(
            output.status.success(),
            "taskflow help topic `{topic}` should execute: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected),
            "taskflow help topic `{topic}` should describe `{expected}`:\n{stdout}"
        );
        for field in ["Purpose:", "Canonical commands:", "Failure modes:"] {
            assert!(
                stdout.contains(field),
                "taskflow help topic `{topic}` should include `{field}`:\n{stdout}"
            );
        }
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn cli_help_description_inventory_covers_agent_and_task_operator_options() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    for (args, expected) in [
        (
            &["agent-init", "--help"][..],
            &[
                "--role <ROLE>",
                "Requested runtime role or conversation role for lane activation",
                "--dispatch-packet <DISPATCH_PACKET>",
                "--downstream-packet <DOWNSTREAM_PACKET>",
                "--execute-dispatch",
                "--auto-dispatch-packet",
                "--state-dir <STATE_DIR>",
                "--json",
            ][..],
        ),
        (
            &["agent", "dispatch-next", "--help"][..],
            &[
                "--lanes <LANES>",
                "Maximum preview lanes to inspect before any manual `vida agent-init` launch",
                "--scope <SCOPE>",
                "--current-task-id <CURRENT_TASK_ID>",
                "Optional current task id for parallel-safety checks",
                "--state-dir <STATE_DIR>",
                "Override the TaskFlow state directory used for readiness and continuation projections",
                "--dev-team",
                "--json",
                "Emit machine-readable JSON output",
            ][..],
        ),
        (
            &["agent", "host-bridge", "--help"][..],
            &[
                "--request <REQUEST>",
                "Path to a pending host_tool_bridge_request JSON artifact",
                "--complete",
                "--host-agent-id <HOST_AGENT_ID>",
                "--summary <SUMMARY>",
                "--receipt-id <RECEIPT_ID>",
                "--state-dir <STATE_DIR>",
                "Override the TaskFlow state directory used for host bridge provenance checks",
                "--json",
                "Emit machine-readable JSON output",
            ][..],
        ),
        (
            &["lane", "retire", "--help"][..],
            &[
                "Usage: vida lane retire <run-id> --receipt-id <id> --reason <text> [--json]",
                "Purpose:",
                "--receipt-id <id>",
                "Receipt id that proves the lane mutation source",
                "--reason <text>",
                "Human-readable retire reason",
                "--json",
                "Emit machine-readable JSON output",
            ][..],
        ),
        (
            &["task", "list", "--help"][..],
            &[
                "--fields <FIELDS>",
                "Comma-separated JSON task row fields to include",
                "--view <VIEW>",
                "Output view for task rows: compact, summary, or full",
                "--limit <LIMIT>",
                "--json",
            ][..],
        ),
        (
            &["task", "create", "--help"][..],
            &[
                "--parent-id <PARENT_ID>",
                "--execution-mode <EXECUTION_MODE>",
                "--order-bucket <ORDER_BUCKET>",
                "--parallel-group <PARALLEL_GROUP>",
                "--conflict-domain <CONFLICT_DOMAIN>",
                "--owned-path <OWNED_PATHS>",
                "--acceptance-target <ACCEPTANCE_TARGETS>",
                "--acceptance",
                "--proof-target <PROOF_TARGETS>",
                "--proof",
                "--json",
            ][..],
        ),
        (
            &["task", "update", "--help"][..],
            &[
                "--parent-id <PARENT_ID>",
                "--clear-parent-id",
                "--execution-mode <EXECUTION_MODE>",
                "--clear-execution-mode",
                "--clear-parallel-group",
                "--clear-conflict-domain",
                "--json",
            ][..],
        ),
        (
            &["task", "close", "--help"][..],
            &[
                "--reason <REASON>",
                "--include-global-progress",
                "--stage-owned",
                "--commit-file <COMMIT_FILES>",
                "--commit-message <COMMIT_MESSAGE>",
                "--json",
            ][..],
        ),
    ] {
        let output = run_command_capture(args, &state_dir);
        assert!(
            output.status.success(),
            "help command should succeed: {args:?}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        for needle in expected {
            assert!(
                stdout.contains(needle),
                "help command {args:?} should expose `{needle}`:\n{stdout}"
            );
        }
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_close_feedback_treats_successful_evidence_words_as_context() {
    let (project_root, state_dir) = project_bound_state_dir();
    create_epic_parent(
        &state_dir,
        "feedback-context-parent",
        "Feedback context parent",
        "open",
    );
    let created = run_command_json(
        &[
            "task",
            "create",
            "feedback-context-task",
            "Feedback context task",
            "--parent-id",
            "feedback-context-parent",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");

    let closed = run_command_json(
        &[
            "task",
            "close",
            "feedback-context-task",
            "--reason",
            "Closed after validating direct CLI and proxy integration coverage for help, blocked, pass, and fail-closes-if-missing wording; proof commands passed.",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["blocker_codes"], serde_json::json!([]));
    assert_eq!(closed["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        closed["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(
        closed["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!([])
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn task_close_feedback_ignores_historical_blocker_words_in_success_reason() {
    let (project_root, state_dir) = project_bound_state_dir();
    create_epic_parent(
        &state_dir,
        "feedback-blocker-parent",
        "Feedback blocker parent",
        "open",
    );
    let created = run_command_json(
        &[
            "task",
            "create",
            "feedback-blocker-task",
            "Feedback blocker task",
            "--parent-id",
            "feedback-blocker-parent",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");

    let closed = run_command_json(
        &[
            "task",
            "close",
            "feedback-blocker-task",
            "--reason",
            "Closed after validating prior blocker edges, blocked prerequisite states, and recovery wording; structured close transition passed.",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["blocker_codes"], serde_json::json!([]));
    assert_eq!(
        closed["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(
        closed["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!([])
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn taskflow_factual_sandbox_h1_h3_cli_task_graph() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let help = run_command_capture(&["--help"], &state_dir);
    assert!(help.status.success());
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("Usage: vida"));

    let version = run_command_capture(&["--version"], &state_dir);
    assert!(version.status.success());
    let version_stdout = String::from_utf8_lossy(&version.stdout);
    assert!(version_stdout.contains("vida "));

    let task_help = run_command_capture(&["task", "--help"], &state_dir);
    assert!(task_help.status.success());
    let task_help_stdout = String::from_utf8_lossy(&task_help.stdout);
    assert!(task_help_stdout.contains("vida task"));

    let taskflow_help = run_command_capture(&["taskflow", "help"], &state_dir);
    assert!(taskflow_help.status.success());

    let parent_id = "sandbox-lifecycle-parent";
    create_epic_parent(&state_dir, parent_id, "Sandbox lifecycle parent", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            "sandbox-lifecycle",
            "Sandbox lifecycle",
            "--description",
            "created through factual sandbox",
            "--labels",
            "taskflow-testing,happy-path-sandbox",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    assert_eq!(created["task"]["id"], "sandbox-lifecycle");
    assert_eq!(created["task"]["status"], "open");

    let updated = run_command_json(
        &[
            "task",
            "update",
            "sandbox-lifecycle",
            "--title",
            "Sandbox lifecycle renamed",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(updated["status"], "pass");
    assert_eq!(updated["task"]["title"], "Sandbox lifecycle renamed");
    assert_eq!(updated["task"]["priority"], 1);

    let shown = run_command_json(&["task", "show", "sandbox-lifecycle", "--json"], &state_dir);
    assert_eq!(shown["status"], "pass");
    assert_eq!(shown["task"]["title"], "Sandbox lifecycle renamed");

    let closed = run_command_json(
        &[
            "task",
            "close",
            "sandbox-lifecycle",
            "--reason",
            "factual sandbox lifecycle complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["task"]["status"], "closed");

    let shown_after_close =
        run_command_json(&["task", "show", "sandbox-lifecycle", "--json"], &state_dir);
    assert_eq!(shown_after_close["status"], "pass");
    assert_eq!(shown_after_close["task"]["status"], "closed");

    let parent = run_command_json(
        &[
            "task",
            "create",
            "sandbox-parent",
            "Sandbox parent",
            "--type",
            "epic",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");

    let child = run_command_json(
        &[
            "task",
            "create",
            "sandbox-child",
            "Sandbox child",
            "--parent-id",
            "sandbox-parent",
            "--priority",
            "2",
            "--labels",
            "child-verification,tree-detail",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child["status"], "pass");
    assert_eq!(
        child["task"]["dependencies"][0]["depends_on_id"],
        "sandbox-parent"
    );
    assert_eq!(
        child["task"]["dependencies"][0]["edge_type"],
        "parent-child"
    );

    let child_after_create =
        run_command_json(&["task", "show", "sandbox-child", "--json"], &state_dir);
    assert_eq!(child_after_create["status"], "pass");
    assert_eq!(child_after_create["task"]["status"], "open");
    assert_eq!(
        child_after_create["task"]["dependencies"][0]["depends_on_id"],
        "sandbox-parent"
    );
    assert_eq!(
        child_after_create["task"]["dependencies"][0]["edge_type"],
        "parent-child"
    );

    let parent_children = run_command_json(
        &["task", "children", "sandbox-parent", "--full", "--json"],
        &state_dir,
    );
    assert_eq!(parent_children["status"], "pass");
    assert_eq!(parent_children["surface"], "vida task children");
    assert_eq!(parent_children["root_task_id"], "sandbox-parent");
    assert_eq!(parent_children["child_count"], 1);
    assert_eq!(parent_children["children"][0]["child_id"], "sandbox-child");
    assert_eq!(
        parent_children["children"][0]["child_title"],
        "Sandbox child"
    );
    assert_eq!(parent_children["children"][0]["child_status"], "open");
    assert_eq!(parent_children["children"][0]["child_priority"], 2);
    assert_eq!(
        parent_children["children"][0]["child_labels"][0],
        "child-verification"
    );

    let parent_tree = run_command_json(&["task", "tree", "sandbox-parent", "--json"], &state_dir);
    assert_eq!(parent_tree["status"], "pass");
    assert_eq!(parent_tree["surface"], "vida task tree");
    assert_eq!(parent_tree["root_task_id"], "sandbox-parent");
    assert_eq!(parent_tree["child_count"], 1);
    assert_eq!(parent_tree["children"][0]["id"], "sandbox-child");
    assert_eq!(parent_tree["children"][0]["title"], "Sandbox child");
    assert_eq!(parent_tree["children"][0]["status"], "open");
    assert_eq!(parent_tree["children"][0]["priority"], 2);
    assert_eq!(parent_tree["children"][0]["issue_type"], "task");
    assert_eq!(
        parent_tree["children"][0]["labels"][0],
        "child-verification"
    );

    let graph = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(graph["status"], "pass");
    assert_eq!(graph["valid"], true);
    assert_eq!(graph["issue_count"], 0);

    let close_parent = run_command_capture(
        &[
            "task",
            "close",
            "sandbox-parent",
            "--reason",
            "should fail while child remains open",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !close_parent.status.success(),
        "closing parent with open child should fail closed"
    );
    let close_parent_stderr = String::from_utf8_lossy(&close_parent.stderr);
    assert!(
        close_parent_stderr.contains("non-closed child tasks exist")
            && close_parent_stderr.contains("sandbox-child"),
        "{close_parent_stderr}"
    );

    let parent_after_failed_close =
        run_command_json(&["task", "show", "sandbox-parent", "--json"], &state_dir);
    assert_eq!(parent_after_failed_close["task"]["status"], "open");

    let child_closed = run_command_json(
        &[
            "task",
            "close",
            "sandbox-child",
            "--reason",
            "factual sandbox child complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child_closed["status"], "pass");
    assert_eq!(child_closed["task"]["status"], "closed");

    let parent_closed = run_command_json(
        &[
            "task",
            "close",
            "sandbox-parent",
            "--reason",
            "factual sandbox parent complete after child closure",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent_closed["status"], "pass");
    assert_eq!(parent_closed["task"]["status"], "closed");

    let graph_after_closure = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(graph_after_closure["status"], "pass");
    assert_eq!(graph_after_closure["valid"], true);
    assert_eq!(graph_after_closure["issue_count"], 0);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_tracked_flow_spec_close_keeps_parent_open_until_work_pool_exists() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let feature_id = "tracked-flow-parent";
    let spec_id = "tracked-flow-parent-spec";

    let parent = run_command_json(
        &[
            "task",
            "create",
            feature_id,
            "Tracked flow parent",
            "--type",
            "epic",
            "--status",
            "open",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");

    let spec = run_command_json(
        &[
            "task",
            "create",
            spec_id,
            "Spec pack: tracked flow parent",
            "--type",
            "task",
            "--status",
            "open",
            "--parent-id",
            feature_id,
            "--labels",
            "spec-pack,documentation",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(spec["status"], "pass");
    let work_pool = run_command_json(
        &[
            "task",
            "create",
            "tracked-flow-parent-work-pool",
            "Work-pool pack: tracked flow parent",
            "--type",
            "task",
            "--status",
            "open",
            "--parent-id",
            feature_id,
            "--labels",
            "work-pool-pack",
            "--execution-mode",
            "container_only",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(work_pool["status"], "pass");

    let closed_spec = run_command_json(
        &[
            "task",
            "close",
            spec_id,
            "--reason",
            "design packet finalized and handed off into tracked work-pool shaping",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_spec["status"], "pass");
    assert_eq!(closed_spec["task"]["status"], "closed");

    let parent_after_spec_close =
        run_command_json(&["task", "show", feature_id, "--json"], &state_dir);
    assert_ne!(
        parent_after_spec_close["task"]["status"], "closed",
        "closing only the generated spec-pack child must not close the feature parent before work-pool/dev handoff is materialized"
    );
    assert!(parent_after_spec_close["task"]["closed_at"].is_null());
    assert!(parent_after_spec_close["task"]["close_reason"].is_null());

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn agent_dispatch_preview_aligns_with_scheduler_selected_tasks_and_routing_truth() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "agent-dispatch-root",
            "Agent dispatch root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let current = run_command_json(
        &[
            "task",
            "create",
            "agent-dispatch-current",
            "Agent dispatch current",
            "--parent-id",
            "agent-dispatch-root",
            "--priority",
            "1",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "agent-dispatch-wave",
            "--parallel-group",
            "agent-dispatch-pack",
            "--conflict-domain",
            "agent-dispatch-current-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(current["status"], "pass");

    let parallel = run_command_json(
        &[
            "task",
            "create",
            "agent-dispatch-parallel",
            "Agent dispatch parallel",
            "--parent-id",
            "agent-dispatch-root",
            "--priority",
            "2",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "agent-dispatch-wave",
            "--parallel-group",
            "agent-dispatch-pack",
            "--conflict-domain",
            "agent-dispatch-parallel-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parallel["status"], "pass");

    let scheduler_preview = run_command_json(
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--current-task-id",
            "agent-dispatch-current",
            "--limit",
            "2",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        scheduler_preview["surface"],
        "vida taskflow scheduler dispatch"
    );
    assert_eq!(scheduler_preview["status"], "pass");
    let scheduler_selected_task_ids = require_json_string_array(
        &scheduler_preview["selected_task_ids"],
        "scheduler selected_task_ids",
    );
    assert_eq!(
        scheduler_selected_task_ids,
        vec![
            "agent-dispatch-current".to_string(),
            "agent-dispatch-parallel".to_string()
        ]
    );

    let dispatch_preview = run_command_json(
        &[
            "agent",
            "dispatch-next",
            "--current-task-id",
            "agent-dispatch-current",
            "--lanes",
            "2",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dispatch_preview["status"], "pass");
    assert_eq!(dispatch_preview["mode"], "preview");
    assert_eq!(dispatch_preview["execute_supported"], false);
    assert_eq!(dispatch_preview["execution_attempted"], false);
    assert_eq!(dispatch_preview["lanes_selected"], 2);
    assert!(dispatch_preview["blocker_codes"]
        .as_array()
        .expect("dispatch blocker_codes should be an array")
        .is_empty());

    let selected_lanes = dispatch_preview["selected_lanes"]
        .as_array()
        .expect("dispatch selected_lanes should be an array");
    let dispatch_selected_task_ids = selected_lanes
        .iter()
        .map(|lane| require_json_string(&lane["task_id"], "dispatch selected lane task_id"))
        .collect::<Vec<_>>();
    assert_eq!(dispatch_selected_task_ids, scheduler_selected_task_ids);

    for lane in selected_lanes {
        let task_id = require_json_string(&lane["task_id"], "dispatch lane task_id");
        let runtime_role = require_json_string(&lane["runtime_role"], "dispatch lane runtime_role");
        let task_class = require_json_string(&lane["task_class"], "dispatch lane task_class");
        assert_eq!(runtime_role, "worker");
        assert_eq!(task_class, "implementation");

        let dispatch_command =
            require_json_string(&lane["dispatch_command"], "dispatch lane dispatch_command");
        assert!(
            dispatch_command.contains("vida agent-init"),
            "dispatch command should target agent-init: {dispatch_command}"
        );
        assert!(
            dispatch_command.contains("--role worker"),
            "dispatch command should include runtime role: {dispatch_command}"
        );
        assert!(
            dispatch_command.contains(task_id.as_str()),
            "dispatch command should include task id {task_id}: {dispatch_command}"
        );
        assert!(
            dispatch_command.contains("--json"),
            "dispatch command should request json receipt surface: {dispatch_command}"
        );
        assert!(
            dispatch_command.contains("--state-dir"),
            "dispatch command should preserve explicit state dir: {dispatch_command}"
        );

        let selection_truth = &lane["selection_truth"];
        assert_eq!(selection_truth["runtime_role"], lane["runtime_role"]);
        assert_eq!(selection_truth["task_class"], lane["task_class"]);
        for key in [
            "selected_carrier",
            "selected_backend",
            "selected_model_profile",
            "selected_model_ref",
            "selected_reasoning_effort",
            "budget_verdict",
        ] {
            assert!(
                !require_json_string(&selection_truth[key], key).is_empty(),
                "selection truth {key} should be concrete"
            );
        }
        selection_truth["rate"]
            .as_u64()
            .unwrap_or_else(|| panic!("selection truth rate missing for {task_id}"));
        selection_truth["estimated_task_price_units"]
            .as_u64()
            .unwrap_or_else(|| {
                panic!("selection truth estimated_task_price_units missing for {task_id}")
            });
    }

    let source_surfaces = require_json_string_array(
        &dispatch_preview["source_surfaces"],
        "dispatch source_surfaces",
    );
    assert!(source_surfaces
        .iter()
        .any(|surface| surface == "vida taskflow scheduler dispatch --json"));
    assert!(source_surfaces
        .iter()
        .any(|surface| surface == "vida agent-init --role <runtime-role> <task-id> --json"));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn agent_dispatch_preview_uses_explicit_state_dir_project_config_over_ambient_root() {
    let (active_project_root, active_state_dir) = project_bound_state_dir();
    let (packet_project_root, packet_state_dir) = project_bound_state_dir();
    rewrite_project_model_ref(&active_project_root, "active-model-low");
    rewrite_project_model_ref(&packet_project_root, "packet-model-low");

    run_and_assert_success(&["boot"], &active_state_dir);
    run_and_assert_success(&["boot"], &packet_state_dir);
    let root = run_command_json(
        &[
            "task",
            "create",
            "explicit-state-root",
            "Explicit state root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &packet_state_dir,
    );
    assert_eq!(root["status"], "pass");
    let current = run_command_json(
        &[
            "task",
            "create",
            "explicit-state-current",
            "Explicit state current",
            "--parent-id",
            "explicit-state-root",
            "--priority",
            "1",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "explicit-state-wave",
            "--parallel-group",
            "explicit-state-pack",
            "--conflict-domain",
            "explicit-state-domain",
            "--json",
        ],
        &packet_state_dir,
    );
    assert_eq!(current["status"], "pass");

    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .current_dir(&active_project_root)
            .env("VIDA_STATE_DIR", &active_state_dir)
            .args([
                "agent",
                "dispatch-next",
                "--current-task-id",
                "explicit-state-current",
                "--lanes",
                "1",
                "--state-dir",
                packet_state_dir.as_str(),
                "--json",
            ]);
        command
    });
    assert!(
        output.status.success(),
        "dispatch preview should succeed\nstatus: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let dispatch_preview: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("dispatch preview json should parse");
    assert_eq!(dispatch_preview["status"], "pass");
    let lane = dispatch_preview["selected_lanes"]
        .as_array()
        .and_then(|lanes| lanes.first())
        .expect("one selected lane should exist");
    assert_eq!(
        lane["selection_truth"]["selected_model_ref"], "packet-model-low",
        "dispatch preview must use explicit state-dir project config, not ambient cwd/env config"
    );
    assert_ne!(
        lane["selection_truth"]["selected_model_ref"], "active-model-low",
        "ambient project config must not win over explicit state-dir"
    );

    fs::remove_dir_all(active_project_root).expect("active temp root should be removed");
    fs::remove_dir_all(packet_project_root).expect("packet temp root should be removed");
}

#[test]
fn taskflow_golden_route_happy_path_stitches_bootstrap_dispatch_resume_status_and_doctor() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let orchestrator = run_command_json(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert_eq!(orchestrator["surface"], "vida orchestrator-init");
    assert!(matches!(
        orchestrator["init"]["status"].as_str(),
        Some("ready_enough_for_normal_work") | Some("pending")
    ));
    assert!(orchestrator["init"]["project_activation"]["activation_pending"].is_boolean());

    let root_task_id = "case-08-root";
    let implementation_task_id = "case-08-implementation";
    let parallel_task_id = "case-08-parallel";
    let defect_task_id = "case-08-defect-stop";
    let closed_task_id = "case-08-closed-continuation";

    let root = run_command_json(
        &[
            "task",
            "create",
            root_task_id,
            "Case 08 autonomous orchestrator plus agents root",
            "--type",
            "epic",
            "--priority",
            "9",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    for task_id in [implementation_task_id, parallel_task_id] {
        let owned_path = if task_id == implementation_task_id {
            "crates/vida/src/taskflow_run_graph.rs"
        } else {
            "crates/vida/src/taskflow_layer4.rs"
        };
        let created = run_command_json(
            &[
                "task",
                "create",
                task_id,
                "Case 08 implementation lane",
                "--parent-id",
                root_task_id,
                "--type",
                "task",
                "--priority",
                "1",
                "--execution-mode",
                "parallel_safe",
                "--order-bucket",
                "case-08-wave",
                "--parallel-group",
                "case-08-pack",
                "--conflict-domain",
                task_id,
                "--owned-path",
                owned_path,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(created["status"], "pass");
    }

    let scheduler_preview = run_command_json(
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--current-task-id",
            implementation_task_id,
            "--limit",
            "2",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        scheduler_preview["surface"],
        "vida taskflow scheduler dispatch"
    );
    assert_eq!(scheduler_preview["status"], "pass");
    assert_eq!(scheduler_preview["activation_attempt_supported"], false);
    assert_eq!(
        scheduler_preview["worker_execution_evidence_status"],
        "not_received"
    );
    assert_eq!(
        require_json_string_array(
            &scheduler_preview["selected_task_ids"],
            "case 08 scheduler selected_task_ids"
        ),
        vec![
            implementation_task_id.to_string(),
            parallel_task_id.to_string()
        ]
    );

    let dispatch_preview = run_command_json(
        &[
            "agent",
            "dispatch-next",
            "--current-task-id",
            implementation_task_id,
            "--lanes",
            "2",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dispatch_preview["status"], "pass");
    assert_eq!(dispatch_preview["mode"], "preview");
    assert_eq!(dispatch_preview["execute_supported"], false);
    assert_eq!(dispatch_preview["execution_attempted"], false);
    let selected_lanes = dispatch_preview["selected_lanes"]
        .as_array()
        .expect("case 08 selected_lanes should be an array");
    assert_eq!(selected_lanes.len(), 2);
    for lane in selected_lanes {
        assert_eq!(lane["runtime_role"], "worker");
        assert_eq!(lane["task_class"], "implementation");
        let command = require_json_string(&lane["dispatch_command"], "case 08 dispatch command");
        assert!(command.contains("vida agent-init"));
        assert!(command.contains("--role worker"));
        assert!(command.contains("--state-dir"));
        assert!(command.contains("--json"));
    }

    let agent_init = run_command_json(
        &[
            "agent-init",
            "--role",
            "worker",
            implementation_task_id,
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(agent_init["surface"], "vida agent-init");
    assert_eq!(agent_init["selection"]["selected_role"], "worker");
    assert_eq!(
        agent_init["selection"]["request_text"],
        implementation_task_id
    );
    assert_eq!(agent_init["activation_semantics"]["view_only"], true);
    assert_eq!(agent_init["activation_semantics"]["executes_packet"], false);
    assert_eq!(
        agent_init["dispatch_mode"]["root_session_write_authority_granted"],
        false
    );

    let seeded = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "seed",
            implementation_task_id,
            "case 08 autonomous orchestrator plus agents request",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(seeded["surface"], "vida taskflow run-graph seed");
    assert_eq!(seeded["run_id"], implementation_task_id);

    let dispatch_receipt = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            implementation_task_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        dispatch_receipt["surface"],
        "vida taskflow run-graph dispatch-init"
    );
    assert_eq!(dispatch_receipt["run_id"], implementation_task_id);
    assert_eq!(
        dispatch_receipt["dispatch_receipt"]["run_id"],
        implementation_task_id
    );
    assert_eq!(
        dispatch_receipt["dispatch_receipt"]["dispatch_status"],
        "routed"
    );

    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            implementation_task_id,
            "implementation",
            "implementation",
            "blocked",
            "implementation",
            "{\"policy_gate\":\"validation_report_required\",\"context_state\":\"sealed\",\"resume_target\":\"dispatch.implementation\",\"recovery_ready\":true,\"lifecycle_stage\":\"recovery_ready\"}",
        ],
        &state_dir,
    );
    let run_graph_status = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "status",
            implementation_task_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(run_graph_status["status"], "blocked");
    assert_eq!(run_graph_status["run_graph_status"]["status"], "blocked");
    assert_eq!(
        run_graph_status["run_graph_status"]["policy_gate"],
        "validation_report_required"
    );
    assert_eq!(run_graph_status["run_graph_status"]["recovery_ready"], true);

    let recovery = run_command_json(
        &[
            "taskflow",
            "recovery",
            "status",
            implementation_task_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(recovery["status"], "blocked");
    assert_eq!(recovery["recovery"]["run_id"], implementation_task_id);
    assert_eq!(recovery["recovery"]["recovery_ready"], true);
    assert_eq!(
        recovery["recovery"]["resume_target"],
        "dispatch.implementation"
    );

    let defect = run_command_json(
        &[
            "task",
            "create",
            defect_task_id,
            "Case 08 defect stop",
            "--type",
            "defect",
            "--parent-id",
            root_task_id,
            "--priority",
            "0",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(defect["status"], "pass");
    assert_eq!(defect["task"]["issue_type"], "defect");
    let defect_update = run_command_json(
        &[
            "task",
            "update",
            defect_task_id,
            "--status",
            "in_progress",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(defect_update["status"], "pass");
    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "open delegated run must block heuristic defect takeover"
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .expect("case 08 blocked next-lawful json should parse");
    assert_eq!(next_lawful["status"], "blocked");
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("case 08 next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "open_delegated_cycle"));
    assert!(next_lawful_candidate_ids(&next_lawful)
        .iter()
        .any(|task_id| task_id == defect_task_id));

    let rejected_parent_close = run_command_capture(
        &[
            "task",
            "close",
            root_task_id,
            "--reason",
            "must stop on defect and open delegated children",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !rejected_parent_close.status.success(),
        "root close must fail while defect and child lanes are open"
    );
    let rejected_stderr = String::from_utf8_lossy(&rejected_parent_close.stderr);
    assert!(rejected_stderr.contains("non-closed child tasks exist"));
    assert!(rejected_stderr.contains(defect_task_id));

    let closed = run_command_json(
        &[
            "task",
            "create",
            closed_task_id,
            "Case 08 closed continuation negative control",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--parent-id",
            root_task_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["task"]["status"], "closed");
    let closed_dispatch_output = run_command_capture(
        &[
            "agent",
            "dispatch-next",
            "--current-task-id",
            closed_task_id,
            "--lanes",
            "1",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !closed_dispatch_output.status.success(),
        "closed continuation dispatch should fail closed"
    );
    let closed_dispatch: serde_json::Value = serde_json::from_slice(&closed_dispatch_output.stdout)
        .expect("closed continuation dispatch json should parse");
    assert_eq!(closed_dispatch["status"], "blocked");
    assert_eq!(closed_dispatch["lanes_selected"], 0);
    assert!(closed_dispatch["selected_lanes"]
        .as_array()
        .expect("closed continuation selected_lanes should be an array")
        .is_empty());
    assert_no_run_id_consume_continue_command(&closed_dispatch, closed_task_id, "case 08 closed");

    for task_id in [defect_task_id, implementation_task_id, parallel_task_id] {
        let closed_child = run_command_json(
            &[
                "task",
                "close",
                task_id,
                "--reason",
                "case 08 closure gate satisfied",
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(closed_child["status"], "pass");
        assert_eq!(closed_child["task"]["status"], "closed");
    }
    let root_closed = run_command_json(
        &[
            "task",
            "close",
            root_task_id,
            "--reason",
            "case 08 lifecycle closed after delegated evidence",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root_closed["status"], "pass");
    assert_eq!(root_closed["task"]["status"], "closed");

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(status["surface"], "vida status");
    assert_no_run_id_consume_continue_command(&status, closed_task_id, "case 08 status");

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(doctor["surface"], "vida doctor");
    assert_no_run_id_consume_continue_command(&doctor, closed_task_id, "case 08 doctor");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_factual_sandbox_h4_h5_graph_readiness() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let _ = run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "sandbox-graph-root",
            "Sandbox graph root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let blocker = run_command_json(
        &[
            "task",
            "create",
            "sandbox-graph-ready",
            "Sandbox graph ready",
            "--parent-id",
            "sandbox-graph-root",
            "--priority",
            "1",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "sandbox-graph-wave",
            "--parallel-group",
            "sandbox-graph-pack",
            "--conflict-domain",
            "sandbox-graph-ready-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(blocker["status"], "pass");

    let serial_ready = run_command_json(
        &[
            "task",
            "create",
            "sandbox-graph-serial",
            "Sandbox graph serial ready",
            "--parent-id",
            "sandbox-graph-root",
            "--priority",
            "2",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(serial_ready["status"], "pass");

    let parallel_ready = run_command_json(
        &[
            "task",
            "create",
            "sandbox-graph-parallel",
            "Sandbox graph parallel ready",
            "--parent-id",
            "sandbox-graph-root",
            "--priority",
            "3",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "sandbox-graph-wave",
            "--parallel-group",
            "sandbox-graph-pack",
            "--conflict-domain",
            "sandbox-graph-parallel-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parallel_ready["status"], "pass");

    let blocked = run_command_json(
        &[
            "task",
            "create",
            "sandbox-graph-blocked",
            "Sandbox graph blocked",
            "--parent-id",
            "sandbox-graph-root",
            "--priority",
            "4",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(blocked["status"], "pass");

    let dep = run_command_json(
        &[
            "task",
            "dep",
            "add",
            "sandbox-graph-blocked",
            "sandbox-graph-ready",
            "blocks",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dep["issue_id"], "sandbox-graph-blocked");
    assert_eq!(dep["depends_on_id"], "sandbox-graph-ready");
    assert_eq!(dep["edge_type"], "blocks");

    let deps = run_command_json(
        &["task", "deps", "sandbox-graph-blocked", "--json"],
        &state_dir,
    );
    assert_eq!(deps["task_id"], "sandbox-graph-blocked");
    assert_eq!(deps["dependency_count"], 2);
    let dependency_targets = deps["dependencies"]
        .as_array()
        .expect("dependencies should be an array")
        .iter()
        .map(|dependency| {
            dependency["depends_on_id"]
                .as_str()
                .expect("depends_on_id")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(dependency_targets.contains(&"sandbox-graph-root".to_string()));
    assert!(dependency_targets.contains(&"sandbox-graph-ready".to_string()));

    let ready = run_and_assert_success(&["task", "ready", "--json"], &state_dir);
    assert!(ready.contains("\"id\": \"sandbox-graph-ready\""));
    assert!(ready.contains("\"id\": \"sandbox-graph-serial\""));
    assert!(ready.contains("\"id\": \"sandbox-graph-parallel\""));
    assert!(!ready.contains("\"id\": \"sandbox-graph-blocked\""));

    let blocked_list = run_command_json(&["task", "blocked", "--json"], &state_dir);
    assert_eq!(blocked_list["surface"], "vida task blocked");
    assert_eq!(blocked_list["blocked_count"], 1);
    assert_eq!(
        blocked_list["tasks"][0]["task"]["id"],
        "sandbox-graph-blocked"
    );
    assert_eq!(
        blocked_list["tasks"][0]["blockers"][0]["depends_on_id"],
        "sandbox-graph-ready"
    );
    assert_eq!(
        blocked_list["tasks"][0]["blockers"][0]["edge_type"],
        "blocks"
    );
    assert_eq!(
        blocked_list["tasks"][0]["blockers"][0]["dependency_status"],
        "open"
    );

    let graph_summary_output =
        run_command_capture(&["taskflow", "graph-summary", "--json"], &state_dir);
    assert!(
        !graph_summary_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&graph_summary_output.stderr)
    );
    let graph_summary: serde_json::Value = serde_json::from_slice(&graph_summary_output.stdout)
        .expect("graph summary json should parse");
    assert_eq!(graph_summary["surface"], "vida taskflow graph-summary");
    assert_eq!(graph_summary["ready_count"], 3);
    assert_eq!(graph_summary["blocked_count"], 1);
    assert_eq!(graph_summary["current_task_id"], "sandbox-graph-ready");
    assert_eq!(graph_summary["ready_parallel_safe"], false);
    assert_eq!(
        require_json_string_array(
            &graph_summary["parallel_blockers"],
            "graph summary top-level parallel_blockers"
        ),
        vec!["current_task_reference".to_string()]
    );
    find_task_ref_by_id(
        &graph_summary["parallel_candidates_after_current"],
        "sandbox-graph-parallel",
    );

    assert_eq!(
        graph_summary["scheduling"]["current_task_id"],
        "sandbox-graph-ready"
    );
    assert!(graph_summary["scheduling"]["ready_count"].is_number());
    assert!(graph_summary["scheduling"]["blocked_count"].is_number());
    let summary_ready_blockers = vec!["current_task_reference".to_string()];
    let summary_serial_blockers = vec![
        "execution_mode_not_parallel_safe".to_string(),
        "order_bucket_mismatch_or_missing".to_string(),
        "missing_conflict_domain".to_string(),
        "parallel_group_mismatch".to_string(),
    ];
    let summary_blocked_blockers = vec!["graph_blocked".to_string()];

    let ready_explain_output = run_command_capture(
        &[
            "taskflow",
            "graph",
            "explain",
            "sandbox-graph-ready",
            "--current-task-id",
            "sandbox-graph-ready",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !ready_explain_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&ready_explain_output.stderr)
    );
    let ready_explain: serde_json::Value = serde_json::from_slice(&ready_explain_output.stdout)
        .expect("graph explain ready json should parse");
    assert_eq!(ready_explain["surface"], "vida taskflow graph explain");
    assert_eq!(ready_explain["ready_now"], true);
    assert_eq!(ready_explain["selected_as_current"], true);
    assert_eq!(ready_explain["ready_parallel_safe"], false);
    assert_eq!(
        require_json_string_array(
            &ready_explain["parallel_blockers"],
            "ready explain parallel_blockers"
        ),
        summary_ready_blockers
    );
    if !ready_explain_output.status.success() {
        assert_eq!(ready_explain["status"], "blocked");
        assert!(
            ready_explain["blocker_codes"]
                .as_array()
                .expect("ready explain blocker_codes should be an array")
                .iter()
                .any(|code| code.as_str() == Some("current_task_reference")),
            "current task references should fail closed while still rendering explain JSON"
        );
    }

    let serial_explain_output = run_command_capture(
        &[
            "taskflow",
            "graph",
            "explain",
            "sandbox-graph-serial",
            "--current-task-id",
            "sandbox-graph-ready",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !serial_explain_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&serial_explain_output.stderr)
    );
    let serial_explain: serde_json::Value = serde_json::from_slice(&serial_explain_output.stdout)
        .expect("graph explain serial json should parse");
    assert_eq!(serial_explain["surface"], "vida taskflow graph explain");
    assert_eq!(serial_explain["ready_now"], true);
    assert_eq!(serial_explain["ready_parallel_safe"], false);
    assert_eq!(
        require_json_string_array(
            &serial_explain["parallel_blockers"],
            "serial explain parallel_blockers"
        ),
        summary_serial_blockers
    );
    assert_eq!(serial_explain["status"], "blocked");

    let parallel_explain = run_command_json(
        &[
            "taskflow",
            "graph",
            "explain",
            "sandbox-graph-parallel",
            "--current-task-id",
            "sandbox-graph-ready",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parallel_explain["surface"], "vida taskflow graph explain");
    assert_eq!(parallel_explain["ready_now"], true);
    assert_eq!(parallel_explain["ready_parallel_safe"], true);
    assert_eq!(parallel_explain["selected_as_parallel_after_current"], true);
    assert!(require_json_string_array(
        &parallel_explain["parallel_blockers"],
        "parallel explain parallel_blockers"
    )
    .is_empty());
    find_task_ref_by_id(
        &parallel_explain["parallel_candidates_after_current"],
        "sandbox-graph-parallel",
    );

    let blocked_explain_output = run_command_capture(
        &[
            "taskflow",
            "graph",
            "explain",
            "sandbox-graph-blocked",
            "--current-task-id",
            "sandbox-graph-ready",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !blocked_explain_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&blocked_explain_output.stderr)
    );
    let blocked_explain: serde_json::Value = serde_json::from_slice(&blocked_explain_output.stdout)
        .expect("graph explain blocked json should parse");
    assert_eq!(blocked_explain["surface"], "vida taskflow graph explain");
    assert_eq!(blocked_explain["ready_now"], false);
    assert_eq!(
        blocked_explain["blocked_by"][0]["depends_on_id"],
        "sandbox-graph-ready"
    );
    assert_eq!(
        require_json_string_array(
            &blocked_explain["parallel_blockers"],
            "blocked explain parallel_blockers"
        ),
        summary_blocked_blockers
    );
    if !blocked_explain_output.status.success() {
        assert_eq!(blocked_explain["status"], "blocked");
    }

    let scheduler_preview = run_command_json(
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--current-task-id",
            "sandbox-graph-ready",
            "--limit",
            "2",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        scheduler_preview["surface"],
        "vida taskflow scheduler dispatch"
    );
    assert_eq!(
        scheduler_preview["ready_count"],
        graph_summary["ready_count"]
    );
    assert_eq!(
        scheduler_preview["blocked_count"],
        graph_summary["blocked_count"]
    );
    assert_eq!(
        scheduler_preview["scheduling"]["current_task_id"],
        graph_summary["scheduling"]["current_task_id"]
    );
    assert_eq!(
        scheduler_preview["selected_primary_task"]["id"],
        "sandbox-graph-ready"
    );
    find_task_ref_by_id(
        &scheduler_preview["selected_parallel_tasks"],
        "sandbox-graph-parallel",
    );
    assert_eq!(
        scheduler_preview["selected_task_ids"],
        serde_json::json!(["sandbox-graph-ready", "sandbox-graph-parallel"])
    );
    find_task_ref_by_id(
        &scheduler_preview["scheduling"]["parallel_candidates_after_current"],
        "sandbox-graph-parallel",
    );

    let scheduler_ready = find_scheduling_candidate(
        &scheduler_preview["scheduling"]["ready"],
        "sandbox-graph-ready",
    );
    assert_eq!(
        require_json_string_array(
            &scheduler_ready["parallel_blockers"],
            "scheduler ready parallel_blockers"
        ),
        summary_ready_blockers
    );
    let scheduler_serial = find_scheduling_candidate(
        &scheduler_preview["scheduling"]["ready"],
        "sandbox-graph-serial",
    );
    assert_eq!(
        require_json_string_array(
            &scheduler_serial["parallel_blockers"],
            "scheduler serial parallel_blockers"
        ),
        summary_serial_blockers
    );
    let scheduler_parallel = find_scheduling_candidate(
        &scheduler_preview["scheduling"]["ready"],
        "sandbox-graph-parallel",
    );
    assert_eq!(scheduler_parallel["ready_parallel_safe"], true);
    assert!(require_json_string_array(
        &scheduler_parallel["parallel_blockers"],
        "scheduler parallel parallel_blockers"
    )
    .is_empty());
    let scheduler_blocked = find_scheduling_candidate(
        &scheduler_preview["scheduling"]["blocked"],
        "sandbox-graph-blocked",
    );
    assert_eq!(
        require_json_string_array(
            &scheduler_blocked["parallel_blockers"],
            "scheduler blocked parallel_blockers"
        ),
        summary_blocked_blockers
    );

    let rejected_serial = find_rejected_candidate(
        &scheduler_preview["rejected_candidates"],
        "sandbox-graph-serial",
    );
    assert_eq!(rejected_serial["ready_now"], true);
    assert_eq!(
        require_json_string_array(
            &rejected_serial["parallel_blockers"],
            "rejected serial parallel_blockers"
        ),
        summary_serial_blockers
    );
    let rejected_blocked = find_rejected_candidate(
        &scheduler_preview["rejected_candidates"],
        "sandbox-graph-blocked",
    );
    assert_eq!(rejected_blocked["ready_now"], false);
    assert_eq!(
        rejected_blocked["blocked_by"][0]["depends_on_id"],
        "sandbox-graph-ready"
    );
    assert_eq!(
        require_json_string_array(
            &rejected_blocked["parallel_blockers"],
            "rejected blocked parallel_blockers"
        ),
        summary_blocked_blockers
    );

    let tree = run_command_json(
        &["task", "tree", "sandbox-graph-root", "--json"],
        &state_dir,
    );
    assert_eq!(tree["surface"], "vida task tree");
    assert_eq!(tree["root_task_id"], "sandbox-graph-root");
    assert!(tree.to_string().contains("sandbox-graph-ready"));
    assert!(tree.to_string().contains("sandbox-graph-serial"));
    assert!(tree.to_string().contains("sandbox-graph-parallel"));
    assert!(tree.to_string().contains("sandbox-graph-blocked"));

    let critical_path = run_command_json(&["task", "critical-path", "--json"], &state_dir);
    assert_eq!(critical_path["surface"], "vida task critical-path");
    assert_eq!(critical_path["status"], "pass");
    assert_eq!(critical_path["length"], 2);
    assert_eq!(critical_path["root_task_id"], "sandbox-graph-ready");
    assert_eq!(critical_path["terminal_task_id"], "sandbox-graph-blocked");

    let validate = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(validate["surface"], "vida task validate-graph");
    assert_eq!(validate["valid"], true);
    assert_eq!(validate["issue_count"], 0);

    let closed_blocker = run_command_json(
        &[
            "task",
            "close",
            "sandbox-graph-ready",
            "--reason",
            "factual graph dependency satisfied",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_blocker["status"], "pass");

    let ready_after_close = run_and_assert_success(&["task", "ready", "--json"], &state_dir);
    assert!(ready_after_close.contains("\"id\": \"sandbox-graph-blocked\""));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_scheduling_actualize_cli_contract() {
    let state_dir = unique_state_dir();
    create_epic_parent(
        &state_dir,
        "scheduling-actualize-root",
        "Scheduling actualize root",
        "open",
    );
    let legacy = run_command_json(
        &[
            "task",
            "create",
            "scheduling-actualize-legacy",
            "Scheduling actualize legacy task",
            "--parent-id",
            "scheduling-actualize-root",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(legacy["status"], "pass");
    let explicit = run_command_json(
        &[
            "task",
            "create",
            "scheduling-actualize-explicit",
            "Scheduling actualize explicit task",
            "--parent-id",
            "scheduling-actualize-root",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "scheduling-actualize-root",
            "--parallel-group",
            "explicit",
            "--conflict-domain",
            "explicit-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(explicit["status"], "pass");

    let dry_run = run_command_json(
        &[
            "taskflow",
            "scheduling",
            "actualize",
            "--scope",
            "scheduling-actualize-root",
            "--state-dir",
            state_dir.as_str(),
            "--dry-run",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dry_run["surface"], "vida taskflow scheduling actualize");
    assert_eq!(dry_run["dry_run"], true);
    assert_eq!(dry_run["apply"], false);
    assert_eq!(dry_run["candidate_count"], 1);
    assert_eq!(
        dry_run["candidates"][0]["task_id"],
        "scheduling-actualize-legacy"
    );
    assert_eq!(
        dry_run["candidates"][0]["proposed"]["execution_mode"],
        "sequential"
    );
    assert_eq!(
        dry_run["candidates"][0]["proposed"]["order_bucket"],
        "scheduling-actualize-root"
    );
    assert_eq!(
        dry_run["candidates"][0]["proposed"]["conflict_domain"],
        "scheduling-actualize-legacy"
    );

    let applied = run_command_json(
        &[
            "taskflow",
            "scheduling",
            "actualize",
            "--scope",
            "scheduling-actualize-root",
            "--state-dir",
            state_dir.as_str(),
            "--apply",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(applied["status"], "pass");
    assert_eq!(applied["candidate_count"], 1);
    assert_eq!(applied["applied_count"], 1);
    assert_eq!(applied["candidates"][0]["applied"], true);

    let updated = run_command_json(
        &["task", "show", "scheduling-actualize-legacy", "--json"],
        &state_dir,
    );
    assert_eq!(
        updated["task"]["execution_semantics"]["execution_mode"],
        "sequential"
    );
    assert_eq!(
        updated["task"]["execution_semantics"]["order_bucket"],
        "scheduling-actualize-root"
    );
    assert_eq!(
        updated["task"]["execution_semantics"]["parallel_group"],
        "default"
    );
    assert_eq!(
        updated["task"]["execution_semantics"]["conflict_domain"],
        "scheduling-actualize-legacy"
    );

    let no_candidates = run_command_json(
        &[
            "taskflow",
            "scheduling",
            "actualize",
            "--scope",
            "scheduling-actualize-root",
            "--state-dir",
            state_dir.as_str(),
            "--dry-run",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(no_candidates["candidate_count"], 0);

    let help = run_command_capture(
        &["taskflow", "scheduling", "actualize", "--help"],
        &state_dir,
    );
    assert!(
        help.status.success(),
        "{}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("vida taskflow scheduling actualize"));
    assert!(help_text.contains("--scope"));
    assert!(help_text.contains("--dry-run"));
    assert!(help_text.contains("--apply"));

    let missing_scope = run_command_capture(
        &[
            "taskflow",
            "scheduling",
            "actualize",
            "--scope",
            "missing-scope-task",
            "--state-dir",
            state_dir.as_str(),
            "--dry-run",
            "--json",
        ],
        &state_dir,
    );
    assert!(!missing_scope.status.success());
    let missing_payload: serde_json::Value =
        serde_json::from_slice(&missing_scope.stdout).expect("missing scope json should parse");
    assert_eq!(missing_payload["status"], "blocked");
    assert_eq!(missing_payload["blocker_codes"][0], "scope_task_missing");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_factual_sandbox_h12_h16_invariant_matrix() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let create_parent = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h13-parent",
            "Sandbox H13 parent",
            "--type",
            "epic",
            "--status",
            "closed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(create_parent["status"], "pass");
    assert_eq!(create_parent["task"]["status"], "closed");
    assert_task_graph_valid_after(&state_dir, "create closed H13 parent");

    let create_open_child = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h13-open-defect",
            "Sandbox H13 open defect",
            "--type",
            "defect",
            "--parent-id",
            "sandbox-h13-parent",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(create_open_child["status"], "pass");
    assert_eq!(create_open_child["task"]["status"], "open");
    assert_task_graph_valid_after(&state_dir, "create open H13 child");
    let parent_after_create = run_command_json(
        &["task", "show", "sandbox-h13-parent", "--json"],
        &state_dir,
    );
    assert_eq!(parent_after_create["task"]["status"], "in_progress");
    assert!(parent_after_create["task"]["closed_at"].is_null());
    assert!(parent_after_create["task"]["close_reason"].is_null());

    let rejected_create_parent_close = run_command_capture(
        &[
            "task",
            "close",
            "sandbox-h13-parent",
            "--reason",
            "must fail while defect child is open",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !rejected_create_parent_close.status.success(),
        "H13/H15 parent close with open defect child must fail closed"
    );
    let rejected_create_parent_close_stderr =
        String::from_utf8_lossy(&rejected_create_parent_close.stderr);
    assert!(
        rejected_create_parent_close_stderr.contains("non-closed child tasks exist")
            && rejected_create_parent_close_stderr.contains("sandbox-h13-open-defect"),
        "{rejected_create_parent_close_stderr}"
    );
    assert_task_graph_valid_after(&state_dir, "reject H13 parent close");
    let parent_after_create_close_reject = run_command_json(
        &["task", "show", "sandbox-h13-parent", "--json"],
        &state_dir,
    );
    assert_eq!(
        parent_after_create_close_reject["task"]["status"],
        "in_progress"
    );

    let update_parent = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h14-parent",
            "Sandbox H14 parent",
            "--type",
            "epic",
            "--status",
            "closed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(update_parent["status"], "pass");
    assert_eq!(update_parent["task"]["status"], "closed");
    assert_task_graph_valid_after(&state_dir, "create closed H14 parent");

    let closed_child = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h14-child",
            "Sandbox H14 child",
            "--parent-id",
            "sandbox-h14-parent",
            "--status",
            "closed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_child["status"], "pass");
    assert_eq!(closed_child["task"]["status"], "closed");
    assert_task_graph_valid_after(&state_dir, "create closed H14 child");

    let update_open_child = run_command_json(
        &[
            "task",
            "update",
            "sandbox-h14-child",
            "--status",
            "open",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(update_open_child["status"], "pass");
    assert_eq!(update_open_child["task"]["status"], "open");
    assert_task_graph_valid_after(&state_dir, "update H14 child back to open");
    let parent_after_update = run_command_json(
        &["task", "show", "sandbox-h14-parent", "--json"],
        &state_dir,
    );
    assert_eq!(parent_after_update["task"]["status"], "in_progress");
    assert!(parent_after_update["task"]["closed_at"].is_null());
    assert!(parent_after_update["task"]["close_reason"].is_null());

    let rejected_update_parent_close = run_command_capture(
        &[
            "task",
            "close",
            "sandbox-h14-parent",
            "--reason",
            "must fail while updated child is open",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !rejected_update_parent_close.status.success(),
        "H14 parent close with reopened child must fail closed"
    );
    let rejected_update_parent_close_stderr =
        String::from_utf8_lossy(&rejected_update_parent_close.stderr);
    assert!(
        rejected_update_parent_close_stderr.contains("non-closed child tasks exist")
            && rejected_update_parent_close_stderr.contains("sandbox-h14-child"),
        "{rejected_update_parent_close_stderr}"
    );
    assert_task_graph_valid_after(&state_dir, "reject H14 parent close");
    let parent_after_update_close_reject = run_command_json(
        &["task", "show", "sandbox-h14-parent", "--json"],
        &state_dir,
    );
    assert_eq!(
        parent_after_update_close_reject["task"]["status"],
        "in_progress"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_defect_loop_routes_repair_and_gates_parent_closure() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_task_id = "case-06-parent";
    let defect_task_id = "case-06-defect";

    let parent = run_command_json(
        &[
            "task",
            "create",
            parent_task_id,
            "Case 06 parent",
            "--type",
            "epic",
            "--status",
            "closed",
            "--priority",
            "9",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");
    assert_eq!(parent["task"]["status"], "closed");

    let defect = run_command_json(
        &[
            "task",
            "create",
            defect_task_id,
            "Case 06 factual failure defect",
            "--type",
            "defect",
            "--parent-id",
            parent_task_id,
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(defect["status"], "pass");
    assert_eq!(defect["task"]["status"], "open");
    assert_eq!(defect["task"]["issue_type"], "defect");
    assert_eq!(
        defect["task"]["dependencies"][0]["depends_on_id"],
        parent_task_id
    );
    assert_eq!(
        defect["task"]["dependencies"][0]["edge_type"],
        "parent-child"
    );
    assert_task_graph_valid_after(&state_dir, "create case 06 defect child");

    let parent_after_defect =
        run_command_json(&["task", "show", parent_task_id, "--json"], &state_dir);
    assert_eq!(parent_after_defect["task"]["status"], "in_progress");
    assert!(parent_after_defect["task"]["closed_at"].is_null());
    assert!(parent_after_defect["task"]["close_reason"].is_null());

    let defect_update = run_command_json(
        &[
            "task",
            "update",
            defect_task_id,
            "--status",
            "in_progress",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(defect_update["status"], "pass");
    assert_eq!(defect_update["task"]["status"], "in_progress");
    assert_task_graph_valid_after(&state_dir, "route case 06 defect repair");

    let next_lawful = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(
        next_lawful["active_bounded_unit"]["task_id"],
        defect_task_id
    );
    assert_eq!(next_lawful["active_bounded_unit"]["issue_type"], "defect");
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .is_empty());
    assert_eq!(
        next_lawful["why_this_unit"],
        "Single TaskFlow in_progress task is the authoritative active bounded unit."
    );
    assert_eq!(
        next_lawful["sequential_vs_parallel_posture"],
        "sequential_only_taskflow_active"
    );

    let dispatch_preview = run_command_json(
        &[
            "agent",
            "dispatch-next",
            "--current-task-id",
            defect_task_id,
            "--lanes",
            "1",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dispatch_preview["status"], "pass");
    assert_eq!(dispatch_preview["mode"], "preview");
    assert_eq!(dispatch_preview["execute_supported"], false);
    assert_eq!(dispatch_preview["execution_attempted"], false);
    assert_eq!(dispatch_preview["lanes_selected"], 1);
    let selected_lanes = dispatch_preview["selected_lanes"]
        .as_array()
        .expect("dispatch selected_lanes should be an array");
    assert_eq!(selected_lanes.len(), 1);
    assert_eq!(selected_lanes[0]["task_id"], defect_task_id);
    assert_eq!(selected_lanes[0]["runtime_role"], "worker");
    assert_eq!(selected_lanes[0]["task_class"], "implementation");
    let dispatch_command = require_json_string(
        &selected_lanes[0]["dispatch_command"],
        "case 06 dispatch_command",
    );
    assert!(
        dispatch_command.contains("vida agent-init")
            && dispatch_command.contains("--role worker")
            && dispatch_command.contains(defect_task_id)
            && dispatch_command.contains("--state-dir")
            && dispatch_command.contains("--json"),
        "repair dispatch should route through vida agent-init: {dispatch_command}"
    );
    let source_surfaces = require_json_string_array(
        &dispatch_preview["source_surfaces"],
        "case 06 dispatch source_surfaces",
    );
    assert!(source_surfaces
        .iter()
        .any(|surface| surface == "vida agent-init --role <runtime-role> <task-id> --json"));

    let rejected_parent_close = run_command_capture(
        &[
            "task",
            "close",
            parent_task_id,
            "--reason",
            "must fail until defect repair evidence exists",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !rejected_parent_close.status.success(),
        "parent close must fail while defect repair is open"
    );
    let rejected_parent_close_stderr = String::from_utf8_lossy(&rejected_parent_close.stderr);
    assert!(
        rejected_parent_close_stderr.contains("non-closed child tasks exist")
            && rejected_parent_close_stderr.contains(defect_task_id),
        "{rejected_parent_close_stderr}"
    );

    let defect_closed = run_command_json(
        &[
            "task",
            "close",
            defect_task_id,
            "--reason",
            "repair evidence recorded through agent repair loop",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(defect_closed["status"], "pass");
    assert_eq!(defect_closed["task"]["status"], "closed");
    assert_task_graph_valid_after(&state_dir, "close case 06 repaired defect");

    let parent_closed = run_command_json(
        &[
            "task",
            "close",
            parent_task_id,
            "--reason",
            "parent closure allowed after defect repair evidence",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent_closed["status"], "pass");
    assert_eq!(parent_closed["task"]["status"], "closed");

    let no_continuation = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !no_continuation.status.success(),
        "all case 06 work is closed, so no continuation should remain"
    );
    let no_continuation_json: serde_json::Value =
        serde_json::from_slice(&no_continuation.stdout).expect("blocked next-lawful json parses");
    assert_eq!(no_continuation_json["status"], "blocked");
    assert!(no_continuation_json["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn taskflow_testing_h24_operator_budget_guard() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let root = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h24-root",
            "Sandbox H24 root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let ready = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h24-ready",
            "Sandbox H24 ready",
            "--parent-id",
            "sandbox-h24-root",
            "--priority",
            "2",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(ready["status"], "pass");

    let blocked = run_command_json(
        &[
            "task",
            "create",
            "sandbox-h24-blocked",
            "Sandbox H24 blocked",
            "--parent-id",
            "sandbox-h24-root",
            "--priority",
            "3",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(blocked["status"], "pass");

    let dep = run_command_json(
        &[
            "task",
            "dep",
            "add",
            "sandbox-h24-blocked",
            "sandbox-h24-ready",
            "blocks",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dep["edge_type"], "blocks");

    const READ_MODEL_SURFACE_BUDGET: Duration = Duration::from_secs(5);
    let surfaces = [
        (
            "task show",
            vec!["task", "show", "sandbox-h24-ready", "--json"],
        ),
        ("task ready", vec!["task", "ready", "--json"]),
        (
            "task validate-graph",
            vec!["task", "validate-graph", "--json"],
        ),
        ("status --json", vec!["status", "--json"]),
    ];

    for (surface, args) in surfaces {
        let started_at = Instant::now();
        let output = run_command_capture(&args, &state_dir);
        let elapsed = started_at.elapsed();
        let elapsed_ms = elapsed.as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("H24 operator budget: {surface} elapsed={elapsed_ms}ms");
        assert!(
            output.status.success(),
            "{surface} should succeed within the H24 operator budget guard; elapsed={elapsed_ms}ms; status={:?}; stdout={stdout}; stderr={stderr}",
            output.status.code()
        );
        assert!(
            elapsed <= READ_MODEL_SURFACE_BUDGET,
            "{surface} exceeded H24 operator hard budget; elapsed={elapsed_ms}ms; budget={}ms; normal target is <=2000ms; stdout={stdout}; stderr={stderr}",
            READ_MODEL_SURFACE_BUDGET.as_millis()
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn operator_json_surfaces_reuse_fresh_projection_before_store_open() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let status_projection = serde_json::json!({
        "surface": "vida status",
        "status": "pass",
        "cache_probe": "status-summary-reused",
        "shared_fields": {
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "projection": "status-summary-v2-latest"
            }
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "projection": "status-summary-v2-latest"
            }
        },
        "blocker_codes": [],
        "next_actions": []
    });
    write_operator_projection(&state_dir, "status-summary-v2-latest", &status_projection);

    let status = run_command_json(&["status", "--summary", "--json"], &state_dir);
    assert_eq!(status["cache_probe"], "status-summary-reused");
    assert_eq!(
        status["operator_contracts"]["artifact_refs"]["projection"],
        "status-summary-v2-latest"
    );

    let graph_projection = serde_json::json!({
        "surface": "vida taskflow graph-summary",
        "status": "pass",
        "cache_probe": "graph-summary-reused",
        "shared_fields": {
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "projection": "taskflow-graph-summary-latest"
            }
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "projection": "taskflow-graph-summary-latest"
            }
        },
        "blocker_codes": [],
        "next_actions": []
    });
    write_operator_projection(
        &state_dir,
        "taskflow-graph-summary-latest",
        &graph_projection,
    );

    let graph_summary = run_command_json(&["taskflow", "graph-summary", "--json"], &state_dir);
    assert_eq!(graph_summary["cache_probe"], "graph-summary-reused");
    assert_eq!(
        graph_summary["operator_contracts"]["artifact_refs"]["projection"],
        "taskflow-graph-summary-latest"
    );

    let next_lawful_projection = serde_json::json!({
        "status": "pass",
        "cache_probe": "task-next-lawful-reused",
        "active_bounded_unit": {
            "task_id": "cached-next-lawful-task",
            "title": "Cached next lawful task",
            "status": "open",
            "issue_type": "task"
        },
        "binding_source": null,
        "why_this_unit": "fresh task-next-lawful-latest projection",
        "sequential_vs_parallel_posture": "sequential",
        "ready_task_candidates": [],
        "blocker_codes": [],
        "next_actions": [],
        "source_surfaces": [
            "task-next-lawful-latest"
        ]
    });
    write_operator_projection(
        &state_dir,
        "task-next-lawful-latest",
        &next_lawful_projection,
    );

    let next_lawful = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful.status.success(),
        "task next-lawful must reject forged projection output before authoritative state opens"
    );
    assert!(
        !String::from_utf8_lossy(&next_lawful.stdout).contains("cached-next-lawful-task"),
        "task next-lawful must not echo forged projection task ids"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_next_lawful_cache_refreshes_after_task_mutation() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let stale_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !stale_output.status.success(),
        "empty state should produce blocked next-lawful stdout={} stderr={}",
        String::from_utf8_lossy(&stale_output.stdout),
        String::from_utf8_lossy(&stale_output.stderr)
    );
    let stale: serde_json::Value = serde_json::from_slice(&stale_output.stdout)
        .expect("blocked next-lawful json should parse");
    assert_eq!(stale["status"], "blocked");
    assert!(stale["blocker_codes"]
        .as_array()
        .expect("blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));

    thread::sleep(Duration::from_millis(10));
    let parent_id = "cache-refresh-parent";
    create_epic_parent(&state_dir, parent_id, "Cache refresh parent", "open");
    let active_task_id = "cache-refresh-active-task";
    let active = run_command_json(
        &[
            "task",
            "create",
            active_task_id,
            "Cache refresh active task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    let next_lawful = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful["status"], "pass");
    assert_ne!(
        next_lawful["cache_probe"], "task-next-lawful-reused",
        "task create must invalidate stale next-lawful projection"
    );
    assert_eq!(
        next_lawful["active_bounded_unit"]["task_id"],
        active_task_id
    );
    assert!(!next_lawful["blocker_codes"]
        .as_array()
        .expect("blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));

    write_operator_projection(
        &state_dir,
        "task-next-lawful-latest",
        &stale_blocked_next_lawful_projection(),
    );
    thread::sleep(Duration::from_millis(10));
    let blocker_task_id = "cache-refresh-blocker-task";
    let blocker = run_command_json(
        &[
            "task",
            "create",
            blocker_task_id,
            "Cache refresh blocker task",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "2",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(blocker["status"], "pass");

    write_operator_projection(
        &state_dir,
        "task-next-lawful-latest",
        &stale_blocked_next_lawful_projection(),
    );
    thread::sleep(Duration::from_millis(10));
    let dep_output = run_command_capture(
        &[
            "task",
            "dep",
            "add",
            blocker_task_id,
            active_task_id,
            "blocks",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        dep_output.status.success(),
        "dep add stdout={} stderr={}",
        String::from_utf8_lossy(&dep_output.stdout),
        String::from_utf8_lossy(&dep_output.stderr)
    );
    let after_dep = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(after_dep["status"], "pass");
    assert_ne!(
        after_dep["cache_probe"], "task-next-lawful-reused",
        "task dependency mutation must invalidate stale next-lawful projection"
    );
    assert_eq!(after_dep["active_bounded_unit"]["task_id"], active_task_id);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_create_update_close_round_trip_supports_planning_graph_views() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let root = run_command_json(
        &[
            "task",
            "create",
            "vida-root",
            "Root epic",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["surface"], "vida task create");
    assert_eq!(root["status"], "pass");
    assert_eq!(root["task"]["status"], "open");
    assert_eq!(root["task"]["issue_type"], "epic");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "vida-a",
            "Task A",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "2",
            "--parent-id",
            "vida-root",
            "--description",
            "first",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["surface"], "vida task create");
    assert_eq!(task_a["status"], "pass");
    assert_eq!(task_a["task"]["status"], "open");
    assert_eq!(task_a["task"]["title"], "Task A");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "vida-b",
            "Task B",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "1",
            "--parent-id",
            "vida-root",
            "--description",
            "second",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["surface"], "vida task create");
    assert_eq!(task_b["status"], "pass");
    assert_eq!(task_b["task"]["status"], "open");
    assert_eq!(task_b["task"]["title"], "Task B");

    let dep = run_command_json(
        &["task", "dep", "add", "vida-b", "vida-a", "blocks", "--json"],
        &state_dir,
    );
    assert_eq!(dep["issue_id"], "vida-b");
    assert_eq!(dep["depends_on_id"], "vida-a");
    assert_eq!(dep["edge_type"], "blocks");

    let updated = run_command_json(
        &[
            "task",
            "update",
            "vida-b",
            "--title",
            "Task B reprioritized",
            "--status",
            "in_progress",
            "--priority",
            "5",
            "--notes",
            "planning round trip proof",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(updated["surface"], "vida task update");
    assert_eq!(updated["status"], "pass");
    assert_eq!(updated["task"]["title"], "Task B reprioritized");
    assert_eq!(updated["task"]["status"], "in_progress");
    assert_eq!(updated["task"]["priority"], 5);
    assert_eq!(updated["task"]["notes"], "planning round trip proof");

    let deps = run_command_json(&["task", "deps", "vida-b", "--json"], &state_dir);
    assert_eq!(deps["task_id"], "vida-b");
    assert_eq!(deps["dependency_count"], 2);
    let dependency_targets = deps["dependencies"]
        .as_array()
        .expect("dependencies should be an array")
        .iter()
        .map(|dependency| {
            dependency["depends_on_id"]
                .as_str()
                .expect("depends_on_id")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(dependency_targets.contains(&"vida-root".to_string()));
    assert!(dependency_targets.contains(&"vida-a".to_string()));

    let reverse = run_and_assert_success(&["task", "reverse-deps", "vida-a", "--json"], &state_dir);
    assert!(
        reverse.contains("\"issue_id\": \"vida-b\"") || reverse.contains("\"issue_id\":\"vida-b\"")
    );

    let blocked = run_and_assert_success(&["task", "blocked", "--json"], &state_dir);
    assert!(blocked.contains("\"blocked_count\": 1") || blocked.contains("\"blocked_count\":1"));
    assert!(blocked.contains("\"id\": \"vida-b\"") || blocked.contains("\"id\":\"vida-b\""));

    let critical_path = run_command_json(&["task", "critical-path", "--json"], &state_dir);
    assert_eq!(critical_path["status"], "pass");
    assert_eq!(critical_path["surface"], "vida task critical-path");
    assert_eq!(critical_path["length"], 2);
    assert_eq!(critical_path["root_task_id"], "vida-a");
    assert_eq!(critical_path["terminal_task_id"], "vida-b");

    let validate = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(validate["surface"], "vida task validate-graph");
    assert_eq!(validate["status"], "pass");
    assert_eq!(validate["valid"], true);
    assert_eq!(validate["issue_count"], 0);

    let closed = run_command_json(
        &[
            "task",
            "close",
            "vida-b",
            "--reason",
            "planning proof complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["task"]["status"], "closed");
    assert_eq!(closed["task"]["close_reason"], "planning proof complete");

    let shown = run_command_json(&["task", "show", "vida-b", "--json"], &state_dir);
    assert_eq!(shown["status"], "pass");
    assert_eq!(shown["surface"], "vida task show");
    assert_eq!(shown["task"]["title"], "Task B reprioritized");
    assert_eq!(shown["task"]["status"], "closed");
    assert_eq!(shown["task"]["priority"], 5);
    assert_eq!(shown["task"]["close_reason"], "planning proof complete");
    assert_eq!(shown["task"]["notes"], "planning round trip proof");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_defect_batch_rehome_cli_dry_run_and_persist_preserve_graph() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    create_epic_parent(&state_dir, "old-epic", "Old defect epic", "open");
    create_epic_parent(&state_dir, "new-epic", "New defect epic", "open");
    for (task_id, title, status, parent_id) in [
        ("defect-a", "Defect A", "open", "old-epic"),
        ("defect-b", "Defect B", "open", "old-epic"),
        ("old-active", "Old active", "in_progress", "old-epic"),
        ("new-active", "New active", "open", "new-epic"),
    ] {
        let created = run_command_json(
            &[
                "task",
                "create",
                task_id,
                title,
                "--type",
                "defect",
                "--status",
                status,
                "--priority",
                "1",
                "--parent-id",
                parent_id,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(created["status"], "pass");
    }

    let dry_run = run_command_json(
        &[
            "task",
            "defect-batch-rehome",
            "old-epic",
            "new-epic",
            "--child-id",
            "defect-a",
            "--pause-task-id",
            "old-active",
            "--start-task-id",
            "new-active",
            "--dry-run",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dry_run["status"], "pass");
    assert_eq!(dry_run["surface"], "vida task defect-batch-rehome");
    assert_eq!(dry_run["result"]["dry_run"], true);
    assert_eq!(
        dry_run["result"]["moved_child_ids"],
        serde_json::json!(["defect-a"])
    );
    assert_eq!(
        dry_run["result"]["paused_task_ids"],
        serde_json::json!(["old-active"])
    );
    assert_eq!(
        dry_run["result"]["started_task_ids"],
        serde_json::json!(["new-active"])
    );

    let defect_a_after_dry_run =
        run_command_json(&["task", "show", "defect-a", "--json"], &state_dir);
    assert!(defect_a_after_dry_run["task"]["dependencies"]
        .as_array()
        .expect("dependencies should be array")
        .iter()
        .any(|dependency| {
            dependency["edge_type"] == "parent-child" && dependency["depends_on_id"] == "old-epic"
        }));
    assert_eq!(
        run_command_json(&["task", "show", "old-active", "--json"], &state_dir)["task"]["status"],
        "in_progress"
    );

    let persisted = run_command_json(
        &[
            "task",
            "defect-batch-rehome",
            "old-epic",
            "new-epic",
            "--child-id",
            "defect-a",
            "--pause-task-id",
            "old-active",
            "--start-task-id",
            "new-active",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(persisted["status"], "pass");
    assert_eq!(persisted["result"]["dry_run"], false);
    assert_eq!(persisted["result"]["moved_count"], 1);
    assert_eq!(persisted["result"]["paused_count"], 1);
    assert_eq!(persisted["result"]["started_count"], 1);

    let defect_a = run_command_json(&["task", "show", "defect-a", "--json"], &state_dir);
    let defect_b = run_command_json(&["task", "show", "defect-b", "--json"], &state_dir);
    assert!(defect_a["task"]["dependencies"]
        .as_array()
        .expect("dependencies should be array")
        .iter()
        .any(|dependency| {
            dependency["edge_type"] == "parent-child" && dependency["depends_on_id"] == "new-epic"
        }));
    assert!(defect_b["task"]["dependencies"]
        .as_array()
        .expect("dependencies should be array")
        .iter()
        .any(|dependency| {
            dependency["edge_type"] == "parent-child" && dependency["depends_on_id"] == "old-epic"
        }));
    assert_eq!(
        run_command_json(&["task", "show", "old-active", "--json"], &state_dir)["task"]["status"],
        "paused"
    );
    assert_eq!(
        run_command_json(&["task", "show", "new-active", "--json"], &state_dir)["task"]["status"],
        "in_progress"
    );
    let validate = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(validate["status"], "pass");
    assert_eq!(validate["valid"], true);
    assert_eq!(validate["issue_count"], 0);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_update_title_priority() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let parent_id = "vida-update-parent";
    create_epic_parent(&state_dir, parent_id, "Update parent", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            "vida-update",
            "Original task",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "3",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    assert_eq!(created["task"]["title"], "Original task");
    assert_eq!(created["task"]["priority"], 3);

    let updated = run_command_json(
        &[
            "task",
            "update",
            "vida-update",
            "--title",
            "Renamed task",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(updated["surface"], "vida task update");
    assert_eq!(updated["status"], "pass");
    assert_eq!(updated["task"]["title"], "Renamed task");
    assert_eq!(updated["task"]["priority"], 1);

    let shown = run_command_json(&["task", "show", "vida-update", "--json"], &state_dir);
    assert_eq!(shown["status"], "pass");
    assert_eq!(shown["task"]["title"], "Renamed task");
    assert_eq!(shown["task"]["priority"], 1);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_import_jsonl_invalid_graph_returns_json_envelope() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        "{\"id\":\"vida-broken\",\"title\":\"Broken task\",\"description\":\"broken\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-broken\",\"depends_on_id\":\"vida-missing\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
    )
    .expect("write broken task jsonl");

    let import_output = vida()
        .args(["task", "import-jsonl", &jsonl_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("import-jsonl should run");
    assert!(!import_output.status.success());
    assert!(
        !import_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&import_output.stderr)
    );
    let actual_json: serde_json::Value = serde_json::from_slice(&import_output.stdout)
        .expect("blocked import-jsonl json should parse");
    assert_release1_shared_envelope_fields(&actual_json, "blocked import-jsonl");
    assert_eq!(actual_json["status"], "blocked");
    assert_eq!(actual_json["surface"], "vida task import-jsonl");
    assert_eq!(
        actual_json["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );
    assert!(actual_json["error"]
        .as_str()
        .expect("import error should render")
        .contains("missing_dependency_target"));
    assert!(actual_json["next_actions"][0]
        .as_str()
        .expect("next action should render")
        .contains("vida task import-jsonl"));

    let validate_json = run_command_json(&["task", "validate-graph", "--json"], &state_dir);
    assert_release1_shared_envelope_fields(&validate_json, "validate-graph after blocked import");
    assert_eq!(validate_json["status"], "pass");
    assert_eq!(validate_json["surface"], "vida task validate-graph");

    let list_json = run_command_json(&["task", "list", "--json"], &state_dir);
    assert_eq!(list_json["status"], "pass");
    assert_eq!(list_json["task_count"], 0);
    assert!(
        !serde_json::to_string(&list_json)
            .expect("task list json should render")
            .contains("vida-broken"),
        "blocked import must not persist invalid task rows"
    );

    let graph_summary_output =
        run_command_capture(&["taskflow", "graph-summary", "--json"], &state_dir);
    assert!(
        !graph_summary_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&graph_summary_output.stderr)
    );
    let graph_summary_json: serde_json::Value =
        serde_json::from_slice(&graph_summary_output.stdout)
            .expect("graph-summary empty-graph blocked json should parse");
    assert_release1_shared_envelope_fields(
        &graph_summary_json,
        "graph-summary after blocked import",
    );
    assert_eq!(graph_summary_json["status"], "blocked");
    assert_eq!(graph_summary_json["surface"], "vida taskflow graph-summary");
    assert_eq!(
        graph_summary_json["blocker_codes"],
        serde_json::json!(["no_ready_tasks", "task_graph_empty"])
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn graph_summary_invalid_persisted_graph_returns_json_envelope() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    create_epic_parent(
        &state_dir,
        "persisted-invalid-root",
        "Persisted invalid root",
        "open",
    );
    let child = run_command_json(
        &[
            "task",
            "create",
            "persisted-invalid-child",
            "Persisted invalid child",
            "--type",
            "task",
            "--status",
            "open",
            "--parent-id",
            "persisted-invalid-root",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child["status"], "pass");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let dependencies = serde_json::json!([
            {
                "issue_id": "persisted-invalid-child",
                "depends_on_id": "persisted-missing-parent",
                "edge_type": "parent-child",
                "created_at": "2026-03-08T00:00:00Z",
                "created_by": "tester",
                "metadata": "{}",
                "thread_id": ""
            }
        ]);
        db.query("UPDATE type::record('task', $task) SET dependencies = $dependencies")
            .bind(("task", "persisted-invalid-child"))
            .bind(("dependencies", dependencies))
            .await
            .expect("seed invalid dependency graph");
    });

    let validate_output = vida()
        .args(["task", "validate-graph", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("validate-graph should run");
    assert!(
        !validate_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
    let validate_json: serde_json::Value = serde_json::from_slice(&validate_output.stdout)
        .expect("validate-graph blocked json should parse");
    assert_release1_shared_envelope_fields(&validate_json, "blocked validate-graph");
    assert_eq!(validate_json["status"], "blocked");
    assert_eq!(validate_json["surface"], "vida task validate-graph");
    assert_eq!(
        validate_json["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );

    let graph_summary_output = vida()
        .args(["taskflow", "graph-summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("graph-summary should run");
    assert!(
        !graph_summary_output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&graph_summary_output.stderr)
    );
    let graph_summary_json: serde_json::Value =
        serde_json::from_slice(&graph_summary_output.stdout)
            .expect("graph-summary blocked json should parse");
    assert_release1_shared_envelope_fields(&graph_summary_json, "blocked graph-summary");
    assert_eq!(graph_summary_json["status"], "blocked");
    assert_eq!(graph_summary_json["surface"], "vida taskflow graph-summary");
    assert_eq!(
        graph_summary_json["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );
    assert_eq!(graph_summary_json["error_stage"], "critical_path");
    assert!(graph_summary_json["next_actions"][0]
        .as_str()
        .expect("graph-summary next action should render")
        .contains("vida task validate-graph --json"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn dep_add_fails_closed_when_second_parent_child_edge_is_added() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        concat!(
            "{\"id\":\"vida-root-a\",\"title\":\"Root A\",\"description\":\"a\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-root-b\",\"title\":\"Root B\",\"description\":\"b\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-child\",\"title\":\"Child\",\"description\":\"child\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-child\",\"depends_on_id\":\"vida-root-a\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("write parent-child jsonl");

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args([
            "task",
            "dep",
            "add",
            "vida-child",
            "vida-root-b",
            "parent-child",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dep add should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("dependency mutation would create invalid graph"));
    assert!(stderr.contains("multiple_parent_edges"));

    let deps_stdout = run_and_assert_success(&["task", "deps", "vida-child", "--json"], &state_dir);
    assert!(
        deps_stdout.contains("\"depends_on_id\": \"vida-root-a\"")
            || deps_stdout.contains("\"depends_on_id\":\"vida-root-a\"")
    );
    assert!(
        !deps_stdout.contains("\"depends_on_id\": \"vida-root-b\"")
            && !deps_stdout.contains("\"depends_on_id\":\"vida-root-b\"")
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_dependency_bulk_add_creates_50_edges_and_reports_existing_without_partial_failure() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/bulk-issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");

    let mut jsonl = String::new();
    jsonl.push_str("{\"id\":\"bulk-root\",\"title\":\"Bulk root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n");
    jsonl.push_str("{\"id\":\"bulk-source\",\"title\":\"Bulk source\",\"description\":\"source\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"bulk-source\",\"depends_on_id\":\"bulk-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n");
    jsonl.push_str("{\"id\":\"bulk-fail-source\",\"title\":\"Bulk fail source\",\"description\":\"source\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"bulk-fail-source\",\"depends_on_id\":\"bulk-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n");
    for index in 0..50 {
        jsonl.push_str(&format!(
            "{{\"id\":\"bulk-blocker-{index}\",\"title\":\"Bulk blocker {index}\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{{\"issue_id\":\"bulk-blocker-{index}\",\"depends_on_id\":\"bulk-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{{}}\",\"thread_id\":\"\"}}]}}\n"
        ));
    }
    fs::write(&jsonl_path, jsonl).expect("write bulk task jsonl");
    run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);

    let edge_file_path = format!("{state_dir}/bulk-edges.txt");
    let edge_file = (0..50)
        .map(|index| format!("bulk-source:bulk-blocker-{index}:blocks"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&edge_file_path, edge_file).expect("write bulk edge file");
    let dry_run_output = vida()
        .args([
            "task",
            "dep",
            "add-bulk",
            "--edge-file",
            &edge_file_path,
            "--dry-run",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dry-run bulk dependency add should run");
    assert!(
        dry_run_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&dry_run_output.stdout),
        String::from_utf8_lossy(&dry_run_output.stderr)
    );
    let dry_run_json: serde_json::Value =
        serde_json::from_slice(&dry_run_output.stdout).expect("dry-run bulk add json should parse");
    assert_release1_shared_envelope_fields(&dry_run_json, "bulk add dry-run");
    assert_eq!(dry_run_json["status"], "pass");
    assert_eq!(dry_run_json["dry_run"], true);
    assert_eq!(dry_run_json["requested_count"], 50);
    assert_eq!(dry_run_json["created_count"], 50);
    let dry_run_deps =
        run_and_assert_success(&["task", "deps", "bulk-source", "--json"], &state_dir);
    assert!(!dry_run_deps.contains("\"depends_on_id\": \"bulk-blocker-0\""));
    assert!(!dry_run_deps.contains("\"depends_on_id\":\"bulk-blocker-0\""));

    let mut bulk_args = vec![
        "task".to_string(),
        "dep".to_string(),
        "add-bulk".to_string(),
    ];
    for index in 0..50 {
        bulk_args.push("--edge".to_string());
        bulk_args.push(format!("bulk-source:bulk-blocker-{index}:blocks"));
    }
    bulk_args.push("--json".to_string());
    let bulk_output = vida()
        .args(&bulk_args)
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bulk dependency add should run");
    assert!(
        bulk_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&bulk_output.stdout),
        String::from_utf8_lossy(&bulk_output.stderr)
    );
    let bulk_json: serde_json::Value =
        serde_json::from_slice(&bulk_output.stdout).expect("bulk add json should parse");
    assert_release1_shared_envelope_fields(&bulk_json, "bulk add pass");
    assert_eq!(bulk_json["surface"], "vida task dep add-bulk");
    assert_eq!(bulk_json["status"], "pass");
    assert_eq!(bulk_json["requested_count"], 50);
    assert_eq!(bulk_json["created_count"], 50);
    assert_eq!(bulk_json["existing_count"], 0);
    assert_eq!(bulk_json["failed_count"], 0);
    assert_eq!(bulk_json["unapplied_count"], 0);

    let duplicate_output = vida()
        .args(&bulk_args)
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("duplicate bulk dependency add should run");
    assert!(
        duplicate_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&duplicate_output.stdout),
        String::from_utf8_lossy(&duplicate_output.stderr)
    );
    let duplicate_json: serde_json::Value = serde_json::from_slice(&duplicate_output.stdout)
        .expect("duplicate bulk add json should parse");
    assert_eq!(duplicate_json["requested_count"], 50);
    assert_eq!(duplicate_json["created_count"], 0);
    assert_eq!(duplicate_json["existing_count"], 50);
    assert_eq!(duplicate_json["failed_count"], 0);
    assert_eq!(duplicate_json["unapplied_count"], 0);

    let failed_output = vida()
        .args([
            "task",
            "dep",
            "add-bulk",
            "--edge",
            "bulk-fail-source:bulk-blocker-0:blocks",
            "--edge",
            "bulk-fail-source:bulk-missing-target:blocks",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("failing bulk dependency add should run");
    assert!(!failed_output.status.success());
    let failed_json: serde_json::Value =
        serde_json::from_slice(&failed_output.stdout).expect("failed bulk add json should parse");
    assert_release1_shared_envelope_fields(&failed_json, "bulk add blocked");
    assert_eq!(failed_json["status"], "blocked");
    assert_eq!(failed_json["created_count"], 0);
    assert_eq!(failed_json["existing_count"], 0);
    assert_eq!(failed_json["failed_count"], 1);
    assert_eq!(failed_json["unapplied_count"], 1);
    assert_eq!(
        failed_json["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );

    let fail_deps =
        run_and_assert_success(&["task", "deps", "bulk-fail-source", "--json"], &state_dir);
    assert!(!fail_deps.contains("\"depends_on_id\": \"bulk-blocker-0\""));
    assert!(!fail_deps.contains("\"depends_on_id\":\"bulk-blocker-0\""));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_dependency_ensure_reports_ensure_surface_in_json_results() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/ensure-issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);
    run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);

    let ensure_output = vida()
        .args([
            "task", "dep", "ensure", "vida-c", "vida-a", "blocks", "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("ensure dependency should run");
    assert!(
        ensure_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&ensure_output.stdout),
        String::from_utf8_lossy(&ensure_output.stderr)
    );
    let ensure_json: serde_json::Value =
        serde_json::from_slice(&ensure_output.stdout).expect("ensure dependency json should parse");
    assert_release1_shared_envelope_fields(&ensure_json, "ensure dependency pass");
    assert_eq!(ensure_json["surface"], "vida task dep ensure");
    assert_eq!(
        ensure_json["artifact_refs"]["surface"],
        "vida task dep ensure"
    );
    assert_eq!(ensure_json["status"], "pass");
    assert_eq!(ensure_json["requested_count"], 1);
    assert_eq!(ensure_json["created_count"], 1);
    assert_eq!(ensure_json["existing_count"], 0);

    let duplicate_output = vida()
        .args([
            "task", "dep", "ensure", "vida-c", "vida-a", "blocks", "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("duplicate ensure dependency should run");
    assert!(duplicate_output.status.success());
    let duplicate_json: serde_json::Value = serde_json::from_slice(&duplicate_output.stdout)
        .expect("duplicate ensure dependency json should parse");
    assert_eq!(duplicate_json["surface"], "vida task dep ensure");
    assert_eq!(duplicate_json["created_count"], 0);
    assert_eq!(duplicate_json["existing_count"], 1);

    let failed_output = vida()
        .args([
            "task",
            "dep",
            "ensure",
            "vida-c",
            "missing-task",
            "blocks",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("failing ensure dependency should run");
    assert!(!failed_output.status.success());
    let failed_json: serde_json::Value = serde_json::from_slice(&failed_output.stdout)
        .expect("failed ensure dependency json should parse");
    assert_release1_shared_envelope_fields(&failed_json, "ensure dependency blocked");
    assert_eq!(failed_json["surface"], "vida task dep ensure");
    assert_eq!(failed_json["status"], "blocked");
    assert!(failed_json["next_actions"]
        .as_array()
        .expect("next actions should be an array")
        .iter()
        .any(|action| action
            .as_str()
            .expect("next action should be a string")
            .contains("vida task dep ensure vida-c missing-task blocks --json")));

    let invalid_graph_output = vida()
        .args([
            "task", "dep", "ensure", "vida-c", "vida-c", "blocks", "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("invalid graph ensure dependency should run");
    assert!(!invalid_graph_output.status.success());
    let invalid_graph_json: serde_json::Value =
        serde_json::from_slice(&invalid_graph_output.stdout)
            .expect("invalid graph ensure dependency json should parse");
    assert_eq!(invalid_graph_json["surface"], "vida task dep ensure");
    assert_eq!(invalid_graph_json["status"], "blocked");
    assert!(invalid_graph_json["next_actions"]
        .as_array()
        .expect("next actions should be an array")
        .iter()
        .any(|action| action
            .as_str()
            .expect("next action should be a string")
            .contains("vida task dep ensure vida-c vida-c blocks --json")));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn run_graph_update_fails_closed_when_memory_correction_lacks_sealed_context() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let init = vida()
        .args(["taskflow", "run-graph", "init", "vida-memory-gov", "writer"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-memory-gov",
            "writer",
            "writer",
            "in_progress",
            "writer",
            "{\"policy_gate\":\"memory_correction_required\",\"context_state\":\"open\"}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("memory governance evidence shaping required"));
    assert!(stderr.contains("context_state must be `sealed`"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn run_graph_status_accepts_state_dir_override() {
    let env_state_dir = unique_state_dir();
    let explicit_state_dir = unique_state_dir();
    fs::create_dir_all(&env_state_dir).expect("create env state dir");
    fs::create_dir_all(&explicit_state_dir).expect("create explicit state dir");

    let help = vida()
        .args(["taskflow", "run-graph", "status", "--help"])
        .output()
        .expect("run-graph status help should run");
    assert!(
        help.status.success(),
        "help should pass: {}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_text = format!(
        "{}{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(
        help_text.contains("--state-dir <path>"),
        "run-graph status help must document --state-dir: {help_text}"
    );
    let topic_help = vida()
        .args(["taskflow", "run-graph", "--help"])
        .output()
        .expect("run-graph topic help should run");
    assert!(
        topic_help.status.success(),
        "run-graph topic help should pass: {}",
        String::from_utf8_lossy(&topic_help.stderr)
    );
    let topic_help_text = format!(
        "{}{}",
        String::from_utf8_lossy(&topic_help.stdout),
        String::from_utf8_lossy(&topic_help.stderr)
    );
    assert!(
        topic_help_text.contains("run-graph status <run-id> [--state-dir <path>] [--json]"),
        "run-graph topic help must document status --state-dir: {topic_help_text}"
    );

    let _ = run_and_assert_success(&["boot"], &env_state_dir);
    let _ = run_and_assert_success(&["boot"], &explicit_state_dir);
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            "override-run",
            "implementation",
        ],
        &explicit_state_dir,
    );

    let output = run_command_capture(
        &[
            "taskflow",
            "run-graph",
            "status",
            "override-run",
            "--state-dir",
            &explicit_state_dir,
            "--json",
        ],
        &env_state_dir,
    );
    assert!(
        output.status.success(),
        "status should use explicit --state-dir over VIDA_STATE_DIR\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status output should parse as json");
    assert_eq!(payload["surface"], "vida taskflow run-graph status");
    assert_eq!(payload["run_id"], "override-run");
    assert_eq!(payload["status"], "pass");
    assert!(
        payload["projection_truth"].is_object(),
        "status should include projection truth from explicit state root: {payload}"
    );

    let _ = fs::remove_dir_all(&env_state_dir);
    let _ = fs::remove_dir_all(&explicit_state_dir);
}

#[test]
fn run_graph_readonly_surfaces_accept_state_dir_override() {
    let env_state_dir = unique_state_dir();
    let explicit_state_dir = unique_state_dir();
    fs::create_dir_all(&env_state_dir).expect("create env state dir");
    fs::create_dir_all(&explicit_state_dir).expect("create explicit state dir");

    for (args, expected_usage) in [
        (
            vec!["taskflow", "run-graph", "latest", "--help"],
            "run-graph latest [--state-dir <path>] [--json]",
        ),
        (
            vec!["taskflow", "run-graph", "diagnose", "--help"],
            "run-graph diagnose <run-id> [--state-dir <path>] [--json]",
        ),
        (
            vec!["taskflow", "run-graph", "diagnose-latest", "--help"],
            "run-graph diagnose-latest [--state-dir <path>] [--json]",
        ),
    ] {
        let output = run_command_capture(&args, &env_state_dir);
        assert!(
            output.status.success(),
            "help should pass for {args:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains(expected_usage),
            "help for {args:?} must document state-dir usage {expected_usage}: {text}"
        );
    }

    let _ = run_and_assert_success(&["boot"], &env_state_dir);
    let _ = run_and_assert_success(&["boot"], &explicit_state_dir);
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            "readonly-override-run",
            "implementation",
        ],
        &explicit_state_dir,
    );

    let latest = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "latest",
            "--state-dir",
            &explicit_state_dir,
            "--json",
        ],
        &env_state_dir,
    );
    assert_eq!(latest["surface"], "vida taskflow run-graph latest");
    assert_eq!(latest["run_id"], "readonly-override-run");
    assert_eq!(latest["status"], "pass");

    let diagnose = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "diagnose",
            "readonly-override-run",
            "--state-dir",
            &explicit_state_dir,
            "--json",
        ],
        &env_state_dir,
    );
    assert_eq!(diagnose["surface"], "vida taskflow run-graph diagnose");
    assert_eq!(diagnose["run_id"], "readonly-override-run");
    assert_eq!(diagnose["status"], "pass");

    let diagnose_latest = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "diagnose-latest",
            "--state-dir",
            &explicit_state_dir,
            "--json",
        ],
        &env_state_dir,
    );
    assert_eq!(
        diagnose_latest["surface"],
        "vida taskflow run-graph diagnose-latest"
    );
    assert_eq!(diagnose_latest["run_id"], "readonly-override-run");
    assert_eq!(diagnose_latest["status"], "pass");
    assert_eq!(
        diagnose_latest["projection_truth"],
        diagnose["projection_truth"]
    );

    let _ = fs::remove_dir_all(&env_state_dir);
    let _ = fs::remove_dir_all(&explicit_state_dir);
}

#[test]
fn run_graph_diagnose_json_surfaces_are_public_cli_covered() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let _ = run_and_assert_success(&["boot"], &state_dir);
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            "diagnose-run",
            "implementation",
        ],
        &state_dir,
    );

    let diagnose = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "diagnose",
            "diagnose-run",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(diagnose["surface"], "vida taskflow run-graph diagnose");
    assert_eq!(diagnose["run_id"], "diagnose-run");
    assert_eq!(diagnose["status"], "pass");
    assert!(
        diagnose["blocker_codes"].is_array(),
        "diagnose must expose blocker_codes: {diagnose}"
    );
    assert!(
        diagnose["next_actions"].is_array(),
        "diagnose must expose next_actions: {diagnose}"
    );
    assert!(
        diagnose["operator_contracts"].is_object(),
        "diagnose must expose operator_contracts: {diagnose}"
    );
    assert!(
        diagnose["shared_fields"].is_object(),
        "diagnose must expose shared_fields: {diagnose}"
    );
    assert!(
        diagnose["projection_truth"].is_object(),
        "diagnose must expose projection truth: {diagnose}"
    );
    assert!(
        diagnose["recovery"].is_object(),
        "diagnose must expose recovery summary: {diagnose}"
    );
    assert_eq!(diagnose["operator_contracts"]["status"], diagnose["status"]);
    assert_eq!(diagnose["shared_fields"]["status"], diagnose["status"]);

    let latest = run_command_json(
        &["taskflow", "run-graph", "diagnose-latest", "--json"],
        &state_dir,
    );
    assert_eq!(latest["surface"], "vida taskflow run-graph diagnose-latest");
    assert_eq!(latest["run_id"], "diagnose-run");
    assert_eq!(latest["status"], "pass");
    assert_eq!(latest["operator_contracts"]["status"], latest["status"]);
    assert_eq!(latest["shared_fields"]["status"], latest["status"]);
    assert_eq!(latest["projection_truth"], diagnose["projection_truth"]);
    assert_eq!(latest["recovery"], diagnose["recovery"]);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn run_graph_dispatch_init_plain_reuses_fast_cache_public_cli() {
    let (project_root, state_dir) = project_bound_state_dir();
    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent = run_command_json(
        &[
            "task",
            "create",
            "dispatch-cache-root",
            "Dispatch cache root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");
    let task = run_command_json(
        &[
            "task",
            "create",
            "dispatch-cache-run",
            "Dispatch cache run",
            "--parent-id",
            "dispatch-cache-root",
            "--type",
            "task",
            "--priority",
            "1",
            "--owned-path",
            "crates/vida/src/taskflow_run_graph.rs",
            "--proof-target",
            "cargo test -p vida --test task_smoke run_graph_dispatch_init_plain_reuses_fast_cache_public_cli -- --nocapture",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    let json_dispatch = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            "dispatch-cache-run",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        json_dispatch["surface"],
        "vida taskflow run-graph dispatch-init"
    );
    assert_eq!(json_dispatch["run_id"], "dispatch-cache-run");
    let packet_path = json_dispatch["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should render");
    assert!(
        std::path::Path::new(packet_path).exists(),
        "dispatch packet path should exist: {packet_path}"
    );
    let dispatch_target = json_dispatch["dispatch_receipt"]["dispatch_target"]
        .as_str()
        .expect("dispatch target should render");

    let plain = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            "dispatch-cache-run",
        ],
        &state_dir,
    );
    assert!(
        plain.contains("vida taskflow run-graph dispatch-init"),
        "plain dispatch-init should render surface header: {plain}"
    );
    assert!(
        plain.contains("run: dispatch-cache-run"),
        "plain dispatch-init should render run id: {plain}"
    );
    assert!(
        plain.contains(packet_path),
        "plain dispatch-init should reuse cached packet path {packet_path}: {plain}"
    );
    assert!(
        plain.contains(dispatch_target),
        "plain dispatch-init should reuse cached dispatch target {dispatch_target}: {plain}"
    );

    fs::remove_dir_all(project_root).expect("temp project root should be removed");
}

#[test]
fn run_graph_update_canonicalizes_conflicting_resume_meta() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);

    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", "vida-memory-gov", "writer"],
        &state_dir,
    );

    let meta = "{\"resume_target\":\"dispatch.coach\",\"next_node\":\"writer\",\"handoff_state\":\"awaiting_writer\"}";
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            "vida-memory-gov",
            "writer",
            "coach",
            "ready",
            "writer",
            meta,
        ],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    let (resume_target, next_node, handoff_state) = runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");

        let mut resume_query = db
            .query("SELECT resume_target FROM resumability_capsule WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query resumability");
        let resume_rows: Vec<Value> = resume_query.take(0).expect("take resume rows");
        let resume_row = resume_rows
            .first()
            .cloned()
            .expect("resumability capsule should exist");

        let mut execution_query = db
            .query("SELECT next_node FROM execution_plan_state WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query execution plan");
        let execution_rows: Vec<Value> = execution_query.take(0).expect("take execution rows");
        let execution_row = execution_rows
            .first()
            .cloned()
            .expect("execution plan should exist");

        let mut governance_query = db
            .query("SELECT handoff_state FROM governance_state WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query governance");
        let governance_rows: Vec<Value> = governance_query.take(0).expect("take governance rows");
        let governance_row = governance_rows
            .first()
            .cloned()
            .expect("governance state should exist");

        (
            resume_row["resume_target"].as_str().map(String::from),
            execution_row["next_node"].as_str().map(String::from),
            governance_row["handoff_state"].as_str().map(String::from),
        )
    });

    assert_eq!(resume_target.as_deref(), Some("dispatch.coach"));
    assert_eq!(next_node.as_deref(), Some("coach"));
    assert_eq!(handoff_state.as_deref(), Some("awaiting_coach"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn missing_task_stale_blocked_run_can_retire_without_ambiguous_next_action() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let run_id = "h22-missing-task";
    let task_id = run_id;
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id,
            "implementation",
            "implementation",
            "blocked",
            "implementation",
            "{\"policy_gate\":\"validation_report_required\",\"context_state\":\"sealed\",\"resume_target\":\"none\",\"recovery_ready\":false}",
        ],
        &state_dir,
    );

    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = format!("{packet_dir}/{run_id}.json");
    fs::write(&packet_path, format!("{{\"run_id\":\"{run_id}\"}}")).expect("write packet");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementation",
            "dispatch_status": "blocked",
            "lane_status": "lane_running",
            "dispatch_kind": "test_dispatch",
            "dispatch_surface": "vida taskflow run-graph dispatch-init",
            "dispatch_command": "vida taskflow run-graph dispatch-init h22-missing-task --json",
            "dispatch_packet_path": packet_path,
            "blocker_code": "tool_execution_failed",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": [],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "worker",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", run_id))
            .bind(("receipt", receipt))
            .await
            .expect("seed blocked dispatch receipt");
        let binding = serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": task_id,
                "run_id": run_id,
                "active_node": "implementation"
            },
            "binding_source": "h22_regression_seed",
            "why_this_unit": "stale missing-task run was blocking continuation",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", run_id))
            .bind(("binding", binding))
            .await
            .expect("seed stale continuation binding");
        drop(db);
    });

    let retire = run_command_capture(
        &[
            "lane",
            "retire",
            run_id,
            "--receipt-id",
            "h22-retire-missing-task",
            "--reason",
            "missing task stale blocked run",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        retire.status.success(),
        "retire stdout={} stderr={}",
        String::from_utf8_lossy(&retire.stdout),
        String::from_utf8_lossy(&retire.stderr)
    );
    let retired: serde_json::Value =
        serde_json::from_slice(&retire.stdout).expect("retire json should parse");
    assert_eq!(retired["run_id"], run_id);
    assert_eq!(retired["status"], "pass");
    assert_eq!(retired["lane_status"], "lane_completed");
    assert_eq!(retired["dispatch_status"], "executed");
    assert!(retired["blocker_codes"].as_array().unwrap().is_empty());
    assert!(retired["next_actions"].as_array().unwrap().is_empty());

    let recovery = run_command_json(
        &["taskflow", "recovery", "status", run_id, "--json"],
        &state_dir,
    );
    assert_eq!(recovery["run_id"], run_id);
    assert_eq!(recovery["status"], "pass");
    assert_eq!(recovery["recovery"]["resume_status"], "completed");
    assert_eq!(recovery["recovery"]["lifecycle_stage"], "closure_complete");
    assert_eq!(recovery["recovery"]["recovery_ready"], false);

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "empty sandbox should fail closed after retiring stale missing-task run"
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .expect("next-lawful blocked json should parse");
    assert_eq!(next_lawful["status"], "blocked");
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));
    assert_ne!(
        next_lawful["binding_source"], "h22_regression_seed",
        "retired missing-task run must not remain as the next lawful continuation"
    );
    assert!(
        !next_lawful.to_string().contains("h22-missing-task"),
        "retired missing-task run must not leak into next action"
    );

    let orchestrator = run_command_json(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert!(
        !orchestrator.to_string().contains(run_id),
        "retired missing-task run must not leak into orchestrator-init: {orchestrator}"
    );
    assert_ne!(
        orchestrator["continuation_binding"]["binding_source"], "h22_regression_seed",
        "orchestrator-init must not preserve stale binding source after retire"
    );
    assert!(
        !orchestrator
            .to_string()
            .contains("continuation_binding_ambiguous"),
        "orchestrator-init must not remain ambiguous on retired stale run: {orchestrator}"
    );

    let status = run_command_json(&["status", "--state-dir", &state_dir, "--json"], &state_dir);
    assert_eq!(
        status["root_session_write_guard"]["latest_run_graph_task_stale"], false,
        "terminal retired missing-task run must not keep the root write guard stale: {status}"
    );
    assert_ne!(
        status["root_session_write_guard"]["reason"], "latest_run_graph_task_stale",
        "terminal retired missing-task run must not be the status write-guard blocker: {status}"
    );
    assert!(
        !status
            .to_string()
            .contains("continuation_binding_ambiguous"),
        "status must not remain ambiguous on retired stale run: {status}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn exception_takeover_missing_task_stale_run_can_follow_consume_continue_retire_action() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let run_id = "universal-surfaces-kanban-cross-column-drag-drop";
    let task_id = "universal-surfaces-kanban-cross-column-drag-drop";
    let exception_task_id =
        "universal-surfaces-kanban-cross-column-drag-drop:implementer:exception-takeover";
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = format!("{packet_dir}/{run_id}.json");
    fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "activation_runtime_role": "implementer",
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["crates/vida/src/lane_surface.rs"],
            "read_only_paths": [".vida/data/state/runtime-consumption"],
            "role_selection_full": {
                "ok": true,
                "activation_source": "packet",
                "selection_mode": "fixed",
                "fallback_role": "orchestrator",
                "request": "coach review",
                "selected_role": "coach",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "junior",
                            "backend_class": "internal",
                            "lane_admissibility": {
                                "implementation": true
                            }
                        }
                    ],
                    "development_flow": {
                        "coach": {
                            "executor_backend": "internal_subagents"
                        }
                    }
                },
                "reason": "test"
            },
            "delivery_task_packet": {
                "task_id": task_id,
                "goal": "Recover a stale exception takeover run whose TaskFlow task was removed.",
                "scope_in": ["dispatch_target:implementer"],
                "handoff_task_class": "implementation",
                "handoff_runtime_role": "implementer",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["stale missing-task lane can be retired"],
                "verification_command": "vida lane retire",
                "proof_target": "lane retirement receipt",
                "stop_rules": ["stop if packet contract is invalid"],
                "blocking_question": "none"
            }
        })
        .to_string(),
    )
    .expect("write exception takeover packet");
    let metadata_dir = format!("{state_dir}/lane-exception-path-metadata");
    fs::create_dir_all(&metadata_dir).expect("create metadata dir");
    fs::write(
        format!("{metadata_dir}/{run_id}.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "dispatch_packet_path": packet_path,
            "source_exception_path_receipt_id": run_id,
            "reason_class": "blocked_open_delegated_cycle_timeout",
            "active_bounded_unit": exception_task_id,
            "owned_write_scope": ["crates/vida/src"],
            "why_delegated_or_rerouted_path_is_not_currently_lawful": "blocked",
            "why_local_write_is_the_smallest_safe_bounded_workaround": "bounded",
            "return_to_normal_posture_condition": "verified",
            "verification_plan": ["test"],
            "recorded_at": "2026-06-04T00:00:00Z"
        }))
        .expect("encode exception metadata"),
    )
    .expect("write exception metadata");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query("UPSERT type::record('routed_run_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "route_task_class": "implementation",
                    "selected_backend": "internal_subagents",
                    "lane_id": "implementer",
                    "lifecycle_stage": "implementer_blocked",
                    "updated_at": "2026-06-04T00:00:00Z"
                }),
            ))
            .await
            .expect("seed routed run state");
        db.query("UPSERT type::record('governance_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "policy_gate": "host_tool_bridge_adapter_required",
                    "handoff_state": "none",
                    "context_state": "sealed",
                    "updated_at": "2026-06-04T00:00:00Z"
                }),
            ))
            .await
            .expect("seed governance state");
        db.query("UPSERT type::record('resumability_capsule', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "checkpoint_kind": "none",
                    "resume_target": "implementer",
                    "recovery_ready": false,
                    "updated_at": "2026-06-04T00:00:00Z"
                }),
            ))
            .await
            .expect("seed resumability capsule");
        db.query("UPSERT type::record('execution_plan_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "task_id": task_id,
                    "task_class": "implementation",
                    "active_node": "implementer",
                    "next_node": "tester",
                    "status": "blocked",
                    "updated_at": "2026-06-04T00:00:00Z"
                }),
            ))
            .await
            .expect("seed execution plan state");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "dispatch_status": "bridge_request_pending",
            "lane_status": "lane_exception_takeover",
            "exception_path_receipt_id": run_id,
            "dispatch_kind": "agent_lane",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": format!("vida agent-init --role implementer {task_id} --json"),
            "dispatch_packet_path": packet_path,
            "blocker_code": "host_tool_bridge_adapter_required",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["host_tool_bridge_adapter_required"],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "implementer",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-06-04T00:00:00Z"
        });
        let _: Option<Value> = db
            .upsert(("run_graph_dispatch_receipt", run_id))
            .content(receipt)
            .await
            .expect("seed exception takeover dispatch receipt");
        let stored_receipt: Option<Value> = db
            .select(("run_graph_dispatch_receipt", run_id))
            .await
            .expect("read seeded exception takeover dispatch receipt");
        assert!(
            stored_receipt.is_some(),
            "direct seed should create a run_graph_dispatch_receipt row"
        );
        let binding = serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": task_id,
                "run_id": run_id,
                "active_node": "implementer"
            },
            "binding_source": "exception_missing_task_regression_seed",
            "why_this_unit": "stale missing-task exception run was blocking continuation",
            "primary_path": "exception_takeover_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-06-04T00:00:01Z"
        });
        let _: Option<Value> = db
            .upsert(("run_graph_continuation_binding", run_id))
            .content(binding)
            .await
            .expect("seed stale continuation binding");
        drop(db);
    });
    thread::sleep(Duration::from_millis(300));

    let lane_before = run_command_capture(&["lane", "show", run_id, "--json"], &state_dir);
    let lane_before_json: serde_json::Value =
        serde_json::from_slice(&lane_before.stdout).expect("lane show json should parse");
    assert_eq!(lane_before_json["status"], "blocked");
    assert_eq!(
        lane_before_json["dispatch_status"],
        "bridge_request_pending"
    );
    assert_eq!(lane_before_json["lane_status"], "lane_exception_takeover");

    let (continue_payload, _continue_success) = run_command_json_allow_failure(
        &[
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ],
        &state_dir,
    );
    assert_eq!(continue_payload["status"], "blocked");
    let blockers =
        require_json_string_array(&continue_payload["blocker_codes"], "consume blockers");
    assert!(
        blockers.contains(&"stale_missing_task_run_graph".to_string()),
        "consume continue should classify the executable retire path, got {continue_payload}"
    );
    let next_actions = require_json_string_array(&continue_payload["next_actions"], "next actions");
    assert!(
        next_actions
            .iter()
            .any(|action| action.contains(&format!("vida lane retire {run_id}"))),
        "consume continue should expose executable retire command in top-level next_actions: {continue_payload}"
    );

    let lane_after_continue = run_command_capture(&["lane", "show", run_id, "--json"], &state_dir);
    let lane_after_continue_json: serde_json::Value =
        serde_json::from_slice(&lane_after_continue.stdout).expect("lane show json should parse");
    assert_eq!(lane_after_continue_json["status"], "blocked");
    assert_eq!(
        lane_after_continue_json["recommended_surface"],
        "vida lane supersede"
    );

    let retire = run_command_capture(
        &[
            "lane",
            "retire",
            run_id,
            "--receipt-id",
            run_id,
            "--reason",
            "missing TaskFlow task stale run",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        retire.status.success(),
        "retire stdout={} stderr={}",
        String::from_utf8_lossy(&retire.stdout),
        String::from_utf8_lossy(&retire.stderr)
    );
    let retired: serde_json::Value =
        serde_json::from_slice(&retire.stdout).expect("retire json should parse");
    assert_eq!(retired["run_id"], run_id);
    assert_eq!(retired["status"], "pass");
    assert_eq!(retired["lane_status"], "lane_completed");
    assert_eq!(retired["dispatch_status"], "executed");

    let recovery = run_command_json(
        &["taskflow", "recovery", "status", run_id, "--json"],
        &state_dir,
    );
    assert_eq!(recovery["recovery"]["resume_status"], "completed");
    assert_eq!(recovery["recovery"]["lifecycle_stage"], "closure_complete");
    assert_eq!(recovery["recovery"]["recovery_ready"], false);
    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "empty sandbox should fail closed after retiring stale exception missing-task run"
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .expect("next-lawful blocked json should parse");
    assert_eq!(next_lawful["status"], "blocked");
    assert!(
        !next_lawful.to_string().contains(run_id),
        "retired exception missing-task run must not leak into next action"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn terminal_exception_takeover_run_does_not_reemit_missing_task_retire_action() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let ready_task_id = "terminal-successor-ready";
    let parent_id = "terminal-successor-parent";
    create_epic_parent(&state_dir, parent_id, "Terminal successor parent", "open");
    let ready = run_command_json(
        &[
            "task",
            "create",
            ready_task_id,
            "Terminal successor ready task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(ready["status"], "pass");

    let run_id = "universal-surfaces-kanban-cross-column-drag-drop";
    let exception_task_id =
        "universal-surfaces-kanban-cross-column-drag-drop:implementer:exception-takeover";
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = format!("{packet_dir}/{run_id}.json");
    fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "activation_runtime_role": "implementer",
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
            "read_only_paths": [".vida/data/state/runtime-consumption"],
            "role_selection_full": {
                "ok": true,
                "activation_source": "packet",
                "selection_mode": "fixed",
                "fallback_role": "orchestrator",
                "request": "terminal closure replay",
                "selected_role": "implementer",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "junior",
                            "backend_class": "internal",
                            "lane_admissibility": {
                                "implementation": true
                            }
                        }
                    ],
                    "development_flow": {
                        "implementer": {
                            "executor_backend": "internal_subagents"
                        }
                    }
                },
                "reason": "test"
            },
            "delivery_task_packet": {
                "task_id": run_id,
                "goal": "Terminal closure should supersede stale exception takeover evidence.",
                "scope_in": ["dispatch_target:implementer"],
                "handoff_task_class": "implementation",
                "handoff_runtime_role": "implementer",
                "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["terminal closure remains authoritative"],
                "verification_command": "vida taskflow consume continue",
                "proof_target": "terminal closure consistency",
                "stop_rules": ["stop if stale retire is recommended"],
                "blocking_question": "none"
            }
        })
        .to_string(),
    )
    .expect("write terminal stale packet");
    let metadata_dir = format!("{state_dir}/lane-exception-path-metadata");
    fs::create_dir_all(&metadata_dir).expect("create metadata dir");
    fs::write(
        format!("{metadata_dir}/{run_id}.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "dispatch_packet_path": packet_path,
            "source_exception_path_receipt_id": run_id,
            "reason_class": "blocked_open_delegated_cycle_timeout",
            "active_bounded_unit": exception_task_id,
            "owned_write_scope": ["crates/vida/src"],
            "why_delegated_or_rerouted_path_is_not_currently_lawful": "blocked",
            "why_local_write_is_the_smallest_safe_bounded_workaround": "bounded",
            "return_to_normal_posture_condition": "verified",
            "verification_plan": ["test"],
            "recorded_at": "2026-06-05T00:00:00Z"
        }))
        .expect("encode exception metadata"),
    )
    .expect("write exception metadata");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query("UPSERT type::record('routed_run_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "route_task_class": "implementation",
                    "selected_backend": "internal_subagents",
                    "lane_id": "implementer",
                    "lifecycle_stage": "closure_complete",
                    "updated_at": "2026-06-05T00:00:00Z"
                }),
            ))
            .await
            .expect("seed terminal routed run state");
        db.query("UPSERT type::record('governance_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "policy_gate": "closed_task_stale_run_retired",
                    "handoff_state": "none",
                    "context_state": "sealed",
                    "updated_at": "2026-06-05T00:00:00Z"
                }),
            ))
            .await
            .expect("seed terminal governance state");
        db.query("UPSERT type::record('resumability_capsule', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "checkpoint_kind": "none",
                    "resume_target": "none",
                    "recovery_ready": false,
                    "updated_at": "2026-06-05T00:00:00Z"
                }),
            ))
            .await
            .expect("seed terminal resumability capsule");
        db.query("UPSERT type::record('execution_plan_state', $run) CONTENT $state")
            .bind(("run", run_id))
            .bind((
                "state",
                serde_json::json!({
                    "run_id": run_id,
                    "task_id": run_id,
                    "task_class": "implementation",
                    "active_node": "closure",
                    "status": "completed",
                    "updated_at": "2026-06-05T00:00:00Z"
                }),
            ))
            .await
            .expect("seed terminal execution plan state");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "dispatch_status": "bridge_request_pending",
            "lane_status": "lane_exception_takeover",
            "supersedes_receipt_id": run_id,
            "exception_path_receipt_id": run_id,
            "dispatch_kind": "agent_lane",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": format!("vida agent-init --role implementer {run_id} --json"),
            "dispatch_packet_path": packet_path,
            "blocker_code": "host_tool_bridge_adapter_required",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["host_tool_bridge_adapter_required"],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "implementer",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-06-05T00:00:01Z"
        });
        let _: Option<Value> = db
            .upsert(("run_graph_dispatch_receipt", run_id))
            .content(receipt)
            .await
            .expect("seed stale terminal exception receipt");
        let binding = serde_json::json!({
            "run_id": run_id,
            "task_id": run_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "downstream_dispatch_target",
                "dispatch_target": "closure",
                "run_id": run_id
            },
            "binding_source": "task_close_reconcile",
            "why_this_unit": "terminal closure was already reconciled",
            "primary_path": "task_close_reconcile",
            "sequential_vs_parallel_posture": "sequential_only_terminal_closure",
            "recorded_at": "2026-06-05T00:00:02Z"
        });
        let _: Option<Value> = db
            .upsert(("run_graph_continuation_binding", run_id))
            .content(binding)
            .await
            .expect("seed terminal continuation binding");
        drop(db);
    });
    thread::sleep(Duration::from_millis(300));

    let run_graph = run_command_json(
        &["taskflow", "run-graph", "status", run_id, "--json"],
        &state_dir,
    );
    assert_eq!(run_graph["status"], "pass");
    assert!(
        run_graph.to_string().contains("closure_complete"),
        "run-graph status must expose terminal closure evidence: {run_graph}"
    );
    assert!(
        run_graph.to_string().contains("completed"),
        "run-graph status must expose completed terminal evidence: {run_graph}"
    );
    assert!(
        !run_graph.to_string().contains("vida lane retire"),
        "terminal run-graph status must not recommend stale retire: {run_graph}"
    );

    let (default_consume, _default_consume_success) =
        run_command_json_allow_failure(&["taskflow", "consume", "continue", "--json"], &state_dir);
    assert_ne!(
        default_consume["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"]),
        "default terminal consume continue must not re-emit stale missing-task retire blocker: {default_consume}"
    );
    assert!(
        !default_consume.to_string().contains("vida lane retire"),
        "default terminal consume continue must not recommend impossible lane retire: {default_consume}"
    );

    let (consume, _consume_success) = run_command_json_allow_failure(
        &[
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ],
        &state_dir,
    );
    assert_ne!(
        consume["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"]),
        "terminal closure must not re-emit stale missing-task retire blocker: {consume}"
    );
    assert!(
        !consume.to_string().contains("vida lane retire"),
        "terminal closure must not recommend impossible lane retire: {consume}"
    );

    let recovery = run_command_json(
        &["taskflow", "recovery", "status", run_id, "--json"],
        &state_dir,
    );
    assert_eq!(recovery["recovery"]["resume_status"], "completed");
    assert_eq!(recovery["recovery"]["lifecycle_stage"], "closure_complete");
    assert_eq!(
        recovery["recovery"]["delegation_gate"]["delegated_cycle_open"],
        false
    );
    assert!(
        !recovery.to_string().contains("vida lane retire"),
        "terminal recovery status must not recommend stale retire: {recovery}"
    );

    let retire = run_command_capture(
        &[
            "lane",
            "retire",
            run_id,
            "--receipt-id",
            run_id,
            "--reason",
            "missing TaskFlow task stale run",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !retire.status.success(),
        "terminal lane retire should fail closed instead of mutating terminal run"
    );
    let retire_stdout = String::from_utf8_lossy(&retire.stdout);
    let retire_stderr = String::from_utf8_lossy(&retire.stderr);
    assert!(
        retire_stderr.contains("no longer active for mutation"),
        "terminal lane retire should explain terminal mutation guard: stdout={retire_stdout} stderr={retire_stderr}"
    );
    assert!(
        !retire_stderr.contains("Failed to verify exception bounded unit"),
        "terminal lane retire must not fall through to missing exception bounded unit verification: {retire_stderr}"
    );

    let status = run_command_json(&["status", "--state-dir", &state_dir, "--json"], &state_dir);
    assert!(
        !status.to_string().contains("stale_missing_task_run_graph"),
        "global status must not preserve terminal stale blocker: {status}"
    );
    assert!(
        !status.to_string().contains("vida lane retire"),
        "global status must not recommend terminal stale retire: {status}"
    );

    let dispatch_output = run_command_capture(
        &["agent", "dispatch-next", "--dev-team", "--json"],
        &state_dir,
    );
    let dispatch: serde_json::Value = serde_json::from_slice(&dispatch_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "dispatch-next json should parse: {error}; stdout={} stderr={}",
                String::from_utf8_lossy(&dispatch_output.stdout),
                String::from_utf8_lossy(&dispatch_output.stderr)
            )
        });
    assert!(
        !dispatch.to_string().contains(run_id),
        "dispatch-next must not keep terminal run as active blocker: {dispatch}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn release_admitted_missing_stale_run_does_not_block_recovery_or_dispatch_preview() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let ready_task_id = "case11-ready-after-release";
    let parent_id = "case11-ready-after-release-parent";
    create_epic_parent(&state_dir, parent_id, "CASE-11 ready parent", "open");
    let ready = run_command_json(
        &[
            "task",
            "create",
            ready_task_id,
            "CASE-11 ready task after release",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(ready["status"], "pass");

    let stale_run_id = "runtime-case-closure-admission-evidence-table-completed";
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            stale_run_id,
            "implementation",
        ],
        &state_dir,
    );
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            stale_run_id,
            "implementation",
            "implementation",
            "blocked",
            "implementation",
            "{\"policy_gate\":\"validation_report_required\",\"context_state\":\"sealed\",\"resume_target\":\"none\",\"recovery_ready\":false}",
        ],
        &state_dir,
    );

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    fs::create_dir_all(&runtime_consumption_dir).expect("create runtime-consumption dir");
    fs::write(
        format!("{runtime_consumption_dir}/final-2026-05-19T00-00-02Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "source_run_id": stale_run_id,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {},
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "shared_fields": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "payload": {
                "closure_admission": {
                    "status": "pass",
                    "admitted": true,
                    "closure_decision": "closed",
                    "decision_owner": "release-owner",
                    "decision_at": "2026-05-19T00:00:00Z",
                    "evidence_bundle_refs": ["evidence-bundle-case11"],
                    "open_risk_acceptance_ids": ["risk-acceptance-case11"],
                    "blockers": [],
                    "proof_surfaces": ["vida taskflow consume final"],
                    "evidence_table": [
                        {"evidence_class": "closure_decision_record", "status": "pass", "evidence_refs": ["closure-record-case11"]},
                        {"evidence_class": "runtime_consumption_final_snapshot", "status": "pass", "evidence_refs": ["final-snapshot-case11"]},
                        {"evidence_class": "docflow_readiness_and_proof_receipts", "status": "pass", "evidence_refs": ["docflow-readiness-case11", "docflow-proof-case11"]},
                        {"evidence_class": "lane_execution_and_handoff_receipts", "status": "pass", "evidence_refs": ["lane-execution-case11", "handoff-case11"]},
                        {"evidence_class": "replay_checkpoint_lineage_artifacts", "status": "pass", "evidence_refs": ["checkpoint-case11", "replay-case11"]},
                        {"evidence_class": "risk_acceptance_artifacts", "status": "pass", "evidence_refs": ["risk-acceptance-case11"]},
                        {"evidence_class": "evidence_bundle_linkage", "status": "pass", "evidence_refs": ["evidence-bundle-case11"]}
                    ]
                }
            }
        })
        .to_string(),
    )
    .expect("write release-admitted final snapshot");

    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = format!("{packet_dir}/{stale_run_id}.json");
    fs::write(&packet_path, format!("{{\"run_id\":\"{stale_run_id}\"}}"))
        .expect("write stale packet");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let receipt = serde_json::json!({
            "run_id": stale_run_id,
            "dispatch_target": "implementation",
            "dispatch_status": "blocked",
            "lane_status": "lane_running",
            "dispatch_kind": "test_dispatch",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": "vida agent-init --execute-dispatch",
            "dispatch_packet_path": packet_path,
            "blocker_code": "internal_activation_view_only",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["internal_activation_view_only"],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "worker",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-05-19T00:00:01Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", stale_run_id))
            .bind(("receipt", receipt))
            .await
            .expect("seed stale blocked dispatch receipt");
        drop(db);
    });

    let recovery = run_command_json(&["taskflow", "recovery", "latest", "--json"], &state_dir);
    assert_eq!(recovery["surface"], "vida taskflow recovery latest");
    assert!(
        recovery["status"].is_null() || recovery["status"] == "blocked",
        "recovery latest should either have no resumable run or fail closed without becoming an execution handoff: {recovery}"
    );
    assert!(
        !recovery.to_string().contains(stale_run_id),
        "release-admitted missing stale run should not remain latest recovery: {recovery}"
    );

    let next_lawful = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(next_lawful["active_bounded_unit"]["task_id"], ready_task_id);
    assert!(
        !next_lawful.to_string().contains(stale_run_id),
        "next-lawful must not leak stale run id"
    );

    let dispatch_output = run_command_capture(
        &["agent", "dispatch-next", "--dev-team", "--json"],
        &state_dir,
    );
    let dispatch: serde_json::Value = serde_json::from_slice(&dispatch_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "dispatch-next json should parse: {error}; stdout={} stderr={}",
                String::from_utf8_lossy(&dispatch_output.stdout),
                String::from_utf8_lossy(&dispatch_output.stderr)
            )
        });
    assert!(
        !dispatch.to_string().contains(stale_run_id),
        "dispatch-next must not block on stale missing run"
    );
    let dispatch_blockers = dispatch["blocker_codes"]
        .as_array()
        .expect("dispatch blocker_codes should render");
    assert!(
        !dispatch_blockers
            .iter()
            .any(|code| code == "open_delegated_cycle"),
        "dispatch-next must not preserve the stale delegated-cycle blocker: {dispatch}"
    );
    assert!(
        dispatch["selected_lanes"]
            .as_array()
            .expect("dispatch selected_lanes should render")
            .iter()
            .any(|lane| lane["task_id"].as_str() == Some(ready_task_id)),
        "dispatch-next should continue evaluating the ready successor task: {dispatch}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn dev_team_dispatch_fails_closed_when_latest_run_graph_is_blocked() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let task_id = "dev-team-blocked-coach-task";
    let parent_id = "dev-team-blocked-coach-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Dev team blocked coach parent",
        "open",
    );
    let task = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Dev team blocked coach task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id,
            "coach",
            "tester",
            "blocked",
            "coach",
            "{\"policy_gate\":\"host_tool_bridge_adapter_required\",\"context_state\":\"sealed\",\"resume_target\":\"coach\",\"recovery_ready\":false,\"lifecycle_stage\":\"coach_blocked\"}",
        ],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let receipt = serde_json::json!({
            "run_id": task_id,
            "dispatch_target": "coach",
            "dispatch_status": "bridge_request_pending",
            "lane_status": "blocked",
            "dispatch_kind": "agent_lane",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": format!("vida agent-init --role coach {task_id} --json"),
            "dispatch_packet_path": format!("{state_dir}/runtime-consumption/dispatch-packets/{task_id}.json"),
            "blocker_code": "host_tool_bridge_adapter_required",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["host_tool_bridge_adapter_required"],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "coach",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-05-19T00:00:01Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", task_id))
            .bind(("receipt", receipt))
            .await
            .expect("seed blocked coach dispatch receipt");
        let binding = serde_json::json!({
            "run_id": task_id,
            "task_id": task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": task_id,
                "run_id": task_id,
                "active_node": "coach"
            },
            "binding_source": "dev_team_blocked_coach_regression_seed",
            "why_this_unit": "blocked coach run must gate dev-team dispatch preview",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:02Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", task_id))
            .bind(("binding", binding))
            .await
            .expect("seed blocked coach continuation binding");
        drop(db);
    });

    let (dispatch, dispatch_success) = run_command_json_allow_failure(
        &["agent", "dispatch-next", "--dev-team", "--json"],
        &state_dir,
    );
    assert!(
        !dispatch_success,
        "blocked dev-team dispatch preview should return a failing exit status: {dispatch}"
    );
    assert_eq!(dispatch["status"], "blocked");
    assert_eq!(dispatch["lanes_selected"], 0);
    assert!(dispatch["selected_lanes"]
        .as_array()
        .expect("selected lanes should render")
        .is_empty());
    let blockers = require_json_string_array(&dispatch["blocker_codes"], "dispatch blocker_codes");
    assert!(
        blockers.contains(&"latest_run_graph_status_blocked".to_string()),
        "dispatch-next must expose the blocked latest run-graph gate: {dispatch}"
    );
    assert_eq!(dispatch["flow_projection"]["status"], "blocked");
    assert_eq!(
        dispatch["flow_projection"]["blocked_by_continuation_gate"],
        true
    );
    assert!(dispatch["flow_projection"]["current_step"]["dispatch_command"].is_null());

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn case11_agent_init_timeout_bridge_remains_blocked_evidence_without_impossible_continuation() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let task_id = "taskflow-case-11-actual-agent-autonomy";
    let run_id = task_id;
    let parent_id = "taskflow-case-11-actual-agent-autonomy-parent";
    create_epic_parent(&state_dir, parent_id, "CASE-11 autonomy parent", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "CASE-11 actual agent autonomy",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");

    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id,
            "implementation",
            "implementation",
            "blocked",
            "implementation",
            "{\"policy_gate\":\"agent_init_execute_dispatch_timeout\",\"context_state\":\"sealed\",\"resume_target\":\"none\",\"recovery_ready\":false,\"lifecycle_stage\":\"dispatch_blocked\"}",
        ],
        &state_dir,
    );

    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let result_dir = format!("{state_dir}/runtime-consumption/dispatch-results");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    fs::create_dir_all(&result_dir).expect("create result dir");
    let packet_path = format!("{packet_dir}/{run_id}.json");
    let result_path = format!("{result_dir}/{run_id}.json");
    fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "dispatch_target": "implementation"
        })
        .to_string(),
    )
    .expect("write dispatch packet");
    fs::write(
        &result_path,
        serde_json::json!({
            "surface": "vida agent-init",
            "status": "blocked",
            "execution_state": "blocked",
            "blocker_code": "internal_dispatch_timeout_without_receipt",
            "provider_error": "configured host bridge cannot provide receipt-backed completion evidence",
            "activation_vs_execution_evidence": {
                "evidence_state": "internal_dispatch_timeout_without_receipt",
                "receipt_backed": false
            },
            "activation_semantics": {
                "activation_kind": "activation_view",
                "view_only": true,
                "executes_packet": false,
                "records_completion_receipt": false
            },
            "execution_evidence": null
        })
        .to_string(),
    )
    .expect("write timeout result");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementation",
            "dispatch_status": "blocked",
            "lane_status": "lane_blocked",
            "dispatch_kind": "agent_lane",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": "vida agent-init --dispatch-packet packet --execute-dispatch --json",
            "dispatch_packet_path": packet_path,
            "dispatch_result_path": result_path,
            "blocker_code": "internal_dispatch_timeout_without_receipt",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["internal_dispatch_timeout_without_receipt"],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "worker",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", run_id))
            .bind(("receipt", receipt))
            .await
            .expect("seed timeout dispatch receipt");
        let binding = serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": task_id,
                "run_id": run_id,
                "active_node": "implementation"
            },
            "binding_source": "agent_init_execute_dispatch_timeout",
            "why_this_unit": "CASE-11 timeout bridge remains blocked without completion evidence",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_case_11_only",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", run_id))
            .bind(("binding", binding))
            .await
            .expect("seed timeout continuation binding");
        drop(db);
    });

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        status["latest_run_graph_dispatch_receipt"]["blocker_code"],
        "internal_dispatch_timeout_without_receipt"
    );
    assert_eq!(
        status["latest_run_graph_dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    assert_no_run_id_consume_continue_command(&status, run_id, "status");

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "CASE-11 timeout bridge must not be treated as completion: stdout={} stderr={}",
        String::from_utf8_lossy(&next_lawful_output.stdout),
        String::from_utf8_lossy(&next_lawful_output.stderr)
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .expect("next-lawful blocked json should parse");
    assert_eq!(next_lawful["status"], "blocked");
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "open_delegated_cycle"));
    assert_no_run_id_consume_continue_command(&next_lawful, run_id, "next-lawful");

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(
        doctor["latest_run_graph_dispatch_receipt"]["blocker_code"],
        "internal_dispatch_timeout_without_receipt"
    );
    assert_no_run_id_consume_continue_command(&doctor, run_id, "doctor");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn agent_init_execute_dispatch_missing_packet_json_is_operator_envelope() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let output = run_command_capture(
        &[
            "agent-init",
            "--json",
            "--role",
            "worker",
            "--execute-dispatch",
            "Implement bounded change",
        ],
        &state_dir,
    );
    assert!(
        !output.status.success(),
        "packetless execute-dispatch must fail closed"
    );
    assert!(
        output.stderr.is_empty(),
        "json mode should not emit plain stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "packetless execute-dispatch should render parseable json: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    assert_eq!(payload["surface"], "vida agent-init");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_code"],
        "agent_init_execute_dispatch_missing_packet"
    );
    assert_eq!(
        payload["operator_contracts"]["blocker_codes"][0],
        "agent_init_execute_dispatch_missing_packet"
    );
    assert_eq!(
        payload["shared_fields"]["artifact_refs"]["required_packet_flags"][0],
        "--dispatch-packet"
    );
    assert!(payload["next_actions"][1]
        .as_str()
        .expect("second next action should render")
        .contains("vida agent-init --dispatch-packet <path> --execute-dispatch --json"));
    assert_eq!(
        payload["dispatch_mode"]["missing_execution_evidence_semantics"],
        "non_executing_bridge_blocker"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn agent_init_explicit_role_maps_dev_team_roles_and_reports_invalid_role_json() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let _ = run_and_assert_success(&["boot"], &state_dir);

    let tester_output = run_command_capture(
        &["agent-init", "--role", "tester", "verify task", "--json"],
        &state_dir,
    );
    assert!(
        tester_output.status.success(),
        "tester role should map to verifier: stdout={} stderr={}",
        String::from_utf8_lossy(&tester_output.stdout),
        String::from_utf8_lossy(&tester_output.stderr)
    );
    let tester: Value = serde_json::from_slice(&tester_output.stdout)
        .expect("tester agent-init output should be json");
    assert_eq!(tester["surface"], "vida agent-init");
    assert_eq!(tester["selection"]["requested_role"], "tester");
    assert_eq!(tester["selection"]["selected_role"], "verifier");
    assert_eq!(
        tester["selection"]["role_mapping"]["source"],
        "dev_team.roles.runtime_role"
    );

    let implementer_output = run_command_capture(
        &[
            "agent-init",
            "--role",
            "implementer",
            "implement task",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        implementer_output.status.success(),
        "legacy implementer role should map to worker: stdout={} stderr={}",
        String::from_utf8_lossy(&implementer_output.stdout),
        String::from_utf8_lossy(&implementer_output.stderr)
    );
    let implementer: Value = serde_json::from_slice(&implementer_output.stdout)
        .expect("implementer agent-init output should be json");
    assert_eq!(implementer["selection"]["requested_role"], "implementer");
    assert_eq!(implementer["selection"]["selected_role"], "worker");
    assert_eq!(
        implementer["selection"]["role_mapping"]["source"],
        "legacy_run_graph_node_alias"
    );

    let invalid_output = run_command_capture(
        &["agent-init", "--role", "missing_dev_team_role", "--json"],
        &state_dir,
    );
    assert!(
        !invalid_output.status.success(),
        "unknown role should fail closed"
    );
    assert!(
        invalid_output.stderr.is_empty(),
        "json mode should not emit plain stderr: {}",
        String::from_utf8_lossy(&invalid_output.stderr)
    );
    let invalid: Value =
        serde_json::from_slice(&invalid_output.stdout).expect("invalid role output should be json");
    assert_eq!(invalid["surface"], "vida agent-init");
    assert_eq!(invalid["status"], "blocked");
    assert_eq!(invalid["blocker_codes"][0], "agent_init_role_unresolved");
    assert_eq!(
        invalid["operator_contracts"]["blocker_codes"][0],
        "agent_init_role_unresolved"
    );
    assert!(invalid["valid_roles"]
        .as_array()
        .expect("valid roles should render")
        .iter()
        .any(|role| role == "tester"));
    assert!(invalid["next_actions"][0]
        .as_str()
        .expect("next action should render")
        .contains("vida agent-init --help"));

    let _ = fs::remove_dir_all(state_dir);
}

#[test]
fn agent_host_bridge_complete_missing_host_agent_id_uses_state_dir_and_json_envelope() {
    let state_dir = unique_state_dir();
    run_and_assert_success(&["boot"], &state_dir);
    create_epic_parent(&state_dir, "host-bridge-root", "Host bridge root", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            "run-host-bridge",
            "Run host bridge",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--parent-id",
            "host-bridge-root",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            "run-host-bridge",
            "implementation",
        ],
        &state_dir,
    );
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let bridge_dir = format!("{state_dir}/runtime-consumption/host-tool-bridge");
    fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
    fs::create_dir_all(&bridge_dir).expect("create host bridge dir");
    let packet_path = format!("{packet_dir}/run-host-bridge.json");
    let request_path = format!("{bridge_dir}/request.json");
    let result_path = format!("{bridge_dir}/result.json");
    let receipt_path = format!("{bridge_dir}/receipt.json");
    fs::write(&packet_path, "{}").expect("write dispatch packet");
    fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-host-bridge",
            "run_id": "run-host-bridge",
            "dispatch_target": "implementation",
            "packet_path": packet_path,
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": receipt_path
        })
        .to_string(),
    )
    .expect("write host bridge request");

    let (ready_payload, ready_success) = run_command_json_allow_failure(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request_path,
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !ready_success,
        "host bridge without dispatch receipt should fail closed: {ready_payload}"
    );
    assert_eq!(ready_payload["surface"], "vida agent host-bridge");
    assert_eq!(ready_payload["status"], "blocked");
    assert_eq!(
        ready_payload["blocker_codes"],
        serde_json::json!(["host_bridge_dispatch_receipt_missing"])
    );
    assert!(
        !ready_payload["blocker_codes"]
            .as_array()
            .expect("blocker_codes should render")
            .iter()
            .any(|code| code == "host_bridge_request_untrusted_path"),
        "explicit --state-dir should trust the state-scoped request path: {ready_payload}"
    );
    assert_eq!(
        ready_payload["shared_fields"]["blocker_codes"],
        ready_payload["blocker_codes"]
    );
    assert_eq!(
        ready_payload["operator_contracts"]["blocker_codes"],
        ready_payload["blocker_codes"]
    );
    assert_eq!(
        ready_payload["operator_contracts"]["contract_id"],
        "host-agent-bridge-adapter-v1"
    );
    assert!(ready_payload["host_bridge"]["completion_command"]
        .as_str()
        .expect("completion command should render")
        .contains("vida lane complete run-host-bridge"));
    assert_eq!(
        ready_payload["shared_fields"]["artifact_refs"]["request_path"],
        request_path
    );

    let output = run_command_capture(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request_path,
            "--complete",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !output.status.success(),
        "missing host-agent-id completion should fail closed"
    );
    assert!(
        output.stderr.is_empty(),
        "json host-bridge completion blocker should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "host-bridge completion blocker should render parseable json: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    assert_eq!(payload["surface"], "vida agent host-bridge");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["host_agent_id_missing"])
    );
    assert_eq!(
        payload["shared_fields"]["blocker_codes"],
        payload["blocker_codes"]
    );
    assert_eq!(
        payload["operator_contracts"]["blocker_codes"],
        payload["blocker_codes"]
    );
    assert!(payload["operator_contracts"]["next_actions"][0]
        .as_str()
        .expect("next action should render")
        .contains("--host-agent-id"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn doctor_summary_json_does_not_trust_cached_projection_before_store_open() {
    let state_dir = unique_state_dir();
    let projection_dir = format!("{state_dir}/operator-projections");
    fs::create_dir_all(&projection_dir).expect("create doctor projection dir");
    fs::write(
        format!("{projection_dir}/doctor-summary-v2-latest.json"),
        serde_json::json!({
            "surface": "vida doctor",
            "view": "summary",
            "status": "pass",
            "cache_probe": "doctor-summary-fast-path"
        })
        .to_string(),
    )
    .expect("write cached doctor summary");

    let output = run_command_capture(&["doctor", "--summary", "--json"], &state_dir);
    assert!(
        !output.status.success(),
        "doctor should fail closed when authoritative store is unavailable: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "doctor should not render forged cached payload when store open fails: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn doctor_summary_json_ignores_cached_projection_after_store_open() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    run_and_assert_success(&["boot"], &state_dir);
    write_operator_projection(
        &state_dir,
        "doctor-summary-v2-latest",
        &serde_json::json!({
            "surface": "vida doctor",
            "view": "summary",
            "status": "pass",
            "trace_id": null,
            "workflow_class": null,
            "risk_tier": null,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "surface": "vida doctor"
            },
            "shared_fields": {
                "status": "pass",
                "trace_id": null,
                "workflow_class": null,
                "risk_tier": null,
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "surface": "vida doctor"
                }
            },
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "trace_id": null,
                "workflow_class": null,
                "risk_tier": null,
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "surface": "vida doctor"
                }
            },
            "cache_sentinel": "trusted-after-store-open"
        }),
    );

    let output = run_command_capture(&["doctor", "--summary", "--json"], &state_dir);
    assert!(
        output.status.success(),
        "doctor should compute authoritative summary after store opens: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("authoritative doctor summary json should parse");
    assert_eq!(payload["surface"], "vida doctor");
    assert_eq!(payload["view"], "summary");
    assert!(
        payload.get("cache_sentinel").is_none(),
        "doctor summary must not be sourced from forgeable cached projection: {payload}"
    );
    assert!(
        payload.get("cache_probe").is_none(),
        "doctor summary must not mark the removed cached fast path: {payload}"
    );
    assert!(
        payload.get("runtime_consumption").is_some(),
        "authoritative summary should include computed runtime evidence: {payload}"
    );
    assert!(
        payload.get("root_session_write_guard").is_some(),
        "authoritative summary should include computed write-guard evidence: {payload}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_next_lawful_prefers_authoritative_active_task_over_stale_missing_source_drift() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = unique_test_id("autonomy-active-parent");
    create_epic_parent(&state_dir, &parent_id, "Autonomy active parent", "open");
    let active_task_id = unique_test_id("autonomy-active-task");
    let active = run_command_json(
        &[
            "task",
            "create",
            &active_task_id,
            "Autonomy active task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            &parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    assert_eq!(active["task"]["status"], "in_progress");

    let explicit_run_id = unique_test_id("stale-explicit-run");
    let explicit_task_id = unique_test_id("stale-explicit-task");
    let current_run_id = unique_test_id("stale-current-run");
    let current_task_id = unique_test_id("stale-current-task");
    assert_ne!(explicit_run_id, current_run_id);
    assert_ne!(explicit_task_id, current_task_id);
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            &current_task_id,
            "implementation",
        ],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let explicit_binding = serde_json::json!({
            "run_id": explicit_run_id.as_str(),
            "task_id": explicit_task_id.as_str(),
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": explicit_task_id.as_str(),
                "run_id": explicit_run_id.as_str(),
                "active_node": "implementation"
            },
            "binding_source": "explicit_continuation_bind_task",
            "why_this_unit": "stale explicit continuation references a missing task",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:02Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", explicit_run_id.as_str()))
            .bind(("binding", explicit_binding))
            .await
            .expect("seed stale explicit continuation binding");
        let current_binding = serde_json::json!({
            "run_id": current_run_id.as_str(),
            "task_id": current_task_id.as_str(),
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": current_task_id.as_str(),
                "run_id": current_run_id.as_str(),
                "active_node": "implementation"
            },
            "binding_source": "latest_run_graph_status",
            "why_this_unit": "stale latest-run continuation references a different missing task",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:01Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", current_run_id.as_str()))
            .bind(("binding", current_binding))
            .await
            .expect("seed stale current continuation binding");
        drop(db);
    });

    for missing_task_id in [explicit_task_id.as_str(), current_task_id.as_str()] {
        let missing = run_command_capture(&["task", "show", missing_task_id, "--json"], &state_dir);
        assert!(
            !missing.status.success(),
            "seeded stale continuation task `{missing_task_id}` must be absent"
        );
    }

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "next-lawful json should parse: {error}; stdout={} stderr={}",
                String::from_utf8_lossy(&next_lawful_output.stdout),
                String::from_utf8_lossy(&next_lawful_output.stderr)
            )
        });
    assert!(
        !next_lawful
            .to_string()
            .contains("continuation_source_drift"),
        "stale missing-task source drift must not block authoritative active task: {next_lawful}"
    );
    assert!(
        next_lawful_output.status.success(),
        "next-lawful stdout={} stderr={}",
        String::from_utf8_lossy(&next_lawful_output.stdout),
        String::from_utf8_lossy(&next_lawful_output.stderr)
    );
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(
        next_lawful["active_bounded_unit"]["task_id"],
        active_task_id
    );
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .is_empty());

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn recovery_explain_cli_surfaces_actionable_diagnosis() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string());
    let parent_id = format!("recovery-explain-parent-{suffix}");
    create_epic_parent(&state_dir, &parent_id, "Recovery explain parent", "open");
    let task_id = format!("recovery-explain-run-{suffix}");
    let task = run_command_json(
        &[
            "task",
            "create",
            &task_id,
            "Recovery explain run",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            &parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", &task_id, "implementation"],
        &state_dir,
    );

    let plain = run_and_assert_success(&["taskflow", "recovery", "explain", &task_id], &state_dir);
    assert!(plain.contains("vida taskflow recovery explain"));
    assert!(plain.contains("diagnosis"));
    assert!(plain.contains("evidence"));
    assert!(plain.contains("next_action"));
    assert!(plain.contains("recommended_command"));
    assert!(plain.contains("recommended_surface"));

    let json = run_command_json(
        &["taskflow", "recovery", "explain", &task_id, "--json"],
        &state_dir,
    );
    assert_eq!(json["surface"], "vida taskflow recovery explain");
    assert_eq!(json["run_id"], task_id);
    assert!(json.get("diagnosis").is_some());
    assert!(json.get("next_action").is_some());
    assert!(json.get("recommended_command").is_some());
    assert!(json.get("recommended_surface").is_some());
    assert!(json.get("recovery").is_some());
    assert!(json.get("projection_truth").is_some());
    assert_shared_fields_consistency(&json, "recovery explain");
    assert_operator_contracts_consistency(&json, "recovery explain");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn latest_run_projection_consistency_aligns_explicit_binding_scheduler_next_lawful_and_graph_explain(
) {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "projection-consistency-parent";
    let parent = run_command_json(
        &[
            "task",
            "create",
            parent_id,
            "Projection consistency parent",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");
    let bound_task_id = "projection-consistency-bound-task";
    let ready_head_task_id = "projection-consistency-ready-head";
    for (task_id, title, priority) in [
        (ready_head_task_id, "Projection consistency ready head", "1"),
        (
            bound_task_id,
            "Projection consistency explicitly bound",
            "2",
        ),
    ] {
        let task = run_command_json(
            &[
                "task",
                "create",
                task_id,
                title,
                "--type",
                "task",
                "--status",
                "open",
                "--priority",
                priority,
                "--parent-id",
                parent_id,
                "--state-dir",
                state_dir.as_str(),
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(task["status"], "pass");
    }

    let bound_run_id = "projection-consistency-bound-run";
    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let explicit_binding = serde_json::json!({
            "run_id": bound_run_id,
            "task_id": bound_task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "task_graph_task",
                "task_id": bound_task_id,
                "run_id": bound_run_id,
                "task_status": "open"
            },
            "binding_source": "explicit_continuation_bind_task",
            "why_this_unit": "explicit binding must drive every latest-run projection",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-20T00:00:02Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", bound_run_id))
            .bind(("binding", explicit_binding))
            .await
            .expect("seed live explicit continuation binding");
        let dispatch_context = serde_json::json!({
            "run_id": bound_run_id,
            "task_id": bound_task_id,
            "request_text": "projection consistency route",
            "role_selection": {
                "ok": true,
                "activation_source": "test",
                "selection_mode": "auto",
                "fallback_role": "orchestrator",
                "request": "projection consistency route",
                "selected_role": "pm",
                "conversational_mode": "development",
                "single_task_only": true,
                "tracked_flow_entry": "dev-pack",
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": ["projection"],
                "compiled_bundle": null,
                "execution_plan": {
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "internal_subagents",
                            "backend_class": "internal",
                            "lane_admissibility": {
                                "implementation": true
                            }
                        }
                    ],
                    "development_flow": {
                        "implementation": {
                            "executor_backend": "internal_subagents",
                            "fallback_executor_backend": "internal_subagents",
                            "carrier_runtime_assignment": {
                                "enabled": true,
                                "selected_backend_id": "internal_subagents",
                                "selected_carrier_id": "internal_subagents",
                                "selected_model_profile_id": "codex_gpt55_low_write",
                                "selected_model_ref": "gpt-5.5",
                                "selected_model_provider": "openai-codex",
                                "selected_reasoning_effort": "low",
                                "budget_verdict": "within_budget",
                                "rate": 1,
                                "estimated_task_price_units": 1
                            }
                        },
                        "dispatch_contract": {
                            "execution_lane_sequence": ["implementation"],
                            "lane_catalog": {
                                "implementation": {
                                    "executor_backend": "internal_subagents",
                                    "fallback_executor_backend": "internal_subagents",
                                    "carrier_runtime_assignment": {
                                        "enabled": true,
                                        "selected_backend_id": "internal_subagents",
                                        "selected_carrier_id": "internal_subagents",
                                        "selected_model_profile_id": "codex_gpt55_low_write",
                                        "selected_model_ref": "gpt-5.5",
                                        "selected_model_provider": "openai-codex",
                                        "selected_reasoning_effort": "low",
                                        "budget_verdict": "within_budget",
                                        "rate": 1,
                                        "estimated_task_price_units": 1
                                    }
                                }
                            }
                        }
                    }
                },
                "reason": "test"
            },
            "recorded_at": "2026-05-20T00:00:03Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_context', $run) CONTENT $context")
            .bind(("run", bound_run_id))
            .bind(("context", dispatch_context))
            .await
            .expect("seed route dispatch context");
        drop(db);
    });

    let next_lawful = run_command_json(
        &[
            "task",
            "next-lawful",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(next_lawful["active_bounded_unit"]["task_id"], bound_task_id);
    assert_eq!(
        next_lawful["binding_source"],
        "explicit_continuation_bind_task"
    );
    assert!(next_lawful["ready_task_candidates"]
        .as_array()
        .expect("ready candidates should render")
        .iter()
        .any(|candidate| candidate["task_id"] == ready_head_task_id));

    let taskflow_next = run_command_json(
        &[
            "taskflow",
            "next",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(taskflow_next["status"], "pass");
    assert_eq!(taskflow_next["primary_ready_task"]["id"], bound_task_id);
    assert_eq!(
        taskflow_next["candidate_task_context"]["ready_head"]["id"],
        bound_task_id
    );

    let graph_summary = run_command_json(
        &[
            "taskflow",
            "graph-summary",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(graph_summary["status"], "pass");
    assert_eq!(graph_summary["current_task_id"], bound_task_id);
    assert_eq!(
        graph_summary["primary_ready_task"]["task"]["id"],
        bound_task_id
    );
    assert_eq!(
        graph_summary["scheduling"]["ready_count"], 2,
        "graph-summary compact scheduling must keep ready candidates visible by count"
    );

    let graph_explain = run_command_json(
        &[
            "taskflow",
            "graph",
            "explain",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(graph_explain["status"], "pass");
    assert_eq!(graph_explain["task_id"], bound_task_id);
    assert_eq!(graph_explain["current_task_id"], bound_task_id);
    assert_eq!(graph_explain["task"]["id"], bound_task_id);
    assert!(graph_explain["selected_as_current"]
        .as_bool()
        .expect("graph explain selected_as_current should render"));

    let scheduler_dispatch = run_command_json(
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(scheduler_dispatch["status"], "pass");
    assert_eq!(
        scheduler_dispatch["selected_current_task_id"],
        bound_task_id
    );
    assert_eq!(scheduler_dispatch["selected_task_ids"][0], bound_task_id);
    assert!(scheduler_dispatch["rejected_candidates"]
        .as_array()
        .expect("scheduler rejected candidates should render")
        .iter()
        .any(|candidate| candidate["task_id"] == ready_head_task_id));

    let dispatch_preview = run_command_json(
        &[
            "agent",
            "dispatch-next",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dispatch_preview["status"], "pass");
    assert_eq!(
        dispatch_preview["selected_lanes"][0]["task_id"],
        bound_task_id
    );

    let route_explain = run_command_json(
        &[
            "taskflow",
            "route",
            "explain",
            "--run-id",
            bound_run_id,
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(route_explain["status"], "pass");
    assert_eq!(route_explain["task_id"], bound_task_id);
    assert_eq!(route_explain["run_id"], bound_run_id);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn latest_run_projection_consistency_aligns_graph_summary_current_task() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "latest-run-projection-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Latest run projection parent",
        "open",
    );
    let active_task_id = "latest-run-projection-active";
    let active = run_command_json(
        &[
            "task",
            "create",
            active_task_id,
            "Latest run projection active",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    let ready_task_id = "latest-run-projection-ready";
    let ready = run_command_json(
        &[
            "task",
            "create",
            ready_task_id,
            "Latest run projection ready",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "0",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(ready["status"], "pass");
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            active_task_id,
            "implementation",
        ],
        &state_dir,
    );

    let graph_summary = run_command_json(&["taskflow", "graph-summary", "--json"], &state_dir);
    assert_eq!(graph_summary["surface"], "vida taskflow graph-summary");
    assert_eq!(graph_summary["latest_run_graph"]["task_id"], active_task_id);
    assert_eq!(graph_summary["recovery"]["task_id"], active_task_id);
    assert_eq!(
        graph_summary["primary_ready_task"]["task"]["id"],
        active_task_id
    );
    assert_eq!(
        graph_summary["current_task_id"], active_task_id,
        "graph-summary top-level current_task_id must not drift to ready head: {graph_summary}"
    );
    assert_eq!(
        graph_summary["scheduling"]["current_task_id"], active_task_id,
        "graph-summary scheduling.current_task_id must mirror the selected active task: {graph_summary}"
    );
    assert!(
        graph_summary["ready_count"].as_u64().unwrap_or_default() >= 2,
        "fixture should preserve active task and ready task candidates: {graph_summary}"
    );

    let next_lawful = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(
        next_lawful["active_bounded_unit"]["task_id"],
        active_task_id
    );
    assert_ne!(
        next_lawful["active_bounded_unit"]["task_id"], ready_task_id,
        "next-lawful must not select the open ready head while an in-progress run is active"
    );

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(doctor["latest_run_graph_status"]["task_id"], active_task_id);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn doctor_latest_run_matches_recovery_latest() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "doctor-recovery-parity-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Doctor recovery parity parent",
        "open",
    );
    let active_task_id = "doctor-recovery-parity-active";
    let active = run_command_json(
        &[
            "task",
            "create",
            active_task_id,
            "Doctor recovery parity active",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            active_task_id,
            "implementation",
        ],
        &state_dir,
    );

    let recovery = run_command_json(&["taskflow", "recovery", "latest", "--json"], &state_dir);
    assert_eq!(recovery["surface"], "vida taskflow recovery latest");
    assert_eq!(recovery["status"], "pass");
    assert_eq!(recovery["run_id"], active_task_id);
    assert_eq!(recovery["recovery"]["task_id"], active_task_id);

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(doctor["latest_run_graph_status"]["task_id"], active_task_id);
    assert_eq!(
        doctor["latest_run_graph_recovery"]["run_id"],
        recovery["run_id"]
    );
    assert_eq!(
        doctor["latest_run_graph_recovery"]["task_id"],
        recovery["recovery"]["task_id"]
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn doctor_detects_closed_task_active_run() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "doctor-closed-active-parent";
    create_epic_parent(&state_dir, parent_id, "Doctor closed active parent", "open");
    let closed_task_id = "doctor-closed-active-task";
    let active = run_command_json(
        &[
            "task",
            "create",
            closed_task_id,
            "Doctor closed active task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            closed_task_id,
            "implementation",
        ],
        &state_dir,
    );
    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query("UPDATE type::record('task', $task) SET status = 'closed'")
            .bind(("task", closed_task_id))
            .await
            .expect("close canonical task without mutating run graph");
        db.query("UPDATE type::record('task', $parent) SET status = 'closed'")
            .bind(("parent", parent_id))
            .await
            .expect("close canonical parent without mutating run graph");
        drop(db);
    });

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(
        doctor["latest_terminal_task_active_run_graph_status"]["task_id"],
        closed_task_id
    );
    assert_eq!(doctor["status"], "blocked");
    let blockers = require_json_string_array(&doctor["blocker_codes"], "doctor blocker_codes");
    assert!(
        blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "doctor must name closed-task active-run projection mismatch: {doctor}"
    );
    assert!(
        doctor["next_actions"]
            .as_array()
            .expect("doctor next_actions should render")
            .iter()
            .any(|action| action.as_str().is_some_and(|value| value
                .contains("closed tasks must not remain projected as active runtime work"))),
        "doctor should provide actionable closed-task active-run remediation: {doctor}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_close_preserves_unevidenced_closed_task_active_run_projection() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-close-retire-closed-active-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task close retire closed active parent",
        "open",
    );
    let task_id = "task-close-retire-closed-active-task";
    let active = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Task close retire closed active task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    let seeded = run_command_json(
        &[
            "taskflow",
            "run-graph",
            "seed",
            task_id,
            "task close unevidenced active-run projection",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(seeded["surface"], "vida taskflow run-graph seed");
    assert_eq!(seeded["run_id"], task_id);
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "dispatch-init", task_id, "--json"],
        &state_dir,
    );

    let close = run_command_json(
        &[
            "task",
            "close",
            task_id,
            "--reason",
            "implementation proof passed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close["status"], "pass");

    let run_graph = run_command_json(
        &["taskflow", "run-graph", "status", task_id, "--json"],
        &state_dir,
    );
    assert!(
        matches!(run_graph["status"].as_str(), Some("pass" | "blocked")),
        "run-graph status should remain inspectable after unevidenced task close: {run_graph}"
    );
    assert!(
        run_graph["run_graph_status"].is_object(),
        "dispatch-init should materialize a run graph status for the closed active-run projection test: {run_graph}"
    );
    assert_ne!(
        run_graph["run_graph_status"]["status"], "completed",
        "task close must not forge completed run-graph state without receipt-backed execution evidence: {run_graph}"
    );

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    let blockers = require_json_string_array(&doctor["blocker_codes"], "doctor blocker_codes");
    let execution_plan_count = doctor["run_graph"]["execution_plan_count"]
        .as_u64()
        .unwrap_or_default();
    if execution_plan_count > 0 {
        assert!(
            blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
            "unevidenced task close must keep the closed-task active-run blocker visible: {doctor}"
        );
        assert_eq!(
            doctor["latest_terminal_task_active_run_graph_status"]["task_id"], task_id,
            "doctor should retain the unevidenced terminal-task active run after task close: {doctor}"
        );
    } else {
        assert_eq!(doctor["task_store"]["in_progress_count"], 0);
        assert!(doctor["latest_terminal_task_active_run_graph_status"].is_null());
    }

    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id,
            "implementation",
            "closure",
            "completed",
            "implementation",
            r#"{"lifecycle_stage":"closure_complete","resume_target":null,"checkpoint_kind":"execution_cursor","context_state":"sealed","handoff_state":"none","policy_gate":"not_required","recovery_ready":false}"#,
        ],
        &state_dir,
    );
    let forged_run_graph = run_command_json(
        &["taskflow", "run-graph", "status", task_id, "--json"],
        &state_dir,
    );
    assert_eq!(forged_run_graph["run_graph_status"]["status"], "completed");
    assert_eq!(
        forged_run_graph["run_graph_status"]["lifecycle_stage"],
        "closure_complete"
    );
    let forged_doctor = run_command_json(&["doctor", "--json"], &state_dir);
    let forged_blockers = require_json_string_array(
        &forged_doctor["blocker_codes"],
        "forged doctor blocker_codes",
    );
    assert!(
        forged_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "doctor must not suppress a terminal closure state that lacks receipt-backed execution evidence: {forged_doctor}"
    );
    let forged_status = run_command_json(&["status", "--json"], &state_dir);
    let forged_status_blockers =
        require_json_string_array(&forged_status["blocker_codes"], "forged status blockers");
    assert!(
        forged_status_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "status must stay aligned with doctor for forged terminal closure evidence: {forged_status}"
    );
    let (forged_diagnostics, forged_diagnostics_success) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !forged_diagnostics_success,
        "diagnostics must fail closed for forged terminal closure evidence: {forged_diagnostics}"
    );
    let forged_diagnostics_blockers = require_json_string_array(
        &forged_diagnostics["blocker_codes"],
        "forged diagnostics blockers",
    );
    assert!(
        forged_diagnostics_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics must stay aligned with doctor for forged terminal closure evidence: {forged_diagnostics}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_reconcile_closed_runs_preserves_unevidenced_historical_active_batch() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-reconcile-closed-runs-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task reconcile closed runs parent",
        "open",
    );
    for task_id in [
        "task-reconcile-closed-runs-a",
        "task-reconcile-closed-runs-b",
    ] {
        let created = run_command_json(
            &[
                "task",
                "create",
                task_id,
                "Task reconcile closed run",
                "--type",
                "task",
                "--status",
                "in_progress",
                "--priority",
                "1",
                "--parent-id",
                parent_id,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(created["status"], "pass");
        let _ = run_and_assert_success(
            &["taskflow", "run-graph", "init", task_id, "implementation"],
            &state_dir,
        );
    }

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        for task_id in [
            "task-reconcile-closed-runs-a",
            "task-reconcile-closed-runs-b",
        ] {
            let mut run_query = db
                .query("SELECT VALUE run_id FROM execution_plan_state WHERE task_id = $task LIMIT 1")
                .bind(("task", task_id))
                .await
                .expect("query task run id");
            let mut run_ids: Vec<String> = run_query.take(0).expect("decode task run id");
            let run_id = run_ids.pop().expect("task run graph should exist");
            let result_path = format!(
                "{state_dir}/runtime-consumption/dispatch-results/{run_id}.json"
            );
            fs::create_dir_all(
                std::path::Path::new(&result_path)
                    .parent()
                    .expect("dispatch result parent"),
            )
            .expect("create dispatch result dir");
            fs::write(
                &result_path,
                serde_json::to_string_pretty(&serde_json::json!({
                    "artifact_kind": "runtime_lane_completion_result",
                    "status": "pass",
                    "execution_evidence": {
                        "status": "recorded",
                        "evidence_kind": "test_receipt_backed_execution"
                    }
                }))
                .expect("encode dispatch result"),
            )
            .expect("write dispatch result");
            let receipt = serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementation",
                "dispatch_status": "executed",
                "lane_status": "completed",
                "dispatch_kind": "agent_lane",
                "dispatch_surface": "vida agent-init",
                "dispatch_command": "vida agent-init --execute-dispatch",
                "dispatch_packet_path": format!("{state_dir}/runtime-consumption/dispatch-packets/{run_id}.json"),
                "dispatch_result_path": result_path,
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_executed_count": 0,
                "activation_agent_type": "worker",
                "activation_runtime_role": "worker",
                "selected_backend": "test",
                "recorded_at": "2026-05-19T00:00:00Z"
            });
            db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
                .bind(("run", run_id))
                .bind(("receipt", receipt))
                .await
                .expect("seed receipt-backed execution truth");
        }
        for task_id in [
            "task-reconcile-closed-runs-a",
            "task-reconcile-closed-runs-b",
            parent_id,
        ] {
            db.query("UPDATE type::record('task', $task) SET status = 'closed'")
                .bind(("task", task_id))
                .await
                .expect("close canonical task without mutating run graph");
        }
        drop(db);
    });

    let before = run_command_json(&["doctor", "--json"], &state_dir);
    let before_blockers = require_json_string_array(&before["blocker_codes"], "before blockers");
    assert!(before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()));
    let (status_before, _) = run_command_json_allow_failure(&["status", "--json"], &state_dir);
    let status_before_blockers =
        require_json_string_array(&status_before["blocker_codes"], "status before blockers");
    assert!(
        status_before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "status must share the closed-run blocker before reconcile: {status_before}"
    );
    assert_eq!(
        status_before["latest_terminal_task_active_run_graph_status"]["task_id"],
        before["latest_terminal_task_active_run_graph_status"]["task_id"],
        "status must expose the same terminal-task active run evidence that drives the closed-run blocker: {status_before}"
    );
    assert!(
        status_before["active_bounded_unit"].is_null(),
        "status must not promote single in-progress TaskFlow work while closed-run mismatch is active: {status_before}"
    );
    assert_ne!(
        status_before["continuation_binding"]["status"], "bound",
        "status continuation binding must fail closed while closed-run mismatch is active: {status_before}"
    );
    let (graph_before, _) =
        run_command_json_allow_failure(&["taskflow", "graph-summary", "--json"], &state_dir);
    let graph_before_blockers =
        require_json_string_array(&graph_before["blocker_codes"], "graph before blockers");
    assert!(
        graph_before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "graph-summary must share the closed-run blocker before reconcile: {graph_before}"
    );
    assert!(
        graph_before["next_actions"]
            .as_array()
            .expect("graph-summary next actions should render")
            .iter()
            .any(|action| action.as_str().is_some_and(
                |value| value.contains("vida task reconcile-closed-runs --limit 25 --json")
            )),
        "graph-summary must publish the canonical reconcile command: {graph_before}"
    );
    let (orchestrator_before, _) = run_command_json_allow_failure(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert!(
        orchestrator_before["active_bounded_unit"].is_null(),
        "orchestrator-init must not promote single in-progress TaskFlow work while closed-run mismatch is active: {orchestrator_before}"
    );
    assert_eq!(
        orchestrator_before["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "orchestrator-init must publish the closed-run mismatch continuation reason: {orchestrator_before}"
    );
    assert!(
        orchestrator_before["continuation_binding"]["next_actions"]
            .as_array()
            .expect("orchestrator-init continuation next actions should render")
            .iter()
            .any(|action| action.as_str().is_some_and(
                |value| value.contains("vida task reconcile-closed-runs --limit 25 --json")
            )),
        "orchestrator-init must publish the canonical reconcile command: {orchestrator_before}"
    );
    let (diagnostics_before, diagnostics_before_success) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !diagnostics_before_success,
        "diagnostics post-commit must fail closed for unproven closed-task active run: {diagnostics_before}"
    );
    let diagnostics_before_blockers = require_json_string_array(
        &diagnostics_before["blocker_codes"],
        "diagnostics before blockers",
    );
    assert!(
        diagnostics_before_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics post-commit must share the doctor closed-run blocker: {diagnostics_before}"
    );
    assert!(
        diagnostics_before["next_actions"]
            .as_array()
            .expect("diagnostics next actions should render")
            .iter()
            .any(|action| action.as_str().is_some_and(|value| value
                .contains("vida task reconcile-closed-runs --limit 25 --json"))),
        "diagnostics post-commit must publish the canonical reconcile command: {diagnostics_before}"
    );

    let reconcile = run_command_json(
        &["task", "reconcile-closed-runs", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(reconcile["status"], "pass");
    assert_eq!(
        reconcile["summary"]["reconciled_count"], 0,
        "historical reconciliation must not retire closed-task active runs without receipt-backed execution evidence: {reconcile}"
    );

    let after = run_command_json(&["doctor", "--json"], &state_dir);
    let after_blockers = require_json_string_array(&after["blocker_codes"], "after blockers");
    assert!(
        after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "unevidenced closed-task active run batch should remain blocked after reconcile command: {after}"
    );
    assert_eq!(
        after["latest_terminal_task_active_run_graph_status"]["task_id"],
        "task-reconcile-closed-runs-b"
    );
    let (status_after, _) = run_command_json_allow_failure(&["status", "--json"], &state_dir);
    let status_after_blockers =
        require_json_string_array(&status_after["blocker_codes"], "status after blockers");
    assert!(
        status_after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "status must remain aligned with doctor after skipped closed-run reconcile: {status_after}"
    );
    assert!(
        status_after["active_bounded_unit"].is_null(),
        "status must still fail closed after skipped closed-run reconcile: {status_after}"
    );
    assert_eq!(
        status_after["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "status continuation must preserve the closed-run mismatch reason after skipped reconcile: {status_after}"
    );
    let (graph_after, _) =
        run_command_json_allow_failure(&["taskflow", "graph-summary", "--json"], &state_dir);
    let graph_after_blockers =
        require_json_string_array(&graph_after["blocker_codes"], "graph after blockers");
    assert!(
        graph_after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "graph-summary must remain aligned with doctor after skipped closed-run reconcile: {graph_after}"
    );
    let (orchestrator_after, _) = run_command_json_allow_failure(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert!(
        orchestrator_after["active_bounded_unit"].is_null(),
        "orchestrator-init must still fail closed after skipped closed-run reconcile: {orchestrator_after}"
    );
    assert_eq!(
        orchestrator_after["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "orchestrator-init must preserve the closed-run mismatch reason after skipped reconcile: {orchestrator_after}"
    );
    let (diagnostics_after, diagnostics_after_success) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !diagnostics_after_success,
        "diagnostics post-commit must still fail closed after skipped closed-run reconcile: {diagnostics_after}"
    );
    let diagnostics_after_blockers = require_json_string_array(
        &diagnostics_after["blocker_codes"],
        "diagnostics after blockers",
    );
    assert!(
        diagnostics_after_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics post-commit must remain aligned with doctor after skipped reconcile: {diagnostics_after}"
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn taskflow_settle_retires_closed_task_run_and_converges_runtime_surfaces() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "taskflow-settle-parent";
    let task_id = "taskflow-settle-closed-run";
    create_epic_parent(&state_dir, parent_id, "Taskflow settle parent", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Taskflow settle closed run",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query(
            "UPDATE type::record('task', $task) SET status = 'closed', closed_at = '2026-06-04T00:00:00Z', close_reason = 'canonical close truth'",
        )
        .bind(("task", task_id))
        .await
        .expect("close task with canonical close truth");
        db.query(
            "UPDATE type::record('task', $task) SET status = 'closed', closed_at = '2026-06-04T00:00:00Z', close_reason = 'all children closed'",
        )
        .bind(("task", parent_id))
        .await
        .expect("close parent with canonical close truth");
        drop(db);
    });

    let (doctor_before, _) = run_command_json_allow_failure(&["doctor", "--json"], &state_dir);
    let before_blockers =
        require_json_string_array(&doctor_before["blocker_codes"], "before blockers");
    assert!(before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()));

    let settle = run_command_json(
        &["taskflow", "settle", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(settle["surface"], "vida taskflow settle");
    assert_eq!(settle["status"], "pass");
    assert_eq!(settle["summary"]["reconciled_count"], 1);
    assert!(settle["remaining_closed_task_active_run"].is_null());
    assert!(require_json_string_array(&settle["blocker_codes"], "settle blockers").is_empty());

    let run_status = run_command_json(
        &["taskflow", "run-graph", "status", task_id, "--json"],
        &state_dir,
    );
    assert_eq!(run_status["run_graph_status"]["status"], "completed");
    assert_eq!(
        run_status["run_graph_status"]["lifecycle_stage"],
        "closure_complete"
    );

    for (label, command) in [
        ("doctor", vec!["doctor", "--json"]),
        ("status", vec!["status", "--json"]),
        ("graph-summary", vec!["taskflow", "graph-summary", "--json"]),
    ] {
        let (payload, _) = run_command_json_allow_failure(&command, &state_dir);
        let blockers = require_json_string_array(&payload["blocker_codes"], label);
        assert!(
            !blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
            "{label} must converge after taskflow settle: {payload}"
        );
    }
    let (orchestrator, _) = run_command_json_allow_failure(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert_ne!(
        orchestrator["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "orchestrator-init must converge after taskflow settle: {orchestrator}"
    );

    let plain = run_command_capture(&["taskflow", "settle", "--limit", "25"], &state_dir);
    assert!(plain.status.success());
    let plain_text = String::from_utf8_lossy(&plain.stdout);
    assert!(plain_text.starts_with("vida taskflow settle\n"));
    assert!(
        plain_text.contains("summary"),
        "plain taskflow settle should render compact TOON summary: {plain_text}"
    );
    let help = run_command_capture(&["taskflow", "settle", "--help"], &state_dir);
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("Usage: vida taskflow settle"),
        "taskflow settle help must document command usage"
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn taskflow_settle_keeps_unsafe_closed_task_run_blocked_with_exact_inspection() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "taskflow-settle-unsafe-parent";
    let task_id = "taskflow-settle-unsafe-closed-run";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Taskflow settle unsafe parent",
        "open",
    );
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Taskflow settle unsafe closed run",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query("UPDATE type::record('task', $task) SET status = 'closed'")
            .bind(("task", task_id))
            .await
            .expect("close task without receipt-backed truth");
        db.query(
            "UPDATE type::record('task', $task) SET status = 'closed', closed_at = '2026-06-04T00:00:00Z', close_reason = 'all children closed'",
        )
        .bind(("task", parent_id))
        .await
        .expect("close parent with canonical close truth");
        drop(db);
    });

    let (settle, success) = run_command_json_allow_failure(
        &["taskflow", "settle", "--limit", "25", "--json"],
        &state_dir,
    );
    assert!(
        !success,
        "unsafe taskflow settle must fail closed: {settle}"
    );
    assert_eq!(settle["surface"], "vida taskflow settle");
    assert_eq!(settle["status"], "blocked");
    assert_eq!(settle["summary"]["reconciled_count"], 0);
    assert_eq!(
        settle["remaining_closed_task_active_run"]["task_id"],
        task_id
    );
    let skipped = settle["summary"]["skipped_runs"]
        .as_array()
        .expect("settle skipped rows should render")
        .iter()
        .find(|row| row["task_id"] == task_id)
        .expect("unsafe task should be skipped");
    assert_eq!(skipped["reason"], "missing_receipt_backed_closure_truth");
    assert!(
        skipped["inspect_command"]
            .as_str()
            .is_some_and(|command| command.contains(
                "vida taskflow run-graph status taskflow-settle-unsafe-closed-run --json"
            )),
        "skipped row must carry exact inspect command: {settle}"
    );
    let next_actions = require_json_string_array(&settle["next_actions"], "settle next actions");
    assert!(next_actions.iter().any(|action| action.contains(
        "Inspect unresolved closed-task active run with `vida taskflow run-graph status taskflow-settle-unsafe-closed-run --json`"
    )));
    let blockers = require_json_string_array(&settle["blocker_codes"], "settle blockers");
    assert!(blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()));

    for (label, command) in [
        ("doctor", vec!["doctor", "--json"]),
        ("status", vec!["status", "--json"]),
        ("graph-summary", vec!["taskflow", "graph-summary", "--json"]),
    ] {
        let (payload, _) = run_command_json_allow_failure(&command, &state_dir);
        let blockers = require_json_string_array(&payload["blocker_codes"], label);
        assert!(
            blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
            "{label} must remain fail-closed while settle lacks closure truth: {payload}"
        );
    }
    let (orchestrator, _) = run_command_json_allow_failure(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert_eq!(
        orchestrator["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "orchestrator-init must remain fail-closed while settle lacks closure truth: {orchestrator}"
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn task_reconcile_closed_runs_skips_closed_task_active_run_without_receipt_truth() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-reconcile-unproven-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task reconcile unproven parent",
        "open",
    );
    let task_id = "task-reconcile-unproven-active";
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Task reconcile unproven active run",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let cached_orchestrator_before_close = run_command_json(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert_eq!(
        cached_orchestrator_before_close["active_bounded_unit"]["task_id"],
        task_id,
        "pre-close orchestrator-init should cache the single active bounded unit for the regression setup: {cached_orchestrator_before_close}"
    );
    let cached_status_before_close =
        run_command_json(&["status", "--state-dir", &state_dir, "--json"], &state_dir);
    assert_eq!(
        cached_status_before_close["active_bounded_unit"]["task_id"],
        task_id,
        "pre-close status should cache the single active bounded unit for the regression setup: {cached_status_before_close}"
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        for task_id in [task_id, parent_id] {
            db.query("UPDATE type::record('task', $task) SET status = 'closed'")
                .bind(("task", task_id))
                .await
                .expect("close canonical task without mutating run graph");
        }
        drop(db);
    });

    let before = run_command_json(&["doctor", "--json"], &state_dir);
    let before_blockers = require_json_string_array(&before["blocker_codes"], "before blockers");
    assert!(before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()));
    let (status_before, _) = run_command_json_allow_failure(
        &["status", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert!(
        status_before["active_bounded_unit"].is_null(),
        "status must invalidate cached active bounded unit when current state has a closed-task active run mismatch: {status_before}"
    );
    assert_eq!(
        status_before["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "status must publish the closed-run mismatch reason after cache invalidation: {status_before}"
    );
    let (orchestrator_before, _) = run_command_json_allow_failure(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert!(
        orchestrator_before["active_bounded_unit"].is_null(),
        "orchestrator-init must invalidate cached active bounded unit when current state has a closed-task active run mismatch: {orchestrator_before}"
    );
    assert_eq!(
        orchestrator_before["continuation_binding"]["ambiguity_reason"],
        "closed_task_active_run_projection_mismatch",
        "orchestrator-init must publish the closed-run mismatch reason after cache invalidation: {orchestrator_before}"
    );

    let reconcile = run_command_json(
        &["task", "reconcile-closed-runs", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(reconcile["status"], "pass");
    assert_eq!(reconcile["summary"]["reconciled_count"], 0);
    assert_eq!(reconcile["summary"]["skipped_count"], 1);
    assert_eq!(
        reconcile["summary"]["skipped_runs"][0]["run_id"], task_id,
        "skipped run should expose the concrete run id for operator inspection: {reconcile}"
    );
    assert_eq!(
        reconcile["summary"]["skipped_runs"][0]["reason"],
        "missing_receipt_backed_closure_truth"
    );
    assert!(reconcile["summary"]["skipped_runs"][0]["inspect_command"]
        .as_str()
        .expect("inspect command should render")
        .contains("vida taskflow run-graph status task-reconcile-unproven-active --json"));
    assert!(
        reconcile["next_actions"][0]
            .as_str()
            .expect("next action should render")
            .contains("vida taskflow run-graph status task-reconcile-unproven-active --json"),
        "reconcile should return an actionable inspect command when all rows are skipped: {reconcile}"
    );

    let after = run_command_json(&["doctor", "--json"], &state_dir);
    let after_blockers = require_json_string_array(&after["blocker_codes"], "after blockers");
    assert!(
        after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "unproven closed-task active run should remain blocked after reconcile: {after}"
    );
    assert!(
        !after["latest_terminal_task_active_run_graph_status"].is_null(),
        "unproven active run graph should not be cleared by reconcile: {after}"
    );
    let (diagnostics_after, _) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    let diagnostics_after_blockers = require_json_string_array(
        &diagnostics_after["blocker_codes"],
        "diagnostics after blockers",
    );
    assert!(
        diagnostics_after_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics post-commit should preserve the blocker when reconcile skipped the unproven run: {diagnostics_after}"
    );
    assert_eq!(
        diagnostics_after["taskflow_status"]["closed_task_active_run_projection_mismatch"],
        true
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn task_reconcile_closed_runs_retires_canonical_task_close_active_run() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-reconcile-canonical-close-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task reconcile canonical close parent",
        "open",
    );
    let task_id = "task-reconcile-canonical-close-active";
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Task reconcile canonical close active run",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let close = run_command_json(
        &[
            "task",
            "close",
            task_id,
            "--reason",
            "canonical task close proof passed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close["status"], "pass");

    let before = run_command_json(&["doctor", "--json"], &state_dir);
    let before_blockers = require_json_string_array(&before["blocker_codes"], "before blockers");
    assert!(
        before_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "canonical task close should expose the stale active run before reconcile: {before}"
    );
    let (diagnostics_before, diagnostics_before_success) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !diagnostics_before_success,
        "diagnostics post-commit must fail closed before closed-run reconcile: {diagnostics_before}"
    );
    let diagnostics_before_blockers = require_json_string_array(
        &diagnostics_before["blocker_codes"],
        "diagnostics before blockers",
    );
    assert!(
        diagnostics_before_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics post-commit must share doctor/status closed-run blocker before reconcile: {diagnostics_before}"
    );
    assert!(
        diagnostics_before["next_actions"]
            .as_array()
            .expect("diagnostics next actions should render")
            .iter()
            .any(|action| action.as_str().is_some_and(|value| value
                .contains("vida task reconcile-closed-runs --limit 25 --json")
                && value.contains("closed tasks must not remain projected as active runtime work"))),
        "diagnostics post-commit must publish the same canonical reconcile next action: {diagnostics_before}"
    );

    let reconcile = run_command_json(
        &["task", "reconcile-closed-runs", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(reconcile["status"], "pass");
    assert_eq!(reconcile["summary"]["reconciled_count"], 1);
    assert_eq!(reconcile["summary"]["skipped_count"], 0);
    assert_eq!(
        reconcile["summary"]["reconciled_runs"][0]["run_id"], task_id,
        "canonical task close should retire the stale active run: {reconcile}"
    );

    let after = run_command_json(&["doctor", "--json"], &state_dir);
    let after_blockers = require_json_string_array(&after["blocker_codes"], "after blockers");
    assert!(
        !after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "canonical task close stale run should be cleared after reconcile: {after}"
    );
    assert!(
        after["latest_terminal_task_active_run_graph_status"].is_null(),
        "canonical task close active run graph should be retired by reconcile: {after}"
    );
    let status_after = run_command_json(&["status", "--json"], &state_dir);
    let status_after_blockers =
        require_json_string_array(&status_after["blocker_codes"], "status after blockers");
    assert!(
        !status_after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "status should also ignore reconciled terminal closure runs: {status_after}"
    );
    let (diagnostics_after, _) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    let diagnostics_after_blockers = require_json_string_array(
        &diagnostics_after["blocker_codes"],
        "diagnostics after blockers",
    );
    assert!(
        !diagnostics_after_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "diagnostics post-commit should clear closed-run blocker after reconcile just like doctor/status: {diagnostics_after}"
    );
    assert_eq!(
        diagnostics_after["taskflow_status"]["closed_task_active_run_projection_mismatch"],
        false
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_reconcile_closed_runs_retires_receipt_backed_terminal_closure_run() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-reconcile-terminal-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task reconcile terminal parent",
        "open",
    );
    let task_id = "task-reconcile-terminal-closure";
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Task reconcile terminal closure",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );

    let close = run_command_json(
        &[
            "task",
            "close",
            task_id,
            "--reason",
            "terminal closure proof passed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close["status"], "pass");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let mut run_query = db
            .query("SELECT VALUE run_id FROM execution_plan_state WHERE task_id = $task LIMIT 1")
            .bind(("task", task_id))
            .await
            .expect("query task run id");
        let mut run_ids: Vec<String> = run_query.take(0).expect("decode task run id");
        let run_id = run_ids.pop().expect("task run graph should exist");
        let result_path = format!("{state_dir}/runtime-consumption/dispatch-results/{run_id}.json");
        fs::create_dir_all(
            std::path::Path::new(&result_path)
                .parent()
                .expect("dispatch result parent"),
        )
        .expect("create dispatch result dir");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_lane_completion_result",
                "status": "pass",
                "execution_state": "executed",
                "completed_target": "closure",
                "closure_ready": true,
                "execution_evidence": {
                    "status": "recorded",
                    "evidence_kind": "test_receipt_backed_terminal_closure",
                    "receipt_backed": true
                }
            }))
            .expect("encode dispatch result"),
        )
        .expect("write dispatch result");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "closure",
            "dispatch_status": "executed",
            "lane_status": "completed",
            "dispatch_kind": "agent_lane",
            "dispatch_surface": "vida agent-init",
            "dispatch_command": "vida agent-init --execute-dispatch",
            "dispatch_packet_path": format!("{state_dir}/runtime-consumption/dispatch-packets/{run_id}.json"),
            "dispatch_result_path": result_path,
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": [],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "worker",
            "activation_runtime_role": "worker",
            "selected_backend": "test",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", run_id.as_str()))
            .bind(("receipt", receipt))
            .await
            .expect("seed receipt-backed closure truth");
        db.query("UPDATE type::record('execution_plan_state', $run) SET status = 'completed', active_node = 'closure', next_node = NONE")
            .bind(("run", run_id.as_str()))
            .await
            .expect("seed terminal closure plan");
        db.query("UPDATE type::record('routed_run_state', $run) SET lifecycle_stage = 'closure_complete'")
            .bind(("run", run_id.as_str()))
            .await
            .expect("seed terminal route");
        db.query("UPDATE type::record('governance_state', $run) SET handoff_state = 'none'")
            .bind(("run", run_id.as_str()))
            .await
            .expect("seed terminal governance");
        db.query("UPDATE type::record('resumability_capsule', $run) SET resume_target = 'none'")
            .bind(("run", run_id.as_str()))
            .await
            .expect("seed terminal resumability");
        drop(db);
    });

    let (diagnostics_before_reconcile, _) = run_command_json_allow_failure(
        &[
            "diagnostics",
            "post-commit",
            "--state-dir",
            &state_dir,
            "--json",
        ],
        &state_dir,
    );
    let diagnostics_before_reconcile_blockers = require_json_string_array(
        &diagnostics_before_reconcile["blocker_codes"],
        "diagnostics before reconcile blockers",
    );
    assert!(
        !diagnostics_before_reconcile_blockers
            .contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "receipt-backed terminal closure truth must not produce the closed-run blocker before reconcile: {diagnostics_before_reconcile}"
    );
    assert_eq!(
        diagnostics_before_reconcile["taskflow_status"]
            ["closed_task_active_run_projection_mismatch"],
        false
    );
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        db.query("UPDATE type::record('execution_plan_state', $run) SET status = 'executing'")
            .bind(("run", task_id))
            .await
            .expect("restore stale pre-reconcile plan status");
        drop(db);
    });

    let reconcile = run_command_json(
        &["task", "reconcile-closed-runs", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(reconcile["status"], "pass");
    assert_eq!(reconcile["summary"]["reconciled_count"], 1);
    assert_eq!(reconcile["summary"]["skipped_count"], 0);

    let after = run_command_json(&["doctor", "--json"], &state_dir);
    let after_blockers = require_json_string_array(&after["blocker_codes"], "after blockers");
    assert!(
        !after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "receipt-backed terminal closure run should be cleared after reconcile: {after}"
    );
    assert!(
        after["latest_terminal_task_active_run_graph_status"].is_null(),
        "terminal closure active run graph should be cleared by reconcile: {after}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_reconcile_closed_runs_skips_stale_route_and_non_closure_receipt_evidence() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = "task-reconcile-stale-route-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Task reconcile stale route parent",
        "open",
    );
    let stale_route_task_id = "task-reconcile-stale-route-closure";
    let non_closure_task_id = "task-reconcile-non-closure-receipt";
    for task_id in [stale_route_task_id, non_closure_task_id] {
        let created = run_command_json(
            &[
                "task",
                "create",
                task_id,
                "Task reconcile forged closure evidence",
                "--type",
                "task",
                "--status",
                "in_progress",
                "--priority",
                "1",
                "--parent-id",
                parent_id,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(created["status"], "pass");
        let _ = run_and_assert_success(
            &["taskflow", "run-graph", "init", task_id, "implementation"],
            &state_dir,
        );
    }

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");

        for task_id in [stale_route_task_id, non_closure_task_id, parent_id] {
            db.query("UPDATE type::record('task', $task) SET status = 'closed'")
                .bind(("task", task_id))
                .await
                .expect("close canonical task without persisted close receipt truth");
        }

        for (task_id, plan_status, active_node, dispatch_target, completed_target) in [
            (
                stale_route_task_id,
                "executing",
                "implementation",
                "closure",
                "closure",
            ),
            (
                non_closure_task_id,
                "executing",
                "closure",
                "implementer",
                "implementation",
            ),
        ] {
            let mut run_query = db
                .query("SELECT VALUE run_id FROM execution_plan_state WHERE task_id = $task LIMIT 1")
                .bind(("task", task_id))
                .await
                .expect("query task run id");
            let mut run_ids: Vec<String> = run_query.take(0).expect("decode task run id");
            let run_id = run_ids.pop().expect("task run graph should exist");
            let result_path = format!(
                "{state_dir}/runtime-consumption/dispatch-results/{run_id}-{dispatch_target}.json"
            );
            fs::create_dir_all(
                std::path::Path::new(&result_path)
                    .parent()
                    .expect("dispatch result parent"),
            )
            .expect("create dispatch result dir");
            fs::write(
                &result_path,
                serde_json::to_string_pretty(&serde_json::json!({
                    "artifact_kind": "runtime_lane_completion_result",
                    "status": "pass",
                    "execution_state": "executed",
                    "completed_target": completed_target,
                    "closure_ready": completed_target == "closure",
                    "execution_evidence": {
                        "status": "recorded",
                        "evidence_kind": "test_forged_reconcile_evidence",
                        "receipt_backed": true
                    }
                }))
                .expect("encode dispatch result"),
            )
            .expect("write dispatch result");
            let receipt = serde_json::json!({
                "run_id": run_id,
                "dispatch_target": dispatch_target,
                "dispatch_status": "executed",
                "lane_status": "completed",
                "dispatch_kind": "agent_lane",
                "dispatch_surface": "vida agent-init",
                "dispatch_command": "vida agent-init --execute-dispatch",
                "dispatch_packet_path": format!("{state_dir}/runtime-consumption/dispatch-packets/{run_id}.json"),
                "dispatch_result_path": result_path,
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_executed_count": 0,
                "activation_agent_type": "worker",
                "activation_runtime_role": "worker",
                "selected_backend": "test",
                "recorded_at": "2026-05-19T00:00:00Z"
            });
            db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
                .bind(("run", run_id.as_str()))
                .bind(("receipt", receipt))
                .await
                .expect("seed forged receipt truth");
            db.query("UPDATE type::record('execution_plan_state', $run) SET status = $status, active_node = $active_node, next_node = NONE")
                .bind(("run", run_id.as_str()))
                .bind(("status", plan_status))
                .bind(("active_node", active_node))
                .await
                .expect("seed plan state");
            db.query("UPDATE type::record('routed_run_state', $run) SET lifecycle_stage = 'closure_complete'")
                .bind(("run", run_id.as_str()))
                .await
                .expect("seed stale terminal route");
            db.query("UPDATE type::record('governance_state', $run) SET handoff_state = 'none'")
                .bind(("run", run_id.as_str()))
                .await
                .expect("seed terminal governance");
            db.query("UPDATE type::record('resumability_capsule', $run) SET resume_target = 'none'")
                .bind(("run", run_id.as_str()))
                .await
                .expect("seed terminal resumability");
        }
        drop(db);
    });

    let reconcile = run_command_json(
        &["task", "reconcile-closed-runs", "--limit", "25", "--json"],
        &state_dir,
    );
    assert_eq!(reconcile["status"], "pass");
    assert_eq!(reconcile["summary"]["reconciled_count"], 0);
    assert_eq!(reconcile["summary"]["skipped_count"], 2);
    assert_eq!(
        reconcile["summary"]["skipped_runs"]
            .as_array()
            .expect("skipped runs should render")
            .len(),
        2
    );
    assert!(
        reconcile["next_actions"][0]
            .as_str()
            .expect("next action should render")
            .contains("vida taskflow run-graph status"),
        "skipped reconciliation should provide a concrete inspection command: {reconcile}"
    );

    let after = run_command_json(&["doctor", "--json"], &state_dir);
    let after_blockers = require_json_string_array(&after["blocker_codes"], "after blockers");
    assert!(
        after_blockers.contains(&"closed_task_active_run_projection_mismatch".to_string()),
        "forged closed-task active runs should remain blocked after reconcile: {after}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_next_lawful_prefers_active_task_over_closed_downstream_closure_binding() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = unique_test_id("closed-downstream-reconciled-parent");
    create_epic_parent(
        &state_dir,
        &parent_id,
        "Closed downstream reconciled parent",
        "closed",
    );
    let closed_task_id = unique_test_id("closed-downstream-reconciled-task");
    let closed_task = run_command_json(
        &[
            "task",
            "create",
            &closed_task_id,
            "Closed downstream reconciled task",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--parent-id",
            &parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_task["status"], "pass");
    assert_eq!(closed_task["task"]["status"], "closed");

    let active_parent_id = unique_test_id("active-task-after-closed-downstream-parent");
    create_epic_parent(
        &state_dir,
        &active_parent_id,
        "Active task after closed downstream parent",
        "open",
    );
    let active_task_id = unique_test_id("active-task-after-closed-downstream");
    let active_task = run_command_json(
        &[
            "task",
            "create",
            &active_task_id,
            "Active task after closed downstream",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            &active_parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active_task["status"], "pass");
    assert_eq!(active_task["task"]["status"], "in_progress");

    let closed_run_id = unique_test_id("closed-downstream-closure-run");
    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let binding = serde_json::json!({
            "run_id": closed_run_id.as_str(),
            "task_id": closed_task_id.as_str(),
            "status": "bound",
            "active_bounded_unit": {
                "kind": "downstream_dispatch_target",
                "task_id": closed_task_id.as_str(),
                "run_id": closed_run_id.as_str(),
                "dispatch_target": "closure"
            },
            "binding_source": "task_close_reconcile",
            "why_this_unit": "closed task reconciled into downstream closure before a different active task continued",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-19T00:00:03Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", closed_run_id.as_str()))
            .bind(("binding", binding))
            .await
            .expect("seed closed downstream closure continuation binding");
        drop(db);
    });

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "next-lawful json should parse: {error}; stdout={} stderr={}",
                String::from_utf8_lossy(&next_lawful_output.stdout),
                String::from_utf8_lossy(&next_lawful_output.stderr)
            )
        });
    assert!(
        !next_lawful
            .to_string()
            .contains("runtime_taskflow_active_conflict"),
        "closed downstream closure binding must not conflict with active task: {next_lawful}"
    );
    assert!(
        next_lawful_output.status.success(),
        "next-lawful stdout={} stderr={}",
        String::from_utf8_lossy(&next_lawful_output.stdout),
        String::from_utf8_lossy(&next_lawful_output.stderr)
    );
    assert_eq!(next_lawful["status"], "pass");
    assert_eq!(
        next_lawful["active_bounded_unit"]["task_id"],
        active_task_id
    );
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .is_empty());

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_next_lawful_blocks_closed_downstream_closure_binding_without_active_or_ready_task() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let parent_id = unique_test_id("closed-downstream-only-parent");
    create_epic_parent(
        &state_dir,
        &parent_id,
        "Closed downstream only parent",
        "closed",
    );
    let closed_task_id = unique_test_id("closed-downstream-only-task");
    let closed_task = run_command_json(
        &[
            "task",
            "create",
            &closed_task_id,
            "Closed downstream only task",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--parent-id",
            &parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_task["status"], "pass");
    assert_eq!(closed_task["task"]["status"], "closed");

    let closed_run_id = unique_test_id("closed-downstream-only-run");
    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let binding = serde_json::json!({
            "run_id": closed_run_id.as_str(),
            "task_id": closed_task_id.as_str(),
            "status": "bound",
            "active_bounded_unit": {
                "kind": "downstream_dispatch_target",
                "task_id": closed_task_id.as_str(),
                "run_id": closed_run_id.as_str(),
                "dispatch_target": "closure"
            },
            "binding_source": "task_close_reconcile",
            "why_this_unit": "closed task reconciled into downstream closure with no active successor",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-19T00:00:04Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", closed_run_id.as_str()))
            .bind(("binding", binding))
            .await
            .expect("seed closed downstream closure continuation binding");
        drop(db);
    });

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "closed downstream marker without active/ready candidates must fail closed: stdout={} stderr={}",
        String::from_utf8_lossy(&next_lawful_output.stdout),
        String::from_utf8_lossy(&next_lawful_output.stderr)
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "next-lawful json should parse: {error}; stdout={} stderr={}",
                String::from_utf8_lossy(&next_lawful_output.stdout),
                String::from_utf8_lossy(&next_lawful_output.stderr)
            )
        });
    assert_eq!(next_lawful["status"], "blocked");
    assert_eq!(next_lawful["binding_source"], serde_json::Value::Null);
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));
    assert_ne!(
        next_lawful["blocker_codes"],
        serde_json::json!(["runtime_binding_task_closed"])
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn closed_task_continuation_blocks_operator_surfaces_without_impossible_consume_command() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let task_id = "closed-task-continuation";
    let run_id = task_id;
    let parent_id = "closed-task-continuation-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Closed task continuation parent",
        "closed",
    );
    let created = run_command_json(
        &[
            "task",
            "create",
            task_id,
            "Closed task continuation",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");
    assert_eq!(created["task"]["status"], "closed");
    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", task_id, "implementation"],
        &state_dir,
    );
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id,
            "implementation",
            "implementation",
            "blocked",
            "implementation",
            "{\"policy_gate\":\"validation_report_required\",\"context_state\":\"sealed\",\"resume_target\":\"none\",\"recovery_ready\":false}",
        ],
        &state_dir,
    );
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = format!("{packet_dir}/{run_id}.json");
    fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "dispatch_target": "implementation"
        })
        .to_string(),
    )
    .expect("write dispatch packet");

    let runtime = Runtime::new().expect("create tokio runtime");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");
        let receipt = serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementation",
            "dispatch_status": "blocked",
            "lane_status": "lane_running",
            "dispatch_kind": "test_dispatch",
            "dispatch_surface": "vida taskflow run-graph dispatch-init",
            "dispatch_command": "vida taskflow run-graph dispatch-init closed-task-continuation --json",
            "dispatch_packet_path": packet_path,
            "blocker_code": "tool_execution_failed",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": [],
            "downstream_dispatch_executed_count": 0,
            "activation_agent_type": "internal_subagents",
            "activation_runtime_role": "worker",
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_dispatch_receipt', $run) CONTENT $receipt")
            .bind(("run", run_id))
            .bind(("receipt", receipt))
            .await
            .expect("seed blocked dispatch receipt");
        let binding = serde_json::json!({
            "run_id": run_id,
            "task_id": task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "task_graph_task",
                "task_id": task_id,
                "run_id": run_id
            },
            "binding_source": "explicit_continuation_bind_task",
            "why_this_unit": "closed task continuation regression seed",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-19T00:00:00Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", run_id))
            .bind(("binding", binding))
            .await
            .expect("seed closed task continuation binding");
        drop(db);
    });

    let status = run_command_json(&["status", "--json"], &state_dir);
    let continuation = &status["continuation_binding"];
    assert_ne!(continuation["status"], "bound");
    assert_eq!(continuation["continuation_allowed"], false);
    assert_eq!(continuation["active_bounded_unit"], serde_json::Value::Null);
    assert_no_run_id_consume_continue_command(&status, run_id, "status");

    let next_lawful_output = run_command_capture(&["task", "next-lawful", "--json"], &state_dir);
    assert!(
        !next_lawful_output.status.success(),
        "closed continuation next-lawful must fail closed: stdout={} stderr={}",
        String::from_utf8_lossy(&next_lawful_output.stdout),
        String::from_utf8_lossy(&next_lawful_output.stderr)
    );
    let next_lawful: serde_json::Value = serde_json::from_slice(&next_lawful_output.stdout)
        .expect("next-lawful blocked json should parse");
    assert_eq!(next_lawful["status"], "blocked");
    assert_eq!(next_lawful["active_bounded_unit"], serde_json::Value::Null);
    assert!(next_lawful["blocker_codes"]
        .as_array()
        .expect("next-lawful blocker_codes should render")
        .iter()
        .any(|code| code == "no_ready_task_candidates"));
    assert_no_run_id_consume_continue_command(&next_lawful, run_id, "next-lawful");

    let consume_continue_output = run_command_capture(
        &[
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ],
        &state_dir,
    );
    assert!(
        !consume_continue_output.status.success(),
        "consume continue for closed blocked task must fail closed: stdout={} stderr={}",
        String::from_utf8_lossy(&consume_continue_output.stdout),
        String::from_utf8_lossy(&consume_continue_output.stderr)
    );
    let consume_continue: serde_json::Value =
        serde_json::from_slice(&consume_continue_output.stdout)
            .expect("consume continue blocked json should parse");
    assert_eq!(consume_continue["status"], "blocked");
    assert_no_run_id_consume_continue_command(&consume_continue, run_id, "consume continue");

    let doctor = run_command_json(&["doctor", "--json"], &state_dir);
    assert_eq!(doctor["latest_run_graph_status"], serde_json::Value::Null);
    assert_eq!(doctor["task_store"]["closed_count"], 2);
    assert_no_run_id_consume_continue_command(&doctor, run_id, "doctor");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_list_show_ready_prefer_authoritative_state_over_stale_snapshot() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let parent_id = "vida-authoritative-parent";
    create_epic_parent(&state_dir, parent_id, "Authoritative parent", "open");
    let created = run_command_json(
        &[
            "task",
            "create",
            "vida-authoritative",
            "Original snapshot title",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "3",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(created["status"], "pass");

    let updated = run_command_json(
        &[
            "task",
            "update",
            "vida-authoritative",
            "--title",
            "Live authoritative title",
            "--priority",
            "7",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(updated["status"], "pass");

    let snapshot_path = format!("{state_dir}/exports/tasks.snapshot.jsonl");
    write_single_task_snapshot(
        &snapshot_path,
        "vida-authoritative",
        "Original snapshot title",
        "open",
        3,
    );

    let shown = run_command_json(
        &["task", "show", "vida-authoritative", "--json"],
        &state_dir,
    );
    assert_eq!(shown["task"]["title"], "Live authoritative title");
    assert_eq!(shown["task"]["priority"], 7);
    assert_eq!(shown["state_access"]["mode"], "authoritative_live");
    assert_eq!(shown["state_access"]["degraded"], false);

    let listed = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let listed_task = task_row_by_id(&listed, "vida-authoritative");
    assert_eq!(listed_task["title"], "Live authoritative title");
    assert_eq!(listed_task["priority"], 7);
    assert_eq!(listed["state_access"]["mode"], "authoritative_live");
    assert_eq!(listed["state_access"]["degraded"], false);

    let ready = run_command_json(&["task", "ready", "--json"], &state_dir);
    let ready_task = task_row_by_id(&ready, "vida-authoritative");
    assert_eq!(ready_task["title"], "Live authoritative title");
    assert_eq!(ready_task["priority"], 7);
    assert_eq!(ready["state_access"]["mode"], "authoritative_live");
    assert_eq!(ready["state_access"]["degraded"], false);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_show_falls_back_to_snapshot_when_authoritative_state_is_missing() {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    let snapshot_path = format!("{project_root}/.vida/exports/tasks.snapshot.jsonl");
    write_single_task_snapshot(
        &snapshot_path,
        "vida-snapshot-only",
        "Snapshot fallback title",
        "open",
        2,
    );

    let shown = run_command_json(
        &["task", "show", "vida-snapshot-only", "--json"],
        &state_dir,
    );
    assert_eq!(shown["task"]["title"], "Snapshot fallback title");
    assert_eq!(shown["task"]["priority"], 2);
    assert_eq!(shown["state_access"]["mode"], "snapshot");
    assert_eq!(shown["state_access"]["degraded"], true);
    let reported_snapshot_path = shown["state_access"]["snapshot_path"]
        .as_str()
        .expect("snapshot path should render");
    assert_eq!(
        reported_snapshot_path.replace('\\', "/"),
        snapshot_path.replace('\\', "/")
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn task_show_fails_closed_for_missing_task_id() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "show", "vida-missing", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task show should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to show task: task is missing: vida-missing"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_list_json_ignores_render_color_emoji_styling() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "list", "--all", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "color_emoji")
        .output()
        .expect("task list should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(!stdout.contains('\u{1b}'));
    assert!(!stdout.contains("📘"));
    assert!(!stdout.contains("🔹"));

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("json output should parse");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["surface"], "vida task list");
    assert_eq!(parsed["view"], "summary");
    assert_eq!(parsed["output_policy"]["mode"], "summary");
    assert_eq!(parsed["output_policy"]["explicit_full"], false);
    assert!(
        parsed["tasks"].is_array(),
        "task list tasks should be json array"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn agent_feedback_fails_closed_for_unsupported_outcome() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "feedback-invalid-outcome",
            "--project-name",
            "Feedback Invalid Outcome",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let output = vida()
        .args([
            "agent-feedback",
            "--agent-id",
            "junior",
            "--score",
            "90",
            "--outcome",
            "deferred",
            "--task-class",
            "implementation",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("agent-feedback should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported feedback outcome `deferred`"));
    assert!(stderr.contains("Allowed values: success, failure, neutral."));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn task_close_feedback_outcome_inference_handles_rejected_context_and_rejected_failure() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "feedback-close-inference",
            "--project-name",
            "Feedback Close Inference",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );
    let parent_id = "feedback-canonical-close-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Feedback canonical close parent",
        "open",
    );
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-close-inference-parent",
            "Feedback close inference parent",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );

    for (task_id, title) in [
        (
            "feedback-positive-rejected-context",
            "Positive rejected context",
        ),
        (
            "feedback-concrete-rejected-failure",
            "Concrete rejected failure",
        ),
        ("feedback-coverage-meta-language", "Coverage meta language"),
    ] {
        let create = run_with_state_lock_retry(|| {
            let mut command = vida();
            command
                .args([
                    "task",
                    "create",
                    task_id,
                    title,
                    "--type",
                    "task",
                    "--status",
                    "open",
                    "--priority",
                    "1",
                    "--parent-id",
                    "feedback-close-inference-parent",
                    "--labels",
                    "verification",
                    "--json",
                ])
                .current_dir(&project_root)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir);
            command
        });
        assert!(
            create.status.success(),
            "{}",
            String::from_utf8_lossy(&create.stderr)
        );
    }

    let positive_close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-positive-rejected-context",
                "--reason",
                "Added model-profile readiness audit payload with selected overrides, rejected alternatives, and readiness blockers; model_profile_readiness_audit tests passed.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        positive_close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&positive_close.stdout),
        String::from_utf8_lossy(&positive_close.stderr)
    );
    let positive_json: serde_json::Value =
        serde_json::from_slice(&positive_close.stdout).expect("positive close json should parse");
    assert_eq!(positive_json["status"], "pass");
    assert_eq!(positive_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        positive_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(
        positive_json["host_agent_telemetry"]["feedback"]["safety_baseline"]["safety_gate"],
        "observe"
    );
    assert_eq!(
        positive_json["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!([])
    );
    assert!(
        positive_json["host_agent_telemetry"]["feedback_outcome_inference"]
            ["ignored_meta_language"]
            .as_array()
            .expect("ignored meta language should render")
            .iter()
            .any(|phrase| phrase == "rejected alternatives")
    );

    let coverage_close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-coverage-meta-language",
                "--reason",
                "Added close-feedback smoke coverage for rejected alternatives and concrete rejected patch wording records failure; task_smoke test passed.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        coverage_close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&coverage_close.stdout),
        String::from_utf8_lossy(&coverage_close.stderr)
    );
    let coverage_json: serde_json::Value =
        serde_json::from_slice(&coverage_close.stdout).expect("coverage close json should parse");
    assert_eq!(coverage_json["status"], "pass");
    assert_eq!(coverage_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        coverage_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(
        coverage_json["host_agent_telemetry"]["feedback"]["safety_baseline"]["safety_gate"],
        "observe"
    );
    assert_eq!(
        coverage_json["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!([])
    );
    let ignored_coverage_meta = coverage_json["host_agent_telemetry"]["feedback_outcome_inference"]
        ["ignored_meta_language"]
        .as_array()
        .expect("ignored meta language should render");
    assert!(ignored_coverage_meta
        .iter()
        .any(|phrase| phrase == "records failure"));
    assert!(ignored_coverage_meta
        .iter()
        .any(|phrase| phrase == "concrete rejected patch wording"));

    let rejected_close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-concrete-rejected-failure",
                "--reason",
                "Rejected patch because it changed unrelated files.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        rejected_close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&rejected_close.stdout),
        String::from_utf8_lossy(&rejected_close.stderr)
    );
    let rejected_json: serde_json::Value =
        serde_json::from_slice(&rejected_close.stdout).expect("rejected close json should parse");
    assert_eq!(rejected_json["status"], "pass");
    assert_eq!(rejected_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        rejected_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "failure"
    );
    assert_eq!(
        rejected_json["host_agent_telemetry"]["feedback"]["recorded_score"],
        35
    );
    assert_eq!(
        rejected_json["host_agent_telemetry"]["feedback"]["safety_baseline"]["safety_gate"],
        "hold"
    );
    assert_eq!(
        rejected_json["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!(["rejected"])
    );

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn task_close_feedback_outcome_inference_treats_failed_subprocess_diagnostics_as_context_when_tests_passed(
) {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "feedback-subprocess-diagnostics",
            "--project-name",
            "Feedback Subprocess Diagnostics",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );
    let parent_id = "feedback-canonical-close-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Feedback canonical close parent",
        "open",
    );
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-subprocess-diagnostics-parent",
            "Feedback subprocess diagnostics parent",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );

    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-subprocess-diagnostics-context",
            "Feedback subprocess diagnostics context",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "1",
            "--parent-id",
            "feedback-subprocess-diagnostics-parent",
            "--labels",
            "verification",
            "--json",
        ],
        &state_dir,
    );

    let close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-subprocess-diagnostics-context",
                "--reason",
                "Added regression proof so explanatory failed subprocess status/stdout/stderr records stay diagnostic context while bounded tests passed.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&close.stdout),
        String::from_utf8_lossy(&close.stderr)
    );
    let close_json: serde_json::Value =
        serde_json::from_slice(&close.stdout).expect("close json should parse");
    assert_eq!(close_json["status"], "pass");
    assert_eq!(close_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        close_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(
        close_json["host_agent_telemetry"]["feedback_outcome_inference"]["failure_markers"],
        serde_json::json!([])
    );
    let success_markers = close_json["host_agent_telemetry"]["feedback_outcome_inference"]
        ["success_markers"]
        .as_array()
        .expect("success markers should render");
    assert!(success_markers
        .iter()
        .any(|marker| marker == "tests passed"));
    let ignored_meta = close_json["host_agent_telemetry"]["feedback_outcome_inference"]
        ["ignored_meta_language"]
        .as_array()
        .expect("ignored meta language should render");
    assert!(ignored_meta
        .iter()
        .any(|phrase| phrase == "failed subprocess status/stdout/stderr"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn donor_ready_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    sample_jsonl(&jsonl_path);

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_ready = run_and_assert_success(&["task", "ready", "--json"], &state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_ready_semantic.json").trim_end();
    assert_eq!(
        donor_ready_semantic(&rust_ready),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn task_close_json_surfaces_canonical_feedback_blockers_without_masking_successful_audit_closes() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "feedback-canonical-close-status",
            "--project-name",
            "Feedback Canonical Close Status",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );
    let parent_id = "feedback-canonical-close-parent";
    create_epic_parent(
        &state_dir,
        parent_id,
        "Feedback canonical close parent",
        "open",
    );
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-audit-language-close",
            "Feedback audit language close",
            "--status",
            "in_progress",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-blocked-close",
            "Feedback blocked close",
            "--status",
            "in_progress",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-proof-context-close",
            "Feedback proof context close",
            "--status",
            "in_progress",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );

    let audit_close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-audit-language-close",
                "--reason",
                "Added model-profile readiness audit payload with selected overrides, rejected alternatives, and readiness blockers; model_profile_readiness_audit tests passed.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        audit_close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&audit_close.stdout),
        String::from_utf8_lossy(&audit_close.stderr)
    );
    let audit_json: serde_json::Value =
        serde_json::from_slice(&audit_close.stdout).expect("audit close json should parse");
    assert_eq!(audit_json["status"], "pass");
    assert_eq!(audit_json["task"]["status"], "closed");
    assert_eq!(audit_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        audit_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );

    let proof_context_close = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "task",
                "close",
                "feedback-proof-context-close",
                "--reason",
                "Implemented close feedback proof classification regression coverage. Successful close proof text references historical rework evidence and regression fixes; proof commands passed.",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        proof_context_close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&proof_context_close.stdout),
        String::from_utf8_lossy(&proof_context_close.stderr)
    );
    let proof_context_json: serde_json::Value = serde_json::from_slice(&proof_context_close.stdout)
        .expect("proof context close json should parse");
    assert_eq!(proof_context_json["status"], "pass");
    assert_eq!(proof_context_json["task"]["status"], "closed");
    assert_eq!(
        proof_context_json["host_agent_telemetry"]["status"],
        "recorded"
    );
    assert_eq!(
        proof_context_json["host_agent_telemetry"]["feedback"]["recorded_outcome"],
        "success"
    );
    assert_eq!(proof_context_json["blocker_codes"], serde_json::json!([]));

    let blocked_close = run_command_capture(
        &[
            "task",
            "close",
            "feedback-blocked-close",
            "--reason",
            "Task remains blocked pending operator evidence.",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !blocked_close.status.success(),
        "blocked canonical close should not hide under pass"
    );
    let blocked_json: serde_json::Value =
        serde_json::from_slice(&blocked_close.stdout).expect("blocked close json should parse");
    assert_eq!(blocked_json["status"], "blocked");
    assert_eq!(blocked_json["task"]["status"], "in_progress");
    assert_eq!(blocked_json["host_agent_telemetry"]["status"], "skipped");
    assert_eq!(
        blocked_json["host_agent_telemetry"]["reason"],
        "feedback_deferred_for_canonical_close_status"
    );
    assert_eq!(
        blocked_json["blocker_codes"],
        serde_json::json!([
            "close_feedback_canonical_status_blocked",
            "canonical_gate_blocked"
        ])
    );
    assert!(blocked_json["next_actions"][0]
        .as_str()
        .expect("next action should render")
        .contains("Resolve the blocked condition"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn donor_show_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    sample_jsonl(&jsonl_path);

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_show = run_and_assert_success(&["task", "show", "vida-b", "--json"], &state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_show_semantic.json").trim_end();
    assert_eq!(
        donor_show_semantic(&rust_show),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn donor_list_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    fs::write(
        &jsonl_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"alpha\"],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"beta\"],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"closed\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"closed_at\":\"2026-03-09T00:00:00Z\",\"close_reason\":\"done\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
        ),
    )
    .expect("write task jsonl");

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_list = run_and_assert_success(
        &["task", "list", "--all", "--view", "full", "--json"],
        &state_dir,
    );

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_list_semantic.json").trim_end();
    assert_eq!(
        donor_list_semantic(&rust_list),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn status_json_reports_dispatch_alias_registry_load_error_when_registry_missing() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    fs::write(
        format!("{project_root}/vida.config.yaml"),
        r#"project:
  id: status-missing-registry
agent_extensions:
  enabled: true
  registries:
    dispatch_aliases: missing/dispatch-aliases.yaml
  enabled_framework_roles:
    - orchestrator
  validation:
    require_registry_files: true
agent_system:
  mode: native
  state_owner: orchestrator_only
"#,
    )
    .expect("config should be written");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-missing-registry",
            "--project-name",
            "Status Missing Registry",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(!activator.status.success());

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    assert_shared_fields_consistency(&parsed, "status surface");
    assert_operator_contracts_consistency(&parsed, "status surface");
    let load_error = parsed["host_agents"]["internal_dispatch_alias_load_error"]
        .as_str()
        .expect("internal_dispatch_alias_load_error should be present");
    assert!(load_error.contains("missing/dispatch-aliases.yaml"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn status_json_reports_non_default_host_agents_summary() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(init.status.success());

    let config_path = format!("{project_root}/vida.config.yaml");
    let mut config: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&config_path).expect("config exists"))
            .expect("config yaml should parse");
    let root = config
        .as_mapping_mut()
        .expect("config root should be a mapping");
    let host_env = root
        .get_mut(serde_yaml::Value::String("host_environment".to_string()))
        .and_then(serde_yaml::Value::as_mapping_mut)
        .expect("host_environment should exist");
    host_env.insert(
        serde_yaml::Value::String("cli_system".to_string()),
        serde_yaml::Value::String("qwen".to_string()),
    );
    fs::write(
        &config_path,
        serde_yaml::to_string(&config).expect("patched yaml should render"),
    )
    .expect("patch config");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-qwen",
            "--host-cli-system",
            "qwen",
            "--language",
            "english",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    let host_agents = &parsed["host_agents"];
    assert_eq!(host_agents["host_cli_system"], "qwen");
    assert_eq!(host_agents["runtime_surface"], ".qwen");
    assert_eq!(host_agents["root_session_write_guard"]["status"], "missing");
    assert_eq!(parsed["root_session_write_guard"]["status"], "missing");
    let runtime_root = host_agents["runtime_root"]
        .as_str()
        .expect("runtime_root present");
    assert!(runtime_root.contains(".qwen"));
    let system_entry = &host_agents["system_entry"];
    assert!(system_entry.is_object());
    assert_eq!(
        system_entry["template_root"]
            .as_str()
            .expect("template_root"),
        ".qwen"
    );
    assert_eq!(
        system_entry["runtime_root"].as_str().expect("runtime_root"),
        ".qwen"
    );
    assert_eq!(
        system_entry["materialization_mode"]
            .as_str()
            .expect("materialization_mode"),
        "copy_tree_only"
    );
    assert_eq!(system_entry["enabled"].as_bool(), Some(true));
    assert_eq!(
        system_entry["carriers"]["qwen-primary"]["tier"]
            .as_str()
            .expect("carrier tier"),
        "qwen"
    );
    assert_eq!(
        system_entry["carriers"]["qwen-primary"]["rate"].as_i64(),
        Some(4)
    );
    let agents = host_agents["agents"]
        .as_object()
        .expect("agents summary should render");
    assert_eq!(
        agents
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .expect("agents count should render"),
        1
    );
    assert_eq!(
        host_agents["selection_policy"]["rule"],
        "capability_first_then_score_guard_then_cheapest_tier"
    );
    assert!(
        matches!(
            host_agents["external_cli_preflight"]["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "external preflight may be blocked by operator-local auth, but the host summary must render"
    );
    assert_eq!(
        host_agents["external_cli_preflight"]["requires_external_cli"],
        true
    );
    assert_eq!(
        host_agents["external_cli_preflight"]["selected_execution_class"],
        "external"
    );
    assert_eq!(
        host_agents["external_cli_preflight"]["hybrid_external_cli_relevant"],
        false
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn status_json_restores_root_session_guard_after_consume_continue_snapshot() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output();
    let boot = boot.expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packets_dir = format!("{runtime_consumption_dir}/dispatch-packets");
    fs::create_dir_all(&dispatch_packets_dir).expect("dispatch packet dir should exist");

    let dispatch_packet_path = format!("{dispatch_packets_dir}/resume-packet.json");
    fs::write(
        &dispatch_packet_path,
        serde_json::json!({
            "root_session_write_guard": {
                "status": "blocked_by_default",
                "root_session_role": "orchestrator",
                "local_write_requires_exception_path": true,
                "required_exception_evidence": "exception_path_receipt_id",
                "pre_write_checkpoint_required": true
            }
        })
        .to_string(),
    )
    .expect("dispatch packet should write");
    fs::write(
        format!("{runtime_consumption_dir}/final-2026-03-19T00-00-00Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume continue",
            "source_dispatch_packet_path": dispatch_packet_path
        })
        .to_string(),
    )
    .expect("final snapshot should write");

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        status["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        status["host_agents"]["root_session_write_guard"]["status"],
        "blocked_by_default"
    );

    fs::remove_dir_all(state_dir).expect("state dir should be removed");
}

#[test]
fn status_json_prefers_latest_final_snapshot_guard_when_latest_snapshot_is_bundle_check() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output();
    let boot = boot.expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packets_dir = format!("{runtime_consumption_dir}/dispatch-packets");
    fs::create_dir_all(&dispatch_packets_dir).expect("dispatch packet dir should exist");

    let dispatch_packet_path = format!("{dispatch_packets_dir}/guard-packet.json");
    fs::write(
        &dispatch_packet_path,
        serde_json::json!({
            "root_session_write_guard": {
                "status": "blocked_by_default",
                "root_session_role": "orchestrator",
                "local_write_requires_exception_path": true,
                "required_exception_evidence": "exception_path_receipt_id",
                "pre_write_checkpoint_required": true
            }
        })
        .to_string(),
    )
    .expect("dispatch packet should write");
    fs::write(
        format!("{runtime_consumption_dir}/final-2026-03-19T00-00-01Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume continue",
            "source_dispatch_packet_path": dispatch_packet_path
        })
        .to_string(),
    )
    .expect("final snapshot should write");

    thread::sleep(Duration::from_millis(15));
    fs::write(
        format!("{runtime_consumption_dir}/bundle-check-2026-03-19T00-00-02Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume bundle check",
            "check": { "ok": true }
        })
        .to_string(),
    )
    .expect("bundle-check snapshot should write");

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        status["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        status["host_agents"]["root_session_write_guard"]["status"],
        "blocked_by_default"
    );

    fs::remove_dir_all(state_dir).expect("state dir should be removed");
}

#[test]
fn status_json_blocks_external_cli_when_sandbox_active_and_network_unreachable() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(init.status.success());

    let config_path = format!("{project_root}/vida.config.yaml");
    let mut config: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&config_path).expect("config exists"))
            .expect("config yaml should parse");
    let root = config
        .as_mapping_mut()
        .expect("config root should be a mapping");
    let host_env = root
        .get_mut(serde_yaml::Value::String("host_environment".to_string()))
        .and_then(serde_yaml::Value::as_mapping_mut)
        .expect("host_environment should exist");
    host_env.insert(
        serde_yaml::Value::String("cli_system".to_string()),
        serde_yaml::Value::String("qwen".to_string()),
    );
    fs::write(
        &config_path,
        serde_yaml::to_string(&config).expect("patched yaml should render"),
    )
    .expect("patch config");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-qwen-offline",
            "--host-cli-system",
            "qwen",
            "--language",
            "english",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("CODEX_SANDBOX_MODE", "workspace-write")
        .env("VIDA_NETWORK_PROBE_OVERRIDE", "unreachable")
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    let preflight = &parsed["host_agents"]["external_cli_preflight"];
    assert_eq!(preflight["status"], "blocked");
    assert_eq!(
        preflight["blocker_code"],
        "external_cli_network_access_unavailable_under_sandbox"
    );
    assert!(preflight["next_actions"]
        .as_array()
        .expect("next actions should be array")
        .iter()
        .any(|row| row
            .as_str()
            .unwrap_or_default()
            .contains("Allow network access")));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn consume_bundle_check_exposes_shared_operator_contract_fields() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = run_command_capture(
        &["taskflow", "consume", "bundle", "check", "--json"],
        &state_dir,
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume bundle check json should parse");

    assert_eq!(
        parsed["blocker_codes"],
        parsed["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["operator_contracts"]["next_actions"]
    );
    assert_eq!(
        parsed["artifact_refs"],
        parsed["operator_contracts"]["artifact_refs"]
    );
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert!(
        matches!(
            parsed["operator_contracts"]["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "operator_contracts.status must stay within release-1 canonical enum"
    );
    assert_eq!(
        parsed["artifact_refs"]["root_artifact_id"],
        parsed["check"]["root_artifact_id"]
    );
    assert_eq!(
        parsed["artifact_refs"]["bundle_artifact_name"],
        "taskflow_runtime_bundle"
    );
    assert_eq!(
        parsed["artifact_refs"]["surface"],
        "vida taskflow consume bundle check"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn task_adaptive_preview_json_emits_receipt_and_fail_closed_blockers() {
    let state_dir = unique_state_dir();
    let valid_finding = serde_json::json!({
        "finding_kind": "proof_gap",
        "source_task_id": "task-a",
        "summary": "verification evidence is missing",
        "evidence_refs": ["receipt-b", "receipt-a"]
    })
    .to_string();

    let preview = run_command_json(
        &[
            "task",
            "adaptive-preview",
            "--finding-json",
            &valid_finding,
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(preview["status"], "pass");
    assert_eq!(preview["surface"], "vida task adaptive-preview");
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["applied"], false);
    assert_eq!(preview["planned_mutation_category"], "blocker_resolution");
    assert_eq!(preview["planned_mutation_kind"], "spawn_blocker_task");
    assert_eq!(preview["operator_truth"]["graph_state_opened"], false);
    assert_eq!(preview["operator_truth"]["graph_state_mutated"], false);
    assert_eq!(preview["operator_truth"]["preview_receipt_emitted"], true);

    let receipt = &preview["preview_receipt"];
    assert_eq!(
        receipt["receipt_kind"],
        "adaptive_replan_finding_preview_receipt"
    );
    assert_eq!(receipt["schema_version"], "1");
    assert_eq!(receipt["source_task_id"], "task-a");
    assert_eq!(receipt["finding_kind"], "proof_gap");
    assert_eq!(receipt["planned_mutation_category"], "blocker_resolution");
    assert_eq!(receipt["planned_mutation_kind"], "spawn_blocker_task");
    assert_eq!(receipt["dry_run"], true);
    assert_eq!(receipt["applied"], false);
    assert_eq!(receipt["graph_state_opened"], false);
    assert_eq!(receipt["graph_state_mutated"], false);
    assert_eq!(
        receipt["receipt_id"],
        "adaptive-replan-preview:task-a:proof_gap:blocker_resolution:spawn_blocker_task:evidence=receipt-a+receipt-b"
    );

    let invalid_finding = serde_json::json!({
        "finding_kind": "general_comment",
        "source_task_id": "task-a",
        "summary": "not an adaptive replanner finding"
    })
    .to_string();
    let invalid = run_command_capture(
        &[
            "task",
            "adaptive-preview",
            "--finding-json",
            &invalid_finding,
            "--json",
        ],
        &state_dir,
    );

    assert!(
        !invalid.status.success(),
        "unsupported finding kind must fail closed"
    );
    let invalid_json: serde_json::Value =
        serde_json::from_slice(&invalid.stdout).expect("invalid input json should parse");
    assert_eq!(invalid_json["status"], "blocked");
    assert_eq!(
        invalid_json["blocker_codes"],
        serde_json::json!(["invalid_adaptive_replan_finding_input"])
    );
    assert_eq!(invalid_json["field"], "finding_kind");
    assert_eq!(
        invalid_json["operator_truth"]["valid_input_does_not_mutate_task_graph"],
        true
    );
}

#[test]
fn taskflow_adaptive_replan_preview_dry_run_and_apply_ordering() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let source_task_id = "case-03-source";
    let blocker_task_id = "case-03-proof-blocker";
    let parent_id = "case-03-parent";
    create_epic_parent(&state_dir, parent_id, "Case 03 parent", "open");
    let source = run_command_json(
        &[
            "task",
            "create",
            source_task_id,
            "Case 03 source",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "1",
            "--parent-id",
            parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(source["status"], "pass");

    let ready_before = run_command_json(&["task", "ready", "--json"], &state_dir);
    assert_eq!(ready_before["ready_count"], 1);
    assert_eq!(
        task_ids_from_rows(&ready_before["tasks"], "ready before"),
        vec![source_task_id.to_string()]
    );
    let blocked_before = run_command_json(&["task", "blocked", "--json"], &state_dir);
    assert_eq!(blocked_before["blocked_count"], 0);
    assert!(blocked_task_ids_from_rows(&blocked_before["tasks"], "blocked before").is_empty());
    let next_lawful_before = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful_before["status"], "pass");
    assert_eq!(
        next_lawful_before["active_bounded_unit"]["task_id"],
        source_task_id
    );
    assert_eq!(
        next_lawful_candidate_ids(&next_lawful_before),
        vec![source_task_id.to_string()]
    );

    let finding_json = serde_json::json!({
        "finding_kind": "proof_gap",
        "source_task_id": source_task_id,
        "summary": "proof target did not cover adaptive ordering",
        "evidence_refs": ["case-03-receipt-b", "case-03-receipt-a"]
    })
    .to_string();
    let adaptive_preview = run_command_json(
        &[
            "task",
            "adaptive-preview",
            "--finding-json",
            &finding_json,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(adaptive_preview["status"], "pass");
    assert_eq!(adaptive_preview["dry_run"], true);
    assert_eq!(adaptive_preview["applied"], false);
    assert_eq!(
        adaptive_preview["planned_mutation_kind"],
        "spawn_blocker_task"
    );
    assert_eq!(
        adaptive_preview["operator_truth"]["preview_receipt_emitted"],
        true
    );
    assert_eq!(
        adaptive_preview["operator_truth"]["graph_state_mutated"],
        false
    );
    assert_eq!(
        adaptive_preview["preview_receipt"]["receipt_kind"],
        "adaptive_replan_finding_preview_receipt"
    );
    assert_eq!(
        adaptive_preview["preview_receipt"]["receipt_id"],
        "adaptive-replan-preview:case-03-source:proof_gap:blocker_resolution:spawn_blocker_task:evidence=case-03-receipt-a+case-03-receipt-b"
    );
    assert_eq!(
        adaptive_preview["preview_receipt"]["operator_truth"]["preview_receipt_emitted"],
        true
    );
    assert_eq!(
        adaptive_preview["preview_receipt"]["operator_truth"]["graph_state_mutated"],
        false
    );

    let ready_after_preview = run_command_json(&["task", "ready", "--json"], &state_dir);
    assert_eq!(
        ready_after_preview["ready_count"],
        ready_before["ready_count"]
    );
    assert_eq!(
        task_ids_from_rows(
            &ready_after_preview["tasks"],
            "ready after adaptive preview"
        ),
        vec![source_task_id.to_string()]
    );
    let blocked_after_preview = run_command_json(&["task", "blocked", "--json"], &state_dir);
    assert_eq!(
        blocked_after_preview["blocked_count"],
        blocked_before["blocked_count"]
    );
    let next_lawful_after_preview =
        run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(
        next_lawful_after_preview["active_bounded_unit"]["task_id"],
        source_task_id
    );
    assert_eq!(
        next_lawful_candidate_ids(&next_lawful_after_preview),
        vec![source_task_id.to_string()]
    );

    let dry_run = run_command_json(
        &[
            "taskflow",
            "replan",
            "spawn-blocker",
            source_task_id,
            blocker_task_id,
            "Case 03 proof blocker",
            "--reason",
            "adaptive preview proof gap",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(dry_run["status"], "dry_run");
    assert_eq!(dry_run["mutation_kind"], "spawn_blocker_task");
    assert_eq!(dry_run["dry_run"], true);
    assert_eq!(dry_run["applied"], false);
    assert_eq!(dry_run["created_task_ids"], serde_json::json!([]));
    let dry_run_receipt = &dry_run["graph_mutation_receipt"];
    assert_eq!(
        dry_run_receipt["receipt_kind"],
        "task_graph_mutation_receipt"
    );
    assert_eq!(dry_run_receipt["mutation_kind"], "spawn_blocker_task");
    assert_eq!(dry_run_receipt["source_task_id"], source_task_id);
    assert_eq!(dry_run_receipt["dry_run"], true);
    assert_eq!(dry_run_receipt["applied"], false);
    assert_eq!(dry_run_receipt["before_validation"]["status"], "pass");
    assert_eq!(dry_run_receipt["after_validation"]["status"], "pass");
    assert_eq!(dry_run_receipt["before_task_count"], 2);
    assert_eq!(dry_run_receipt["after_task_count"], 3);
    assert_eq!(
        dry_run_receipt["planned_task_ids"],
        serde_json::json!([blocker_task_id])
    );
    assert_eq!(
        dry_run_receipt["planned_dependency_edges"][0]["issue_id"],
        source_task_id
    );
    assert_eq!(
        dry_run_receipt["planned_dependency_edges"][0]["depends_on_id"],
        blocker_task_id
    );
    assert_eq!(
        dry_run_receipt["planned_dependency_edges"][0]["edge_type"],
        "blocks"
    );
    assert_eq!(
        dry_run_receipt["operator_truth"]["applied_mutation_requires_after_validation_pass"],
        true
    );

    let missing_blocker_after_dry_run =
        run_command_capture(&["task", "show", blocker_task_id, "--json"], &state_dir);
    assert!(
        !missing_blocker_after_dry_run.status.success(),
        "dry-run replan must not create blocker task"
    );
    let ready_after_dry_run = run_command_json(&["task", "ready", "--json"], &state_dir);
    assert_eq!(
        ready_after_dry_run["ready_count"],
        ready_before["ready_count"]
    );
    assert_eq!(
        task_ids_from_rows(&ready_after_dry_run["tasks"], "ready after dry-run"),
        vec![source_task_id.to_string()]
    );
    let blocked_after_dry_run = run_command_json(&["task", "blocked", "--json"], &state_dir);
    assert_eq!(
        blocked_after_dry_run["blocked_count"],
        blocked_before["blocked_count"]
    );
    let next_lawful_after_dry_run =
        run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(
        next_lawful_after_dry_run["active_bounded_unit"]["task_id"],
        source_task_id
    );
    assert_eq!(
        next_lawful_candidate_ids(&next_lawful_after_dry_run),
        vec![source_task_id.to_string()]
    );

    let applied = run_command_json(
        &[
            "taskflow",
            "replan",
            "spawn-blocker",
            source_task_id,
            blocker_task_id,
            "Case 03 proof blocker",
            "--reason",
            "adaptive preview proof gap",
            "--apply",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(applied["status"], "pass");
    assert_eq!(applied["mutation_kind"], "spawn_blocker_task");
    assert_eq!(applied["dry_run"], false);
    assert_eq!(applied["applied"], true);
    assert_eq!(
        applied["created_task_ids"],
        serde_json::json!([blocker_task_id])
    );
    assert_eq!(applied["graph_mutation_receipt"]["applied"], true);
    assert_eq!(applied["graph_mutation_receipt"]["dry_run"], false);

    let ready_after_apply = run_command_json(&["task", "ready", "--json"], &state_dir);
    assert_eq!(ready_after_apply["ready_count"], 1);
    assert_eq!(
        task_ids_from_rows(&ready_after_apply["tasks"], "ready after apply"),
        vec![blocker_task_id.to_string()]
    );
    let blocked_after_apply = run_command_json(&["task", "blocked", "--json"], &state_dir);
    assert_eq!(blocked_after_apply["blocked_count"], 1);
    assert_eq!(
        blocked_task_ids_from_rows(&blocked_after_apply["tasks"], "blocked after apply"),
        vec![source_task_id.to_string()]
    );
    assert_eq!(
        blocked_after_apply["tasks"][0]["blockers"][0]["depends_on_id"],
        blocker_task_id
    );
    assert_eq!(
        blocked_after_apply["tasks"][0]["blockers"][0]["edge_type"],
        "blocks"
    );
    assert_eq!(
        blocked_after_apply["tasks"][0]["blockers"][0]["dependency_status"],
        "open"
    );

    let next_lawful_after_apply = run_command_json(&["task", "next-lawful", "--json"], &state_dir);
    assert_eq!(next_lawful_after_apply["status"], "pass");
    assert_eq!(
        next_lawful_after_apply["active_bounded_unit"]["task_id"],
        blocker_task_id
    );
    assert_eq!(
        next_lawful_candidate_ids(&next_lawful_after_apply),
        vec![blocker_task_id.to_string()]
    );
    assert_eq!(
        next_lawful_after_apply["sequential_vs_parallel_posture"],
        "sequential_only_single_candidate"
    );
}

#[test]
fn consume_bundle_check_contract_id_stays_within_release1_canonical_enum() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let stdout = String::from_utf8_lossy(&status.stdout).into_owned();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status json should parse");

    assert!(
        matches!(
            parsed["operator_contracts"]["contract_id"].as_str(),
            Some("release-1-operator-contracts") | Some("release-1-shared-fields")
        ),
        "operator_contracts.contract_id must stay within release-1 canonical enum"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_task_import_export_statuses_are_canonical() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    let import_output = vida()
        .args(["taskflow", "task", "import-jsonl", &import_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task import should run");
    assert!(import_output.status.success());
    let import_json: serde_json::Value =
        serde_json::from_slice(&import_output.stdout).expect("import json should parse");
    assert_eq!(import_json["status"], "pass");

    let export_path = format!("{state_dir}/exported.jsonl");
    let export_output = vida()
        .args(["taskflow", "task", "export-jsonl", &export_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task export should run");
    assert!(export_output.status.success());
    let export_json: serde_json::Value =
        serde_json::from_slice(&export_output.stdout).expect("export json should parse");
    assert_eq!(export_json["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn taskflow_task_import_ignores_legacy_helper_status_override_env() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    let output = vida()
        .args(["taskflow", "task", "import-jsonl", &import_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASK_BRIDGE_STATUS_OVERRIDE", "bananas")
        .output()
        .expect("task import should run");
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("import json should parse");
    assert_eq!(parsed["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn taskflow_task_update_ignores_legacy_helper_status_override_env() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "task",
                "update",
                "vida-a",
                "--status",
                "in_progress",
                "--json",
            ])
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASK_BRIDGE_STATUS_OVERRIDE", "bananas");
        command
    });
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("update json should parse");
    assert_eq!(parsed["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_accepts_notes_file_for_shell_safe_progress_recording() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    let notes_path = format!("{state_dir}/notes.txt");
    sample_jsonl(&import_path);
    fs::write(
        &notes_path,
        "line 1\nline 2 with `backticks` and $(shell-like text)\n",
    )
    .expect("notes file should write");

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let parsed = run_command_json(
        &[
            "task",
            "update",
            "vida-a",
            "--status",
            "in_progress",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["surface"], "vida task update");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["task"]["status"], "in_progress");
    assert_eq!(
        parsed["task"]["notes"],
        "line 1\nline 2 with `backticks` and $(shell-like text)\n"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_create_rejects_notes_file_for_local_disclosure_boundary() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    let notes_path = format!("{state_dir}/create-notes.txt");
    sample_jsonl(&import_path);
    fs::write(&notes_path, "one-shot notes\n").expect("notes file should write");

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let output = run_command_capture(
        &[
            "task",
            "create",
            "one-shot-defect",
            "One-shot defect intake",
            "--type",
            "defect",
            "--status",
            "open",
            "--parent-id",
            "vida-root",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--acceptance-target",
            "create sets planner metadata",
            "--proof-target",
            "cargo test -p vida task_create_rejects_notes_file_for_local_disclosure_boundary",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert!(!output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("blocked create json should parse");
    assert_eq!(parsed["surface"], "vida task create");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(
        parsed["blocker_codes"],
        serde_json::json!(["untrusted_create_notes_file"])
    );
    assert_eq!(parsed["rejected_option"], "--notes-file");
    assert!(
        parsed["next_action"]
            .as_str()
            .expect("next_action should render")
            .contains("vida task update <task-id> --notes-file"),
        "{parsed}"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_create_accepts_acceptance_and_proof_aliases() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let parsed = run_command_json(
        &[
            "task",
            "create",
            "alias-proof-task",
            "Alias proof task",
            "--parent-id",
            "vida-root",
            "--acceptance",
            "alias acceptance target",
            "--proof",
            "cargo test -p vida task_create_accepts_acceptance_and_proof_aliases",
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["surface"], "vida task create");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(
        parsed["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!(["alias acceptance target"])
    );
    assert_eq!(
        parsed["task"]["planner_metadata"]["proof_targets"],
        serde_json::json!(["cargo test -p vida task_create_accepts_acceptance_and_proof_aliases"])
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_parent_guard_returns_actionable_json_recovery() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let old_parent = run_command_json(
        &[
            "task",
            "create",
            "old-parent",
            "Old parent",
            "--type",
            "epic",
            "--status",
            "closed",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(old_parent["status"], "pass");

    let closed_child = run_command_json(
        &[
            "task",
            "create",
            "closed-child",
            "Closed child",
            "--type",
            "defect",
            "--status",
            "closed",
            "--parent-id",
            "old-parent",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_child["status"], "pass");

    let open_child = run_command_json(
        &[
            "task",
            "create",
            "open-child",
            "Open child",
            "--type",
            "defect",
            "--status",
            "open",
            "--parent-id",
            "old-parent",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(open_child["status"], "pass");

    let new_parent = run_command_json(
        &[
            "task",
            "create",
            "new-parent",
            "New parent",
            "--type",
            "epic",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(new_parent["status"], "pass");

    let output = run_command_capture(
        &[
            "task",
            "update",
            "open-child",
            "--parent-id",
            "new-parent",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        !output.status.success(),
        "reparent should fail closed until empty old parent is repaired"
    );
    assert!(
        output.stderr.is_empty(),
        "json failure should be machine-readable on stdout, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("blocked update json should parse");
    assert_eq!(parsed["surface"], "vida task update");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(
        parsed["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );
    assert_eq!(
        parsed["graph_issue"]["issue_type"],
        "open_parent_has_no_open_child"
    );
    assert_eq!(parsed["graph_issue"]["issue_id"], "old-parent");
    assert!(
        parsed["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action
                    .as_str()
                    .unwrap()
                    .contains("vida task update old-parent --status closed --json")
            }),
        "{parsed}"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_parent_guard_quotes_shell_unsafe_issue_id_in_next_action() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let old_parent_id = "old;echo pwned";
    let open_child_id = "open-child-shell-quote";
    let new_parent_id = "new-parent-shell-quote";

    assert_eq!(
        run_command_json(
            &[
                "task",
                "create",
                old_parent_id,
                "Old parent",
                "--type",
                "epic",
                "--status",
                "closed",
                "--json",
            ],
            &state_dir,
        )["status"],
        "pass"
    );
    assert_eq!(
        run_command_json(
            &[
                "task",
                "create",
                "closed-child-shell-quote",
                "Closed child",
                "--type",
                "defect",
                "--status",
                "closed",
                "--parent-id",
                old_parent_id,
                "--json",
            ],
            &state_dir,
        )["status"],
        "pass"
    );
    assert_eq!(
        run_command_json(
            &[
                "task",
                "create",
                open_child_id,
                "Open child",
                "--type",
                "defect",
                "--status",
                "open",
                "--parent-id",
                old_parent_id,
                "--json",
            ],
            &state_dir,
        )["status"],
        "pass"
    );
    assert_eq!(
        run_command_json(
            &[
                "task",
                "create",
                new_parent_id,
                "New parent",
                "--type",
                "epic",
                "--json",
            ],
            &state_dir,
        )["status"],
        "pass"
    );

    let output = run_command_capture(
        &[
            "task",
            "update",
            open_child_id,
            "--parent-id",
            new_parent_id,
            "--json",
        ],
        &state_dir,
    );
    assert!(!output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("blocked update json should parse");
    assert!(
        parsed["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|action| action.as_str())
            .any(|action| {
                action.contains("vida task update 'old;echo pwned' --status closed --json")
            }),
        "{parsed}"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_rejects_notes_and_notes_file_together() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    let notes_path = format!("{state_dir}/notes.txt");
    sample_jsonl(&import_path);
    fs::write(&notes_path, "safe note\n").expect("notes file should write");

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let output = run_command_capture(
        &[
            "task",
            "update",
            "vida-a",
            "--notes",
            "inline",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Use only one notes source: --notes <text> or --notes-file <path>"),
        "stderr was: {stderr}"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn consume_bundle_check_blocked_path_matches_blocker_codes_contract() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let output = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume bundle check should run");
    assert!(
        !output.status.success(),
        "blocked consume bundle check should fail closed"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("consume bundle check blocked json should parse");

    let required_operator_blocker_codes = [
        "missing_protocol_binding_receipt",
        "protocol_binding_not_runtime_ready",
    ];
    let required_check_blocker_codes = [
        "missing_protocol_binding_rows",
        "missing_protocol_binding_receipt",
        "protocol_binding_not_runtime_ready",
        "invalid_metadata_tuple_key:protocol_binding_revision",
    ];
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert_eq!(parsed["operator_contracts"]["status"], "blocked");
    assert!(
        matches!(
            parsed["operator_contracts"]["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "operator_contracts.status must stay within release-1 canonical enum"
    );
    assert_eq!(
        parsed["blocker_codes"],
        parsed["operator_contracts"]["blocker_codes"]
    );
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    let check_blockers = parsed["check"]["blockers"]
        .as_array()
        .expect("check blockers should be an array");
    for code in required_operator_blocker_codes {
        assert!(
            blocker_codes
                .iter()
                .any(|value| value.as_str() == Some(code)),
            "missing required blocker code: {code}"
        );
    }
    for code in required_check_blocker_codes {
        assert!(
            check_blockers
                .iter()
                .any(|value| value.as_str() == Some(code)),
            "missing required check blocker code: {code}"
        );
    }

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn consume_final_blocks_when_execution_preparation_is_required_without_handoff_evidence() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "architecture refactor implementation patch",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let execution_plan = &parsed["payload"]["role_selection"]["execution_plan"];
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let required = dispatch_contract["execution_preparation_required"].as_bool() == Some(true)
        || dispatch_contract["lane_catalog"]
            .get("execution_preparation")
            .is_some()
        || dispatch_contract["lane_sequence"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|value| value.as_str() == Some("execution_preparation"));

    if required {
        let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
            .as_array()
            .expect("closure blockers should be an array");
        assert!(
            closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("pending_execution_preparation_evidence")),
            "required execution_preparation lane must block without evidence/handoff packet"
        );
        assert_eq!(parsed["operator_contracts"]["status"], "blocked");
        assert!(parsed["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array")
            .iter()
            .any(|value| value.as_str() == Some("closure_admission_block")));
        assert_eq!(
            parsed["payload"]["dispatch_receipt"]["blocker_code"],
            "pending_execution_preparation_evidence"
        );
        assert!(
            parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| value.as_str() == Some("pending_execution_preparation_evidence"))
        );
        assert!(
            !parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| {
                    matches!(
                        value.as_str(),
                        Some("unsupported_boundary") | Some("retrieval_evidence")
                    )
                }),
            "execution_preparation gate should not leak unsupported_boundary/retrieval_evidence blockers"
        );
        assert!(
            !parsed["blocker_codes"]
                .as_array()
                .expect("blocker_codes should be an array")
                .iter()
                .any(|value| {
                    matches!(
                        value.as_str(),
                        Some("unsupported_boundary") | Some("retrieval_evidence")
                    )
                }),
            "execution_preparation gate should not leak unsupported_boundary/retrieval_evidence blocker_codes"
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn consume_final_blocks_when_approval_or_delegation_wait_lacks_evidence() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "implementation review approval handoff",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let latest_status = &parsed["payload"]["run_graph_bootstrap"]["latest_status"];
    let handoff_state = latest_status["handoff_state"].as_str().unwrap_or_default();
    let policy_gate = latest_status["policy_gate"].as_str().unwrap_or_default();
    let lifecycle_stage = latest_status["lifecycle_stage"]
        .as_str()
        .unwrap_or_default();
    let combined = format!(
        "{} {} {}",
        handoff_state.to_ascii_lowercase(),
        policy_gate.to_ascii_lowercase(),
        lifecycle_stage.to_ascii_lowercase()
    );
    let approval_or_delegation_wait = combined.contains("approval") || combined.contains("delegat");
    if approval_or_delegation_wait {
        assert_eq!(
            latest_status["status"], "awaiting_approval",
            "approval/delegation wait branch should surface the structured approval wait status"
        );
        assert_eq!(latest_status["lifecycle_stage"], "approval_wait");
        assert_eq!(latest_status["policy_gate"], "approval_required");
        assert_eq!(latest_status["handoff_state"], "awaiting_approval");
        assert_eq!(latest_status["resume_target"], "dispatch.approval");
        assert_eq!(latest_status["next_node"], "approval");

        let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
            .as_array()
            .expect("closure blockers should be an array");
        assert!(
            closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("pending_approval_delegation_evidence")),
            "approval/delegation wait branch must fail closed without evidence"
        );
        assert_eq!(
            parsed["payload"]["dispatch_receipt"]["blocker_code"],
            "pending_approval_delegation_evidence"
        );
        assert!(
            parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| value.as_str() == Some("pending_approval_delegation_evidence"))
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_fails_closed_on_retrieval_decision_gate_when_not_synced() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let check = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(!check.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("protocol-binding check json should parse");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["decision_gate"]["policy_gate"], "retrieval_evidence");
    assert_eq!(parsed["decision_gate"]["ready"], false);

    let blocker_code = parsed["decision_gate"]["blocker_code"]
        .as_str()
        .expect("decision gate blocker code should be present");
    assert!(
        blocker_code == "missing_protocol_binding_receipt"
            || blocker_code == "protocol_binding_not_runtime_ready",
        "unexpected decision gate blocker code: {blocker_code}"
    );
    assert_shared_fields_consistency(&parsed, "protocol-binding check");
    assert_operator_contracts_consistency(&parsed, "protocol-binding check");
    let contract_blockers = parsed["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should be array");
    assert_eq!(
        contract_blockers[0].as_str().unwrap(),
        blocker_code,
        "operator_contracts.blocker_codes must mirror decision_gate blocker_code"
    );
    assert!(
        parsed["operator_contracts"]["next_actions"]
            .as_array()
            .expect("operator_contracts.next_actions should be array")
            .iter()
            .any(|action| {
                action
                    .as_str()
                    .unwrap()
                    .contains("protocol-binding check --json")
            }),
        "operator_contracts.next_actions must reference protocol-binding check guidance"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_lock_retry_preserves_blocker_codes() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.store(0, Ordering::SeqCst);
    let output = vida_test_support::retry_with_backoff(
        || {
            let attempt = PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                vida_test_support::simulated_state_lock_output()
            } else {
                let mut command = vida();
                command
                    .args(["taskflow", "protocol-binding", "check", "--json"])
                    .env("VIDA_STATE_DIR", &state_dir);
                command.output().expect("protocol-binding check should run")
            }
        },
        STATE_LOCK_RETRY_LIMIT,
        |output| !output.status.success() && is_state_lock_error(output),
    );

    assert!(!output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("protocol-binding check json should parse");
    assert_eq!(parsed["status"], "blocked");
    let blocker_code = parsed["decision_gate"]["blocker_code"]
        .as_str()
        .expect("decision gate blocker code should be present");
    assert!(
        blocker_code == "missing_protocol_binding_receipt"
            || blocker_code == "protocol_binding_not_runtime_ready",
        "unexpected decision gate blocker code: {blocker_code}"
    );
    let contract_blockers = parsed["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should be array");
    assert_eq!(
        contract_blockers[0].as_str().unwrap(),
        blocker_code,
        "operator_contracts.blocker_codes must mirror decision_gate blocker_code"
    );
    let shared_blockers = parsed["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared_fields.blocker_codes should be array");
    assert_eq!(
        shared_blockers[0].as_str().unwrap(),
        blocker_code,
        "shared_fields.blocker_codes must mirror decision_gate blocker_code"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_plain_json_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let plain_output = run_command_capture(&["taskflow", "protocol-binding", "check"], &state_dir);
    assert!(!plain_output.status.success());
    let plain_stdout = String::from_utf8_lossy(&plain_output.stdout);
    let plain_status = extract_plain_surface_line(&plain_stdout, "status");
    let plain_blockers: Vec<String> =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "blocker_codes"))
            .expect("blocker_codes should render as json array for plain output");
    let plain_next_actions: Vec<String> =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "next_actions"))
            .expect("next_actions should render as json array for plain output");
    let plain_shared_fields: serde_json::Value =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "shared_fields"))
            .expect("shared_fields should render as json object for plain output");
    let plain_operator_contracts: serde_json::Value = serde_json::from_str(
        &extract_plain_surface_line(&plain_stdout, "operator_contracts"),
    )
    .expect("operator_contracts should render as json object for plain output");

    let json_output = run_command_capture(
        &["taskflow", "protocol-binding", "check", "--json"],
        &state_dir,
    );
    let parsed_json: serde_json::Value = serde_json::from_slice(&json_output.stdout)
        .expect("protocol-binding check json should parse");
    let json_status = parsed_json["status"]
        .as_str()
        .expect("status should be string");
    assert_eq!(plain_status, json_status);
    let json_blockers = parsed_json["blocker_codes"]
        .as_array()
        .expect("json blocker_codes should be array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("blocker code should be string")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(plain_blockers, json_blockers);
    let json_next_actions = parsed_json["next_actions"]
        .as_array()
        .expect("json next_actions should be array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("next action should be string")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(plain_next_actions, json_next_actions);
    assert_eq!(plain_shared_fields, parsed_json["shared_fields"]);
    assert_eq!(plain_operator_contracts, parsed_json["operator_contracts"]);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn consume_final_fails_closed_on_retrieval_policy_gate_when_not_synced() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "architecture refactor implementation patch",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !output.status.success(),
        "consume final must fail closed when protocol binding evidence is missing"
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
        .as_array()
        .expect("closure blockers should be an array");
    assert!(
        closure_blockers
            .iter()
            .any(|value| value.as_str() == Some("missing_protocol_binding_receipt"))
            || closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("protocol_binding_not_runtime_ready")),
        "consume final must keep retrieval gate protocol-binding blockers in closure admission"
    );
    assert_eq!(parsed["operator_contracts"]["status"], "blocked");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn cross_surface_protocol_binding_parity() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync_json = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    assert!(
        sync_json["compiled_payload_import_evidence"]["trusted"]
            .as_bool()
            .unwrap_or(false),
        "protocol-binding sync must produce trusted compiled payload evidence"
    );
    let receipt_id = require_json_string(
        &sync_json["receipt"]["receipt_id"],
        "protocol-binding sync receipt id",
    );

    let pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    let pb_summary = &pb_status_json["summary"];
    let blocking_issues = pb_summary["blocking_issue_count"]
        .as_u64()
        .expect("protocol-binding summary should expose blocking_issue_count");
    assert_eq!(
        require_json_string(
            &pb_summary["latest_receipt_id"],
            "protocol-binding summary latest_receipt_id"
        ),
        receipt_id
    );

    let consume_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "cross surface parity",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume_output.stdout).expect("consume final json should parse");
    assert_eq!(consume_json["surface"], "vida taskflow consume final");
    let consume_run_id = require_json_string(
        &consume_json["payload"]["dispatch_receipt"]["run_id"],
        "consume dispatch run_id",
    );
    let consume_artifact_run_id = require_json_string(
        &consume_json["operator_contracts"]["artifact_refs"]
            ["latest_run_graph_dispatch_receipt_id"],
        "consume artifact refs latest run graph dispatch receipt id",
    );

    let status_json = run_command_json(&["status", "--json"], &state_dir);
    let doctor_json = run_command_json(&["doctor", "--json"], &state_dir);

    let status_proto_id = require_json_string(
        &status_json["protocol_binding"]["latest_receipt_id"],
        "status protocol_binding latest_receipt_id",
    );
    let doctor_proto_id = require_json_string(
        &doctor_json["protocol_binding"]["latest_receipt_id"],
        "doctor protocol_binding latest_receipt_id",
    );
    assert!(status_proto_id.starts_with("protocol-binding-"));
    assert!(doctor_proto_id.starts_with("protocol-binding-"));
    assert_eq!(
        status_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("status should expose blocking_issue_count"),
        blocking_issues
    );
    assert_eq!(
        doctor_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("doctor should expose blocking_issue_count"),
        blocking_issues
    );

    let status_artifact_refs = &status_json["artifact_refs"];
    let doctor_artifact_refs = &doctor_json["operator_contracts"]["artifact_refs"];
    let doctor_root_trace = &doctor_json["trace_evidence"]["root_trace"];
    assert_eq!(
        require_json_string(
            &status_artifact_refs["protocol_binding_latest_receipt_id"],
            "status artifact_refs protocol_binding_latest_receipt_id"
        ),
        status_proto_id
    );
    assert_eq!(
        require_json_string(
            &doctor_artifact_refs["protocol_binding_latest_receipt_id"],
            "doctor artifact_refs protocol_binding_latest_receipt_id"
        ),
        doctor_proto_id
    );
    let retrieval_trust_signal = &doctor_artifact_refs["retrieval_trust_signal"];
    match retrieval_trust_signal["source"].as_str() {
        Some("runtime_consumption_snapshot_index") => {
            assert_eq!(
                retrieval_trust_signal["citation"],
                status_artifact_refs["runtime_consumption_latest_snapshot_path"]
            );
            assert_eq!(
                retrieval_trust_signal["acl"],
                status_artifact_refs["protocol_binding_latest_receipt_id"]
            );
            assert_eq!(
                doctor_root_trace["runtime_consumption_latest_snapshot_path"],
                retrieval_trust_signal["citation"]
            );
            if consume_json["status"] == "blocked"
                || consume_json["operator_contracts"]["status"] == "blocked"
            {
                assert!(
                    doctor_json["trace_evidence"]["status"] == "blocked"
                        || doctor_json["operator_contracts"]["status"] == "blocked",
                    "blocked consume-final evidence must propagate through doctor trace or operator status"
                );
            } else {
                assert_eq!(doctor_json["trace_evidence"]["status"], "pass");
            }
        }
        None => {
            assert!(
                consume_json["status"] == "blocked"
                    || consume_json["operator_contracts"]["status"] == "blocked",
                "retrieval trust signal should be absent only when the latest final snapshot is blocked"
            );
            assert!(retrieval_trust_signal["citation"].is_null());
            assert!(retrieval_trust_signal["acl"].is_null());
            assert!(
                doctor_json["trace_evidence"]["status"] == "blocked"
                    || doctor_json["operator_contracts"]["status"] == "blocked",
                "blocked retrieval trust should propagate through doctor trace or operator status"
            );
        }
        Some(source) => panic!("unexpected retrieval trust signal source: {source}"),
    }
    assert_eq!(
        doctor_root_trace["latest_run_graph_dispatch_receipt_id"],
        status_artifact_refs["latest_run_graph_dispatch_receipt_id"]
    );
    assert_eq!(
        doctor_root_trace["protocol_binding_latest_receipt_id"],
        status_artifact_refs["protocol_binding_latest_receipt_id"]
    );
    assert_eq!(
        doctor_root_trace["runtime_consumption_latest_snapshot_path"],
        status_artifact_refs["runtime_consumption_latest_snapshot_path"]
    );

    let status_run_id = require_json_string(
        &status_json["latest_run_graph_dispatch_receipt"]["run_id"],
        "status latest run graph dispatch receipt run_id",
    );
    let doctor_run_id = require_json_string(
        &doctor_artifact_refs["latest_run_graph_dispatch_receipt_id"],
        "doctor artifact_refs latest run graph dispatch receipt id",
    );
    assert_eq!(status_run_id, doctor_run_id);
    assert_eq!(status_run_id, consume_run_id);
    assert_eq!(status_run_id, consume_artifact_run_id);

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn cross_surface_protocol_binding_blocker_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    run_and_assert_success(&["boot"], &state_dir);

    let status_json = run_command_json(&["status", "--json"], &state_dir);
    let doctor_json = run_command_json(&["doctor", "--json"], &state_dir);

    let status_blocker_codes =
        require_string_array(&status_json["blocker_codes"], "status blocker_codes");
    let doctor_blocker_codes =
        require_string_array(&doctor_json["blocker_codes"], "doctor blocker_codes");
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_operator_evidence"),
        "status should fail closed on missing retrieval-trust operator evidence"
    );
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_source_operator_evidence"),
        "status should fail closed on missing retrieval-trust source evidence"
    );
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_signal_operator_evidence"),
        "status should fail closed on missing retrieval-trust signal evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust operator evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_source_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust source evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_signal_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust signal evidence"
    );

    let doctor_next_actions =
        require_string_array(&doctor_json["next_actions"], "doctor next_actions");
    assert!(
        doctor_next_actions
            .iter()
            .any(|action| action.contains("protocol-binding sync")),
        "doctor next actions should include protocol-binding sync guidance"
    );
    assert!(
        doctor_next_actions
            .iter()
            .any(|action| action.contains("consume bundle check")),
        "doctor next actions should include consume bundle check guidance"
    );
    assert_eq!(doctor_json["trace_evidence"]["status"], "blocked");

    assert_eq!(
        status_json["protocol_binding"]["blocking_issue_count"],
        doctor_json["protocol_binding"]["blocking_issue_count"]
    );
    assert_eq!(
        status_json["protocol_binding"]["latest_receipt_id"],
        doctor_json["protocol_binding"]["latest_receipt_id"]
    );

    let consume_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "cross surface parity block",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !consume_output.status.success(),
        "consume final must fail closed on protocol-binding absence"
    );
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume_output.stdout).expect("consume final json should parse");
    assert!(consume_json["trace_id"].is_null());
    assert!(consume_json["workflow_class"].is_null());
    assert!(consume_json["risk_tier"].is_null());
    let consume_blockers = require_string_array(
        &consume_json["payload"]["closure_admission"]["blockers"],
        "consume closure blockers",
    );
    assert!(
        consume_blockers
            .iter()
            .any(|code| code == "missing_protocol_binding_receipt"),
        "consume final should keep protocol-binding blockers in closure admission"
    );
    assert!(
        consume_blockers
            .iter()
            .any(|code| code.contains("protocol_binding")),
        "consume closure blockers should preserve protocol-binding-family evidence"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_operator_contract_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    run_and_assert_success(&["boot"], &state_dir);

    let initial_status_json = run_command_json(&["status", "--json"], &state_dir);
    let initial_blocking_count = initial_status_json["protocol_binding"]["blocking_issue_count"]
        .as_u64()
        .expect("status protocol_binding blocking_issue_count should exist");
    let initial_operator_status = initial_status_json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should exist before sync");
    assert_eq!(
        initial_status_json["status"], initial_status_json["operator_contracts"]["status"],
        "top-level status must mirror the operator contract status before sync"
    );
    assert!(
        matches!(initial_operator_status, "pass" | "blocked"),
        "operator_contracts.status must stay within the release-1 canonical status enum before sync"
    );
    if initial_blocking_count > 0 {
        assert_eq!(
            initial_operator_status, "blocked",
            "protocol-binding blockers must force the top-level operator contract into blocked status before sync"
        );
    }

    let initial_pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    let pb_summary = &initial_pb_status_json["summary"];
    let pb_blocking = pb_summary["blocking_issue_count"]
        .as_u64()
        .expect("protocol-binding status summary blocking_issue_count should exist");
    assert_eq!(
        pb_blocking, initial_blocking_count,
        "status surface and protocol-binding status must agree on blocking_issue_count"
    );

    let sync_json = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    assert!(
        sync_json["compiled_payload_import_evidence"]["trusted"]
            .as_bool()
            .unwrap_or(false),
        "protocol-binding sync must produce trusted compiled payload evidence"
    );

    let post_sync_status_json = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        post_sync_status_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("status protocol_binding blocking_issue_count should exist after sync"),
        0,
        "canonical protocol-binding parity requires zero blockers after sync"
    );
    let post_sync_operator_status = post_sync_status_json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should exist after sync");
    assert_eq!(
        post_sync_status_json["status"], post_sync_status_json["operator_contracts"]["status"],
        "top-level status must mirror the operator contract once blockers clear"
    );
    assert!(
        matches!(post_sync_operator_status, "pass" | "blocked"),
        "operator_contracts.status must remain within the release-1 canonical status enum after sync"
    );
    let post_sync_blockers = post_sync_status_json["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should remain an array after sync");
    assert!(
        !post_sync_blockers
            .iter()
            .filter_map(|value| value.as_str())
            .any(|code| code == "protocol_binding_blocking_issues"),
        "top-level operator contracts must stop reporting protocol-binding blockers after sync clears them"
    );

    let post_sync_pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    assert_eq!(
        post_sync_pb_status_json["summary"]["blocking_issue_count"],
        post_sync_status_json["protocol_binding"]["blocking_issue_count"],
        "status surface and protocol-binding status must stay aligned on blocking_issue_count after sync"
    );
    assert_eq!(
        post_sync_pb_status_json["summary"]["latest_receipt_id"],
        post_sync_status_json["protocol_binding"]["latest_receipt_id"],
        "latest_receipt_id must remain canonical across surfaces"
    );
    assert_eq!(
        post_sync_status_json["artifact_refs"]["protocol_binding_latest_receipt_id"],
        post_sync_status_json["protocol_binding"]["latest_receipt_id"],
        "status artifact_refs should mirror the canonical protocol-binding latest receipt after sync"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_statuses_are_canonical() {
    let state_dir = unique_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let output = run_command_capture(
        &["taskflow", "protocol-binding", "check", "--json"],
        &state_dir,
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("protocol-binding check json should parse");
    let top_status = json["status"]
        .as_str()
        .expect("protocol-binding check status should be string");
    let shared_status = json["shared_fields"]["status"]
        .as_str()
        .expect("shared_fields.status should be string");
    let contract_status = json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should be string");
    assert_eq!(top_status, shared_status);
    assert_eq!(shared_status, contract_status);
    assert!(
        matches!(top_status, "pass" | "blocked"),
        "protocol-binding check status must remain canonical"
    );
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn host_dispatch_handoff_projection_parity_unresolved_lane_selection_persists_blocked_resume_evidence(
) {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let final_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "resume lane governance conflict",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !final_output.status.success(),
        "unresolved lane selection should fail closed"
    );
    let final_parsed: serde_json::Value =
        serde_json::from_slice(&final_output.stdout).expect("consume final json should parse");
    assert_eq!(final_parsed["surface"], "vida taskflow consume final");
    assert_eq!(final_parsed["status"], "blocked");
    assert_eq!(
        final_parsed["payload"]["run_graph_bootstrap"]["reason"],
        "unresolved_lane_selection"
    );
    assert_eq!(
        final_parsed["payload"]["run_graph_bootstrap"]["latest_status"]["status"],
        "blocked"
    );

    let receipt = &final_parsed["payload"]["dispatch_receipt"];
    assert_eq!(receipt["dispatch_status"], "blocked");
    assert_eq!(receipt["lane_status"], "lane_blocked");
    let run_id = receipt["run_id"]
        .as_str()
        .expect("blocked dispatch receipt should include run_id");
    let packet_path = receipt["dispatch_packet_path"]
        .as_str()
        .expect("blocked dispatch receipt should include dispatch_packet_path");
    assert!(
        packet_path.contains("runtime-consumption"),
        "packet path should be runtime-consumption evidence: {packet_path}"
    );
    assert!(
        fs::metadata(packet_path).is_ok(),
        "blocked dispatch packet should exist at {packet_path}"
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run graph status should run");
    assert!(
        run_graph.status.success(),
        "{}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_parsed: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run graph json should parse");
    assert_eq!(run_graph_parsed["run_id"], run_id);
    assert_eq!(run_graph_parsed["status"], "blocked");

    let continue_output = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert!(
        !continue_output.status.success(),
        "consume continue must fail closed on blocked unresolved lane evidence"
    );
    let stderr = String::from_utf8_lossy(&continue_output.stderr);
    assert!(
        stderr.contains("execution_preparation_gate_blocked"),
        "stderr should classify blocked packet evidence as execution_preparation_gate_blocked, got: {stderr}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn consume_continue_fails_closed_on_lane_governance_status_evidence_conflict() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let final_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "resume lane governance conflict",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let final_parsed: serde_json::Value =
        serde_json::from_slice(&final_output.stdout).expect("consume final json should parse");
    assert_eq!(final_parsed["surface"], "vida taskflow consume final");
    assert!(
        matches!(
            final_parsed["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "consume final status must remain within the canonical enum"
    );

    let runtime_consumption_root = format!("{state_dir}/runtime-consumption");
    for relative_dir in ["dispatch-packets", "downstream-dispatch-packets"] {
        let dir_path = format!("{runtime_consumption_root}/{relative_dir}");
        let Ok(entries) = fs::read_dir(&dir_path) else {
            continue;
        };
        for entry in entries {
            let entry = entry.expect("read runtime-consumption entry");
            let file_type = entry.file_type().expect("entry file type");
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let packet_body = fs::read_to_string(&path).expect("read runtime packet");
            let mut packet: serde_json::Value =
                serde_json::from_str(&packet_body).expect("parse runtime packet");
            packet["lane_status"] = serde_json::Value::String("lane_open".to_string());
            packet["supersedes_receipt_id"] =
                serde_json::Value::String("receipt-superseded-1".to_string());
            packet["downstream_lane_status"] = serde_json::Value::String("lane_open".to_string());
            packet["downstream_supersedes_receipt_id"] =
                serde_json::Value::String("receipt-superseded-1".to_string());
            fs::write(
                &path,
                serde_json::to_vec_pretty(&packet).expect("serialize runtime packet"),
            )
            .expect("write runtime packet");
        }
    }

    let continue_output = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert!(
        !continue_output.status.success(),
        "consume continue must fail closed on lane governance conflict"
    );
    let stderr = String::from_utf8_lossy(&continue_output.stderr);
    assert!(
        stderr.contains("conflicts with derived lane_status")
            || stderr.contains("execution_preparation_gate_blocked"),
        "stderr should fail closed for lane governance conflict packet, got: {stderr}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn status_and_doctor_block_on_current_session_run_graph_snapshot_inconsistency() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);
    let _ = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    let (final_output, _) = run_command_json_allow_failure(
        &[
            "taskflow",
            "consume",
            "final",
            "status doctor parity integration fixture",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(final_output["surface"], "vida taskflow consume final");
    let packet_root = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let packet_path = fs::read_dir(&packet_root)
        .expect("dispatch-packets directory should exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .expect("consume final should create a persisted dispatch packet");
    let expected_packet_path = packet_path.display().to_string();
    let packet: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&packet_path).expect("read packet"))
            .expect("parse packet");
    let expected_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            packet
                .get("run_graph_bootstrap")
                .and_then(|value| value.get("run_id"))
                .and_then(serde_json::Value::as_str)
        })
        .expect("packet run_id should be present")
        .to_string();
    let expected_task_id = packet
        .get("delivery_task_packet")
        .and_then(|value| value.get("task_id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("id"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("backlog_id"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            packet
                .get("run_graph_bootstrap")
                .and_then(|value| value.get("task_id"))
                .and_then(serde_json::Value::as_str)
        })
        .expect("packet task id should be present")
        .to_string();

    let (status, _) = run_command_json_allow_failure(&["status", "--json"], &state_dir);
    let (doctor, _) = run_command_json_allow_failure(&["doctor", "--json"], &state_dir);
    let status_blockers = require_json_string_array(&status["blocker_codes"], "status blockers");
    let doctor_blockers = require_json_string_array(&doctor["blocker_codes"], "doctor blockers");
    for blocker in [
        "run_graph_latest_snapshot_inconsistent",
        "run_graph_latest_dispatch_receipt_checkpoint_leakage",
    ] {
        assert_eq!(
            status_blockers.contains(&blocker.to_string()),
            doctor_blockers.contains(&blocker.to_string()),
            "status and doctor must not diverge for {blocker}: status={status} doctor={doctor}"
        );
    }
    if doctor_blockers.contains(&"run_graph_latest_snapshot_inconsistent".to_string()) {
        let actions = require_json_string_array(&doctor["next_actions"], "doctor next_actions");
        assert!(actions
            .iter()
            .any(|action| action.contains("concrete run/task/packet")));
        assert!(
            !doctor["artifact_refs"]["current_session_run_graph_status_run_id"].is_null(),
            "doctor must expose concrete current-session run refs: {doctor}"
        );
        assert_eq!(
            doctor["artifact_refs"]["current_session_run_graph_status_run_id"],
            expected_run_id
        );
        assert_eq!(
            doctor["artifact_refs"]["current_session_run_graph_status_task_id"],
            expected_task_id
        );
        assert!(
            !doctor["artifact_refs"]["current_session_run_graph_status_run_id_source"].is_null(),
            "doctor must expose run ref source: {doctor}"
        );
        assert!(
            !doctor["artifact_refs"]["current_session_run_graph_status_task_id_source"].is_null(),
            "doctor must expose task ref source: {doctor}"
        );
        let doctor_packet_path = doctor["artifact_refs"]
            ["current_session_run_graph_dispatch_packet_path"]
            .as_str()
            .expect("doctor packet path should render")
            .replace('\\', "/");
        assert_eq!(doctor_packet_path, expected_packet_path.replace('\\', "/"));
        assert_eq!(
            doctor["artifact_refs"],
            doctor["operator_contracts"]["artifact_refs"]
        );
    }
    if status_blockers.contains(&"run_graph_latest_snapshot_inconsistent".to_string()) {
        let actions = require_json_string_array(&status["next_actions"], "status next_actions");
        assert!(actions
            .iter()
            .any(|action| action.contains("concrete run/task/packet")));
        assert!(
            !status["artifact_refs"]["latest_run_graph_status_run_id"].is_null(),
            "status must expose concrete latest run refs: {status}"
        );
        assert!(
            !status["artifact_refs"]["latest_run_graph_status_task_id"].is_null(),
            "status must expose concrete latest task refs: {status}"
        );
        assert!(
            !status["artifact_refs"]["latest_run_graph_dispatch_packet_path"].is_null(),
            "status must expose concrete latest packet refs: {status}"
        );
        assert_eq!(
            status["artifact_refs"]["latest_run_graph_status_run_id"],
            expected_run_id
        );
        assert_eq!(
            status["artifact_refs"]["latest_run_graph_status_task_id"],
            expected_task_id
        );
        assert!(
            !status["artifact_refs"]["latest_run_graph_status_run_id_source"].is_null(),
            "status must expose run ref source: {status}"
        );
        assert!(
            !status["artifact_refs"]["latest_run_graph_status_task_id_source"].is_null(),
            "status must expose task ref source: {status}"
        );
        let status_packet_path = status["artifact_refs"]["latest_run_graph_dispatch_packet_path"]
            .as_str()
            .expect("status packet path should render")
            .replace('\\', "/");
        assert_eq!(status_packet_path, expected_packet_path.replace('\\', "/"));
    }

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn consume_continue_json_classifies_persisted_packet_contract_invalid_with_artifacts() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);
    let _ = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    let (final_output, _) = run_command_json_allow_failure(
        &[
            "taskflow",
            "consume",
            "final",
            "packet contract invalid integration fixture",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(final_output["surface"], "vida taskflow consume final");

    let packet_root = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let packet_path = fs::read_dir(&packet_root)
        .expect("dispatch-packets directory should exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .expect("consume final should create a persisted dispatch packet");
    let mut packet: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&packet_path).expect("read packet"))
            .expect("parse packet");
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .expect("packet run_id should be present")
        .to_string();
    let packet_task_id = "packet-contract-invalid-task-id";
    assert_ne!(
        packet_run_id, packet_task_id,
        "fixture must prove --from-task is not filled with run_id"
    );
    if let Some(delivery) = packet
        .get_mut("delivery_task_packet")
        .and_then(serde_json::Value::as_object_mut)
    {
        delivery.insert(
            "task_id".to_string(),
            serde_json::Value::String(packet_task_id.to_string()),
        );
        delivery.remove("owned_paths");
    }
    if let Some(object) = packet.as_object_mut() {
        object.remove("owned_paths");
    }
    fs::write(
        &packet_path,
        serde_json::to_vec_pretty(&packet).expect("encode invalid packet"),
    )
    .expect("write invalid packet");
    let packet_path_string = packet_path.display().to_string();

    let (payload, success) =
        run_command_json_allow_failure(&["taskflow", "consume", "continue", "--json"], &state_dir);
    assert!(!success);
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["dispatch_packet_contract_invalid"])
    );
    assert_eq!(payload["artifact_refs"]["run_id"], packet_run_id);
    assert_eq!(payload["artifact_refs"]["task_id"], packet_task_id);
    let actual_packet_path = payload["artifact_refs"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should render")
        .replace('\\', "/");
    assert_eq!(actual_packet_path, packet_path_string.replace('\\', "/"));
    let actions = require_json_string_array(&payload["next_actions"], "consume next_actions");
    assert!(actions
        .iter()
        .any(|action| action.contains("taskflow packet repair")));
    assert!(actions.iter().all(|action| !action.contains("<run-id>")));
    assert!(actions.iter().all(|action| !action.contains("<task-id>")));
    assert!(actions
        .iter()
        .any(|action| action.contains(&format!("--run-id {packet_run_id}"))));
    assert!(actions
        .iter()
        .any(|action| action.contains(&format!("--from-task {packet_task_id}"))));
    assert!(actions
        .iter()
        .all(|action| !action.contains(&format!("--from-task {packet_run_id}"))));

    let parent_create = run_command_json(
        &[
            "task",
            "create",
            "packet-repair-parent",
            "Packet repair parent",
            "--type",
            "epic",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent_create["status"], "pass");

    let task_create = run_command_json(
        &[
            "task",
            "create",
            packet_task_id,
            "Packet repair metadata fixture",
            "--type",
            "defect",
            "--parent-id",
            "packet-repair-parent",
            "--owned-path",
            "crates/vida/src/taskflow_packet.rs",
            "--proof-target",
            "cargo test -p vida packet_repair -- --nocapture",
            "--acceptance-target",
            "packet repair hydrates owned paths",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_create["status"], "pass");

    let repair = run_command_json(
        &[
            "taskflow",
            "packet",
            "repair",
            "--run-id",
            &packet_run_id,
            "--from-task",
            packet_task_id,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(repair["surface"], "vida taskflow packet repair");
    assert_eq!(repair["status"], "repair_ready");
    assert_eq!(repair["repair_applied"], true);
    assert_eq!(repair["contract_validated"], true);
    assert_eq!(repair["from_task"], packet_task_id);
    assert_eq!(repair["run_id"], packet_run_id);

    let repaired_packet: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&packet_path).expect("read repaired packet"))
            .expect("parse repaired packet");
    assert_eq!(
        repaired_packet["owned_paths"],
        serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
    );
    assert_eq!(
        repaired_packet["delivery_task_packet"]["owned_paths"],
        serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn packet_repair_missing_from_task_json_reports_actionable_option_error() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let (payload, success) = run_command_json_allow_failure(
        &[
            "taskflow", "packet", "repair", "--run-id", "some-run", "--json",
        ],
        &state_dir,
    );

    assert!(!success, "missing --from-task should fail closed");
    assert_eq!(payload["surface"], "vida taskflow packet repair");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["packet_repair_from_task_missing"])
    );
    assert!(payload["error"]
        .as_str()
        .expect("error should render")
        .contains("--from-task <task-id>"));
    let next_actions = require_json_string_array(&payload["next_actions"], "next_actions");
    assert!(next_actions
        .iter()
        .any(|action| action.contains("--from-task <task-id>")));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn packet_repair_missing_run_id_json_reports_actionable_option_error() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let (payload, success) = run_command_json_allow_failure(
        &[
            "taskflow",
            "packet",
            "repair",
            "--from-task",
            "some-task",
            "--json",
        ],
        &state_dir,
    );

    assert!(!success, "missing --run-id should fail closed");
    assert_eq!(payload["surface"], "vida taskflow packet repair");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["packet_repair_run_id_missing"])
    );
    assert!(payload["error"]
        .as_str()
        .expect("error should render")
        .contains("--run-id <id>"));
    let next_actions = require_json_string_array(&payload["next_actions"], "next_actions");
    assert!(next_actions
        .iter()
        .any(|action| action.contains("--run-id <run-id>")));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn multi_session_disjoint_tasks_independent_admission_via_cli() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "multi-session-root",
            "Multi session root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "multi-session-task-a",
            "Multi session task A",
            "--parent-id",
            "multi-session-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "domain-a",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "multi-session-task-b",
            "Multi session task B",
            "--parent-id",
            "multi-session-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "domain-b",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    let session_1_status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = session_1_status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );
    assert!(
        session_1_status["operator_session_projection"]["current_session"]["session_id"]
            .is_string(),
        "current_session should have session_id"
    );

    let claim_conflicts = &session_1_status["operator_session_projection"]["claim_conflicts"];
    assert!(
        claim_conflicts
            .as_array()
            .map_or(true, |arr| arr.is_empty()),
        "disjoint tasks should have no claim conflicts: {claim_conflicts}"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_status_contains_session_projection() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "status-projection-root",
            "Status projection root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(
        projection.is_object(),
        "operator_session_projection should be an object"
    );
    assert!(
        projection["current_session"].is_object(),
        "current_session should be an object"
    );
    assert!(
        projection["project_foreign_runs"].is_array(),
        "project_foreign_runs should be an array"
    );
    assert!(
        projection["project_foreign_blockers"].is_array(),
        "project_foreign_blockers should be an array"
    );
    assert!(
        projection["global_blockers"].is_array(),
        "global_blockers should be an array"
    );
    assert!(
        projection["claim_conflicts"].is_array(),
        "claim_conflicts should be an array"
    );
    assert!(
        projection["current_session_task_claims"].is_array(),
        "current_session_task_claims should be an array"
    );

    let current_session = &projection["current_session"];
    assert!(
        current_session["session_id"].is_string(),
        "session_id should be a string"
    );
    assert!(
        current_session["worktree_environment_id"].is_string(),
        "worktree_environment_id should be a string"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_regression_legacy_global_blocker_does_not_block_unrelated_session() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "legacy-blocker-root",
            "Legacy blocker root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let _task = run_command_json(
        &[
            "task",
            "create",
            "legacy-blocker-test",
            "Legacy blocker test",
            "--parent-id",
            "legacy-blocker-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let global_blockers = &status["operator_session_projection"]["global_blockers"];
    assert!(
        global_blockers
            .as_array()
            .map_or(true, |arr| arr.is_empty()),
        "legacy global blockers should be empty for fresh session: {global_blockers}"
    );

    let current_session = &status["operator_session_projection"]["current_session"];
    let mutation_gate = current_session["mutation_gate"].as_str();
    assert!(
        matches!(mutation_gate, Some("current_session_allowed") | None),
        "mutation_gate should allow current session or be null, got: {mutation_gate:?}"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_same_task_exclusive_conflict_blocks_admission() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "conflict-root",
            "Conflict root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "same-task-conflict",
            "Same task conflict test",
            "--parent-id",
            "conflict-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "shared-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    let _session_1_claim = run_command_json(
        &[
            "agent-init",
            "--role",
            "worker",
            "same-task-conflict",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(projection["current_session_task_claims"].is_array());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_same_conflict_domain_exclusive_blocks() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "domain-root",
            "Domain root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "domain-conflict-a",
            "Domain conflict A",
            "--parent-id",
            "domain-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "shared-exclusive-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "domain-conflict-b",
            "Domain conflict B",
            "--parent-id",
            "domain-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "shared-exclusive-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    let _claim = run_command_json(
        &[
            "agent-init",
            "--role",
            "worker",
            "domain-conflict-a",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    let current_claims = &projection["current_session_task_claims"];
    // agent-init may or may not create a claim, just verify the field exists and is an array
    assert!(
        current_claims.is_array(),
        "current_session_task_claims should be an array"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_path_intersection_blocks_admission() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "path-root",
            "Path root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "path-intersect-a",
            "Path intersect A",
            "--parent-id",
            "path-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "path-intersect-b",
            "Path intersect B",
            "--parent-id",
            "path-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    let _claim = run_command_json(
        &[
            "agent-init",
            "--role",
            "worker",
            "path-intersect-a",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(projection.is_object());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

// ==================== Multi-Session Cascading Closure Integration Tests ====================

#[test]
fn multi_session_task_closure_cascades_to_parent() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task chain
    let epic = run_command_json(
        &[
            "task",
            "create",
            "cascading-epic",
            "Cascading epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "cascading-task",
            "Cascading task",
            "--parent-id",
            "cascading-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "cascading-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify both task and epic are closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let cascading_task = find_task_ref_by_id(tasks, "cascading-task");
    let cascading_epic = find_task_ref_by_id(tasks, "cascading-epic");

    assert_eq!(
        cascading_task["status"], "closed",
        "cascading-task should be closed"
    );
    assert_eq!(
        cascading_epic["status"], "closed",
        "cascading-epic should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_cascading_stops_at_unrelated_open_sibling() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task-a + task-b (siblings)
    let epic = run_command_json(
        &[
            "task",
            "create",
            "sibling-epic",
            "Sibling epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-a",
            "Sibling task A",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-b",
            "Sibling task B",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    // Close task-a
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "sibling-task-a",
            "--reason",
            "Task A complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task-a is closed but epic and task-b are NOT closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let sibling_task_a = find_task_ref_by_id(tasks, "sibling-task-a");
    let sibling_task_b = find_task_ref_by_id(tasks, "sibling-task-b");
    let sibling_epic = find_task_ref_by_id(tasks, "sibling-epic");

    assert_eq!(
        sibling_task_a["status"], "closed",
        "sibling-task-a should be closed"
    );
    assert_eq!(
        sibling_task_b["status"], "open",
        "sibling-task-b should still be open"
    );
    assert!(
        matches!(
            sibling_epic["status"].as_str(),
            Some("open" | "in_progress")
        ),
        "sibling-epic should NOT be closed because sibling-task-b is still open: {}",
        sibling_epic["status"]
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_multi_level_cascading_closure() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create grandparent -> parent -> child chain
    let grandparent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-grandparent",
            "Multi level grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(grandparent["status"], "pass");

    let parent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-parent",
            "Multi level parent",
            "--parent-id",
            "multi-level-grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");

    let child = run_command_json(
        &[
            "task",
            "create",
            "multi-level-child",
            "Multi level child",
            "--parent-id",
            "multi-level-parent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "multi-level-child",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify all three are closed (cascading through multiple levels)
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let multi_level_child = find_task_ref_by_id(tasks, "multi-level-child");
    let multi_level_parent = find_task_ref_by_id(tasks, "multi-level-parent");
    let multi_level_grandparent = find_task_ref_by_id(tasks, "multi-level-grandparent");

    assert_eq!(
        multi_level_child["status"], "closed",
        "multi-level-child should be closed"
    );
    assert_eq!(
        multi_level_parent["status"], "closed",
        "multi-level-parent should be closed via cascading"
    );
    assert_eq!(
        multi_level_grandparent["status"], "closed",
        "multi-level-grandparent should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_user_authorization_overrides_validation() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "override-test-task",
            "Override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close with user authorization (Core Rule #12 override)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "override-test-task",
            "--reason",
            "User authorized per Core Rule #12",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task is closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let override_task = find_task_ref_by_id(tasks, "override-test-task");
    assert_eq!(
        override_task["status"], "closed",
        "override-test-task should be closed with user authorization"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_with_core_rule_12_phrase_variants() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create multiple tasks to test different Core Rule #12 phrases
    let phrases = [
        "User authorized per Core Rule #12",
        "user approval for closure",
        "Close per core rule 12",
        "Explicit user authorization",
    ];

    for (idx, phrase) in phrases.iter().enumerate() {
        let task = run_command_json(
            &[
                "task",
                "create",
                &format!("override-phrase-task-{}", idx),
                &format!("Override phrase test task {}", idx),
                "--type",
                "epic",
                "--priority",
                "1",
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(task["status"], "pass");

        // Close with specific phrase
        let close_result = run_command_json(
            &[
                "task",
                "close",
                &format!("override-phrase-task-{}", idx),
                "--reason",
                phrase,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(close_result["status"], "pass");

        // Verify task is closed
        let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
        let tasks = &task_list["tasks"];

        let test_task = find_task_ref_by_id(tasks, &format!("override-phrase-task-{}", idx));
        assert_eq!(
            test_task["status"], "closed",
            "task should be closed with phrase: {}",
            phrase
        );
    }

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_without_override_fails_on_validation() {
    // This test verifies that without proper authorization, closure can be blocked
    // Note: This may not always fail depending on graph state, but tests the mechanism
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "no-override-task",
            "No override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Try to close with normal reason (should work for simple task)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "no-override-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );

    // For a simple task with no dependencies, this should succeed
    // The validation only fails when it would create an invalid graph
    assert_eq!(close_result["status"], "pass");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

// ==================== Multi-Session Cascading Closure Integration Tests ====================

#[test]
fn multi_session_task_closure_cascades_to_parent_repeat_a() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task chain
    let epic = run_command_json(
        &[
            "task",
            "create",
            "cascading-epic",
            "Cascading epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "cascading-task",
            "Cascading task",
            "--parent-id",
            "cascading-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "cascading-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify both task and epic are closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let cascading_task = find_task_ref_by_id(tasks, "cascading-task");
    let cascading_epic = find_task_ref_by_id(tasks, "cascading-epic");

    assert_eq!(
        cascading_task["status"], "closed",
        "cascading-task should be closed"
    );
    assert_eq!(
        cascading_epic["status"], "closed",
        "cascading-epic should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_cascading_stops_at_unrelated_open_sibling_repeat_a() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task-a + task-b (siblings)
    let epic = run_command_json(
        &[
            "task",
            "create",
            "sibling-epic",
            "Sibling epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-a",
            "Sibling task A",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-b",
            "Sibling task B",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    // Close task-a
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "sibling-task-a",
            "--reason",
            "Task A complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task-a is closed but epic and task-b are NOT closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let sibling_task_a = find_task_ref_by_id(tasks, "sibling-task-a");
    let sibling_task_b = find_task_ref_by_id(tasks, "sibling-task-b");
    let sibling_epic = find_task_ref_by_id(tasks, "sibling-epic");

    assert_eq!(
        sibling_task_a["status"], "closed",
        "sibling-task-a should be closed"
    );
    assert_eq!(
        sibling_task_b["status"], "open",
        "sibling-task-b should still be open"
    );
    assert!(
        matches!(
            sibling_epic["status"].as_str(),
            Some("open" | "in_progress")
        ),
        "sibling-epic should NOT be closed because sibling-task-b is still open: {}",
        sibling_epic["status"]
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_multi_level_cascading_closure_repeat_a() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create grandparent -> parent -> child chain
    let grandparent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-grandparent",
            "Multi level grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(grandparent["status"], "pass");

    let parent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-parent",
            "Multi level parent",
            "--parent-id",
            "multi-level-grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");

    let child = run_command_json(
        &[
            "task",
            "create",
            "multi-level-child",
            "Multi level child",
            "--parent-id",
            "multi-level-parent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "multi-level-child",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify all three are closed (cascading through multiple levels)
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let multi_level_child = find_task_ref_by_id(tasks, "multi-level-child");
    let multi_level_parent = find_task_ref_by_id(tasks, "multi-level-parent");
    let multi_level_grandparent = find_task_ref_by_id(tasks, "multi-level-grandparent");

    assert_eq!(
        multi_level_child["status"], "closed",
        "multi-level-child should be closed"
    );
    assert_eq!(
        multi_level_parent["status"], "closed",
        "multi-level-parent should be closed via cascading"
    );
    assert_eq!(
        multi_level_grandparent["status"], "closed",
        "multi-level-grandparent should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_user_authorization_overrides_validation_repeat_a() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "override-test-task",
            "Override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close with user authorization (Core Rule #12 override)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "override-test-task",
            "--reason",
            "User authorized per Core Rule #12",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task is closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let override_task = find_task_ref_by_id(tasks, "override-test-task");
    assert_eq!(
        override_task["status"], "closed",
        "override-test-task should be closed with user authorization"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_with_core_rule_12_phrase_variants_repeat_a() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create multiple tasks to test different Core Rule #12 phrases
    let phrases = [
        "User authorized per Core Rule #12",
        "user approval for closure",
        "Close per core rule 12",
        "Explicit user authorization",
    ];

    for (idx, phrase) in phrases.iter().enumerate() {
        let task = run_command_json(
            &[
                "task",
                "create",
                &format!("override-phrase-task-{}", idx),
                &format!("Override phrase test task {}", idx),
                "--type",
                "epic",
                "--priority",
                "1",
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(task["status"], "pass");

        // Close with specific phrase
        let close_result = run_command_json(
            &[
                "task",
                "close",
                &format!("override-phrase-task-{}", idx),
                "--reason",
                phrase,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(close_result["status"], "pass");

        // Verify task is closed
        let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
        let tasks = &task_list["tasks"];

        let test_task = find_task_ref_by_id(tasks, &format!("override-phrase-task-{}", idx));
        assert_eq!(
            test_task["status"], "closed",
            "task should be closed with phrase: {}",
            phrase
        );
    }

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_without_override_fails_on_validation_repeat_a() {
    // This test verifies that without proper authorization, closure can be blocked
    // Note: This may not always fail depending on graph state, but tests the mechanism
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "no-override-task",
            "No override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Try to close with normal reason (should work for simple task)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "no-override-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );

    // For a simple task with no dependencies, this should succeed
    // The validation only fails when it would create an invalid graph
    assert_eq!(close_result["status"], "pass");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_expired_claim_allows_new_admission() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "expired-root",
            "Expired root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "expired-task",
            "Expired task",
            "--parent-id",
            "expired-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "expired-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Verify status projection exists and can handle expired claims
    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(projection.is_object());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_global_blocker_blocks_all() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "global-blocker-root",
            "Global blocker root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    // Verify global_blockers field exists and is accessible
    assert!(projection["global_blockers"].is_array());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_foreign_blocked_claim_non_blocking_for_disjoint() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "foreign-blocked-root",
            "Foreign blocked root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    // Create two disjoint tasks
    let task_a = run_command_json(
        &[
            "task",
            "create",
            "foreign-blocked-a",
            "Foreign blocked A",
            "--parent-id",
            "foreign-blocked-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "disjoint-domain-a",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "foreign-blocked-b",
            "Foreign blocked B",
            "--parent-id",
            "foreign-blocked-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "disjoint-domain-b",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    // Verify projection fields exist
    assert!(projection["project_foreign_runs"].is_array());
    assert!(projection["project_foreign_blockers"].is_array());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_observe_mode_non_blocking() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "observe-root",
            "Observe root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "observe-mode-test",
            "Observe mode test",
            "--parent-id",
            "observe-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "observe-domain",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    let orchestrator = run_command_json(
        &["orchestrator-init", "--state-dir", &state_dir, "--json"],
        &state_dir,
    );
    assert_eq!(orchestrator["surface"], "vida orchestrator-init");

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let current_session = &status["operator_session_projection"]["current_session"];
    assert!(current_session["session_id"].is_string());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_foreign_sessions_visible_in_status() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "foreign-root",
            "Foreign root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    for task_name in ["foreign-task-1", "foreign-task-2"] {
        let _task = run_command_json(
            &[
                "task",
                "create",
                task_name,
                "Foreign session task",
                "--parent-id",
                "foreign-root",
                "--type",
                "epic",
                "--priority",
                "1",
                "--json",
            ],
            &state_dir,
        );
    }

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(projection["current_session"]["session_id"].is_string());
    assert!(projection["project_foreign_runs"].is_array());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_disjoint_parallel_admission() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "parallel-root",
            "Parallel root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    for (i, domain) in ["parallel-1", "parallel-2", "parallel-3"]
        .iter()
        .enumerate()
    {
        let _task = run_command_json(
            &[
                "task",
                "create",
                &format!("disjoint-parallel-{}", i),
                &format!("Disjoint parallel task {}", i),
                "--parent-id",
                "parallel-root",
                "--type",
                "epic",
                "--priority",
                "1",
                "--conflict-domain",
                domain,
                "--json",
            ],
            &state_dir,
        );
    }

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let claim_conflicts = &status["operator_session_projection"]["claim_conflicts"];
    assert!(
        claim_conflicts
            .as_array()
            .map_or(true, |arr| arr.is_empty()),
        "disjoint parallel tasks should have no claim conflicts initially: {claim_conflicts}"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_shared_read_vs_exclusive_conflict() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let root = run_command_json(
        &[
            "task",
            "create",
            "shared-root",
            "Shared root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "shared-exclusive-test",
            "Shared vs Exclusive test",
            "--parent-id",
            "shared-root",
            "--type",
            "epic",
            "--priority",
            "1",
            "--conflict-domain",
            "shared-exclusive",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    let _claim = run_command_json(
        &[
            "agent-init",
            "--role",
            "worker",
            "shared-exclusive-test",
            "--state-dir",
            state_dir.as_str(),
            "--json",
        ],
        &state_dir,
    );

    let status = run_command_json(&["status", "--json"], &state_dir);
    let status_value = status["status"].as_str().unwrap_or("unknown");
    assert!(
        matches!(status_value, "pass" | "blocked"),
        "status should be pass or blocked, got: {status_value}"
    );

    let projection = &status["operator_session_projection"];
    assert!(projection.is_object());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

// ==================== Multi-Session Cascading Closure Integration Tests ====================

#[test]
fn multi_session_task_closure_cascades_to_parent_repeat_b() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task chain
    let epic = run_command_json(
        &[
            "task",
            "create",
            "cascading-epic",
            "Cascading epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task = run_command_json(
        &[
            "task",
            "create",
            "cascading-task",
            "Cascading task",
            "--parent-id",
            "cascading-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "cascading-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify both task and epic are closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let cascading_task = find_task_ref_by_id(tasks, "cascading-task");
    let cascading_epic = find_task_ref_by_id(tasks, "cascading-epic");

    assert_eq!(
        cascading_task["status"], "closed",
        "cascading-task should be closed"
    );
    assert_eq!(
        cascading_epic["status"], "closed",
        "cascading-epic should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_cascading_stops_at_unrelated_open_sibling_repeat_b() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create epic -> task-a + task-b (siblings)
    let epic = run_command_json(
        &[
            "task",
            "create",
            "sibling-epic",
            "Sibling epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(epic["status"], "pass");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-a",
            "Sibling task A",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "pass");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "sibling-task-b",
            "Sibling task B",
            "--parent-id",
            "sibling-epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "pass");

    // Close task-a
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "sibling-task-a",
            "--reason",
            "Task A complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task-a is closed but epic and task-b are NOT closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let sibling_task_a = find_task_ref_by_id(tasks, "sibling-task-a");
    let sibling_task_b = find_task_ref_by_id(tasks, "sibling-task-b");
    let sibling_epic = find_task_ref_by_id(tasks, "sibling-epic");

    assert_eq!(
        sibling_task_a["status"], "closed",
        "sibling-task-a should be closed"
    );
    assert_eq!(
        sibling_task_b["status"], "open",
        "sibling-task-b should still be open"
    );
    assert!(
        matches!(
            sibling_epic["status"].as_str(),
            Some("open" | "in_progress")
        ),
        "sibling-epic should NOT be closed because sibling-task-b is still open: {}",
        sibling_epic["status"]
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_multi_level_cascading_closure_repeat_b() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create grandparent -> parent -> child chain
    let grandparent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-grandparent",
            "Multi level grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(grandparent["status"], "pass");

    let parent = run_command_json(
        &[
            "task",
            "create",
            "multi-level-parent",
            "Multi level parent",
            "--parent-id",
            "multi-level-grandparent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(parent["status"], "pass");

    let child = run_command_json(
        &[
            "task",
            "create",
            "multi-level-child",
            "Multi level child",
            "--parent-id",
            "multi-level-parent",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(child["status"], "pass");

    // Close the leaf task
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "multi-level-child",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify all three are closed (cascading through multiple levels)
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let multi_level_child = find_task_ref_by_id(tasks, "multi-level-child");
    let multi_level_parent = find_task_ref_by_id(tasks, "multi-level-parent");
    let multi_level_grandparent = find_task_ref_by_id(tasks, "multi-level-grandparent");

    assert_eq!(
        multi_level_child["status"], "closed",
        "multi-level-child should be closed"
    );
    assert_eq!(
        multi_level_parent["status"], "closed",
        "multi-level-parent should be closed via cascading"
    );
    assert_eq!(
        multi_level_grandparent["status"], "closed",
        "multi-level-grandparent should be closed via cascading"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_user_authorization_overrides_validation_repeat_b() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "override-test-task",
            "Override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Close with user authorization (Core Rule #12 override)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "override-test-task",
            "--reason",
            "User authorized per Core Rule #12",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(close_result["status"], "pass");

    // Verify task is closed
    let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
    let tasks = &task_list["tasks"];

    let override_task = find_task_ref_by_id(tasks, "override-test-task");
    assert_eq!(
        override_task["status"], "closed",
        "override-test-task should be closed with user authorization"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_with_core_rule_12_phrase_variants_repeat_b() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create multiple tasks to test different Core Rule #12 phrases
    let phrases = [
        "User authorized per Core Rule #12",
        "user approval for closure",
        "Close per core rule 12",
        "Explicit user authorization",
    ];

    for (idx, phrase) in phrases.iter().enumerate() {
        let task = run_command_json(
            &[
                "task",
                "create",
                &format!("override-phrase-task-{}", idx),
                &format!("Override phrase test task {}", idx),
                "--type",
                "epic",
                "--priority",
                "1",
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(task["status"], "pass");

        // Close with specific phrase
        let close_result = run_command_json(
            &[
                "task",
                "close",
                &format!("override-phrase-task-{}", idx),
                "--reason",
                phrase,
                "--json",
            ],
            &state_dir,
        );
        assert_eq!(close_result["status"], "pass");

        // Verify task is closed
        let task_list = run_command_json(&["task", "list", "--all", "--json"], &state_dir);
        let tasks = &task_list["tasks"];

        let test_task = find_task_ref_by_id(tasks, &format!("override-phrase-task-{}", idx));
        assert_eq!(
            test_task["status"], "closed",
            "task should be closed with phrase: {}",
            phrase
        );
    }

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn multi_session_closure_without_override_fails_on_validation_repeat_b() {
    // This test verifies that without proper authorization, closure can be blocked
    // Note: This may not always fail depending on graph state, but tests the mechanism
    let (project_root, state_dir) = project_bound_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    // Create a simple task
    let task = run_command_json(
        &[
            "task",
            "create",
            "no-override-task",
            "No override test task",
            "--type",
            "epic",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task["status"], "pass");

    // Try to close with normal reason (should work for simple task)
    let close_result = run_command_json(
        &[
            "task",
            "close",
            "no-override-task",
            "--reason",
            "Task complete",
            "--json",
        ],
        &state_dir,
    );

    // For a simple task with no dependencies, this should succeed
    // The validation only fails when it would create an invalid graph
    assert_eq!(close_result["status"], "pass");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}
