use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub mod domain_conformance;
pub mod engine_conformance;
pub mod failure_injection;
pub mod model;
pub mod shadow_diff;

#[cfg(unix)]
const DEFAULT_TIMEOUT_ARGS: [&str; 3] = ["-k", "5s", "120s"];
pub const STATE_LOCK_ERROR_MESSAGE: &str = "LOCK is already locked";
const FIXTURE_CREATED_AT: &str = "2026-03-08T00:00:00Z";

struct RecoveringMutex(Mutex<()>);

impl RecoveringMutex {
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn process_lock() -> &'static RecoveringMutex {
    static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
    LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
}

pub fn bounded_binary_command(binary_path: impl AsRef<OsStr>) -> Command {
    #[cfg(windows)]
    {
        Command::new(binary_path)
    }
    #[cfg(unix)]
    {
        let mut command = Command::new(timeout_program());
        command.args(DEFAULT_TIMEOUT_ARGS);
        command.arg(binary_path);
        command
    }
}

pub fn temp_fixture_dir() -> assert_fs::TempDir {
    assert_fs::TempDir::new().expect("assert_fs temp dir should create")
}

pub fn assert_text_snapshot(actual: impl AsRef<str>, expected: &str) {
    snapbox::Assert::new().eq(actual.as_ref(), expected);
}

#[derive(Debug, Clone, Copy)]
pub struct CliContractCase<'a> {
    pub surface: &'a str,
    pub args: &'a [&'a str],
}

pub fn release1_operator_shape_error(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    for key in [
        "surface",
        "status",
        "blocker_codes",
        "next_actions",
        "artifact_refs",
        "shared_fields",
        "operator_contracts",
    ] {
        if !object.contains_key(key) {
            return Some(format!("missing {key}"));
        }
    }
    if !value["surface"].is_string() {
        return Some("surface must be a string".to_string());
    }
    if !value["status"].is_string() {
        return Some("status must be a string".to_string());
    }
    if !value["blocker_codes"].is_array() {
        return Some("blocker_codes must be an array".to_string());
    }
    if !value["next_actions"].is_array() {
        return Some("next_actions must be an array".to_string());
    }
    if !value["artifact_refs"].is_object() {
        return Some("artifact_refs must be an object".to_string());
    }

    let shared_fields = &value["shared_fields"];
    let operator_contracts = &value["operator_contracts"];
    for mirrored in ["status", "blocker_codes", "next_actions", "artifact_refs"] {
        if value[mirrored] != shared_fields[mirrored] {
            return Some(format!("shared_fields.{mirrored} drifted"));
        }
        if value[mirrored] != operator_contracts[mirrored] {
            return Some(format!("operator_contracts.{mirrored} drifted"));
        }
    }
    if operator_contracts["contract_id"] != "release-1-operator-contracts" {
        return Some("operator_contracts.contract_id drifted".to_string());
    }
    if operator_contracts["schema_version"] != "release-1-v1" {
        return Some("operator_contracts.schema_version drifted".to_string());
    }
    None
}

pub fn assert_release1_operator_shape(surface: &str, value: &Value) {
    assert_eq!(value["surface"], surface);
    let error = release1_operator_shape_error(value);
    assert!(
        error.is_none(),
        "{surface} must keep release-1 JSON shape: {} got: {value:#}",
        error.unwrap_or_default()
    );
}

pub fn assert_cli_contract_matrix<'a>(
    cases: impl IntoIterator<Item = CliContractCase<'a>>,
    mut run_json: impl FnMut(&[&str]) -> Value,
) {
    for case in cases {
        let value = run_json(case.args);
        assert_release1_operator_shape(case.surface, &value);
    }
}

#[cfg(unix)]
pub fn bounded_command(
    program: impl AsRef<OsStr>,
    timeout_args: impl IntoIterator<Item = &'static str>,
) -> Command {
    let mut command = Command::new(timeout_program());
    command.args(timeout_args);
    command.arg(program);
    command
}

#[cfg(unix)]
fn timeout_program() -> OsString {
    ["/usr/bin/timeout", "/bin/timeout"]
        .iter()
        .map(OsString::from)
        .find(|path| Path::new(path).is_file())
        .unwrap_or_else(|| OsString::from("timeout"))
}

#[cfg(windows)]
pub fn bounded_command(
    program: impl AsRef<OsStr>,
    _timeout_args: impl IntoIterator<Item = &'static str>,
) -> Command {
    Command::new(program)
}

pub fn simulated_state_lock_output() -> Output {
    Output {
        status: failing_exit_status(),
        stdout: Vec::new(),
        stderr: format!("{STATE_LOCK_ERROR_MESSAGE}\n").into_bytes(),
    }
}

