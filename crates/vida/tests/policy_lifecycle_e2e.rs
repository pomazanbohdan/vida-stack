use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn unique_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vida-policy-lifecycle-e2e-{}-{nanos}-{count}",
        std::process::id()
    ))
}

fn run_json(args: &[&str], state_dir: &Path) -> Value {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("vida command should run");
    assert_command_success(args, &output);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "command output should be JSON: {args:?}; error={error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_command_success(args: &[&str], output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_bundle(path: &Path, version: u32, source: &str) {
    fs::write(
        path,
        serde_json::to_vec(&json!({
            "schema": 1,
            "policy_id": "rhai.runtime.quality-gate",
            "version": version,
            "engine_abi": "rhai-policy-engine-v1",
            "source": source,
        }))
        .expect("bundle JSON should serialize"),
    )
    .expect("bundle should be written");
}

fn write_fixtures(path: &Path, expected: i64) {
    fs::write(
        path,
        format!(
            "{{\"fixture_id\":\"quality-gate-1\",\"context\":{{\"value\":41}},\"expected\":{expected}}}\n"
        ),
    )
    .expect("fixture corpus should be written");
}

fn write_receipt(path: &Path, bundle_id: &str, digest: &str, test_id: &str) {
    fs::write(
        path,
        serde_json::to_vec(&json!({
            "bundle_id": bundle_id,
            "test_id": test_id,
            "content_digest": digest,
            "passed": true,
        }))
        .expect("receipt JSON should serialize"),
    )
    .expect("receipt should be written");
}

