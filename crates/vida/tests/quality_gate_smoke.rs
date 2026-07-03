use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    format!(
        "{}/vida-quality-gate-smoke-{nanos}-{count}",
        std::env::temp_dir().display()
    )
}

fn run_and_assert_failure(args: &[&str], state_dir: &str) -> (String, String) {
    let output = run_command_capture(args, state_dir);
    assert!(
        !output.status.success(),
        "command should fail: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn run_and_assert_success(args: &[&str], state_dir: &str) -> String {
    let output = run_command_capture(args, state_dir);
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_and_assert_success_without_state_dir(args: &[&str]) -> String {
    let output = vida().args(args).output().expect("vida command should run");
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_command_capture(args: &[&str], state_dir: &str) -> std::process::Output {
    vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("vida command should run")
}

fn run_command_json_allow_failure(args: &[&str], state_dir: &str) -> (Value, bool) {
    let output = run_command_capture(args, state_dir);
    let stdout = String::from_utf8_lossy(&output.stdout);
    (
        serde_json::from_str(&stdout).expect("stdout should be json"),
        output.status.success(),
    )
}

fn init_git_repo(path: &str) {
    fs::create_dir_all(path).expect("create git repo dir");
    let init = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(path)
        .output()
        .expect("git init should run");
    assert!(
        init.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
}

#[test]
fn quality_gate_prepush_help_documents_advisor_output_modes() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let help = run_and_assert_success(&["quality", "gate", "--help"], &state_dir);

    assert!(help.contains("--prepush"));
    assert!(help.contains("--advise"));
    assert!(help.contains("--coverage-file"));
    assert!(help.contains("--crap-file"));
    assert!(help.contains("--crap-baseline-file"));
    assert!(help.contains("--task-exception-note"));
    assert!(help.contains("--coverage-threshold"));
    assert!(help.contains("Default output is compact TOON/plain"));
}

#[test]
fn quality_gate_prepush_default_output_is_compact_toon() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let project_root = unique_state_dir();
    init_git_repo(&project_root);

    let output = run_and_assert_success(
        &[
            "quality",
            "gate",
            "--prepush",
            "--advise",
            "--project-root",
            &project_root,
        ],
        &state_dir,
    );

    assert!(output.starts_with("vida quality gate\n"));
    assert!(output.contains("status: pass"));
    assert!(output.contains("blocker_codes[0]:"));
    assert!(!output.trim_start().starts_with('{'));
    assert!(!output.contains("--json"));

    let _ = fs::remove_dir_all(&state_dir);
    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn quality_gate_prepush_runs_in_plain_git_repo_without_vida_state() {
    let project_root = unique_state_dir();
    init_git_repo(&project_root);

    let output = run_and_assert_success_without_state_dir(&[
        "quality",
        "gate",
        "--prepush",
        "--project-root",
        &project_root,
    ]);

    assert!(output.starts_with("vida quality gate\n"));
    assert!(output.contains("status: pass"));
    assert!(!output.contains("pending_activation"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn quality_gate_prepush_json_advise_reports_codegen_and_coverage_remediation() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let project_root = unique_state_dir();
    init_git_repo(&project_root);
    let generated_path = format!("{project_root}/src/generated/client.rs");
    fs::create_dir_all(format!("{project_root}/src/generated")).expect("create generated dir");
    fs::write(&generated_path, "pub fn generated() {}\n").expect("write generated file");
    let coverage_path = format!("{project_root}/coverage/lcov.info");
    fs::create_dir_all(format!("{project_root}/coverage")).expect("create coverage dir");
    fs::write(
        &coverage_path,
        "TN:\nSF:src/generated/client.rs\nLF:10\nLH:8\nend_of_record\n",
    )
    .expect("write lcov");

    let (payload, success) = run_command_json_allow_failure(
        &[
            "quality",
            "gate",
            "--prepush",
            "--json",
            "--advise",
            "--project-root",
            &project_root,
            "--coverage-file",
            &coverage_path,
            "--coverage-threshold",
            "90",
        ],
        &state_dir,
    );

    assert!(!success, "quality failures should fail closed");
    assert_eq!(payload["surface"], "vida quality gate");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["codegen_dirty_files", "coverage_below_threshold"])
    );
    assert_eq!(
        payload["codegen_dirty_files"],
        serde_json::json!(["src/generated/client.rs"])
    );
    assert_eq!(payload["coverage_percent"], 80.0);
    assert_eq!(payload["coverage_threshold"], 90.0);
    assert_eq!(payload["additional_covered_lines_needed"], 1);
    assert_eq!(
        payload["top_uncovered_changed_files"][0]["path"],
        "src/generated/client.rs"
    );
    assert!(payload["suggested_action"]
        .as_str()
        .is_some_and(|action| action.contains("git add")));

    let _ = fs::remove_dir_all(&state_dir);
    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn quality_gate_prepush_json_reports_crap_hotspot_blockers() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let project_root = unique_state_dir();
    init_git_repo(&project_root);
    let source_dir = format!("{project_root}/crates/vida/src");
    fs::create_dir_all(&source_dir).expect("create source dir");
    let hot_path = format!("{source_dir}/hot.rs");
    fs::write(&hot_path, "pub fn hot() {}\n").expect("write source");
    let crap_path = format!("{project_root}/workspace-crap.json");
    fs::write(
        &crap_path,
        r#"{"entries":[{"file":"crates/vida/src/hot.rs","function":"hot","line":7,"crate":"vida","crap":1200.0,"cyclomatic":80.0,"coverage":12.5}]}"#,
    )
    .expect("write crap json");

    let (payload, success) = run_command_json_allow_failure(
        &[
            "quality",
            "gate",
            "--prepush",
            "--json",
            "--project-root",
            &project_root,
            "--crap-file",
            &crap_path,
        ],
        &state_dir,
    );

    assert!(!success, "touched high-CRAP functions should fail closed");
    assert_eq!(payload["status"], "blocked");
    assert!(payload["blocker_codes"]
        .as_array()
        .is_some_and(|codes| codes.contains(&serde_json::json!(
            "touched_crap_hotspots_without_exception"
        ))));
    assert_eq!(payload["crap"]["count_gt_1000"], 1);
    assert_eq!(payload["crap"]["touched_hotspots"][0]["function"], "hot");
    assert_eq!(payload["artifact_refs"]["crap_file"], crap_path);

    let _ = fs::remove_dir_all(&state_dir);
    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn quality_gate_prepush_json_reports_crap_baseline_growth_blocker() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let project_root = unique_state_dir();
    init_git_repo(&project_root);
    let source_dir = format!("{project_root}/crates/vida/src");
    fs::create_dir_all(&source_dir).expect("create source dir");
    let hot_path = format!("{source_dir}/hot.rs");
    fs::write(&hot_path, "pub fn hot() {}\n").expect("write source");
    let crap_path = format!("{project_root}/workspace-crap.json");
    let baseline_path = format!("{project_root}/workspace-crap-baseline.json");
    fs::write(
        &crap_path,
        r#"{"entries":[{"file":"crates/vida/src/hot.rs","function":"hot","line":7,"crate":"vida","crap":1200.0,"cyclomatic":80.0,"coverage":12.5}]}"#,
    )
    .expect("write crap json");
    fs::write(
        &baseline_path,
        r#"{"entries":[{"file":"crates/vida/src/hot.rs","function":"hot","line":7,"crate":"vida","crap":950.0,"cyclomatic":80.0,"coverage":12.5}]}"#,
    )
    .expect("write baseline json");

    let (payload, success) = run_command_json_allow_failure(
        &[
            "quality",
            "gate",
            "--prepush",
            "--json",
            "--project-root",
            &project_root,
            "--crap-file",
            &crap_path,
            "--crap-baseline-file",
            &baseline_path,
            "--task-exception-note",
            "reviewed-exception-for-touched-hotspot",
        ],
        &state_dir,
    );

    assert!(!success, "worsened CRAP>1000 should fail closed");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["crap_gt_1000_growth"])
    );
    assert_eq!(payload["crap"]["worsened_hotspots"][0]["function"], "hot");
    assert_eq!(
        payload["crap"]["worsened_hotspots"][0]["previous_crap"],
        950.0
    );
    assert_eq!(
        payload["artifact_refs"]["crap_baseline_file"],
        baseline_path
    );

    let _ = fs::remove_dir_all(&state_dir);
    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn quality_gate_prepush_default_output_names_crap_hotspot_and_remediation() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let project_root = unique_state_dir();
    init_git_repo(&project_root);
    let source_dir = format!("{project_root}/crates/vida/src");
    fs::create_dir_all(&source_dir).expect("create source dir");
    let hot_path = format!("{source_dir}/hot.rs");
    fs::write(&hot_path, "pub fn hot() {}\n").expect("write source");
    let crap_path = format!("{project_root}/workspace-crap.json");
    fs::write(
        &crap_path,
        r#"{"entries":[{"file":"crates/vida/src/hot.rs","function":"hot","line":7,"crate":"vida","crap":1200.0,"cyclomatic":80.0,"coverage":12.5}]}"#,
    )
    .expect("write crap json");

    let (stdout, stderr) = run_and_assert_failure(
        &[
            "quality",
            "gate",
            "--prepush",
            "--project-root",
            &project_root,
            "--crap-file",
            &crap_path,
        ],
        &state_dir,
    );

    assert!(
        stderr.is_empty(),
        "quality gate should report on stdout: {stderr}"
    );
    assert!(stdout.starts_with("vida quality gate\n"));
    assert!(stdout.contains("touched_crap_hotspots_without_exception"));
    assert!(stdout.contains("touched_hotspots[1]{file,function,line,crate,crap"));
    assert!(stdout.contains("crates/vida/src/hot.rs,hot,7,vida,1200"));
    assert!(stdout.contains("--task-exception-note"));

    let _ = fs::remove_dir_all(&state_dir);
    let _ = fs::remove_dir_all(&project_root);
}
