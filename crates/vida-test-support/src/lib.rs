use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_TIMEOUT_ARGS: [&str; 3] = ["-k", "5s", "120s"];

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
    bounded_command(binary_path, DEFAULT_TIMEOUT_ARGS)
}

pub fn bounded_command(
    program: impl AsRef<OsStr>,
    timeout_args: impl IntoIterator<Item = &'static str>,
) -> Command {
    let mut command = Command::new("timeout");
    command.args(timeout_args);
    command.arg(program);
    command
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
