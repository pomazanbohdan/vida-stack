use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root should resolve")
}

fn run_command(args: &[&str], state_dir: &Path) -> Output {
    vida()
        .args(args)
        .current_dir(repo_root())
        .env("VIDA_ROOT", repo_root())
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("run vida command")
}

fn run_success_text(args: &[&str], state_dir: &Path) -> String {
    let output = run_command(args, state_dir);
    assert!(
        output.status.success(),
        "vida command failed: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_success_json(args: &[&str], state_dir: &Path) -> Value {
    let output = run_command(args, state_dir);
    assert!(
        output.status.success(),
        "vida command failed: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "vida command should emit JSON: {error}\noutput={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn run_blocked_json(args: &[&str], state_dir: &Path) -> Value {
    let output = run_command(args, state_dir);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "blocked vida command should emit JSON: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn run_graph_resume_convergence_smoke() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let state_dir = std::env::temp_dir().join(format!(
        "vida-run-graph-resume-convergence-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&state_dir).expect("state dir should be created");

    let parent_id = format!("run-graph-resume-convergence-epic-{nonce}");
    let task_id = format!("run-graph-resume-convergence-{nonce}");
    run_success_json(
        &[
            "task",
            "create",
            parent_id.as_str(),
            "Routed receipt resume convergence smoke",
            "--type",
            "epic",
            "--status",
            "in_progress",
            "--description",
            "isolated integration smoke parent",
            "--json",
        ],
        &state_dir,
    );
    run_success_json(
        &[
            "task",
            "create",
            task_id.as_str(),
            "Routed receipt resume convergence smoke task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--parent-id",
            parent_id.as_str(),
            "--description",
            "isolated integration smoke task",
            "--owned-path",
            "crates/vida/src/taskflow_consume_resume.rs",
            "--json",
        ],
        &state_dir,
    );
    let seed = run_success_json(
        &[
            "taskflow",
            "run-graph",
            "seed",
            task_id.as_str(),
            "Configured routed receipt resume convergence",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(seed["payload"]["status"]["status"], "ready");

    let meta = r#"{"next_node":"coder","lane_id":"coder_lane","lifecycle_stage":"coder_dispatch_ready","handoff_state":"awaiting_coder","resume_target":"dispatch.coder_lane","context_state":"sealed","checkpoint_kind":"execution_cursor","policy_gate":"not_required","recovery_ready":true}"#;
    run_success_text(
        &[
            "taskflow",
            "run-graph",
            "update",
            task_id.as_str(),
            "specification",
            "coder",
            "ready",
            "specification",
            meta,
        ],
        &state_dir,
    );

    let dispatch = run_success_json(
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            task_id.as_str(),
            "--json",
        ],
        &state_dir,
    );
    let dispatch_receipt = &dispatch["dispatch_receipt"];
    assert_eq!(dispatch_receipt["dispatch_status"], "routed");
    assert_eq!(dispatch_receipt["lane_status"], "lane_running");
    let packet_path = dispatch_receipt["dispatch_packet_path"]
        .as_str()
        .expect("routed dispatch should persist a packet path");
    assert!(Path::new(packet_path).exists());

    let consume = run_blocked_json(
        &[
            "taskflow",
            "consume",
            "continue",
            "--run-id",
            task_id.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(consume["status"], "blocked");
    assert!(
        consume["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.iter().any(|code| code == "open_delegated_cycle")),
        "routed handoff without completion evidence must stay fail-closed"
    );
    assert_eq!(consume["dispatch_receipt"]["dispatch_status"], "routed");
    assert_eq!(consume["dispatch_receipt"]["lane_status"], "lane_open");
    assert!(consume["dispatch_receipt"]["dispatch_result_path"].is_null());

    fs::remove_dir_all(&state_dir).expect("isolated state dir should be removed");
}
