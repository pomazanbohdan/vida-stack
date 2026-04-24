use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    fs::create_dir_all(&path).expect("temp dir should create");
    path
}

fn write_mock_vida(bin_dir: &Path) {
    let path = bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" });
    #[cfg(unix)]
    fs::write(
        &path,
        "#!/bin/sh\nprintf 'vida %s\\n' \"$*\"\nprintf 'VIDA_STATE_DIR=%s\\n' \"${VIDA_STATE_DIR:-}\"\n",
    )
    .expect("mock vida should write");
    #[cfg(windows)]
    fs::write(
        &path,
        "@echo off\r\necho vida %*\r\necho VIDA_STATE_DIR=%VIDA_STATE_DIR%\r\n",
    )
    .expect("mock vida should write");
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("permissions");
    }
}

struct EnvGuard {
    key: &'static str,
    value: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self {
            key,
            value: previous,
        }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self {
            key,
            value: previous,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.value {
            unsafe {
                std::env::set_var(self.key, value);
            }
        } else {
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }
}

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Self {
        let previous = std::env::current_dir().expect("current dir should resolve");
        std::env::set_current_dir(path).expect("current dir should change");
        Self { previous }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}

#[test]
fn help_command_delegates_to_vida_taskflow_help() {
    let bin_dir = temp_dir("taskflow-cli-bin");
    write_mock_vida(&bin_dir);
    let vida_guard = EnvGuard::set(
        "VIDA_TASKFLOW_VIDA_BIN",
        bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" }),
    );
    let unset_state_guard = EnvGuard::unset("VIDA_STATE_DIR");
    let project_root = temp_dir("taskflow-cli-project");
    fs::write(
        project_root.join("vida.config.yaml"),
        "project:\n  id: demo\n",
    )
    .expect("config");
    let dir_guard = CurrentDirGuard::change_to(&project_root);

    let output = Command::new(env!("CARGO_BIN_EXE_taskflow"))
        .args(["help", "parallelism"])
        .output()
        .expect("taskflow binary should run");

    drop(dir_guard);
    drop(unset_state_guard);
    drop(vida_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida taskflow help parallelism"));
    assert!(stdout.contains("VIDA_STATE_DIR="));
}

#[test]
fn delegated_commands_bind_project_local_state_dir_when_missing() {
    let bin_dir = temp_dir("taskflow-cli-bin");
    write_mock_vida(&bin_dir);
    let vida_guard = EnvGuard::set(
        "VIDA_TASKFLOW_VIDA_BIN",
        bin_dir.join(if cfg!(windows) { "vida.bat" } else { "vida" }),
    );
    let unset_state_guard = EnvGuard::unset("VIDA_STATE_DIR");
    let project_root = temp_dir("taskflow-cli-project");
    fs::write(
        project_root.join("vida.config.yaml"),
        "project:\n  id: demo\n",
    )
    .expect("config");
    let dir_guard = CurrentDirGuard::change_to(&project_root);

    let output = Command::new(env!("CARGO_BIN_EXE_taskflow"))
        .args(["validate-routing", "--json"])
        .output()
        .expect("taskflow binary should run");

    drop(dir_guard);
    drop(unset_state_guard);
    drop(vida_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida taskflow validate-routing --json"));
    assert!(stdout.contains(&format!(
        "VIDA_STATE_DIR={}",
        project_root.join(".vida/data/state").display()
    )));
}

#[test]
fn root_help_renders_without_delegation() {
    let output = Command::new(env!("CARGO_BIN_EXE_taskflow"))
        .arg("--help")
        .output()
        .expect("taskflow help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Standalone TaskFlow CLI wrapper"));
    assert!(stdout.contains("validate-routing --json"));
}
