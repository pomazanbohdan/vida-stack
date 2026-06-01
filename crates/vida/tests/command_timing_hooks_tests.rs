use std::process::Command;

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
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
