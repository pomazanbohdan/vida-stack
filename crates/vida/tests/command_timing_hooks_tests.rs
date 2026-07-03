use std::process::Command;

use serde_json::Value;

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn dev_gate_script() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/vida-dev-gate.ps1");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("dev gate script should read at {}: {error}", path.display())
    })
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

#[test]
fn coverage_gate_default_blocks_missing_artifacts_without_workspace_run() {
    let script = dev_gate_script();
    let missing_check = script
        .split("if (-not $RefreshCoverage)")
        .nth(1)
        .expect("coverage gate should branch on RefreshCoverage");

    assert!(
        missing_check.contains("coverage-artifact-admission")
            && missing_check
                .contains("existing coverage artifacts required; pass -RefreshCoverage"),
        "missing artifact admission should emit an actionable blocked timing record"
    );
    assert!(
        missing_check.contains("exit_status = \"blocked\"")
            && missing_check.contains("artifact_refs = $missingArtifacts"),
        "missing artifact record should be blocked and include artifact refs"
    );
    assert!(
        missing_check.contains("exit 2"),
        "missing artifacts should fail fast before any workspace coverage run"
    );
}

#[test]
fn coverage_gate_reuses_artifacts_by_default_and_refresh_runs_workspace_proofs() {
    let script = dev_gate_script();

    assert!(
        script.contains(
            "Add-SkippedRecord \"cargo-llvm-cov-nextest-workspace-lcov\" \"existing LCOV artifact reused; pass -RefreshCoverage to regenerate\""
        ),
        "default coverage mode should reuse existing LCOV artifact instead of running workspace coverage"
    );
    assert!(
        script.contains(
            "Add-SkippedRecord \"cargo-crap-workspace-json\" \"existing cargo-crap artifact reused; pass -RefreshCoverage to regenerate\""
        ),
        "default coverage mode should reuse existing CRAP artifact instead of running workspace CRAP"
    );
    for expected in [
        "$llvmCovCommand.Add(\"cargo\")",
        "$llvmCovCommand.Add(\"llvm-cov\")",
        "$llvmCovCommand.Add(\"nextest\")",
        "$llvmCovCommand.Add(\"--workspace\")",
        "$crapCommand.Add(\"cargo\")",
        "$crapCommand.Add(\"crap\")",
        "$crapCommand.Add(\"--workspace\")",
    ] {
        assert!(
            script.contains(expected),
            "RefreshCoverage path should include workspace proof token {expected}"
        );
    }
}

#[test]
fn coverage_gate_preserves_diagnostics_for_flaky_workspace_and_crap_regressions() {
    let script = dev_gate_script();

    assert!(
        script.contains("latest-{0}-artifacts.json")
            && script.contains("[progress] {0} artifacts ready before wait")
            && script.contains("stdout: {0}")
            && script.contains("stderr: {0}"),
        "long coverage proofs should publish artifact paths before waiting"
    );
    assert!(
        script.contains("New-NextestSummary")
            && script.contains("summary_artifact_path")
            && script.contains("nextest_summary"),
        "flaky nextest workspace failures should leave a compact summary artifact"
    );
    for expected in [
        "\"quality\"",
        "\"gate\"",
        "\"--prepush\"",
        "\"--crap-file\"",
        "$crapPath",
        "\"--coverage-threshold\"",
        "$thresholdText",
    ] {
        assert!(
            script.contains(expected),
            "coverage gate should route CRAP regression evidence through vida quality gate token {expected}"
        );
    }
}

#[test]
fn proof_ladder_smoke_covers_lock_contention_retry_contract() {
    let script = dev_gate_script();

    assert!(
        script.contains("state_store_read_lock_contention")
            && script.contains("Invoke-InstalledVidaStatusWithRetry")
            && script.contains("-MaxAttempts 3"),
        "script smoke should exercise retryable state lock contention"
    );
    assert!(
        script.contains("expected one retryable attempt")
            && script.contains("final attempt status was"),
        "lock contention smoke should assert retry and final pass diagnostics"
    );
}