pub fn simulated_success_output(stdout: impl Into<Vec<u8>>) -> Output {
    Output {
        status: successful_exit_status(),
        stdout: stdout.into(),
        stderr: Vec::new(),
    }
}

#[cfg(unix)]
fn failing_exit_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn failing_exit_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    ExitStatus::from_raw(1)
}

#[cfg(unix)]
fn successful_exit_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    ExitStatus::from_raw(0)
}

#[cfg(windows)]
fn successful_exit_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    ExitStatus::from_raw(0)
}

pub fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    fs::create_dir_all(&path).expect("temp dir should create");
    path
}

#[derive(Debug, Clone)]
pub struct LargeBacklogFixture {
    pub root_id: String,
    pub direct_child_count: usize,
    pub total_task_count: usize,
    pub open_child_count: usize,
    pub closed_child_count: usize,
    pub in_progress_child_count: usize,
    pub blocked_open_child_count: usize,
    pub primary_ready_id: String,
    pub blocked_task_id: String,
}

pub fn write_large_backlog_jsonl(
    path: &Path,
    direct_child_count: usize,
) -> io::Result<LargeBacklogFixture> {
    assert!(
        direct_child_count >= 12,
        "large backlog fixture needs at least one full status bucket"
    );

    let root_id = "large-backlog-root".to_string();
    let mut lines = Vec::with_capacity(direct_child_count + 1);
    lines.push(task_json_line(
        &root_id,
        "Large backlog root",
        "root",
        "open",
        "epic",
        0,
        &[],
    ));

    let mut open_child_count = 0;
    let mut closed_child_count = 0;
    let mut in_progress_child_count = 0;
    let mut blocked_open_child_count = 0;
    let mut primary_ready_id = String::new();
    let mut blocked_task_id = String::new();

    for index in 0..direct_child_count {
        let task_id = format!("large-task-{index:04}");
        let status_bucket = index % 10;
        let status = match status_bucket {
            0 => {
                closed_child_count += 1;
                "closed"
            }
            1 => {
                in_progress_child_count += 1;
                "in_progress"
            }
            _ => {
                open_child_count += 1;
                "open"
            }
        };
        let is_blocked_open = status == "open" && status_bucket == 2;
        if is_blocked_open {
            blocked_open_child_count += 1;
            if blocked_task_id.is_empty() {
                blocked_task_id = task_id.clone();
            }
        } else if status == "open" && primary_ready_id.is_empty() {
            primary_ready_id = task_id.clone();
        }

        let priority = if primary_ready_id == task_id { 0 } else { 5 };
        let title = format!("Large backlog task {index:04}");
        let description = format!("fixture task {index:04}");
        let blocker_id = index
            .checked_sub(1)
            .map(|previous| format!("large-task-{previous:04}"));
        let mut dependencies = vec![DependencySpec {
            issue_id: &task_id,
            depends_on_id: &root_id,
            edge_type: "parent-child",
        }];
        if let (true, Some(blocker_id)) = (is_blocked_open, blocker_id.as_deref()) {
            dependencies.push(DependencySpec {
                issue_id: &task_id,
                depends_on_id: blocker_id,
                edge_type: "blocks",
            });
        }

        lines.push(task_json_line(
            &task_id,
            &title,
            &description,
            status,
            "task",
            priority,
            &dependencies,
        ));
    }

    fs::write(path, lines.join("\n") + "\n")?;

    Ok(LargeBacklogFixture {
        root_id,
        direct_child_count,
        total_task_count: direct_child_count + 1,
        open_child_count,
        closed_child_count,
        in_progress_child_count,
        blocked_open_child_count,
        primary_ready_id,
        blocked_task_id,
    })
}

struct DependencySpec<'a> {
    issue_id: &'a str,
    depends_on_id: &'a str,
    edge_type: &'a str,
}

