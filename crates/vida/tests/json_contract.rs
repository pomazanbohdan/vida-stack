use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use vida_test_support::{self as support, CliContractCase};

fn vida() -> Command {
    support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}/vida-json-contract-{}-{nanos}-{counter}",
        std::env::temp_dir().display(),
        std::process::id()
    )
}

fn run_json(state_dir: &str, args: &[&str]) -> serde_json::Value {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .unwrap_or_else(|error| panic!("{} should run: {error}", args.join(" ")));
    parse_json_output(args, &output)
}

fn parse_json_output(args: &[&str], output: &Output) -> serde_json::Value {
    assert!(
        !output.stdout.is_empty(),
        "{} should emit JSON on stdout; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{} stdout should parse as JSON: {error}\nstdout={}\nstderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn json_contract_major_operator_surfaces_keep_release1_shape() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: {}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let cases = [
        CliContractCase {
            args: &["status", "--json"],
            surface: "vida status",
        },
        CliContractCase {
            args: &["doctor", "--json"],
            surface: "vida doctor",
        },
        CliContractCase {
            args: &["task", "ready", "--json"],
            surface: "vida task ready",
        },
        CliContractCase {
            args: &["task", "validate-graph", "--json"],
            surface: "vida task validate-graph",
        },
        CliContractCase {
            args: &["taskflow", "graph-summary", "--json"],
            surface: "vida taskflow graph-summary",
        },
        CliContractCase {
            args: &["taskflow", "scheduler", "dispatch", "--json"],
            surface: "vida taskflow scheduler dispatch",
        },
    ];
    support::assert_cli_contract_matrix(cases, |args| run_json(&state_dir, args));

    let reset_state_dir = unique_state_dir();
    std::fs::create_dir_all(&reset_state_dir).expect("reset state dir should exist");
    let reset = vida()
        .args([
            "state",
            "reset",
            "--archive",
            "--reinit",
            "--state-dir",
            &reset_state_dir,
            "--json",
        ])
        .output()
        .expect("state reset should run");
    assert!(
        reset.status.success(),
        "state reset should succeed: {}",
        String::from_utf8_lossy(&reset.stderr)
    );
    let reset_value = parse_json_output(&["state", "reset", "--json"], &reset);
    support::assert_release1_operator_shape("vida state reset", &reset_value);
}

#[test]
fn json_contract_harness_rejects_missing_operator_fields() {
    let valid = json!({
        "surface": "vida test",
        "status": "blocked",
        "blocker_codes": ["example_blocker"],
        "next_actions": ["repair the example"],
        "artifact_refs": {},
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": ["example_blocker"],
            "next_actions": ["repair the example"],
            "artifact_refs": {}
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": ["example_blocker"],
            "next_actions": ["repair the example"],
            "artifact_refs": {}
        }
    });
    assert_eq!(support::release1_operator_shape_error(&valid), None);

    let mut missing_blockers = valid.clone();
    missing_blockers
        .as_object_mut()
        .expect("valid object")
        .remove("blocker_codes");
    assert_eq!(
        support::release1_operator_shape_error(&missing_blockers).as_deref(),
        Some("missing blocker_codes")
    );

    let mut missing_actions = valid;
    missing_actions
        .as_object_mut()
        .expect("valid object")
        .remove("next_actions");
    assert_eq!(
        support::release1_operator_shape_error(&missing_actions).as_deref(),
        Some("missing next_actions")
    );
}
