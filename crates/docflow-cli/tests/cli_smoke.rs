use std::process::Command;

#[test]
fn overview_command_runs_as_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_docflow"))
        .args(["overview", "--registry-count", "4", "--relation-count", "2"])
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow overview"));
    assert!(stdout.contains("registry_rows: 4"));
    assert!(stdout.contains("relation_edges: 2"));
}

#[test]
fn relations_command_runs_as_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_docflow"))
        .args(["relations", "--edge-count", "3"])
        .output()
        .expect("docflow binary should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "relations\n  total_edges: 3");
}
