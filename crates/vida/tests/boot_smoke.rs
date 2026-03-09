use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("/tmp/vida-test-state-{}-{}", std::process::id(), nanos)
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
    let output = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida boot scaffold ready"));
    assert!(stdout.contains("authoritative state store: kv-surrealkv"));
    assert!(stdout.contains("authoritative state spine: initialized (state-v1, 8 entity surfaces, mutation root vida task)"));
    assert!(stdout.contains("framework instruction bundle: seeded"));
    assert!(stdout.contains("instruction source tree: _vida/instructions -> instruction_memory"));
    assert!(stdout.contains(
        "instruction ingest: 3 imported, 0 unchanged, 0 updated from _vida/instructions"
    ));
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration preflight: compatible / no_migration_required (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 1 imported, 0 unchanged, 0 updated from _vida/framework-memory"
    ));
}

#[test]
fn boot_supports_color_render_mode() {
    let output = vida()
        .args(["boot", "--render", "color"])
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run with color render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida boot scaffold ready\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34mauthoritative state store\u{1b}[0m"));
}

#[test]
fn boot_is_idempotent_for_unchanged_source_trees() {
    let state_dir = unique_state_dir();
    let first = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("first boot should run");
    assert!(first.status.success());

    let second = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("second boot should run");
    assert!(second.status.success());

    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains(
        "instruction ingest: 0 imported, 3 unchanged, 0 updated from _vida/instructions"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 0 imported, 1 unchanged, 0 updated from _vida/framework-memory"
    ));
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration preflight: compatible / no_migration_required (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
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
fn memory_surface_reports_effective_bundle() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida memory"));
    assert!(stdout.contains("effective instruction bundle root: framework-agent-definition"));
    assert!(stdout.contains("mandatory chain: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains("source version tuple: framework-agent-definition@v1, framework-instruction-contract@v1, framework-prompt-template-config@v1"));
    assert!(stdout.contains("receipt: not-persisted"));
}

#[test]
fn memory_surface_supports_color_render_mode() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["memory", "--render", "color"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run with color render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida memory\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34meffective instruction bundle root\u{1b}[0m"));
}

#[test]
fn memory_surface_cli_render_overrides_env() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["memory", "--render", "color_emoji"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "plain")
        .output()
        .expect("memory should run with CLI render override");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("📘 vida memory"));
    assert!(stdout.contains("🔹"));
}

#[test]
fn memory_surface_fails_closed_on_invalid_render_env() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "invalid_mode")
        .output()
        .expect("memory should run with invalid render env");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value") || stderr.contains("possible values"));
}

#[test]
fn memory_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
}

#[test]
fn status_surface_reports_backend_and_bundle_receipt() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida status"));
    assert!(stdout.contains("backend: kv-surrealkv state-v1 instruction-v1"));
    assert!(stdout.contains(
        "state spine: initialized (state-v1, 8 entity surfaces, mutation root vida task)"
    ));
    assert!(stdout
        .contains("latest effective bundle receipt: effective-bundle-framework-agent-definition-"));
    assert!(stdout.contains("latest effective bundle root: framework-agent-definition"));
    assert!(stdout.contains("latest effective bundle artifact count: 3"));
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration state: compatible / no_migration_required (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
}

#[test]
fn status_surface_supports_color_emoji_render_mode_via_env() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "color_emoji")
        .output()
        .expect("status should run with color emoji render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("📘 vida status"));
    assert!(stdout.contains("✅") || stdout.contains("🔹"));
}

#[test]
fn status_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
}

#[test]
fn doctor_surface_reports_integrity_checks() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida doctor"));
    assert!(stdout.contains("storage metadata: ok (kv-surrealkv state-v1 instruction-v1)"));
    assert!(stdout.contains(
        "authoritative state spine: ok (state-v1, 8 entity surfaces, mutation root vida task)"
    ));
    assert!(stdout.contains("boot compatibility: ok (compatible (normal_boot_allowed))"));
    assert!(stdout.contains(
        "migration preflight: ok (compatible / no_migration_required (normal_boot_allowed))"
    ));
    assert!(stdout.contains(
        "migration receipts: ok (compatibility=1, application=0, verification=0, cutover=0, rollback=0)"
    ));
    assert!(stdout.contains("effective instruction bundle: ok (framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config)"));
}

#[test]
fn doctor_surface_supports_color_render_mode() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["doctor", "--render", "color"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run with color render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida doctor\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;32mok\u{1b}[0m"));
}

#[test]
fn doctor_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
}

#[test]
fn reserved_families_fail_closed_without_help() {
    for command in ["task"] {
        let output = vida()
            .arg(command)
            .output()
            .expect("reserved command should run");
        assert!(
            !output.status.success(),
            "{command} should stay fail-closed in Binary Foundation"
        );
    }
}

#[test]
fn unknown_command_fails_closed() {
    let output = vida()
        .arg("unknown")
        .output()
        .expect("unknown command should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command family `unknown`"));
}

#[test]
fn boot_with_extra_argument_fails_closed() {
    let output = vida()
        .args(["boot", "unexpected"])
        .output()
        .expect("boot with extra arg should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported `vida boot` argument `unexpected`"));
}