fn task_json_line(
    id: &str,
    title: &str,
    description: &str,
    status: &str,
    issue_type: &str,
    priority: usize,
    dependencies: &[DependencySpec<'_>],
) -> String {
    let dependencies = dependencies
        .iter()
        .map(|dependency| {
            format!(
                "{{\"issue_id\":\"{}\",\"depends_on_id\":\"{}\",\"type\":\"{}\",\"created_at\":\"{}\",\"created_by\":\"tester\",\"metadata\":\"{{}}\",\"thread_id\":\"\"}}",
                dependency.issue_id,
                dependency.depends_on_id,
                dependency.edge_type,
                FIXTURE_CREATED_AT
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"id\":\"{id}\",\"title\":\"{title}\",\"description\":\"{description}\",\"status\":\"{status}\",\"priority\":{priority},\"issue_type\":\"{issue_type}\",\"created_at\":\"{FIXTURE_CREATED_AT}\",\"created_by\":\"tester\",\"updated_at\":\"{FIXTURE_CREATED_AT}\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{dependencies}]}}"
    )
}

pub struct CommandContext {
    cwd: PathBuf,
    env: Vec<(String, String)>,
}

impl CommandContext {
    pub fn empty() -> Self {
        Self::capture(std::iter::empty::<(String, String)>())
    }

    pub fn capture(env: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            cwd: std::env::current_dir().expect("current dir should resolve"),
            env: env
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    pub fn diagnostics(&self, output: &Output) -> String {
        let env = if self.env.is_empty() {
            "<none>".to_string()
        } else {
            self.env
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "status={:?}\ncwd={}\nenv={}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            self.cwd.display(),
            env,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

pub struct ProcessGuard {
    _lock: MutexGuard<'static, ()>,
    original_dir: Option<PathBuf>,
    env: Vec<(String, Option<OsString>)>,
}

impl ProcessGuard {
    pub fn new() -> Self {
        Self {
            _lock: process_lock().lock(),
            original_dir: None,
            env: Vec::new(),
        }
    }

    pub fn set_env(&mut self, key: &'static str, value: impl AsRef<OsStr>) {
        self.capture_env(key);
        unsafe {
            std::env::set_var(key, value);
        }
    }

    pub fn unset_env(&mut self, key: &'static str) {
        self.capture_env(key);
        unsafe {
            std::env::remove_var(key);
        }
    }

    pub fn change_current_dir(&mut self, path: &Path) {
        if self.original_dir.is_none() {
            self.original_dir = Some(std::env::current_dir().expect("current dir should resolve"));
        }
        std::env::set_current_dir(path).expect("current dir should change");
    }

    fn capture_env(&mut self, key: &'static str) {
        if self.env.iter().any(|(existing, _)| existing == key) {
            return;
        }
        self.env.push((key.to_string(), std::env::var_os(key)));
    }
}

impl Default for ProcessGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(original_dir) = &self.original_dir {
            let _ = std::env::set_current_dir(original_dir);
        }
        for (key, value) in self.env.iter().rev() {
            if let Some(value) = value {
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

pub fn write_executable_script(
    path: &Path,
    #[cfg_attr(not(unix), allow(unused_variables))] unix_body: &str,
    #[cfg_attr(unix, allow(unused_variables))] windows_body: &str,
) {
    #[cfg(unix)]
    {
        fs::write(path, unix_body).expect("executable script should write");
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("permissions");
    }
    #[cfg(windows)]
    fs::write(path, windows_body).expect("executable script should write");
}

pub fn retry_with_backoff<F, P>(mut op: F, attempts: usize, mut should_retry: P) -> Output
where
    F: FnMut() -> Output,
    P: FnMut(&Output) -> bool,
{
    let mut last = None;
    let mut delay_ms = 1;
    for _ in 0..attempts {
        let output = op();
        if !should_retry(&output) {
            return output;
        }
        last = Some(output);
        thread::sleep(Duration::from_millis(delay_ms));
        delay_ms = (delay_ms * 2).min(100);
    }
    last.expect("retry helper should capture at least one output")
}

pub fn command_output_with_retry<F, P>(mut build: F, attempts: usize, should_retry: P) -> Output
where
    F: FnMut() -> Command,
    P: FnMut(&Output) -> bool,
{
    retry_with_backoff(
        || build().output().expect("bounded command should run"),
        attempts,
        should_retry,
    )
}

pub fn command_output_with_retry_errors<F, P, E>(
    mut build: F,
    attempts: usize,
    mut should_retry_output: P,
    mut should_retry_error: E,
) -> Output
where
    F: FnMut() -> Command,
    P: FnMut(&Output) -> bool,
    E: FnMut(&io::Error) -> bool,
{
    let mut last = None;
    let mut delay_ms = 1;
    for _ in 0..attempts {
        match build().output() {
            Ok(output) if !should_retry_output(&output) => return output,
            Ok(output) => {
                last = Some(output);
            }
            Err(error) if should_retry_error(&error) => {}
            Err(error) => panic!("bounded command should run: {error}"),
        }
        thread::sleep(Duration::from_millis(delay_ms));
        delay_ms = (delay_ms * 2).min(100);
    }
    last.expect("retry helper should capture at least one output")
}
