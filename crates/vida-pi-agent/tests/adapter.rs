use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn adapter_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vida-pi-agent")
}

fn fake_pi_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vida-pi-agent-fake-pi")
}

fn run_adapter(args: &[&str], stdin: &str, scenario: &str) -> std::process::Output {
    run_adapter_with_env(args, stdin, scenario, &[])
}

fn run_adapter_with_env(
    args: &[&str],
    stdin: &str,
    scenario: &str,
    envs: &[(&str, &str)],
) -> std::process::Output {
    let mut command = Command::new(adapter_bin());
    command
        .arg("--pi-command")
        .arg(fake_pi_bin())
        .args(args)
        .env("VIDA_PI_AGENT_FAKE_SCENARIO", scenario)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("adapter should spawn");
    child
        .stdin
        .as_mut()
        .expect("adapter stdin should be open")
        .write_all(stdin.as_bytes())
        .expect("prompt should write to adapter stdin");
    child.wait_with_output().expect("adapter should exit")
}

fn parse_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("adapter stdout should be JSON")
}

fn temp_workdir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "vida-pi-agent-adapter-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("temp workdir should be created");
    path
}

#[test]
fn success_extracts_final_answer_from_agent_end_messages() {
    let output = run_adapter(
        &[
            "--mode",
            "rpc",
            "--model",
            "openai-codex/gpt-5.5",
            "--thinking-level",
            "medium",
        ],
        "hello from stdin",
        "success",
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "result");
    assert_eq!(payload["subtype"], "success");
    assert_eq!(payload["is_error"], false);
    assert_eq!(payload["result"], "fake final: hello from stdin");
    assert_eq!(payload["raw_provider"]["provider"], "pi");
    assert_eq!(payload["raw_provider"]["mode"], "rpc");
    assert_eq!(payload["raw_provider"]["terminal_event"], "agent_end");
}

#[test]
fn invalid_model_returns_nonzero_parseable_error_json() {
    let output = run_adapter(
        &["--mode", "rpc", "--model", "openai-codex/missing"],
        "prompt",
        "invalid_model",
    );
    assert!(!output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "result");
    assert_eq!(payload["subtype"], "error_during_execution");
    assert_eq!(payload["is_error"], true);
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("error message should be a string")
            .contains("set_model")
    );
}

#[test]
fn timeout_without_agent_end_returns_nonzero_parseable_error_json() {
    let output = run_adapter(
        &["--mode", "rpc", "--timeout-seconds", "1"],
        "prompt",
        "timeout",
    );
    assert!(!output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "result");
    assert_eq!(payload["is_error"], true);
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("error message should be a string")
            .contains("Timed out")
    );
}

#[test]
fn prompt_can_be_read_from_positional_args() {
    let output = run_adapter(&["positional", "prompt"], "", "success");
    assert!(output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["result"], "fake final: positional prompt");
}

#[test]
fn process_exits_after_dispatch() {
    let start = Instant::now();
    let output = run_adapter(&[], "exit check", "success");
    assert!(output.status.success());
    assert!(start.elapsed() < Duration::from_secs(5));
    let payload = parse_stdout(&output);
    assert_eq!(payload["result"], "fake final: exit check");
}

#[test]
fn scope_guard_allows_in_scope_touched_path() {
    let workdir = temp_workdir("in-scope");
    fs::create_dir_all(workdir.join("src")).expect("src should exist");
    fs::write(workdir.join("src/lib.rs"), "").expect("file should exist");
    let workdir_arg = workdir.display().to_string();
    let output = run_adapter(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
            "--owned-path",
            "src",
        ],
        "prompt",
        "touched_in_scope",
    );
    assert!(
        output.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let payload = parse_stdout(&output);
    assert_eq!(payload["scope_guard"]["status"], "validated");
    assert_eq!(payload["scope_guard"]["valid"], true);
    assert_eq!(
        payload["raw_provider"]["provider_result_json"]["touched_paths"][0],
        "src/lib.rs"
    );
    let _ = fs::remove_dir_all(workdir);
}

#[test]
fn scope_guard_rejects_out_of_scope_touched_path_with_parseable_error_json() {
    let workdir = temp_workdir("out-of-scope");
    fs::create_dir_all(workdir.join("src")).expect("src should exist");
    fs::create_dir_all(workdir.join("docs")).expect("docs should exist");
    let workdir_arg = workdir.display().to_string();
    let output = run_adapter(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
            "--owned-path",
            "src",
        ],
        "prompt",
        "touched_out_of_scope",
    );
    assert!(!output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "result");
    assert_eq!(payload["is_error"], true);
    assert_eq!(payload["scope_guard"]["status"], "violation");
    assert_eq!(payload["scope_guard"]["valid"], false);
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("outside owned paths")
    );
    let _ = fs::remove_dir_all(workdir);
}