#[test]
fn policy_lifecycle_cli_round_trip_survives_restart_and_rolls_back_lkg() {
    let root = unique_root();
    let state_dir = root.join("state");
    let store = root.join("policy-store.json");
    let bundle_v1 = root.join("bundle-v1.json");
    let bundle_v2 = root.join("bundle-v2.json");
    let fixtures_v1 = root.join("fixtures-v1.jsonl");
    let fixtures_v2 = root.join("fixtures-v2.jsonl");
    let receipt_v1 = root.join("receipt-v1.json");
    let receipt_v2 = root.join("receipt-v2.json");
    fs::create_dir_all(&state_dir).expect("isolated state root should be created");
    write_bundle(&bundle_v1, 1, "ctx.value");
    write_bundle(&bundle_v2, 2, "ctx.value + 1");
    write_fixtures(&fixtures_v1, 41);
    write_fixtures(&fixtures_v2, 42);

    let bundle_v1_arg = bundle_v1.to_string_lossy().into_owned();
    let bundle_v2_arg = bundle_v2.to_string_lossy().into_owned();
    let fixtures_v1_arg = fixtures_v1.to_string_lossy().into_owned();
    let fixtures_v2_arg = fixtures_v2.to_string_lossy().into_owned();
    let store_arg = store.to_string_lossy().into_owned();
    let receipt_v1_arg = receipt_v1.to_string_lossy().into_owned();
    let receipt_v2_arg = receipt_v2.to_string_lossy().into_owned();

    let check_v1 = run_json(
        &["policy", "check", "--bundle", &bundle_v1_arg, "--json"],
        &state_dir,
    );
    assert_eq!(check_v1["status"], "pass");
    assert_eq!(check_v1["policy"]["policy_id"], "rhai.runtime.quality-gate");
    let digest_v1 = check_v1["policy"]["content_digest"]
        .as_str()
        .expect("v1 digest")
        .to_string();
    let check_v1_repeat = run_json(
        &["policy", "check", "--bundle", &bundle_v1_arg, "--json"],
        &state_dir,
    );
    assert_eq!(
        check_v1_repeat["policy"]["content_digest"],
        digest_v1.as_str()
    );

    let test_v1 = run_json(
        &[
            "policy",
            "test",
            "--bundle",
            &bundle_v1_arg,
            "--fixtures",
            &fixtures_v1_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(test_v1["status"], "pass");
    assert_eq!(test_v1["fixture_execution"]["report"]["passed"], 1);
    write_receipt(
        &receipt_v1,
        "rhai.runtime.quality-gate@1",
        &digest_v1,
        "quality-gate-v1",
    );

    let import_v1 = run_json(
        &[
            "policy",
            "import",
            "--bundle",
            &bundle_v1_arg,
            "--store",
            &store_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(import_v1["status"], "pass");
    assert_eq!(import_v1["bundle"]["lifecycle"], "candidate");

    let shadow_status = run_json(
        &["policy", "status", "--store", &store_arg, "--json"],
        &state_dir,
    );
    assert_eq!(shadow_status["status"], "pass");
    assert_eq!(shadow_status["active_pointer"], Value::Null);
    assert_eq!(shadow_status["bundles"][0]["lifecycle"], "candidate");

    let activate_v1 = run_json(
        &[
            "policy",
            "activate",
            "--store",
            &store_arg,
            "--bundle-id",
            "rhai.runtime.quality-gate@1",
            "--test-receipt",
            &receipt_v1_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(activate_v1["status"], "pass");
    assert_eq!(activate_v1["active_pointer"], "rhai.runtime.quality-gate@1");

    let status_after_v1 = run_json(
        &["policy", "status", "--store", &store_arg, "--json"],
        &state_dir,
    );
    assert_eq!(
        status_after_v1["active_pointer"],
        "rhai.runtime.quality-gate@1"
    );

    let check_v2 = run_json(
        &["policy", "check", "--bundle", &bundle_v2_arg, "--json"],
        &state_dir,
    );
    let digest_v2 = check_v2["policy"]["content_digest"]
        .as_str()
        .expect("v2 digest")
        .to_string();
    assert_ne!(digest_v1, digest_v2);
    let test_v2 = run_json(
        &[
            "policy",
            "test",
            "--bundle",
            &bundle_v2_arg,
            "--fixtures",
            &fixtures_v2_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(test_v2["status"], "pass");
    write_receipt(
        &receipt_v2,
        "rhai.runtime.quality-gate@2",
        &digest_v2,
        "quality-gate-v2",
    );
    let import_v2 = run_json(
        &[
            "policy",
            "import",
            "--bundle",
            &bundle_v2_arg,
            "--store",
            &store_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(import_v2["status"], "pass");
    let activate_v2 = run_json(
        &[
            "policy",
            "activate",
            "--store",
            &store_arg,
            "--bundle-id",
            "rhai.runtime.quality-gate@2",
            "--test-receipt",
            &receipt_v2_arg,
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(activate_v2["status"], "pass");
    assert_eq!(activate_v2["active_pointer"], "rhai.runtime.quality-gate@2");
    assert_eq!(
        activate_v2["last_known_good"],
        "rhai.runtime.quality-gate@1"
    );

    let restarted_status = run_json(
        &["policy", "status", "--store", &store_arg, "--json"],
        &state_dir,
    );
    assert_eq!(
        restarted_status["active_pointer"],
        "rhai.runtime.quality-gate@2"
    );
    assert_eq!(
        restarted_status["last_known_good"],
        "rhai.runtime.quality-gate@1"
    );
    let check_v1_after_restart = run_json(
        &["policy", "check", "--bundle", &bundle_v1_arg, "--json"],
        &state_dir,
    );
    assert_eq!(
        check_v1_after_restart["policy"]["content_digest"],
        digest_v1.as_str()
    );
    let restarted_v2 = restarted_status["bundles"]
        .as_array()
        .expect("restart status should list bundles")
        .iter()
        .find(|bundle| bundle["bundle_id"] == "rhai.runtime.quality-gate@2")
        .expect("restart status should retain v2");
    assert_eq!(restarted_v2["content_digest"], digest_v2.as_str());

    let rollback = run_json(
        &[
            "policy",
            "rollback",
            "--store",
            &store_arg,
            "--bundle-id",
            "rhai.runtime.quality-gate@2",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(rollback["status"], "pass");
    assert_eq!(rollback["active_pointer"], "rhai.runtime.quality-gate@1");
    assert_eq!(rollback["last_known_good"], "rhai.runtime.quality-gate@1");

    let final_status = run_json(
        &["policy", "status", "--store", &store_arg, "--json"],
        &state_dir,
    );
    assert_eq!(
        final_status["active_pointer"],
        "rhai.runtime.quality-gate@1"
    );
    assert_eq!(final_status["bundles"][1]["lifecycle"], "quarantined");
    assert_eq!(final_status["bundles"][0]["content_digest"], digest_v1);

    fs::remove_dir_all(root).expect("isolated lifecycle root should be cleaned up");
}
