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

#[test]
fn requirement_analysis_cli_contract() {
    let json_output = vida()
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "ra-artifact-schema-cli-design-20260630",
            "--input",
            "operator request",
            "--json",
        ])
        .output()
        .expect("requirement analyze json should run");
    assert!(
        json_output.status.success(),
        "requirement analyze json should succeed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let value = parse_json_output(&["requirement", "analyze", "--json"], &json_output);
    support::assert_release1_operator_shape("vida requirement analyze", &value);
    assert_eq!(value["status"].as_str(), Some("pass"));
    let artifact = &value["artifact"];
    for field in [
        "source_inputs",
        "requirement_classification",
        "depth_mode",
        "requirement_atoms",
        "selected_methods",
        "selected_roles",
        "role_findings_summary",
        "detected_conflicts",
        "open_questions",
        "working_assumptions",
        "solution_options",
        "recommended_option",
        "readiness_verdict",
        "downstream_routes",
        "acceptance_criteria",
        "test_matrix",
        "output_contract",
        "codebase_impact",
        "developer_handoff",
    ] {
        assert!(artifact.get(field).is_some(), "missing field {field}");
    }
    assert_eq!(
        artifact["task_id"].as_str(),
        Some("ra-artifact-schema-cli-design-20260630")
    );
    assert!(artifact["request_id"].is_null());
    assert!(artifact["open_questions"]["critical"].is_array());
    assert!(artifact["open_questions"]["important"].is_array());
    assert!(artifact["open_questions"]["optional"].is_array());
    assert_eq!(
        artifact["output_contract"]["default"]["mode"],
        "compact_toon_plain"
    );
    assert_eq!(
        artifact["output_contract"]["json"]["mode"],
        "machine_readable"
    );

    let missing_identity = vida()
        .args(["requirement", "analyze", "--json"])
        .output()
        .expect("requirement analyze missing identity should run");
    assert!(
        !missing_identity.status.success(),
        "missing identity should fail closed"
    );
    let missing_identity_value =
        parse_json_output(&["requirement", "analyze", "--json"], &missing_identity);
    support::assert_release1_operator_shape("vida requirement analyze", &missing_identity_value);
    assert_eq!(missing_identity_value["status"].as_str(), Some("blocked"));
    assert_eq!(
        missing_identity_value["blocker_codes"],
        json!(["missing_requirement_identity"])
    );

    let unreadable_source = vida()
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "task-1",
            "--source-file",
            "missing-requirement-source.md",
            "--json",
        ])
        .output()
        .expect("requirement analyze unreadable source should run");
    assert!(
        !unreadable_source.status.success(),
        "unreadable source should fail closed"
    );
    let unreadable_source_value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &unreadable_source,
    );
    support::assert_release1_operator_shape("vida requirement analyze", &unreadable_source_value);
    assert_eq!(unreadable_source_value["status"].as_str(), Some("blocked"));
    assert_eq!(
        unreadable_source_value["blocker_codes"],
        json!(["requirement_source_unreadable"])
    );

    let default_output = vida()
        .args([
            "requirement",
            "analyze",
            "--request-id",
            "request-1",
            "--input",
            "operator request",
        ])
        .output()
        .expect("requirement analyze default should run");
    assert!(
        default_output.status.success(),
        "requirement analyze default should succeed: {}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert!(stdout.starts_with("vida requirement analyze\n"));
    assert!(stdout.contains("required_fields[22]{name,meaning}:"));
    assert!(stdout.contains("readiness_statuses[4]{status,meaning}:"));
    assert!(stdout.contains("ready,Downstream implementation can start from this artifact."));
    assert!(stdout.contains("output_modes[2]{mode,contract}:"));
    assert!(stdout.contains("allowed_next_node: developer"));
}
