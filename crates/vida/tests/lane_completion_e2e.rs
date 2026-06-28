use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn unique_lane_state_root(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn run_vida_json_with_state(
    args: &[&str],
    state_root: &std::path::Path,
) -> (serde_json::Value, bool) {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_root)
        .output()
        .expect("vida command should launch");
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "json output should parse for args {args:?}: {error}\nstatus: {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (payload, output.status.success())
}

#[test]
fn host_bridge_missing_request_json_parse_error_is_machine_readable() {
    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--complete")
        .arg("--decision")
        .arg("pass")
        .arg("--verdict")
        .arg("pass")
        .arg("--allowed-next-node")
        .arg("designer")
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "json parse errors should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse error stdout should be JSON");
    assert_eq!(payload["surface"].as_str(), Some("vida agent host-bridge"));
    assert!(payload["blocker_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|code| code.as_str() == Some("cli_parse_error")));
    assert!(payload["error"]
        .as_str()
        .is_some_and(|error| { error.contains("--request <REQUEST>") }));
}

#[test]
fn lane_exception_takeover_json_parse_error_is_machine_readable() {
    let output = vida()
        .arg("lane")
        .arg("exception-takeover")
        .arg("ldr-032")
        .arg("--json")
        .output()
        .expect("vida lane exception-takeover should launch");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "lane json parse errors should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane parse error stdout should be JSON");
    assert_eq!(
        payload["surface"].as_str(),
        Some("vida lane exception-takeover")
    );
    assert!(payload["blocker_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|code| code.as_str() == Some("lane_parse_error")));
    assert!(payload["reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("Invalid or incomplete arguments")));
}

#[test]
fn host_bridge_completion_command_resolves_packet_next_target() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "vida-lane-completion-e2e-{}-{nanos}",
        std::process::id()
    ));
    let state_root = root.join(".vida/data/state");
    let packet_path = state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
    let request_path = state_root.join("host-tool-bridge/requests/request.json");
    let result_path = state_root.join("host-tool-bridge/results/result.json");
    let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
    std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
        .expect("create packet parent");
    std::fs::create_dir_all(request_path.parent().expect("request parent"))
        .expect("create request parent");
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "run_id": "run-analyst",
            "dispatch_target": "analyst",
            "downstream_dispatch_active_target": "analyst",
            "downstream_dispatch_target": "designer"
        })
        .to_string(),
    )
    .expect("write packet");
    std::fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-analyst",
            "run_id": "run-analyst",
            "task_id": "run-analyst",
            "dispatch_target": "analyst",
            "allowed_next_node": "closure",
            "packet_path": packet_path.display().to_string(),
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        })
        .to_string(),
    )
    .expect("write request");

    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--request")
        .arg(&request_path)
        .arg("--state-dir")
        .arg(&state_root)
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge output should be JSON");
    assert!(
        payload["blocker_codes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|code| code.as_str() == Some("host_bridge_dispatch_receipt_missing")),
        "fixture should remain blocked only by missing DB receipt: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let command = payload["host_bridge"]["completion_command"]
        .as_str()
        .expect("completion command should render");
    assert!(
        !command.contains("--allowed-next-node"),
        "completion command must not trust packet/request next-node routing: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn host_bridge_completion_command_does_not_read_packet_outside_state_root() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "vida-lane-completion-boundary-e2e-{}-{nanos}",
        std::process::id()
    ));
    let state_root = root.join(".vida/data/state");
    let outside_packet_path = root.join("outside-packet.json");
    let request_path = state_root.join("host-tool-bridge/requests/request.json");
    let result_path = state_root.join("host-tool-bridge/results/result.json");
    let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
    std::fs::create_dir_all(request_path.parent().expect("request parent"))
        .expect("create request parent");
    std::fs::write(
        &outside_packet_path,
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "run_id": "run-analyst",
            "dispatch_target": "analyst",
            "downstream_dispatch_active_target": "analyst",
            "downstream_dispatch_target": "leaked-target"
        })
        .to_string(),
    )
    .expect("write outside packet");
    std::fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-analyst",
            "run_id": "run-analyst",
            "task_id": "run-analyst",
            "dispatch_target": "analyst",
            "packet_path": outside_packet_path.display().to_string(),
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        })
        .to_string(),
    )
    .expect("write request");

    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--request")
        .arg(&request_path)
        .arg("--state-dir")
        .arg(&state_root)
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge output should be JSON");
    let command = payload["host_bridge"]["completion_command"]
        .as_str()
        .expect("completion command should render");
    assert!(
        !command.contains("leaked-target"),
        "completion command must not disclose out-of-state-root packet fields: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!command.contains("--allowed-next-node"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn lane_public_surface_matrix_fails_closed_with_json_contracts() {
    let root = unique_lane_state_root("vida-lane-surface-matrix");
    let state_root = root.join(".vida/data/state");
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_root)
        .output()
        .expect("boot should launch");
    assert!(
        boot.status.success(),
        "boot should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );

    for (label, args, expected_surface, expected_blocker) in [
        (
            "run_lane root",
            vec!["lane", "--json"],
            "vida lane",
            "unsupported_blocker_code",
        ),
        (
            "run_lane show missing run",
            vec!["lane", "show", "matrix-missing-run", "--json"],
            "vida lane show",
            "missing_lane_receipt",
        ),
        (
            "run_lane takeover-ready missing run",
            vec!["lane", "takeover-ready", "matrix-missing-run", "--json"],
            "vida lane takeover-ready",
            "missing_lane_receipt",
        ),
    ] {
        let (payload, success) = run_vida_json_with_state(&args, &state_root);
        assert!(!success, "{label} should fail closed: {payload}");
        assert_eq!(
            payload["surface"].as_str(),
            Some(expected_surface),
            "{label}"
        );
        assert_eq!(payload["status"].as_str(), Some("blocked"), "{label}");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .expect("blocker_codes should be an array")
                .iter()
                .any(|code| code.as_str() == Some(expected_blocker)),
            "{label} should expose {expected_blocker}: {payload}"
        );
    }

    let _ = std::fs::remove_dir_all(root);
}
