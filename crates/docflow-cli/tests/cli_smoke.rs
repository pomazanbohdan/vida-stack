use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rstest::rstest;
use sha2::{Digest, Sha256};

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

fn protocol_compression_hash_content(content: &str) -> String {
    let excluded = [
        "updated_at:",
        "protocol_compression_audit_at:",
        "protocol_compression_before_tokens:",
        "protocol_compression_after_tokens:",
        "protocol_compression_content_sha256:",
    ];
    let mut stripped = content
        .lines()
        .filter(|line| {
            !excluded
                .iter()
                .any(|prefix| line.trim_start().starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n");
    if content.ends_with('\n') {
        stripped.push('\n');
    }
    stripped
}

fn write_active_canon_fixture(root: &std::path::Path, source_path: &str) -> PathBuf {
    let directory = root.join("vida/config/instructions/instruction-contracts");
    fs::create_dir_all(&directory).expect("instruction directory should be created");
    fs::create_dir_all(root.join("vida/config/docflow")).expect("docflow config should be created");
    fs::write(root.join("vida.config.yaml"), "version: 1\n").expect("root marker");
    fs::write(
        root.join("vida/config/docflow/docsys_policy.yaml"),
        "schema_version: 1\nprofiles:\n  active-canon:\n    - vida/config/instructions\n  active-canon-strict:\n    - vida/config/instructions\n",
    )
    .expect("docflow policy");

    let without_hash = format!(
        "# Synthetic Protocol\n\n-----\nartifact_path: instruction-contracts/synthetic\nartifact_type: instruction_contract\nartifact_version: '1'\nartifact_revision: test\nschema_version: '1'\nstatus: canonical\nsource_path: {source_path}\ncreated_at: 2026-07-03T00:00:00+03:00\nupdated_at: 2026-07-03T00:00:00+03:00\nchangelog_ref: synthetic.changelog.jsonl\nprotocol_authoring_gate: enforced\nprotocol_compression_status: audit_passed\nprotocol_compression_algorithm: semantic-atom-coverage\nprotocol_compression_baseline_ref: test\nprotocol_compression_audit_at: 2026-07-03T00:00:00+03:00\nprotocol_compression_before_tokens: 100\nprotocol_compression_after_tokens: 80\n"
    );
    let hash = Sha256::digest(protocol_compression_hash_content(&without_hash).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let path = directory.join("synthetic.md");
    fs::write(
        &path,
        format!("{without_hash}protocol_compression_content_sha256: {hash}\n"),
    )
    .expect("protocol fixture");
    fs::write(directory.join("synthetic.changelog.jsonl"), "{}\n").expect("protocol changelog");
    path
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
    vida_test_support::assert_text_snapshot(stdout.trim_end(), "relations\n  total_edges: 3");
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
    assert!(stdout.contains("machine-readable output is explicit opt-in"));
    assert!(!stdout.contains("docflow init --json"));
    assert!(!stdout.contains("docflow check-file --path <file> --json"));
    assert!(!stdout.contains("Prefer --json"));
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
fn init_help_keeps_json_as_explicit_machine_mode() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["init", "--help"])
        .output()
        .expect("docflow init help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow init"));
    assert!(stdout.contains("Machine-readable mode:"));
    assert!(stdout.contains("explicit JSON output is opt-in for stable machine payloads"));
    assert!(!stdout.contains("docflow init --json"));
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

#[rstest]
#[case::check_help_exposes_json_mode("check", true)]
#[case::fastcheck_help_does_not_expose_json_mode("fastcheck", false)]
fn help_exposes_json_mode_when_command_supports_it(#[case] command: &str, #[case] has_json: bool) {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args([command, "--help"])
        .output()
        .expect("docflow help should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.contains("--json"), has_json);
}

#[test]
fn check_json_renders_blocked_and_pass_envelopes() {
    let context = vida_test_support::CommandContext::empty();
    let blocked_temp = vida_test_support::temp_fixture_dir();
    let blocked_root = blocked_temp.path().to_path_buf();
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

    let pass_temp = vida_test_support::temp_fixture_dir();
    let pass_root = pass_temp.path().to_path_buf();
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
}

#[test]
fn check_default_and_json_bind_relative_paths_to_root() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("check-root-binding");
    fs::create_dir_all(root.join("docs/process")).expect("process dir should be created");
    fs::write(root.join("docs/process/a.md"), "# a\n").expect("process markdown");
    let root_arg = root.to_string_lossy().to_string();

    let default_output = run_docflow_owned(vec![
        "check".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "docs/process/a.md".to_string(),
    ]);
    assert_docflow_success(&context, &default_output);
    let default_stdout = String::from_utf8_lossy(&default_output.stdout);
    assert!(default_stdout.contains("\"path\":\"docs/process/a.md\""));
    assert!(default_stdout.contains("\"missing_footer\""));
    assert!(!default_stdout.contains(&root_arg));

    let json_output = run_docflow_owned(vec![
        "check".to_string(),
        "--root".to_string(),
        root_arg,
        "--json".to_string(),
        "docs/process/a.md".to_string(),
    ]);
    assert_docflow_success(&context, &json_output);
    let parsed: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("check json should parse");
    assert_eq!(parsed["surface"], "docflow check");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["rows"][0]["path"], "docs/process/a.md");
    assert!(
        parsed["rows"][0]["issues"]
            .as_array()
            .expect("issues should be an array")
            .contains(&serde_json::Value::String("missing_footer".to_string()))
    );

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn active_canon_source_path_is_consistent_across_public_docflow_surfaces() {
    let context = vida_test_support::CommandContext::empty();
    let fixture = vida_test_support::temp_fixture_dir();
    let root = fixture.path().to_path_buf();
    let path = write_active_canon_fixture(
        &root,
        "vida/config/instructions/instruction-contracts/synthetic.md",
    );
    let root_arg = root.to_string_lossy().to_string();
    let path_arg = path.to_string_lossy().to_string();

    let check = run_docflow_owned_at_root(
        &root,
        vec![
            "check",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert_docflow_success(&context, &check);
    let check_json: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("check json should parse");
    assert_eq!(check_json["status"], "pass", "{check_json}");
    assert_eq!(check_json["row_count"], 0, "{check_json}");

    let fastcheck = run_docflow_owned_at_root(
        &root,
        vec![
            "fastcheck",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
        ],
    );
    assert_docflow_success(&context, &fastcheck);
    assert!(
        !String::from_utf8_lossy(&fastcheck.stdout)
            .contains("invalid_protocol_compression_source_path")
    );

    let readiness = run_docflow_owned_at_root(
        &root,
        vec![
            "readiness-check",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert_docflow_success(&context, &readiness);
    let readiness_json: serde_json::Value =
        serde_json::from_slice(&readiness.stdout).expect("readiness json should parse");
    assert_eq!(readiness_json["status"], "pass", "{readiness_json}");
    assert_eq!(readiness_json["row_count"], 0, "{readiness_json}");

    let proofcheck = run_docflow_owned_at_root(
        &root,
        vec![
            "proofcheck",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert_docflow_success(&context, &proofcheck);
    let proofcheck_json: serde_json::Value =
        serde_json::from_slice(&proofcheck.stdout).expect("proofcheck json should parse");
    assert_eq!(proofcheck_json["verdict"], "ok", "{proofcheck_json}");
    assert_eq!(proofcheck_json["fastcheck_rows"], 0, "{proofcheck_json}");
    assert_eq!(proofcheck_json["readiness_rows"], 0, "{proofcheck_json}");

    let check_file =
        run_docflow_owned_at_root(&root, vec!["check-file", "--path", &path_arg, "--json"]);
    assert_docflow_success(&context, &check_file);
    let check_file_json: serde_json::Value =
        serde_json::from_slice(&check_file.stdout).expect("check-file json should parse");
    assert_eq!(
        check_file_json["validation"]["verdict"], "ok",
        "{check_file_json}"
    );
}

#[test]
fn active_canon_source_path_public_surfaces_block_lexical_traversal() {
    let context = vida_test_support::CommandContext::empty();
    let fixture = vida_test_support::temp_fixture_dir();
    let root = fixture.path().to_path_buf();
    let path = write_active_canon_fixture(
        &root,
        "vida/config/instructions/instruction-contracts/../synthetic.md",
    );
    let root_arg = root.to_string_lossy().to_string();
    let path_arg = path.to_string_lossy().to_string();

    let check = run_docflow_owned_at_root(
        &root,
        vec![
            "check",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert_docflow_success(&context, &check);
    let check_json: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("check json should parse");
    assert_eq!(check_json["status"], "blocked", "{check_json}");
    assert!(
        check_json["rows"][0]["issues"]
            .as_array()
            .expect("check issues")
            .iter()
            .any(|issue| issue == "invalid_protocol_compression_source_path")
    );

    let fastcheck = run_docflow_owned_at_root(
        &root,
        vec![
            "fastcheck",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
        ],
    );
    assert_docflow_success(&context, &fastcheck);
    assert!(
        String::from_utf8_lossy(&fastcheck.stdout)
            .contains("invalid_protocol_compression_source_path")
    );

    let readiness = run_docflow_owned_at_root(
        &root,
        vec![
            "readiness-check",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert_docflow_success(&context, &readiness);
    let readiness_json: serde_json::Value =
        serde_json::from_slice(&readiness.stdout).expect("readiness json should parse");
    assert_eq!(readiness_json["status"], "blocked", "{readiness_json}");
    assert!(readiness_json["row_count"].as_u64().unwrap_or(0) > 0);

    let proofcheck = run_docflow_owned_at_root(
        &root,
        vec![
            "proofcheck",
            "--root",
            &root_arg,
            "--profile",
            "active-canon",
            "--json",
        ],
    );
    assert!(
        !proofcheck.status.success(),
        "{}",
        context.diagnostics(&proofcheck)
    );
    let proofcheck_json: serde_json::Value =
        serde_json::from_slice(&proofcheck.stdout).expect("proofcheck json should parse");
    assert_eq!(proofcheck_json["verdict"], "blocking", "{proofcheck_json}");
    assert!(proofcheck_json["fastcheck_rows"].as_u64().unwrap_or(0) > 0);

    let check_file =
        run_docflow_owned_at_root(&root, vec!["check-file", "--path", &path_arg, "--json"]);
    assert_docflow_success(&context, &check_file);
    let check_file_json: serde_json::Value =
        serde_json::from_slice(&check_file.stdout).expect("check-file json should parse");
    assert_eq!(check_file_json["validation"]["verdict"], "blocking");
    assert!(
        check_file_json["validation"]["issues"]
            .as_array()
            .expect("validation issues")
            .iter()
            .any(|issue| issue["code"] == "invalid_protocol_compression_source_path")
    );
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

fn run_docflow_owned(args: Vec<String>) -> std::process::Output {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(args)
        .output()
        .expect("docflow binary should run")
}

fn run_docflow_owned_at_root(root: &std::path::Path, args: Vec<&str>) -> std::process::Output {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .env("VIDA_ROOT", root)
        .args(args)
        .output()
        .expect("docflow binary should run")
}

fn assert_docflow_success(
    context: &vida_test_support::CommandContext,
    output: &std::process::Output,
) {
    assert!(output.status.success(), "{}", context.diagnostics(output));
}

#[test]
fn file_commands_render_validation_readiness_reporting_and_read_errors() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("file-commands");
    write_task_doc(&root, "TASK-FILE");
    let file = root.join("docs/process/a.md");
    let file_arg = file.to_string_lossy().to_string();
    let body = fs::read_to_string(&file).expect("fixture body should be readable");

    let validation = run_docflow_owned(vec![
        "validate-footer".to_string(),
        "--path".to_string(),
        "docs/process/a.md".to_string(),
        "--content".to_string(),
        body.clone(),
    ]);
    assert_docflow_success(&context, &validation);
    assert!(String::from_utf8_lossy(&validation.stdout).starts_with("validation\n"));

    let readiness = run_docflow_owned(vec![
        "readiness".to_string(),
        "--path".to_string(),
        "docs/process/a.md".to_string(),
        "--content".to_string(),
        body,
    ]);
    assert_docflow_success(&context, &readiness);
    assert!(String::from_utf8_lossy(&readiness.stdout).starts_with("readiness\n"));

    for (command, root_key, count_key) in [
        ("check-file", "validation", "issue_count"),
        ("readiness-file", "readiness", "row_count"),
        ("report-check", "reporting", "issue_count"),
    ] {
        let output = run_docflow_owned(vec![
            command.to_string(),
            "--path".to_string(),
            file_arg.clone(),
            "--json".to_string(),
        ]);
        assert_docflow_success(&context, &output);
        let parsed: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("file command json should parse");
        assert_eq!(parsed[root_key]["path"], file_arg);
        assert!(parsed[root_key][count_key].is_number(), "{parsed}");
    }

    let missing = run_docflow_owned(vec![
        "check-file".to_string(),
        "--path".to_string(),
        root.join("missing.md").to_string_lossy().to_string(),
        "--json".to_string(),
    ]);
    assert_docflow_success(&context, &missing);
    let missing_json: serde_json::Value =
        serde_json::from_slice(&missing.stdout).expect("missing file json should parse");
    assert_eq!(missing_json["validation"]["verdict"], "blocking");
    assert_eq!(
        missing_json["validation"]["errors"][0]["code"],
        "read_error"
    );

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn relation_changelog_and_impact_commands_cover_docflow_fixture_paths() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("relation-impact");
    write_task_doc(&root, "TASK-IMPACT");
    let file = root.join("docs/process/a.md");
    let file_arg = file.to_string_lossy().to_string();
    let root_arg = root.to_string_lossy().to_string();

    let deps = run_docflow_owned(vec![
        "deps".to_string(),
        "--path".to_string(),
        file_arg.clone(),
    ]);
    assert_docflow_success(&context, &deps);
    let deps_json: serde_json::Value =
        serde_json::from_slice(&deps.stdout).expect("deps json should parse");
    assert_eq!(deps_json["path"], file_arg);

    for command in ["links", "deps-map"] {
        let output = run_docflow_owned(vec![
            command.to_string(),
            "--path".to_string(),
            file_arg.clone(),
        ]);
        assert_docflow_success(&context, &output);
    }

    let changelog = run_docflow_owned(vec![
        "changelog".to_string(),
        root.join("docs/process/a.changelog.jsonl")
            .to_string_lossy()
            .to_string(),
        "--limit".to_string(),
        "1".to_string(),
        "--newest-first".to_string(),
    ]);
    assert_docflow_success(&context, &changelog);
    assert!(String::from_utf8_lossy(&changelog.stdout).starts_with("changelog\n"));

    let changelog_task = run_docflow_owned(vec![
        "changelog-task".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "--task-id".to_string(),
        "TASK-IMPACT".to_string(),
    ]);
    assert_docflow_success(&context, &changelog_task);
    assert!(String::from_utf8_lossy(&changelog_task.stdout).starts_with("changelog-task\n"));

    let task_summary = run_docflow_owned(vec![
        "task-summary".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "--task-id".to_string(),
        "TASK-IMPACT".to_string(),
        "--format".to_string(),
        "jsonl".to_string(),
    ]);
    assert_docflow_success(&context, &task_summary);
    let summary_line = String::from_utf8_lossy(&task_summary.stdout)
        .lines()
        .next()
        .expect("task summary should render a row")
        .to_string();
    let summary_json: serde_json::Value =
        serde_json::from_str(&summary_line).expect("task summary jsonl should parse");
    assert_eq!(summary_json["summary"], "task");
    assert_eq!(summary_json["task_id"], "TASK-IMPACT");

    let artifact_impact = run_docflow_owned(vec![
        "artifact-impact".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "--artifact".to_string(),
        "process/a".to_string(),
        "--format".to_string(),
        "jsonl".to_string(),
    ]);
    assert_docflow_success(&context, &artifact_impact);
    let artifact_json: serde_json::Value =
        serde_json::from_slice(&artifact_impact.stdout).expect("artifact impact json should parse");
    assert_eq!(artifact_json["command"], "artifact-impact");
    assert_eq!(artifact_json["artifact"], "process/a");

    let task_impact = run_docflow_owned(vec![
        "task-impact".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "--task-id".to_string(),
        "TASK-IMPACT".to_string(),
        "--format".to_string(),
        "jsonl".to_string(),
    ]);
    assert_docflow_success(&context, &task_impact);
    let task_impact_json: serde_json::Value =
        serde_json::from_slice(&task_impact.stdout).expect("task impact json should parse");
    assert_eq!(task_impact_json["command"], "task-impact");
    assert_eq!(task_impact_json["task_id"], "TASK-IMPACT");

    fs::remove_dir_all(root).expect("root should be removed");
}

#[test]
fn tree_scan_commands_cover_registry_validation_and_readiness_outputs() {
    let context = vida_test_support::CommandContext::empty();
    let root = unique_docflow_root("tree-scan");
    write_task_doc(&root, "TASK-TREE");
    let root_arg = root.to_string_lossy().to_string();

    for (command, expected) in [
        ("summary", "summary\n"),
        ("overview-scan", "docflow overview\n"),
        ("relations-scan", "relations\n"),
        ("scan", "{\"artifact_path\""),
        ("registry-scan", "registry\n"),
        ("registry", "{\"artifact_path\""),
        ("validate-tree", "validation-tree\n"),
        ("readiness-tree", "readiness-tree\n"),
    ] {
        let output = run_docflow_owned(vec![
            command.to_string(),
            "--root".to_string(),
            root_arg.clone(),
        ]);
        assert_docflow_success(&context, &output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.starts_with(expected) || stdout.contains(expected),
            "expected {expected:?} in {command} stdout: {stdout}"
        );
    }

    let readiness_check = run_docflow_owned(vec![
        "readiness-check".to_string(),
        "--root".to_string(),
        root_arg.clone(),
        "--format".to_string(),
        "toon".to_string(),
        "docs/process/a.md".to_string(),
    ]);
    assert_docflow_success(&context, &readiness_check);
    assert!(String::from_utf8_lossy(&readiness_check.stdout).contains("readiness-check"));

    fs::remove_dir_all(root).expect("root should be removed");
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
    assert!(!toon.status.success(), "{}", context.diagnostics(&toon));
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
    assert!(!json.status.success(), "{}", context.diagnostics(&json));
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
    assert!(!toon.status.success(), "{}", context.diagnostics(&toon));
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
    assert!(!json.status.success(), "{}", context.diagnostics(&json));
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
    assert!(!proof.status.success(), "{}", context.diagnostics(&proof));
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
        !closeout.status.success(),
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
    assert!(!json.status.success(), "{}", context.diagnostics(&json));
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
fn proofcheck_unsupported_format_fails_closed_with_diagnostics() {
    let context = vida_test_support::CommandContext::empty();
    let output = vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_docflow"))
        .args(["proofcheck", "--format", "xml"])
        .output()
        .expect("docflow proofcheck unsupported format should run");

    assert!(output.status.success(), "{}", context.diagnostics(&output));
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("proofcheck json should parse");
    assert_eq!(parsed["command"], "proofcheck");
    assert_eq!(parsed["verdict"], "blocking");
    assert_eq!(parsed["error"], "unsupported_format:xml");
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
    assert!(!toon.status.success(), "{}", context.diagnostics(&toon));
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
    assert!(!json.status.success(), "{}", context.diagnostics(&json));
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
    assert_closeout_changed_ignores_repo_local_fsmonitor_helper();
}

#[test]
fn closeout_changed_disables_repo_local_fsmonitor_helper() {
    assert_closeout_changed_ignores_repo_local_fsmonitor_helper();
}

fn assert_closeout_changed_ignores_repo_local_fsmonitor_helper() {
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
        !closeout.status.success(),
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
    assert!(!proof.status.success(), "{}", context.diagnostics(&proof));
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
        !closeout.status.success(),
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
