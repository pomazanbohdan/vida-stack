use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_docflow_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vida-docflow-cli-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[test]
fn overview_command_runs_as_binary() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["overview", "--registry-count", "4", "--relation-count", "2"])
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow overview"));
    assert!(stdout.contains("registry_rows: 4"));
    assert!(stdout.contains("relation_edges: 2"));
}

#[test]
fn relations_command_runs_as_binary() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["relations", "--edge-count", "3"])
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim_end(),
        "relations\n  total_edges: 3",
        "{}",
        context.diagnostics(&output)
    );
}

#[test]
fn init_prints_agent_bootstrap_instructions() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .arg("init")
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mode: agent_bootstrap"));
    assert!(stdout.contains("AGENTS.sidecar.md"));
    assert!(stdout.contains("docflow readiness-check --profile active-canon"));
}

#[test]
fn init_json_prints_machine_readable_agent_bootstrap() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["init", "--json"])
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"mode\":\"agent_bootstrap\""));
    assert!(stdout.contains("\"safe_first_commands\""));
    assert!(stdout.contains("\"next_actions\""));
}

#[test]
fn root_help_renders_as_binary() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .arg("--help")
        .output()
        .expect("docflow help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Standalone DocFlow CLI"));
    assert!(stdout.contains("Usage: docflow"));
    assert!(stdout.contains("<COMMAND>"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("readiness-check"));
}

#[test]
fn check_help_exposes_json_mode() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["check", "--help"])
        .output()
        .expect("docflow check help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--json"));
}

#[test]
fn check_json_renders_blocked_and_pass_envelopes() {
    let context = vida_test_support::CommandContext::empty();
    let blocked_root = unique_docflow_root("blocked");
    fs::create_dir_all(blocked_root.join("docs/process")).expect("process dir should be created");
    fs::write(blocked_root.join("docs/process/a.md"), "# a\n").expect("process markdown");

    let blocked_output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "check",
            "--root",
            blocked_root.to_str().expect("blocked root should be utf8"),
            "--json",
            "docs/process/a.md",
        ])
        .output()
        .expect("docflow check json should run");

    assert!(
        blocked_output.status.success(),
        "{}",
        context.diagnostics(&blocked_output)
    );
    let blocked_json: serde_json::Value =
        serde_json::from_slice(&blocked_output.stdout).expect("blocked json should parse");
    assert_eq!(blocked_json["surface"], "docflow check");
    assert_eq!(blocked_json["status"], "blocked");
    assert_eq!(blocked_json["row_count"], 1);
    assert_eq!(blocked_json["rows"][0]["path"], "docs/process/a.md");
    let issues = blocked_json["rows"][0]["issues"]
        .as_array()
        .expect("issues should be an array");
    assert!(issues.contains(&serde_json::Value::String("missing_footer".to_string())));

    let pass_root = unique_docflow_root("pass");
    fs::create_dir_all(&pass_root).expect("pass root should be created");
    let pass_output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "check",
            "--root",
            pass_root.to_str().expect("pass root should be utf8"),
            "--json",
        ])
        .output()
        .expect("docflow check json should run");

    assert!(
        pass_output.status.success(),
        "{}",
        context.diagnostics(&pass_output)
    );
    let pass_json: serde_json::Value =
        serde_json::from_slice(&pass_output.stdout).expect("pass json should parse");
    assert_eq!(pass_json["surface"], "docflow check");
    assert_eq!(pass_json["status"], "pass");
    assert_eq!(pass_json["row_count"], 0);
    assert!(
        pass_json["rows"]
            .as_array()
            .expect("rows should be an array")
            .is_empty()
    );

    fs::remove_dir_all(blocked_root).expect("blocked root should be removed");
    fs::remove_dir_all(pass_root).expect("pass root should be removed");
}

#[test]
fn version_renders_as_binary() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .arg("--version")
        .output()
        .expect("docflow version should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim_end(),
        format!("docflow {}", env!("CARGO_PKG_VERSION"))
    );
}
