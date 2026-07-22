use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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

fn run_text(args: &[&str], state_dir: &Path) -> String {
    let output = vida()
        .args(args)
        .current_dir(repo_root())
        .env("VIDA_ROOT", repo_root())
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("run vida command");
    assert!(
        output.status.success(),
        "vida command failed: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_json(args: &[&str], state_dir: &Path) -> Value {
    let output = run_text(args, state_dir);
    serde_json::from_str(&output)
        .unwrap_or_else(|error| panic!("vida command should emit JSON: {error}\noutput={output}"))
}

#[test]
fn run_graph_specification_dispatch_convergence_smoke() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let state_dir = std::env::temp_dir().join(format!(
        "vida-specification-dispatch-convergence-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&state_dir).expect("state dir should be created");

    let task_id = format!("specification-dispatch-convergence-{nonce}");
    let seed = run_json(
        &[
            "taskflow",
            "run-graph",
            "seed",
            task_id.as_str(),
            "Configured specification first-node dispatch",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(seed["payload"]["status"]["status"], "ready");

    let meta = r#"{"next_node":"coder","lane_id":"coder_lane","lifecycle_stage":"coder_dispatch_ready","handoff_state":"awaiting_coder","resume_target":"dispatch.coder_lane","context_state":"sealed","checkpoint_kind":"execution_cursor","policy_gate":"not_required","recovery_ready":true}"#;
    run_text(
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

    let dispatch = run_json(
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            task_id.as_str(),
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(
        dispatch["run_graph_bootstrap"]["status"],
        "dispatch_init_ready"
    );
    assert_eq!(
        dispatch["run_graph_bootstrap"]["latest_status"]["active_node"],
        "coder"
    );
    assert_eq!(
        dispatch["run_graph_bootstrap"]["latest_status"]["context_state"],
        "sealed"
    );
    assert_eq!(
        dispatch["run_graph_bootstrap"]["latest_status"]["resume_target"],
        "dispatch.coder_lane"
    );
    assert_eq!(dispatch["dispatch_receipt"]["dispatch_target"], "coder");
    assert!(dispatch["dispatch_packet_path"]
        .as_str()
        .is_some_and(|path| { Path::new(path).exists() }));

    fs::remove_dir_all(&state_dir).expect("isolated state dir should be removed");
}
