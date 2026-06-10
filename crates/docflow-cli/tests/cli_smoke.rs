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
fn fastcheck_help_does_not_expose_json_mode() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["fastcheck", "--help"])
        .output()
        .expect("docflow fastcheck help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("--json"));
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

fn write_task_doc(root: &std::path::Path, task_id: &str) {
    fs::create_dir_all(root.join("docs/process")).expect("process dir should be created");
    fs::write(
        root.join("docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: test\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-06-05T00:00:00Z\nupdated_at: 2026-06-05T00:00:00Z\n",
    )
    .expect("markdown should be written");
    fs::write(
        root.join("docs/process/a.changelog.jsonl"),
        format!(
            "{{\"ts\":\"2026-06-05T00:00:00Z\",\"event\":\"artifact_updated\",\"artifact_path\":\"process/a\",\"task_id\":\"{task_id}\",\"reason\":\"test\"}}\n"
        ),
    )
    .expect("changelog should be written");
}

fn write_task_protocol_doc(root: &std::path::Path, task_id: &str) {
    fs::create_dir_all(root.join("vida/config/instructions/instruction-contracts"))
        .expect("instruction-contracts dir should be created");
    fs::write(
        root.join("vida/config/instructions/instruction-contracts/unregistered-test-protocol.md"),
        "# unregistered test protocol\n\n-----\nartifact_path: instruction-contracts/unregistered-test-protocol\nartifact_type: instruction_contract\nartifact_version: 1\nartifact_revision: test\nsource_path: vida/config/instructions/instruction-contracts/unregistered-test-protocol.md\nstatus: draft\nchangelog_ref: unregistered-test-protocol.changelog.jsonl\ncreated_at: 2026-06-05T00:00:00Z\nupdated_at: 2026-06-05T00:00:00Z\n",
    )
    .expect("protocol markdown should be written");
    fs::write(
        root.join("vida/config/instructions/instruction-contracts/unregistered-test-protocol.changelog.jsonl"),
        format!(
            "{{\"ts\":\"2026-06-05T00:00:00Z\",\"event\":\"artifact_updated\",\"artifact_path\":\"instruction-contracts/unregistered-test-protocol\",\"task_id\":\"{task_id}\",\"reason\":\"test\"}}\n"
        ),
    )
    .expect("protocol changelog should be written");
}

#[test]
fn proofcheck_help_exposes_task_json_and_compact_modes() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["proofcheck", "--help"])
        .output()
        .expect("docflow proofcheck help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--task <TASK_ID>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--compact"));
    assert!(stdout.contains("--format <FORMAT>"));
}

#[test]
fn closeout_help_exposes_changed_task_json_and_compact_modes() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["closeout", "--help"])
        .output()
        .expect("docflow closeout help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--changed"));
    assert!(stdout.contains("--task <TASK_ID>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--compact"));
    assert!(stdout.contains("--format <FORMAT>"));
}

