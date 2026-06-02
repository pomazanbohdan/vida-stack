use std::process::Command;

use serde_json::Value;

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn vida_timing_from_stderr(stderr: &[u8]) -> Value {
    let stderr = String::from_utf8_lossy(stderr);
    let line = stderr
        .lines()
        .find(|line| line.contains("\"vida_timing\""))
        .unwrap_or_else(|| panic!("timing JSON should be emitted to stderr, got: {stderr}"));
    serde_json::from_str::<Value>(line).expect("timing stderr line should be valid JSON")
}

#[test]
fn command_timing_env_emits_json_summary_to_stderr() {
    let output = vida()
        .args(["task", "help"])
        .env("VIDA_COMMAND_TIMING_ENABLED", "true")
        .env("VIDA_COMMAND_OUTPUT_MODE", "json")
        .output()
        .expect("vida task help should run with timing enabled");

    assert!(
        output.status.success(),
        "vida task help should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("\"vida_timing\""),
        "timing JSON should be emitted to stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("\"command\":\"vida task\""),
        "timing JSON should name the task command family, got: {stderr}"
    );
}

#[test]
fn operator_command_timing_breakdown_includes_budget_fields() {
    let output = vida()
        .args(["task", "help"])
        .env("VIDA_COMMAND_TIMING_ENABLED", "true")
        .env("VIDA_COMMAND_OUTPUT_MODE", "json")
        .env("VIDA_COMMAND_TIMING_BUDGET_MS", "3600000")
        .output()
        .expect("vida task help should run with timing enabled");

    assert!(output.status.success());
    let payload = vida_timing_from_stderr(&output.stderr);
    let timing = &payload["vida_timing"];
    assert_eq!(timing["command"], "vida task");
    assert_eq!(timing["budget_ms"], 3_600_000);
    assert_eq!(timing["over_budget"], false);
    assert!(
        timing["phases_ms"].as_object().is_some(),
        "timing JSON should include phase breakdown, got: {payload}"
    );
    assert_eq!(
        timing["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .len(),
        0
    );
}

#[test]
fn operator_command_latency_next_actions_name_slow_stage() {
    let output = vida()
        .args(["task", "help"])
        .env("VIDA_COMMAND_TIMING_ENABLED", "true")
        .env("VIDA_COMMAND_OUTPUT_MODE", "json")
        .env("VIDA_COMMAND_TIMING_BUDGET_MS", "0")
        .output()
        .expect("vida task help should run with over-budget timing enabled");

    assert!(output.status.success());
    let payload = vida_timing_from_stderr(&output.stderr);
    let timing = &payload["vida_timing"];
    assert_eq!(timing["over_budget"], true);
    assert!(
        timing["slowest_phase"]["name"].as_str().is_some() || timing["slowest_phase"].is_null(),
        "slowest_phase should be named or explicitly null, got: {payload}"
    );
    let next_actions = timing["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert!(
        next_actions.iter().any(|action| action
            .as_str()
            .is_some_and(|value| value.contains("cached projection or read-model"))),
        "over-budget timing should recommend cache/read-model action, got: {payload}"
    );
}

#[test]
fn command_timing_min_threshold_filters_fast_commands() {
    let output = vida()
        .args(["task", "help"])
        .env("VIDA_COMMAND_TIMING_ENABLED", "true")
        .env("VIDA_COMMAND_OUTPUT_MODE", "json")
        .env("VIDA_COMMAND_TIMING_MIN_MS", "3600000")
        .output()
        .expect("vida task help should run with timing threshold");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("\"vida_timing\""),
        "high timing threshold should suppress timing output, got: {stderr}"
    );
}

#[test]
fn command_timing_quiet_mode_suppresses_timing_summary() {
    let output = vida()
        .args(["task", "help"])
        .env("VIDA_COMMAND_TIMING_ENABLED", "true")
        .env("VIDA_COMMAND_OUTPUT_MODE", "quiet")
        .env("VIDA_COMMAND_TIMING_PRINT_SUMMARY", "true")
        .output()
        .expect("vida task help should run with quiet timing mode");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("vida_timing"),
        "quiet output mode should suppress timing summary, got: {stderr}"
    );
}
