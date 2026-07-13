use std::process::Command;

#[test]
fn project_activator_public_surface_exposes_projection_refresh_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_vida"))
        .args(["project-activator", "--help"])
        .output()
        .expect("vida project-activator help should execute");
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--repair"));
    assert!(help.contains("--json"));
}
