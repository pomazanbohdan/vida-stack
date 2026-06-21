use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use vida_test_support as support;

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

fn release1_shape_error(value: &serde_json::Value) -> Option<String> {
    let object = value.as_object()?;
    for key in [
        "surface",
        "status",
        "blocker_codes",
        "next_actions",
        "artifact_refs",
        "shared_fields",
        "operator_contracts",
    ] {
        if !object.contains_key(key) {
            return Some(format!("missing {key}"));
        }
    }
    if !value["surface"].is_string() {
        return Some("surface must be a string".to_string());
    }
    if !value["status"].is_string() {
        return Some("status must be a string".to_string());
    }
    if !value["blocker_codes"].is_array() {
        return Some("blocker_codes must be an array".to_string());
    }
    if !value["next_actions"].is_array() {
        return Some("next_actions must be an array".to_string());
    }
    if !value["artifact_refs"].is_object() {
        return Some("artifact_refs must be an object".to_string());
    }

    let shared_fields = &value["shared_fields"];
    let operator_contracts = &value["operator_contracts"];
    for mirrored in ["status", "blocker_codes", "next_actions", "artifact_refs"] {
        if value[mirrored] != shared_fields[mirrored] {
            return Some(format!("shared_fields.{mirrored} drifted"));
        }
        if value[mirrored] != operator_contracts[mirrored] {
            return Some(format!("operator_contracts.{mirrored} drifted"));
        }
    }
    if operator_contracts["contract_id"] != "release-1-operator-contracts" {
        return Some("operator_contracts.contract_id drifted".to_string());
    }
    if operator_contracts["schema_version"] != "release-1-v1" {
        return Some("operator_contracts.schema_version drifted".to_string());
    }
    None
}

fn assert_release1_shape(surface: &str, value: &serde_json::Value) {
    assert_eq!(value["surface"], surface);
    let error = release1_shape_error(value);
    assert!(
        error.is_none(),
        "{surface} must keep release-1 JSON shape: {} got: {value:#}",
        error.unwrap_or_default()
    );
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

    let cases: &[(&[&str], &str)] = &[
        (&["status", "--json"], "vida status"),
        (&["doctor", "--json"], "vida doctor"),
        (&["task", "ready", "--json"], "vida task ready"),
        (
            &["task", "validate-graph", "--json"],
            "vida task validate-graph",
        ),
        (
            &["taskflow", "graph-summary", "--json"],
            "vida taskflow graph-summary",
        ),
        (
            &["taskflow", "scheduler", "dispatch", "--json"],
            "vida taskflow scheduler dispatch",
        ),
    ];
    for (args, surface) in cases {
        let value = run_json(&state_dir, args);
        assert_release1_shape(surface, &value);
    }

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
    assert_release1_shape("vida state reset", &reset_value);
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
    assert_eq!(release1_shape_error(&valid), None);

    let mut missing_blockers = valid.clone();
    missing_blockers
        .as_object_mut()
        .expect("valid object")
        .remove("blocker_codes");
    assert_eq!(
        release1_shape_error(&missing_blockers).as_deref(),
        Some("missing blocker_codes")
    );

    let mut missing_actions = valid;
    missing_actions
        .as_object_mut()
        .expect("valid object")
        .remove("next_actions");
    assert_eq!(
        release1_shape_error(&missing_actions).as_deref(),
        Some("missing next_actions")
    );
}