#[test]
fn proofcheck_task_outputs_default_toon_and_explicit_compact_json() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("proofcheck-task");
    write_task_doc(&root, "TASK-1");

    let toon = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "proofcheck",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-1",
        ])
        .output()
        .expect("docflow proofcheck task toon should run");
    assert!(toon.status.success(), "{}", context.diagnostics(&toon));
    let toon_stdout = String::from_utf8_lossy(&toon.stdout);
    assert!(toon_stdout.starts_with("proofcheck\n  mode: task"));
    assert!(toon_stdout.contains("task_id: TASK-1"));
    assert!(toon_stdout.contains("changed_doc_count: 1"));
    assert!(!toon_stdout.trim_start().starts_with('{'));
    assert!(!toon_stdout.contains("--json"));

    let json = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "proofcheck",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-1",
            "--json",
            "--compact",
        ])
        .output()
        .expect("docflow proofcheck task json should run");
    assert!(json.status.success(), "{}", context.diagnostics(&json));
    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("proofcheck task json should parse");
    assert_eq!(parsed["command"], "proofcheck");
    assert_eq!(parsed["mode"], "task");
    assert_eq!(parsed["task_id"], "TASK-1");
    assert_eq!(parsed["changed_doc_count"], 1);
    assert_eq!(
        parsed["changed_docs"]
            .as_array()
            .expect("changed_docs array")
            .len(),
        0
    );
    assert!(parsed["task_close_allowed"].is_boolean());

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn closeout_task_outputs_default_toon_and_explicit_json() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("closeout-task");
    write_task_doc(&root, "TASK-CLOSEOUT");

    let toon = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-CLOSEOUT",
        ])
        .output()
        .expect("docflow closeout task toon should run");
    assert!(toon.status.success(), "{}", context.diagnostics(&toon));
    let toon_stdout = String::from_utf8_lossy(&toon.stdout);
    assert!(toon_stdout.starts_with("closeout\n  mode: task"));
    assert!(toon_stdout.contains("task_id: TASK-CLOSEOUT"));
    assert!(toon_stdout.contains("changed_doc_count: 1"));
    assert!(toon_stdout.contains("protocol_coverage_rows: 0"));
    assert!(!toon_stdout.trim_start().starts_with('{'));
    assert!(!toon_stdout.contains("--json"));

    let json = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-CLOSEOUT",
            "--json",
        ])
        .output()
        .expect("docflow closeout task json should run");
    assert!(json.status.success(), "{}", context.diagnostics(&json));
    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("closeout task json should parse");
    assert_eq!(parsed["command"], "closeout");
    assert_eq!(parsed["mode"], "task");
    assert_eq!(parsed["task_id"], "TASK-CLOSEOUT");
    assert_eq!(parsed["changed_doc_count"], 1);
    assert_eq!(parsed["protocol_coverage_rows"], 0);
    assert!(parsed["task_close_allowed"].is_boolean());

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn closeout_and_proofcheck_task_share_protocol_coverage_blocker() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("closeout-protocol-coverage");
    write_task_protocol_doc(&root, "TASK-PROTOCOL");

    let proof = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "proofcheck",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-PROTOCOL",
            "--json",
        ])
        .output()
        .expect("docflow proofcheck task protocol blocker should run");
    assert!(proof.status.success(), "{}", context.diagnostics(&proof));
    let proof_json: serde_json::Value =
        serde_json::from_slice(&proof.stdout).expect("proofcheck protocol json should parse");
    assert_eq!(proof_json["verdict"], "blocking");
    assert_eq!(proof_json["task_close_allowed"], false);
    assert_eq!(proof_json["protocol_coverage_rows"], 1);
    assert!(
        proof_json["blocker_codes"]
            .as_array()
            .expect("proof blockers should be array")
            .contains(&serde_json::Value::String(
                "docflow_protocol_coverage_blocking".to_string()
            )),
        "{proof_json}"
    );

    let closeout = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-PROTOCOL",
            "--json",
        ])
        .output()
        .expect("docflow closeout task protocol blocker should run");
    assert!(
        closeout.status.success(),
        "{}",
        context.diagnostics(&closeout)
    );
    let closeout_json: serde_json::Value =
        serde_json::from_slice(&closeout.stdout).expect("closeout protocol json should parse");
    assert_eq!(closeout_json["verdict"], proof_json["verdict"]);
    assert_eq!(
        closeout_json["task_close_allowed"],
        proof_json["task_close_allowed"]
    );
    assert_eq!(
        closeout_json["protocol_coverage_rows"],
        proof_json["protocol_coverage_rows"]
    );
    assert_eq!(closeout_json["blocker_codes"], proof_json["blocker_codes"]);

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn proofcheck_task_uses_root_docflow_evidence_with_default_profile() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("proofcheck-task-default-profile");
    write_task_doc(&root, "TASK-DEFAULT");

    let json = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "proofcheck",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--task",
            "TASK-DEFAULT",
            "--json",
            "--compact",
        ])
        .output()
        .expect("docflow proofcheck task json should run with default profile");
    assert!(json.status.success(), "{}", context.diagnostics(&json));
    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("proofcheck task json should parse");
    assert_eq!(parsed["command"], "proofcheck");
    assert_eq!(parsed["mode"], "task");
    assert_eq!(parsed["task_id"], "TASK-DEFAULT");
    assert_eq!(parsed["changed_doc_count"], 1);
    assert_eq!(parsed["verdict"], "blocking");
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    assert!(blocker_codes.contains(&serde_json::Value::String(
        "docflow_check_blocking".to_string()
    )));
    assert!(!blocker_codes.contains(&serde_json::Value::String(
        "docflow_closeout_failed".to_string()
    )));

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn closeout_changed_outputs_default_toon_and_explicit_json() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("closeout-changed");
    write_task_doc(&root, "TASK-2");
    let git_init = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("init")
        .output()
        .expect("git init should run");
    assert!(
        git_init.status.success(),
        "{}",
        String::from_utf8_lossy(&git_init.stderr)
    );

    let toon = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--changed",
        ])
        .output()
        .expect("docflow closeout changed toon should run");
    assert!(toon.status.success(), "{}", context.diagnostics(&toon));
    let toon_stdout = String::from_utf8_lossy(&toon.stdout);
    assert!(toon_stdout.starts_with("closeout\n  mode: changed"));
    assert!(toon_stdout.contains("changed_doc_count: 1"));
    assert!(!toon_stdout.trim_start().starts_with('{'));
    assert!(!toon_stdout.contains("--json"));

    let json = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--changed",
            "--json",
        ])
        .output()
        .expect("docflow closeout changed json should run");
    assert!(json.status.success(), "{}", context.diagnostics(&json));
    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("closeout changed json should parse");
    assert_eq!(parsed["command"], "closeout");
    assert_eq!(parsed["mode"], "changed");
    assert_eq!(parsed["changed_doc_count"], 1);
    assert_eq!(parsed["changed_docs"][0], "docs/process/a.md");
    assert!(parsed["task_close_allowed"].is_boolean());

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn closeout_changed_ignores_repo_local_fsmonitor_helper() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("closeout-fsmonitor");
    write_task_doc(&root, "TASK-FSMON");
    let git_init = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("init")
        .output()
        .expect("git init should run");
    assert!(
        git_init.status.success(),
        "{}",
        String::from_utf8_lossy(&git_init.stderr)
    );

    let sentinel = root.join("fsmonitor-sentinel.txt");
    let helper = root.join("fsmonitor-helper.cmd");
    let helper_path = helper.to_string_lossy().replace('\\', "/");
    fs::write(
        &helper,
        format!(
            "@echo off\r\necho invoked>\"{}\"\r\nexit /b 0\r\n",
            sentinel.display()
        ),
    )
    .expect("fsmonitor helper should be written");
    let git_config = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["config", "core.fsmonitor", &helper_path])
        .output()
        .expect("git config should run");
    assert!(
        git_config.status.success(),
        "{}",
        String::from_utf8_lossy(&git_config.stderr)
    );

    let git_status = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--short", "--", ":(glob)**/*.md"])
        .output()
        .expect("git status should run");
    assert!(
        git_status.status.success(),
        "{}",
        String::from_utf8_lossy(&git_status.stderr)
    );
    assert!(
        sentinel.exists(),
        "repo-local fsmonitor helper should execute during plain git status"
    );
    fs::remove_file(&sentinel).expect("sentinel should be removable before closeout");

    let closeout = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--changed",
        ])
        .output()
        .expect("docflow closeout changed should run");
    assert!(
        closeout.status.success(),
        "{}",
        context.diagnostics(&closeout)
    );
    let stdout = String::from_utf8_lossy(&closeout.stdout);
    assert!(stdout.starts_with("closeout\n  mode: changed"));
    assert!(stdout.contains("changed_doc_count: 1"));
    assert!(
        !sentinel.exists(),
        "repo-local fsmonitor helper should not execute during docflow closeout"
    );

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn closeout_and_proofcheck_report_missing_evidence_blockers() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("missing-evidence");
    fs::create_dir_all(&root).expect("root should be created");
    let git_init = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("init")
        .output()
        .expect("git init should run");
    assert!(
        git_init.status.success(),
        "{}",
        String::from_utf8_lossy(&git_init.stderr)
    );

    let proof = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "proofcheck",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--task",
            "TASK-MISSING",
            "--json",
        ])
        .output()
        .expect("docflow proofcheck missing evidence should run");
    assert!(proof.status.success(), "{}", context.diagnostics(&proof));
    let proof_json: serde_json::Value =
        serde_json::from_slice(&proof.stdout).expect("proofcheck missing json should parse");
    assert_eq!(proof_json["verdict"], "blocking");
    assert_eq!(proof_json["task_close_allowed"], false);
    assert_eq!(
        proof_json["blocker_codes"][0],
        "missing_docflow_task_evidence"
    );

    let closeout = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([
            "closeout",
            "--root",
            root.to_str().expect("root should be utf8"),
            "--profile",
            "",
            "--changed",
            "--json",
        ])
        .output()
        .expect("docflow closeout no changed docs should run");
    assert!(
        closeout.status.success(),
        "{}",
        context.diagnostics(&closeout)
    );
    let closeout_json: serde_json::Value =
        serde_json::from_slice(&closeout.stdout).expect("closeout no changed json should parse");
    assert_eq!(closeout_json["verdict"], "blocking");
    assert_eq!(closeout_json["task_close_allowed"], false);
    assert_eq!(closeout_json["blocker_codes"][0], "no_changed_docflow_docs");

    fs::remove_dir_all(root).expect("root should be removed");
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
