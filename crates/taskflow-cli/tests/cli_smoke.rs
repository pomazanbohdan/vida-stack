use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(prefix: &str) -> PathBuf {
    vida_test_support::temp_dir(prefix)
}

fn write_mock_vida(bin_dir: &Path) {
    let path = bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" });
    vida_test_support::write_executable_script(
        &path,
        "#!/bin/sh\nprintf 'vida %s\\n' \"$*\"\nprintf 'VIDA_STATE_DIR=%s\\n' \"${VIDA_STATE_DIR:-}\"\n",
        "@echo off\r\necho vida %*\r\necho VIDA_STATE_DIR=%VIDA_STATE_DIR%\r\n",
    );
}

#[test]
fn help_command_delegates_to_vida_taskflow_help() {
    let bin_dir = temp_dir("taskflow-cli-bin");
    write_mock_vida(&bin_dir);
    let vida_bin = bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" });
    let project_root = temp_dir("taskflow-cli-project");
    fs::write(
        project_root.join("vida.config.yaml"),
        "project:\n  id: demo\n",
    )
    .expect("config");
    let mut process_guard = vida_test_support::ProcessGuard::new();
    process_guard.set_env("VIDA_TASKFLOW_VIDA_BIN", &vida_bin);
    process_guard.unset_env("VIDA_STATE_DIR");
    process_guard.change_current_dir(&project_root);
    let context = vida_test_support::CommandContext::capture([
        ("VIDA_TASKFLOW_VIDA_BIN", vida_bin.display().to_string()),
        ("VIDA_STATE_DIR", "<unset>".to_string()),
    ]);

    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_taskflow"))
        .args(["help", "parallelism"])
        .output()
        .expect("taskflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida taskflow help parallelism"));
    assert!(stdout.contains("VIDA_STATE_DIR="));
}

#[test]
fn delegated_commands_bind_project_local_state_dir_when_missing() {
    let bin_dir = temp_dir("taskflow-cli-bin");
    write_mock_vida(&bin_dir);
    let vida_bin = bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" });
    let project_root = temp_dir("taskflow-cli-project");
    fs::write(
        project_root.join("vida.config.yaml"),
        "project:\n  id: demo\n",
    )
    .expect("config");
    let mut process_guard = vida_test_support::ProcessGuard::new();
    process_guard.set_env("VIDA_TASKFLOW_VIDA_BIN", &vida_bin);
    process_guard.unset_env("VIDA_STATE_DIR");
    process_guard.change_current_dir(&project_root);
    let context = vida_test_support::CommandContext::capture([
        ("VIDA_TASKFLOW_VIDA_BIN", vida_bin.display().to_string()),
        ("VIDA_STATE_DIR", "<unset>".to_string()),
    ]);

    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_taskflow"))
        .args(["validate-routing", "--json"])
        .output()
        .expect("taskflow binary should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida taskflow validate-routing --json"));
    assert!(stdout.contains(&format!(
        "VIDA_STATE_DIR={}",
        project_root.join(".vida/data/state").display()
    )));
}

#[test]
fn root_help_renders_without_delegation() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_taskflow"))
        .arg("--help")
        .output()
        .expect("taskflow help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Standalone TaskFlow CLI wrapper"));
    assert!(stdout.contains("validate-routing --json"));
}

#[test]
fn version_renders_without_delegation() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_taskflow"))
        .arg("--version")
        .output()
        .expect("taskflow version should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim_end(),
        format!("taskflow {}", env!("CARGO_PKG_VERSION"))
    );
}
