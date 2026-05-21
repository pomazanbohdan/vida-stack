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

fn project_bound_state_dir() -> (String, String) {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    fs::create_dir_all(&state_dir).expect("create project-bound state dir");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("write AGENTS.md");
    fs::write(
        format!("{project_root}/vida.config.yaml"),
        "project:\n  id: test\n",
    )
    .expect("write vida.config.yaml");
    for relative in [".vida/config", ".vida/db", ".vida/project"] {
        fs::create_dir_all(format!("{project_root}/{relative}"))
            .expect("runtime project marker dir should exist");
    }
    (project_root, state_dir)
}

static PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn sample_jsonl(path: &str) {
    fs::write(
        path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
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

fn write_operator_projection(state_dir: &str, projection_name: &str, payload: &serde_json::Value) {
    let projection_dir = format!("{state_dir}/operator-projections");
    fs::create_dir_all(&projection_dir).expect("operator projection dir should exist");
    fs::write(
        format!("{projection_dir}/{projection_name}.json"),
        serde_json::to_string_pretty(payload).expect("operator projection should render"),
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
            "selected_model_ref": "gpt-5.4",
            "selected_model_provider": "openai",
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
        .unwrap_or_else(|| panic!("scheduling candidates missing or not an array"))
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
    stderr.contains(vida_test_support::STATE_LOCK_ERROR_MESSAGE)
        || stderr.contains("timed out while waiting for authoritative datastore lock")
}

fn run_with_state_lock_retry<F>(mut builder: F) -> std::process::Output
where
    F: FnMut() -> Command,
{
    vida_test_support::command_output_with_retry_errors(
        &mut builder,
        STATE_LOCK_RETRY_LIMIT,
        |output| !output.status.success() && is_state_lock_error(output),
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

    let list_stdout = run_and_assert_success(&["task", "list", "--json"], &state_dir);
    assert!(
        list_stdout.contains("\"id\": \"vida-b\"") || list_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        list_stdout.contains("\"id\": \"vida-a\"") || list_stdout.contains("\"id\":\"vida-a\"")
    );

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
        &[
            "task",
            "dep",
            "add",
            "vida-c",
            "vida-root",
            "parent-child",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        dep_add_stdout.contains("\"issue_id\": \"vida-c\"")
            || dep_add_stdout.contains("\"issue_id\":\"vida-c\"")
    );
    assert!(
        dep_add_stdout.contains("\"depends_on_id\": \"vida-root\"")
            || dep_add_stdout.contains("\"depends_on_id\":\"vida-root\"")
    );

    let deps_after_add_stdout =
        run_and_assert_success(&["task", "deps", "vida-c", "--json"], &state_dir);
    assert!(
        deps_after_add_stdout.contains("\"depends_on_id\": \"vida-root\"")
            || deps_after_add_stdout.contains("\"depends_on_id\":\"vida-root\"")
    );

    let dep_remove_stdout = run_and_assert_success(
        &[
            "task",
            "dep",
            "remove",
            "vida-c",
            "vida-root",
            "parent-child",
            "--json",
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
        deps_after_remove_stdout.contains("\"dependency_count\": 0")
            || deps_after_remove_stdout.contains("\"dependency_count\":0")
    );

    let _ = fs::remove_dir_all(&state_dir);
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
        &["task", "children", "sandbox-parent", "--json"],
        &state_dir,
    );
    assert_eq!(parent_children["status"], "pass");
    assert_eq!(parent_children["surface"], "vida task children");
    assert_eq!(parent_children["root_task_id"], "sandbox-parent");
    assert_eq!(parent_children["child_count"], 1);
    assert_eq!(parent_children["children"][0]["child_id"], "sandbox-child");
    assert_eq!(parent_children["children"][0]["child_status"], "open");

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
        close_parent_stderr.contains("open child tasks exist")
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
fn taskflow_golden_route_happy_path_stitches_bootstrap_dispatch_resume_status_and_doctor() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let orchestrator = run_command_json(&["orchestrator-init", "--json"], &state_dir);
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
    assert!(rejected_stderr.contains("open child tasks exist"));
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

    let summary_ready =
        find_scheduling_candidate(&graph_summary["scheduling"]["ready"], "sandbox-graph-ready");
    let summary_ready_blockers = require_json_string_array(
        &summary_ready["parallel_blockers"],
        "summary ready parallel_blockers",
    );
    assert_eq!(summary_ready["ready_now"], true);
    assert_eq!(summary_ready["ready_parallel_safe"], false);
    assert_eq!(
        summary_ready_blockers,
        vec!["current_task_reference".to_string()]
    );

    let summary_serial = find_scheduling_candidate(
        &graph_summary["scheduling"]["ready"],
        "sandbox-graph-serial",
    );
    let summary_serial_blockers = require_json_string_array(
        &summary_serial["parallel_blockers"],
        "summary serial parallel_blockers",
    );
    assert_eq!(summary_serial["ready_now"], true);
    assert_eq!(summary_serial["ready_parallel_safe"], false);
    assert!(summary_serial_blockers
        .iter()
        .any(|blocker| blocker == "execution_mode_not_parallel_safe"));

    let summary_parallel = find_scheduling_candidate(
        &graph_summary["scheduling"]["ready"],
        "sandbox-graph-parallel",
    );
    assert_eq!(summary_parallel["ready_now"], true);
    assert_eq!(summary_parallel["ready_parallel_safe"], true);
    assert!(require_json_string_array(
        &summary_parallel["parallel_blockers"],
        "summary parallel parallel_blockers"
    )
    .is_empty());

    let summary_blocked = find_scheduling_candidate(
        &graph_summary["scheduling"]["blocked"],
        "sandbox-graph-blocked",
    );
    let summary_blocked_blockers = require_json_string_array(
        &summary_blocked["parallel_blockers"],
        "summary blocked parallel_blockers",
    );
    assert_eq!(summary_blocked["ready_now"], false);
    assert_eq!(
        summary_blocked["blocked_by"][0]["depends_on_id"],
        "sandbox-graph-ready"
    );
    assert_eq!(summary_blocked_blockers, vec!["graph_blocked".to_string()]);

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
        rejected_create_parent_close_stderr.contains("open child tasks exist")
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
        rejected_update_parent_close_stderr.contains("open child tasks exist")
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
        "single TaskFlow in_progress task is the lawful continuation"
    );
    assert_eq!(
        next_lawful["sequential_vs_parallel_posture"],
        "sequential_only_active_task"
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
        rejected_parent_close_stderr.contains("open child tasks exist")
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
fn task_update_title_priority() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

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
fn validate_graph_broken_edge_matches_golden_fixture() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        "{\"id\":\"vida-broken\",\"title\":\"Broken task\",\"description\":\"broken\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-broken\",\"depends_on_id\":\"vida-missing\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
    )
    .expect("write broken task jsonl");

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "validate-graph", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("validate-graph should run");
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("validate-graph json should parse");
    assert_release1_shared_envelope_fields(&actual_json, "blocked validate-graph");
    let expected =
        include_str!("../../../tests/golden/taskflow/validate_graph_missing_dependency.json")
            .trim_end();
    assert_eq!(
        serde_json::to_string_pretty(&actual_json["issues"]).expect("actual json should render"),
        normalize_json_fixture(expected)
    );

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
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

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

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn release_admitted_missing_stale_run_does_not_block_recovery_or_dispatch_preview() {
    let (project_root, state_dir) = project_bound_state_dir();

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let ready_task_id = "case11-ready-after-release";
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
        format!("{runtime_consumption_dir}/final-2026-05-19T00-00-00Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
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
    assert_eq!(
        recovery["status"],
        serde_json::Value::Null,
        "release-admitted missing stale run should not remain latest recovery"
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
        dispatch["next_actions"]
            .as_array()
            .expect("dispatch next_actions should render")
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|value| value.contains(ready_task_id))),
        "dispatch-next should continue evaluating the ready successor task: {dispatch}"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn case11_agent_init_timeout_bridge_remains_blocked_evidence_without_impossible_continuation() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let task_id = "taskflow-case-11-actual-agent-autonomy";
    let run_id = task_id;
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
fn task_next_lawful_prefers_authoritative_active_task_over_stale_missing_source_drift() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let active_task_id = "autonomy-active-task";
    let active = run_command_json(
        &[
            "task",
            "create",
            active_task_id,
            "Autonomy active task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active["status"], "pass");
    assert_eq!(active["task"]["status"], "in_progress");

    let explicit_run_id = "stale-explicit-run";
    let explicit_task_id = "stale-explicit-task";
    let current_run_id = "stale-current-task";
    let current_task_id = "stale-current-task";
    assert_ne!(explicit_run_id, current_run_id);
    assert_ne!(explicit_task_id, current_task_id);
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "init",
            current_task_id,
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
            "run_id": explicit_run_id,
            "task_id": explicit_task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": explicit_task_id,
                "run_id": explicit_run_id,
                "active_node": "implementation"
            },
            "binding_source": "explicit_continuation_bind_task",
            "why_this_unit": "stale explicit continuation references a missing task",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:02Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", explicit_run_id))
            .bind(("binding", explicit_binding))
            .await
            .expect("seed stale explicit continuation binding");
        let current_binding = serde_json::json!({
            "run_id": current_run_id,
            "task_id": current_task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "run_graph_task",
                "task_id": current_task_id,
                "run_id": current_run_id,
                "active_node": "implementation"
            },
            "binding_source": "latest_run_graph_status",
            "why_this_unit": "stale latest-run continuation references a different missing task",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only_open_cycle",
            "recorded_at": "2026-05-19T00:00:01Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", current_run_id))
            .bind(("binding", current_binding))
            .await
            .expect("seed stale current continuation binding");
        drop(db);
    });

    for missing_task_id in [explicit_task_id, current_task_id] {
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
fn task_next_lawful_prefers_active_task_over_closed_downstream_closure_binding() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);
    let closed_task_id = "closed-downstream-reconciled-task";
    let closed_task = run_command_json(
        &[
            "task",
            "create",
            closed_task_id,
            "Closed downstream reconciled task",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_task["status"], "pass");
    assert_eq!(closed_task["task"]["status"], "closed");

    let active_task_id = "active-task-after-closed-downstream";
    let active_task = run_command_json(
        &[
            "task",
            "create",
            active_task_id,
            "Active task after closed downstream",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(active_task["status"], "pass");
    assert_eq!(active_task["task"]["status"], "in_progress");

    let closed_run_id = "closed-downstream-closure-run";
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
            "run_id": closed_run_id,
            "task_id": closed_task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "downstream_dispatch_target",
                "task_id": closed_task_id,
                "run_id": closed_run_id,
                "dispatch_target": "closure"
            },
            "binding_source": "task_close_reconcile",
            "why_this_unit": "closed task reconciled into downstream closure before a different active task continued",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-19T00:00:03Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", closed_run_id))
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
    let closed_task_id = "closed-downstream-only-task";
    let closed_task = run_command_json(
        &[
            "task",
            "create",
            closed_task_id,
            "Closed downstream only task",
            "--type",
            "task",
            "--status",
            "closed",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed_task["status"], "pass");
    assert_eq!(closed_task["task"]["status"], "closed");

    let closed_run_id = "closed-downstream-only-run";
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
            "run_id": closed_run_id,
            "task_id": closed_task_id,
            "status": "bound",
            "active_bounded_unit": {
                "kind": "downstream_dispatch_target",
                "task_id": closed_task_id,
                "run_id": closed_run_id,
                "dispatch_target": "closure"
            },
            "binding_source": "task_close_reconcile",
            "why_this_unit": "closed task reconciled into downstream closure with no active successor",
            "primary_path": "normal_delivery_path",
            "sequential_vs_parallel_posture": "sequential_only",
            "recorded_at": "2026-05-19T00:00:04Z"
        });
        db.query("UPSERT type::record('run_graph_continuation_binding', $run) CONTENT $binding")
            .bind(("run", closed_run_id))
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
    assert_eq!(doctor["task_store"]["closed_count"], 1);
    assert_no_run_id_consume_continue_command(&doctor, run_id, "doctor");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_list_show_ready_prefer_authoritative_state_over_stale_snapshot() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

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

    let listed = run_command_json(&["task", "list", "--json"], &state_dir);
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
        .args(["task", "list", "--json"])
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
    let _ = run_command_json(
        &[
            "task",
            "create",
            "feedback-audit-language-close",
            "Feedback audit language close",
            "--status",
            "in_progress",
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
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"beta\"],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"closed\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"closed_at\":\"2026-03-09T00:00:00Z\",\"close_reason\":\"done\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
        ),
    )
    .expect("write task jsonl");

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_list = run_and_assert_success(&["task", "list", "--json"], &state_dir);

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
        serde_yaml::Value::String("opencode".to_string()),
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
            "status-opencode",
            "--host-cli-system",
            "opencode",
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
    assert_eq!(host_agents["host_cli_system"], "opencode");
    assert_eq!(host_agents["runtime_surface"], ".opencode");
    assert_eq!(host_agents["root_session_write_guard"]["status"], "missing");
    assert_eq!(parsed["root_session_write_guard"]["status"], "missing");
    let runtime_root = host_agents["runtime_root"]
        .as_str()
        .expect("runtime_root present");
    assert!(runtime_root.contains(".opencode"));
    let system_entry = &host_agents["system_entry"];
    assert!(system_entry.is_object());
    assert_eq!(
        system_entry["template_root"]
            .as_str()
            .expect("template_root"),
        ".opencode"
    );
    assert_eq!(
        system_entry["runtime_root"].as_str().expect("runtime_root"),
        ".opencode"
    );
    assert_eq!(
        system_entry["materialization_mode"]
            .as_str()
            .expect("materialization_mode"),
        "copy_tree_only"
    );
    assert_eq!(system_entry["enabled"].as_bool(), Some(true));
    assert_eq!(
        system_entry["carriers"]["opencode-primary"]["tier"]
            .as_str()
            .expect("carrier tier"),
        "opencode"
    );
    assert_eq!(
        system_entry["carriers"]["opencode-primary"]["rate"].as_i64(),
        Some(4)
    );
    let agents = host_agents["agents"]
        .as_object()
        .expect("agents summary should render");
    let opencode = agents
        .get("opencode-primary")
        .expect("opencode carrier summary should render");
    assert_eq!(opencode["tier"].as_str().expect("tier"), "opencode");
    assert_eq!(opencode["rate"].as_i64(), Some(4));
    assert_eq!(
        opencode["default_runtime_role"]
            .as_str()
            .expect("default runtime role"),
        "worker"
    );
    assert_eq!(opencode["feedback_count"].as_u64(), Some(0));
    assert_eq!(opencode["effective_score"].as_u64(), Some(70));
    assert_eq!(opencode["lifecycle_state"].as_str(), Some("probation"));
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
    assert_eq!(dry_run_receipt["before_task_count"], 1);
    assert_eq!(dry_run_receipt["after_task_count"], 2);
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
fn task_create_accepts_metadata_one_shot_for_shell_safe_intake() {
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

    let parsed = run_command_json(
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
            "cargo test -p vida task_create_accepts_metadata_one_shot_for_shell_safe_intake",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["surface"], "vida task create");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["task"]["notes"], "one-shot notes\n");
    assert_eq!(
        parsed["task"]["planner_metadata"]["owned_paths"],
        serde_json::json!(["crates/vida/src/task_surface.rs"])
    );
    assert_eq!(
        parsed["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!(["create sets planner metadata"])
    );
    assert_eq!(
        parsed["task"]["planner_metadata"]["proof_targets"],
        serde_json::json!([
            "cargo test -p vida task_create_accepts_metadata_one_shot_for_shell_safe_intake"
        ])
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
