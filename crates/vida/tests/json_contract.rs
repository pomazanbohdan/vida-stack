use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use vida_test_support::{self as support, CliContractCase};

fn vida() -> Command {
    support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}/vida-json-contract-{}-{nanos}-{counter}",
        std::env::temp_dir().display(),
        std::process::id()
    )
}

fn unique_local_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::current_dir()
        .expect("test cwd should resolve")
        .join("target")
        .join("tmp")
        .join(format!("{prefix}-{}-{nanos}-{counter}", std::process::id()))
}

fn write_requirement_project_markers(project_root: &std::path::Path) {
    fs::write(project_root.join("AGENTS.md"), "# test project\n")
        .expect("project bootstrap marker should be written");
    fs::write(project_root.join("vida.config.yaml"), "project: {}\n")
        .expect("project marker should be written");
    for marker in [".vida/config", ".vida/db", ".vida/project"] {
        fs::create_dir_all(project_root.join(marker)).expect("project runtime marker should exist");
    }
}

fn run_json(state_dir: &str, args: &[&str]) -> serde_json::Value {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .unwrap_or_else(|error| panic!("{} should run: {error}", args.join(" ")));
    parse_json_output(args, &output)
}

fn parse_json_output(args: &[&str], output: &Output) -> serde_json::Value {
    assert!(
        !output.stdout.is_empty(),
        "{} should emit JSON on stdout; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{} stdout should parse as JSON: {error}\nstdout={}\nstderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn json_contract_major_operator_surfaces_keep_release1_shape() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: {}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let cases = [
        CliContractCase {
            args: &["status", "--json"],
            surface: "vida status",
        },
        CliContractCase {
            args: &["doctor", "--json"],
            surface: "vida doctor",
        },
        CliContractCase {
            args: &["task", "ready", "--json"],
            surface: "vida task ready",
        },
        CliContractCase {
            args: &["task", "validate-graph", "--json"],
            surface: "vida task validate-graph",
        },
        CliContractCase {
            args: &["taskflow", "graph-summary", "--json"],
            surface: "vida taskflow graph-summary",
        },
        CliContractCase {
            args: &["taskflow", "scheduler", "dispatch", "--json"],
            surface: "vida taskflow scheduler dispatch",
        },
    ];
    support::assert_cli_contract_matrix(cases, |args| run_json(&state_dir, args));

    let reset_state_dir = unique_state_dir();
    std::fs::create_dir_all(&reset_state_dir).expect("reset state dir should exist");
    let reset = vida()
        .args([
            "state",
            "reset",
            "--archive",
            "--reinit",
            "--state-dir",
            &reset_state_dir,
            "--json",
        ])
        .output()
        .expect("state reset should run");
    assert!(
        reset.status.success(),
        "state reset should succeed: {}",
        String::from_utf8_lossy(&reset.stderr)
    );
    let reset_value = parse_json_output(&["state", "reset", "--json"], &reset);
    support::assert_release1_operator_shape("vida state reset", &reset_value);
}

#[test]
fn json_contract_harness_rejects_missing_operator_fields() {
    let valid = json!({
        "surface": "vida test",
        "status": "blocked",
        "blocker_codes": ["example_blocker"],
        "next_actions": ["repair the example"],
        "artifact_refs": {},
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": ["example_blocker"],
            "next_actions": ["repair the example"],
            "artifact_refs": {}
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": ["example_blocker"],
            "next_actions": ["repair the example"],
            "artifact_refs": {}
        }
    });
    assert_eq!(support::release1_operator_shape_error(&valid), None);

    let mut missing_blockers = valid.clone();
    missing_blockers
        .as_object_mut()
        .expect("valid object")
        .remove("blocker_codes");
    assert_eq!(
        support::release1_operator_shape_error(&missing_blockers).as_deref(),
        Some("missing blocker_codes")
    );

    let mut missing_actions = valid;
    missing_actions
        .as_object_mut()
        .expect("valid object")
        .remove("next_actions");
    assert_eq!(
        support::release1_operator_shape_error(&missing_actions).as_deref(),
        Some("missing next_actions")
    );
}

