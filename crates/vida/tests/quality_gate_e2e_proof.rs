use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

const FIXTURE: &str =
    include_str!("../../../vida/policies/builtin/v1/fixtures/quality-gate-e2e-matrix.jsonl");
const RAW_EVIDENCE_MARKER: &str = "RAW_EVIDENCE_SECRET_MUST_NOT_ESCAPE_PUBLIC_PROJECTIONS";

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn canonical_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn unique_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vida-quality-gate-e2e-proof-{nanos}-{count}"))
}

fn run_json(args: &[&str], root: &Path) -> Value {
    let state_dir = root.join("state");
    let output = vida()
        .args(args)
        .env("VIDA_ROOT", canonical_root())
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("vida command should run");
    assert_success(args, &output);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "command should emit JSON: {args:?}; error={error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_success(args: &[&str], output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> Value {
    serde_json::from_str(
        FIXTURE
            .lines()
            .next()
            .expect("quality-gate fixture should contain one JSONL row"),
    )
    .expect("quality-gate fixture row should be valid JSON")
}

fn matrix(fixture: &Value) -> Value {
    let profiles = fixture["profiles"]
        .as_array()
        .expect("fixture profiles should be an array");
    let checks = fixture["checks"]
        .as_array()
        .expect("fixture checks should be an array");
    let mut profile_rows = serde_json::Map::new();
    let mut check_rows = serde_json::Map::new();
    for profile in profiles.iter().filter_map(Value::as_str) {
        profile_rows.insert(
            profile.to_string(),
            json!({"status":"pass","evidence_refs":[format!("{profile}-evidence")]}),
        );
    }
    for check in checks.iter().filter_map(Value::as_str) {
        check_rows.insert(
            check.to_string(),
            json!({"status":"pass","evidence_refs":[format!("{check}-check")]}),
        );
    }
    let mut categories = serde_json::Map::new();
    for category in fixture["zombie_d"]["base_categories"]
        .as_array()
        .expect("base categories should be an array")
        .iter()
        .filter_map(Value::as_str)
    {
        categories.insert(
            category.to_string(),
            if category == "M" {
                json!({"status":"na","reason":"single fixture path"})
            } else {
                json!({"status":"pass","evidence_refs":[format!("{category}-evidence")]})
            },
        );
    }
    for category in fixture["zombie_d"]["optional_categories"]
        .as_array()
        .expect("optional categories should be an array")
        .iter()
        .filter_map(Value::as_str)
    {
        categories.insert(
            category.to_string(),
            json!({"status":"pass","evidence_refs":[format!("{category}-evidence")]}),
        );
    }
    json!({
        "schema_version": 1,
        "metadata": {"schema_version": 1, "applicable_categories": fixture["zombie_d"]["applicable_categories"]},
        "categories": categories,
        "quality_gate": {"schema_version": 1, "profiles": profile_rows, "checks": check_rows, "doubts": []},
        "doubts": []
    })
}

#[test]
fn quality_gate_e2e_matrix_covers_profiles_zombie_d_public_projection_and_restart() {
    let fixture = fixture();
    let expected_profiles = fixture["expected"]["effective_profiles"]
        .as_array()
        .expect("expected profiles should be an array");
    assert_eq!(expected_profiles.len(), 8);
    assert_eq!(fixture["profiles"].as_array().unwrap().len(), 8);
    assert_eq!(fixture["checks"].as_array().unwrap(), expected_profiles);
    assert!(fixture["expected"]["native_parity"].as_bool().unwrap());
    assert!(fixture["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|task| task["multi_profile"].as_bool() == Some(true)));

    let root = unique_root();
    fs::create_dir_all(root.join("state")).expect("state root should be created");

    let parent_id = "quality-gate-e2e-parent";
    let task_id = "quality-gate-e2e-all-profiles";
    assert_eq!(
        run_json(
            &[
                "task",
                "create",
                parent_id,
                "Quality gate E2E parent",
                "--type",
                "epic",
                "--json"
            ],
            &root,
        )["status"],
        "pass"
    );
    let created = run_json(
        &[
            "task", "create", task_id,
            "Quality gate replay persistence cross-surface all profiles",
            "--type", "task", "--status", "in_progress", "--parent-id", parent_id,
            "--description",
            "contract security a11y visual performance resilience property observability replay persistence cross-surface",
            "--owned-path", "crates/vida/tests/quality_gate_e2e_proof.rs",
            "--proof-target", "zombie_d_matrix", "--json",
        ],
        &root,
    );
    assert_eq!(created["status"], "pass");

    let evidence = matrix(&fixture).to_string();
    let attached = run_json(
        &[
            "task",
            "proof",
            "attach-evidence",
            task_id,
            "--proof-target",
            "zombie_d_matrix",
            "--result",
            "pass",
            "--command",
            "quality-gate-e2e-fixture",
            "--artifact-ref",
            RAW_EVIDENCE_MARKER,
            "--evidence",
            &evidence,
            "--json",
        ],
        &root,
    );
    assert_eq!(attached["status"], "pass");
    assert_eq!(attached["proof_target"], "zombie_d_matrix");

    let proof_status = run_json(&["task", "proof", "status", task_id, "--json"], &root);
    assert_eq!(proof_status["status"], "pass");
    assert_eq!(
        proof_status["proof_targets"][0]["target"],
        "zombie_d_matrix"
    );
    assert_eq!(proof_status["proof_targets"][0]["status"], "satisfied");
    assert!(!proof_status.to_string().contains(RAW_EVIDENCE_MARKER));

    let compact = run_json(
        &["task", "show", task_id, "--view", "compact", "--json"],
        &root,
    );
    assert_eq!(compact["task_status"], "in_progress");
    assert!(!compact.to_string().contains(RAW_EVIDENCE_MARKER));
    assert!(compact["task"].get("notes").is_none());

    let closed = run_json(
        &[
            "task",
            "close",
            task_id,
            "--reason",
            "quality gate matrix proven",
            "--json",
        ],
        &root,
    );
    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["task"]["status"], "closed");

    let restarted_proof_status = run_json(&["task", "proof", "status", task_id, "--json"], &root);
    assert_eq!(restarted_proof_status["status"], "pass");
    assert_eq!(
        restarted_proof_status["proof_targets"][0]["status"],
        "satisfied"
    );
    assert!(!restarted_proof_status
        .to_string()
        .contains(RAW_EVIDENCE_MARKER));

    let projection = run_json(&["taskflow", "consume", "agent-system", "--json"], &root);
    assert!(projection["snapshot"].is_object());
    assert_eq!(
        projection["snapshot"]["dev_team_readiness"]["zombie_d_gate"]["status"],
        "ready"
    );
    assert!(!projection.to_string().contains(RAW_EVIDENCE_MARKER));

    fs::remove_dir_all(root).expect("temporary project root should be cleaned up");
}
