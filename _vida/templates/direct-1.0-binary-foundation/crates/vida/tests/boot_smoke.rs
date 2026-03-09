use std::process::Command;

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

#[test]
fn root_help_succeeds() {
    let output = vida().arg("--help").output().expect("root help should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("boot"));
    assert!(stdout.contains("task"));
    assert!(stdout.contains("memory"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("doctor"));
}

#[test]
fn boot_succeeds() {
    let output = vida().arg("boot").output().expect("boot should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida boot scaffold ready"));
}

#[test]
fn command_family_help_succeeds() {
    for command in ["boot", "task", "memory", "status", "doctor"] {
        let output = vida()
            .args([command, "--help"])
            .output()
            .expect("command help should run");

        assert!(output.status.success(), "{command} help should succeed");
    }
}

#[test]
fn reserved_families_fail_closed_without_help() {
    for command in ["task", "memory", "status", "doctor"] {
        let output = vida().arg(command).output().expect("reserved command should run");
        assert!(
            !output.status.success(),
            "{command} should stay fail-closed in Binary Foundation"
        );
    }
}