#[test]
fn guarded_write_mode_loads_prewrite_guard_extension_and_env() {
    let workdir = temp_workdir("guard-argv-env");
    fs::create_dir_all(workdir.join("src")).expect("src should exist");
    let workdir_arg = workdir.display().to_string();
    let output = run_adapter(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
            "--owned-path",
            "src",
        ],
        "prompt",
        "guard_argv_env",
    );
    assert!(
        output.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let payload = parse_stdout(&output);
    assert_eq!(payload["scope_guard"]["pre_write_enforcement"], true);
    assert_eq!(payload["scope_guard"]["pre_write_guard"]["active"], true);
    let provider = &payload["raw_provider"]["provider_result_json"];
    let argv = provider["argv"].as_array().expect("argv should render");
    assert!(
        argv.iter()
            .any(|value| value.as_str() == Some("--extension"))
    );
    let extension_index = argv
        .iter()
        .position(|value| value.as_str() == Some("--extension"))
        .expect("extension flag should exist");
    let extension_path = argv[extension_index + 1]
        .as_str()
        .expect("extension path should follow flag");
    assert!(extension_path.ends_with("vida-owned-write-scope-guard.ts"));
    assert_eq!(
        provider["env"]["VIDA_PI_AGENT_SCOPE_GUARD_MODE"],
        "guarded-write"
    );
    assert_eq!(
        provider["env"]["VIDA_PI_AGENT_OWNED_PATHS_JSON"],
        "[\"src\"]"
    );
    assert_eq!(
        provider["env"]["VIDA_PI_AGENT_PREWRITE_GUARD_ACTIVE"],
        "true"
    );
    let _ = fs::remove_dir_all(workdir);
}

#[test]
fn guarded_write_mode_rejects_no_extensions_before_dispatch() {
    let workdir = temp_workdir("no-extensions");
    fs::create_dir_all(workdir.join("src")).expect("src should exist");
    let workdir_arg = workdir.display().to_string();
    let output = run_adapter(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
            "--owned-path",
            "src",
            "--no-extensions",
        ],
        "prompt",
        "success",
    );
    assert!(!output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "result");
    assert_eq!(payload["is_error"], true);
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("message")
            .contains("--no-extensions is forbidden")
    );
    let _ = fs::remove_dir_all(workdir);
}

#[test]
fn capabilities_json_reports_prewrite_guard_support_without_prompt_or_pi() {
    let output = run_adapter(&["--capabilities-json"], "", "timeout");
    assert!(output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["type"], "capabilities");
    assert_eq!(payload["scope_guard"]["pre_write_enforcement"], true);
    assert_eq!(payload["scope_guard"]["explicit_extension_arg"], true);
}

#[test]
fn guarded_write_mode_without_owned_paths_fails_before_dispatch() {
    let workdir = temp_workdir("missing-owned");
    let workdir_arg = workdir.display().to_string();
    let output = run_adapter(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
        ],
        "prompt",
        "success",
    );
    assert!(!output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["scope_guard"]["status"], "missing_owned_paths");
    assert_eq!(payload["scope_guard"]["valid"], false);
    let _ = fs::remove_dir_all(workdir);
}

#[test]
fn scope_guard_reads_owned_paths_from_dispatch_packet_env() {
    let workdir = temp_workdir("packet-owned");
    fs::create_dir_all(workdir.join("src")).expect("src should exist");
    fs::write(workdir.join("src/lib.rs"), "").expect("file should exist");
    let packet_path = workdir.join("packet.json");
    fs::write(
        &packet_path,
        serde_json::to_string(&serde_json::json!({
            "delivery_task_packet": {"owned_paths": ["src"]}
        }))
        .expect("packet should render"),
    )
    .expect("packet should write");
    let workdir_arg = workdir.display().to_string();
    let packet_arg = packet_path.display().to_string();
    let output = run_adapter_with_env(
        &[
            "--workdir",
            &workdir_arg,
            "--scope-guard-mode",
            "guarded-write",
        ],
        "prompt",
        "touched_in_scope",
        &[("VIDA_DISPATCH_PACKET_PATH", &packet_arg)],
    );
    assert!(output.status.success());
    let payload = parse_stdout(&output);
    assert_eq!(payload["scope_guard"]["status"], "validated");
    assert!(
        payload["scope_guard"]["owned_path_sources"][0]
            .as_str()
            .expect("source")
            .contains("packet:")
    );
    let _ = fs::remove_dir_all(workdir);
}