#[test]
fn requirement_analysis_source_file_is_project_bounded_and_redacted() {
    let state_dir = unique_local_dir("vida-json-contract-source-state");
    let project_root = unique_local_dir("vida-json-contract-source-project");
    fs::create_dir_all(&project_root).expect("project root should exist");
    write_requirement_project_markers(&project_root);
    fs::write(
        project_root.join("requirements.md"),
        "SECRET_TOKEN=must-not-be-serialized\nBuild the feature.",
    )
    .expect("source fixture should be written");
    fs::write(project_root.join("requirements.md"), {
        let bare_pem_label = "PRIVATE KEY";
        let bare_pem_begin = format!("-----BEGIN {bare_pem_label}-----");
        let bare_pem_body = "BAREPEMBODYSHOULDNOTLEAK";
        let bare_pem_end = format!("-----END {bare_pem_label}-----");
        [
            bare_pem_begin.as_str(),
            bare_pem_body,
            bare_pem_end.as_str(),
            "Build the feature.",
            "Password reset: allow users to rotate credentials.",
            "Token-based auth: add OAuth.",
            "SECRET_TOKEN=[redacted-test-value]",
            "SECRET_TOKEN = [redacted-test-value]",
            "api_key: [redacted-test-value]",
            "private-key: [redacted-test-value]",
            "PRIVATE_KEY=\"-----BEGIN TEST KEY-----",
            "MIISECRETBODYSHOULDNOTLEAK",
            "-----END TEST KEY-----\"",
            "Keep operator output compact.",
            "Add JSON proof.",
            "Validate blocked source paths.",
            "Reject symlinks.",
            "Keep developer handoff complete.",
            "Document source metadata.",
            "Preserve project-root semantics.",
            "Avoid stale artifact fields.",
            "Keep readiness verdict stable.",
            "Preserve requirement twelve.",
            "Preserve requirement thirteen.",
        ]
        .join("\n")
    })
    .expect("source fixture should be rewritten");

    let json_output = vida()
        .current_dir(&project_root)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "source-file-redaction",
            "--source-file",
            "requirements.md",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("requirement analyze json should run");
    assert!(
        json_output.status.success(),
        "project-relative regular source should succeed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &json_output,
    );
    let artifact = &value["artifact"];
    let source_text = artifact["source_inputs"][0]["text"]
        .as_str()
        .expect("source text should render");
    let bare_pem_begin_marker = format!("-----BEGIN {}-----", "PRIVATE KEY");
    let bare_pem_end_marker = format!("-----END {}-----", "PRIVATE KEY");
    assert_eq!(artifact["source_inputs"][0]["kind"], "source_file");
    assert!(source_text.contains("Build the feature"));
    assert!(source_text.contains("Preserve requirement thirteen"));
    assert!(
        !source_text.contains("SECRET_TOKEN"),
        "raw source-file content must not be serialized: {source_text}"
    );
    assert!(
        !source_text.contains("api_key") && !source_text.contains("[redacted-test-value]"),
        "spaced and YAML-style source-file secrets must be redacted: {source_text}"
    );
    for marker in [
        "private-key",
        "PRIVATE_KEY",
        "MIISECRETBODYSHOULDNOTLEAK",
        "-----BEGIN TEST KEY-----",
        "-----END TEST KEY-----",
        bare_pem_begin_marker.as_str(),
        "BAREPEMBODYSHOULDNOTLEAK",
        bare_pem_end_marker.as_str(),
    ] {
        assert!(
            !source_text.contains(marker),
            "source-file secret marker {marker} must be redacted: {source_text}"
        );
    }
    let source_metadata = artifact["source_inputs"][0]["source_metadata"]
        .as_str()
        .expect("source metadata should render");
    assert!(source_metadata.starts_with("file:requirements.md:bytes="));
    assert!(source_metadata.contains(":blake3="));
    let public_analysis_text = artifact["source_inputs"][0]["analysis_text"]
        .as_str()
        .expect("redacted source analysis should render");
    assert!(
        !public_analysis_text.contains("SECRET_TOKEN"),
        "redacted analysis must not disclose source-file secrets: {public_analysis_text}"
    );
    assert!(
        !public_analysis_text.contains("api_key")
            && !public_analysis_text.contains("[redacted-test-value]"),
        "redacted analysis must hide spaced and YAML-style source-file secrets: {public_analysis_text}"
    );
    for marker in [
        "private-key",
        "PRIVATE_KEY",
        "MIISECRETBODYSHOULDNOTLEAK",
        "-----BEGIN TEST KEY-----",
        "-----END TEST KEY-----",
        bare_pem_begin_marker.as_str(),
        "BAREPEMBODYSHOULDNOTLEAK",
        bare_pem_end_marker.as_str(),
    ] {
        assert!(
            !public_analysis_text.contains(marker),
            "redacted analysis must hide source-file secret marker {marker}: {public_analysis_text}"
        );
    }
    let raw_source = fs::read_to_string(project_root.join("requirements.md"))
        .expect("source fixture should still be readable");
    let raw_digest = blake3::hash(raw_source.as_bytes()).to_string();
    let public_digest = blake3::hash(public_analysis_text.as_bytes()).to_string();
    assert!(
        !source_metadata.contains(&raw_digest),
        "source metadata must not hash unredacted source-file contents: {source_metadata}"
    );
    assert!(
        source_metadata.contains(&public_digest),
        "source metadata should hash the public redacted source text: {source_metadata}"
    );
    assert!(
        source_metadata.contains(":redacted=true"),
        "source metadata should disclose that public source text was redacted: {source_metadata}"
    );
    assert!(
        source_text.contains("Password reset: allow users to rotate credentials.")
            && source_text.contains("Token-based auth: add OAuth."),
        "prose requirement headings mentioning password/token should be preserved: {source_text}"
    );
    assert!(
        public_analysis_text.contains("Password reset: allow users to rotate credentials.")
            && public_analysis_text.contains("Token-based auth: add OAuth."),
        "analysis text should preserve prose requirement headings mentioning password/token: {public_analysis_text}"
    );
    assert!(
        public_analysis_text.contains("Preserve requirement thirteen"),
        "redacted analysis should preserve requirements beyond the atom cap: {public_analysis_text}"
    );
    assert!(artifact["requirement_atoms"]
        .as_array()
        .expect("atoms should render")
        .iter()
        .any(|atom| atom["text"]
            .as_str()
            .is_some_and(|text| text.contains("Build the feature"))));
    assert!(
        !artifact["requirement_atoms"]
            .as_array()
            .expect("atoms should render")
            .iter()
            .any(|atom| atom["text"]
                .as_str()
                .is_some_and(|text| text.contains("SECRET_TOKEN")
                    || text.contains("api_key")
                    || text.contains("private-key")
                    || text.contains("PRIVATE_KEY")
                    || text.contains("MIISECRETBODYSHOULDNOTLEAK")
                    || text.contains("BAREPEMBODYSHOULDNOTLEAK")
                    || text.contains("[redacted-test-value]"))),
        "source-file secrets must not leak through requirement atoms"
    );
    assert!(
        artifact["requirement_atoms"]
            .as_array()
            .expect("atoms should render")
            .iter()
            .any(|atom| atom["text"].as_str().is_some_and(
                |text| text.contains("Password reset: allow users to rotate credentials")
            ))
            && artifact["requirement_atoms"]
                .as_array()
                .expect("atoms should render")
                .iter()
                .any(|atom| atom["text"]
                    .as_str()
                    .is_some_and(|text| text.contains("Token-based auth: add OAuth"))),
        "requirement atoms should preserve prose headings mentioning password/token"
    );

    let nested_dir = project_root.join("nested");
    fs::create_dir_all(&nested_dir).expect("nested cwd should exist");
    let nested_output = vida()
        .current_dir(&nested_dir)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "nested-source-file",
            "--source-file",
            "requirements.md",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("nested source resolution should run");
    assert!(
        nested_output.status.success(),
        "source paths should resolve from project root even when cwd is nested: {}",
        String::from_utf8_lossy(&nested_output.stderr)
    );

    let absolute_source = project_root.join("requirements.md");
    let absolute_output = vida()
        .current_dir(&project_root)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "absolute-source",
            "--source-file",
            absolute_source
                .to_str()
                .expect("absolute source path should be utf8"),
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("absolute source rejection should run");
    assert!(
        !absolute_output.status.success(),
        "absolute source paths should fail closed"
    );
    let absolute_value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &absolute_output,
    );
    assert_eq!(absolute_value["status"], "blocked");
    assert_eq!(
        absolute_value["blocker_codes"],
        json!(["requirement_source_unreadable"])
    );

    let traversal_output = vida()
        .current_dir(&project_root)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "traversal-source",
            "--source-file",
            "../requirements.md",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("traversal source rejection should run");
    assert!(
        !traversal_output.status.success(),
        "parent traversal source paths should fail closed"
    );

    let stray_root = std::path::PathBuf::from(unique_state_dir());
    fs::create_dir_all(&stray_root).expect("stray cwd should exist");
    fs::write(stray_root.join("requirements.md"), "Build outside project.")
        .expect("stray source should write");
    let stray_output = vida()
        .current_dir(&stray_root)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "stray-source",
            "--source-file",
            "requirements.md",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("stray source rejection should run");
    assert!(
        !stray_output.status.success(),
        "source files should fail closed when project root resolution fails"
    );
    let stray_value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &stray_output,
    );
    assert_eq!(stray_value["status"], "blocked");
    assert_eq!(
        stray_value["blocker_codes"],
        json!(["requirement_source_unreadable"])
    );

    let _ = fs::remove_dir_all(&project_root);
    let _ = fs::remove_dir_all(&stray_root);
    let _ = fs::remove_dir_all(&state_dir);
}

