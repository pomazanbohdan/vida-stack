use std::process::Command;

fn vida_service() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida-service"))
}

#[test]
fn lifecycle_plan_default_output_is_compact_plain() {
    let output = vida_service()
        .arg("lifecycle-plan")
        .output()
        .expect("run vida-service lifecycle-plan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("vida-service lifecycle-plan\n"));
    assert!(stdout.contains("  mode: dry_run\n"));
    assert!(stdout.contains("  service_name: vida-service\n"));
    assert!(stdout.contains("  apply_requires_token: true\n"));
    assert!(stdout.contains("  restart_policy: on_failure\n"));
    assert!(!stdout.contains('{'));
}

#[test]
fn lifecycle_plan_json_output_is_machine_readable() {
    let output = vida_service()
        .args(["lifecycle-plan", "--json"])
        .output()
        .expect("run vida-service lifecycle-plan --json");

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is json");
    assert_eq!(value["mode"], "dry_run");
    assert_eq!(value["service_name"], "vida-service");
    assert_eq!(value["apply_requires_token"], true);
    assert_eq!(value["restart_policy"], "on_failure");
}

#[test]
fn ipc_matrix_default_and_json_outputs_cover_local_ipc_contract() {
    let default_output = vida_service()
        .arg("ipc-matrix")
        .output()
        .expect("run vida-service ipc-matrix");

    assert!(default_output.status.success());
    let stdout = String::from_utf8(default_output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("vida-service ipc-matrix\n"));
    assert!(stdout.contains("  rows: 2\n"));
    assert!(
        stdout.contains("windows,interprocess_local_socket_named_pipe,tarpc_length_delimited_json")
    );
    assert!(stdout.contains("unix,interprocess_local_socket,tarpc_length_delimited_json"));

    let json_output = vida_service()
        .args(["ipc-matrix", "--json"])
        .output()
        .expect("run vida-service ipc-matrix --json");

    assert!(json_output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("stdout is json");
    let rows = value.as_array().expect("matrix is array");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row["domain_mutation_logic"] == false));
}

#[test]
fn help_documents_default_commands_and_json_modes() {
    let root_help = vida_service()
        .arg("--help")
        .output()
        .expect("run vida-service --help");

    assert!(root_help.status.success());
    let root = String::from_utf8(root_help.stdout).expect("stdout is utf8");
    assert!(root.contains("foreground"));
    assert!(root.contains("lifecycle-plan"));
    assert!(root.contains("ipc-matrix"));

    let lifecycle_help = vida_service()
        .args(["lifecycle-plan", "--help"])
        .output()
        .expect("run vida-service lifecycle-plan --help");

    assert!(lifecycle_help.status.success());
    let lifecycle = String::from_utf8(lifecycle_help.stdout).expect("stdout is utf8");
    assert!(lifecycle.contains("--json"));
    assert!(lifecycle.contains("--mode"));

    let ipc_help = vida_service()
        .args(["ipc-matrix", "--help"])
        .output()
        .expect("run vida-service ipc-matrix --help");

    assert!(ipc_help.status.success());
    let ipc = String::from_utf8(ipc_help.stdout).expect("stdout is utf8");
    assert!(ipc.contains("--json"));
}
