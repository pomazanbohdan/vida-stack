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
    assert!(stdout.contains("Usage: docflow <COMMAND>"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("readiness-check"));
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