#[cfg(unix)]
#[test]
fn requirement_analysis_source_file_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let state_dir = unique_state_dir();
    let project_root = format!("{}-project", unique_state_dir());
    fs::create_dir_all(&project_root).expect("project root should exist");
    write_requirement_project_markers(std::path::Path::new(&project_root));
    let sensitive_path = format!("{}-sensitive.env", unique_state_dir());
    fs::write(
        &sensitive_path,
        "VIDA_SECRET_TOKEN=local-file-disclosure-proof",
    )
    .expect("sensitive fixture should be written");
    symlink(&sensitive_path, format!("{project_root}/requirements.md"))
        .expect("symlink fixture should be created");

    let output = vida()
        .current_dir(&project_root)
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "symlink-source",
            "--source-file",
            "requirements.md",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("symlink source rejection should run");
    assert!(!output.status.success(), "symlinks should fail closed");
    let value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &output,
    );
    assert_eq!(value["status"], "blocked");
    assert_eq!(
        value["blocker_codes"],
        json!(["requirement_source_unreadable"])
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("VIDA_SECRET_TOKEN"),
        "blocked payload must not disclose symlink target content"
    );

    let _ = fs::remove_dir_all(&project_root);
    let _ = fs::remove_file(&sensitive_path);
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn requirement_analysis_cli_contract() {
    let json_output = vida()
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "ra-artifact-schema-cli-design-20260630",
            "--input",
            "operator request",
            "--json",
        ])
        .output()
        .expect("requirement analyze json should run");
    assert!(
        json_output.status.success(),
        "requirement analyze json should succeed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let value = parse_json_output(&["requirement", "analyze", "--json"], &json_output);
    support::assert_release1_operator_shape("vida requirement analyze", &value);
    assert_eq!(value["status"].as_str(), Some("pass"));
    let artifact = &value["artifact"];
    for field in [
        "source_inputs",
        "requirement_classification",
        "depth_mode",
        "requirement_atoms",
        "selected_methods",
        "selected_roles",
        "role_findings_summary",
        "detected_conflicts",
        "open_questions",
        "working_assumptions",
        "solution_options",
        "recommended_option",
        "readiness_verdict",
        "downstream_routes",
        "acceptance_criteria",
        "test_matrix",
        "output_contract",
        "codebase_impact",
        "developer_handoff",
    ] {
        assert!(artifact.get(field).is_some(), "missing field {field}");
    }
    assert_eq!(
        artifact["task_id"].as_str(),
        Some("ra-artifact-schema-cli-design-20260630")
    );
    assert!(artifact["request_id"].is_null());
    assert!(artifact["open_questions"]["critical"].is_array());
    assert!(artifact["open_questions"]["important"].is_array());
    assert!(artifact["open_questions"]["optional"].is_array());
    assert_eq!(
        artifact["output_contract"]["default"]["mode"],
        "compact_toon_plain"
    );
    assert_eq!(
        artifact["output_contract"]["json"]["mode"],
        "machine_readable"
    );

    let missing_identity = vida()
        .args(["requirement", "analyze", "--json"])
        .output()
        .expect("requirement analyze missing identity should run");
    assert!(
        !missing_identity.status.success(),
        "missing identity should fail closed"
    );
    let missing_identity_value =
        parse_json_output(&["requirement", "analyze", "--json"], &missing_identity);
    support::assert_release1_operator_shape("vida requirement analyze", &missing_identity_value);
    assert_eq!(missing_identity_value["status"].as_str(), Some("blocked"));
    assert_eq!(
        missing_identity_value["blocker_codes"],
        json!(["missing_requirement_identity"])
    );

    let unreadable_source = vida()
        .args([
            "requirement",
            "analyze",
            "--task-id",
            "task-1",
            "--source-file",
            "missing-requirement-source.md",
            "--json",
        ])
        .output()
        .expect("requirement analyze unreadable source should run");
    assert!(
        !unreadable_source.status.success(),
        "unreadable source should fail closed"
    );
    let unreadable_source_value = parse_json_output(
        &["requirement", "analyze", "--source-file", "--json"],
        &unreadable_source,
    );
    support::assert_release1_operator_shape("vida requirement analyze", &unreadable_source_value);
    assert_eq!(unreadable_source_value["status"].as_str(), Some("blocked"));
    assert_eq!(
        unreadable_source_value["blocker_codes"],
        json!(["requirement_source_unreadable"])
    );

    let default_output = vida()
        .args([
            "requirement",
            "analyze",
            "--request-id",
            "request-1",
            "--input",
            "operator request",
        ])
        .output()
        .expect("requirement analyze default should run");
    assert!(
        default_output.status.success(),
        "requirement analyze default should succeed: {}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert!(stdout.starts_with("vida requirement analyze\n"));
    assert!(stdout.contains("required_fields[22]{name,meaning}:"));
    assert!(stdout.contains("readiness_statuses[4]{status,meaning}:"));
    assert!(stdout.contains("ready,Downstream implementation can start from this artifact."));
    assert!(stdout.contains("output_modes[2]{mode,contract}:"));
    assert!(stdout.contains("allowed_next_node: developer"));
}
