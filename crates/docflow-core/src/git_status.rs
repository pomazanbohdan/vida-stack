use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::DocflowCoreError;

pub const DEFAULT_GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeGitStatusInput {
    pub root: PathBuf,
    pub pathspecs: Vec<String>,
    pub timeout: Duration,
}

impl SafeGitStatusInput {
    #[must_use]
    pub fn markdown(root: PathBuf) -> Self {
        Self {
            root,
            pathspecs: vec![":(glob)**/*.md".to_string()],
            timeout: DEFAULT_GIT_STATUS_TIMEOUT,
        }
    }
}

pub fn changed_markdown_paths(input: SafeGitStatusInput) -> Result<Vec<String>, DocflowCoreError> {
    let output = run_git_status_with_timeout(input)?;
    if !output.status.success() {
        return Err(DocflowCoreError::GitStatusFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(markdown_paths_from_status_stdout(&output.stdout))
}

fn markdown_paths_from_status_stdout(stdout: &[u8]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    let mut records = stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let status = &record[..2];
        if status == b" D" || status == b"D " || status == b"DD" {
            continue;
        }
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        if matches!(status[0], b'R' | b'C') || matches!(status[1], b'R' | b'C') {
            let _ = records.next();
        }
        if path.ends_with(".md") && !path.is_empty() {
            paths.insert(path.replace('\\', "/"));
        }
    }
    paths.into_iter().collect()
}

fn run_git_status_with_timeout(input: SafeGitStatusInput) -> Result<Output, DocflowCoreError> {
    let pathspecs = if input.pathspecs.is_empty() {
        vec![":(glob)**/*.md".to_string()]
    } else {
        input.pathspecs
    };
    let timeout = if input.timeout.is_zero() {
        DEFAULT_GIT_STATUS_TIMEOUT
    } else {
        input.timeout
    };

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&input.root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", git_null_config_path())
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_KEY_0")
        .env_remove("GIT_CONFIG_VALUE_0")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=",
            "-c",
            "core.untrackedCache=false",
        ])
        .args(["status", "--porcelain=v1", "-z", "--"])
        .args(pathspecs);
    bounded_command_output(command, timeout)
}

fn bounded_command_output(
    mut command: Command,
    timeout: Duration,
) -> Result<Output, DocflowCoreError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| DocflowCoreError::GitStatusIo(error.to_string()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| DocflowCoreError::GitStatusIo("child stdout was not piped".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| DocflowCoreError::GitStatusIo("child stderr was not piped".to_string()))?;
    let stdout_reader = thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        output
    });
    let stderr_reader = thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stderr.read_to_end(&mut output);
        output
    });

    match child
        .wait_timeout(timeout)
        .map_err(|error| DocflowCoreError::GitStatusIo(error.to_string()))?
    {
        Some(status) => {
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default();
            Ok(Output {
                status,
                stdout,
                stderr,
            })
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            Err(DocflowCoreError::GitStatusTimedOut)
        }
    }
}

#[cfg(windows)]
fn git_null_config_path() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn git_null_config_path() -> &'static str {
    "/dev/null"
}

#[cfg(test)]
mod tests {
    use super::{
        SafeGitStatusInput, bounded_command_output, changed_markdown_paths,
        markdown_paths_from_status_stdout,
    };
    use crate::DocflowCoreError;
    use std::fs;
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn git_status_parser_returns_changed_markdown_only() {
        let stdout =
            b" M docs/a.md\0 D docs/removed.md\0R  docs/new.md\0docs/old.md\0?? notes.txt\0?? docs/new.md\0";
        assert_eq!(
            markdown_paths_from_status_stdout(stdout),
            vec!["docs/a.md".to_string(), "docs/new.md".to_string()]
        );
    }

    #[test]
    fn git_status_parser_preserves_raw_markdown_path_spacing() {
        let stdout = b"??  lead.md\0";
        assert_eq!(
            markdown_paths_from_status_stdout(stdout),
            vec![" lead.md".to_string()]
        );
    }

    #[test]
    fn bounded_command_output_times_out_and_kills_child() {
        let error = bounded_command_output(sleep_command(), Duration::from_millis(25))
            .expect_err("slow command should time out");

        assert!(matches!(error, DocflowCoreError::GitStatusTimedOut));
    }

    #[test]
    fn changed_markdown_paths_disables_repo_local_fsmonitor_helper() {
        let root = unique_temp_root("docflow-core-git-status");
        fs::create_dir_all(root.join("docs")).expect("docs dir should be created");
        fs::write(root.join("docs/changed.md"), "# Changed\n").expect("doc should be written");
        let git_init = Command::new("git")
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
        let helper = root.join(fsmonitor_helper_name());
        write_fsmonitor_helper(&helper, &sentinel);
        let helper_path = helper.to_string_lossy().replace('\\', "/");
        let git_config = Command::new("git")
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

        let git_status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["status", "--short", "--", ":(glob)**/*.md"])
            .output()
            .expect("plain git status should run");
        assert!(
            git_status.status.success(),
            "{}",
            String::from_utf8_lossy(&git_status.stderr)
        );
        assert!(sentinel.exists(), "plain git status should use fsmonitor");
        fs::remove_file(&sentinel).expect("sentinel should be removed");

        let paths = changed_markdown_paths(SafeGitStatusInput {
            root: root.clone(),
            pathspecs: vec![":(glob)**/*.md".to_string()],
            timeout: Duration::from_secs(10),
        })
        .expect("safe git status should run");
        assert_eq!(paths, vec!["docs/changed.md".to_string()]);
        assert!(
            !sentinel.exists(),
            "safe git status should not use repo-local fsmonitor"
        );

        fs::remove_dir_all(root).expect("temp root should be removed");
    }

    fn unique_temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
    }

    #[cfg(windows)]
    fn sleep_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping -n 6 127.0.0.1 >NUL"]);
        command
    }

    #[cfg(not(windows))]
    fn sleep_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        command
    }

    #[cfg(windows)]
    fn fsmonitor_helper_name() -> &'static str {
        "fsmonitor-helper.cmd"
    }

    #[cfg(not(windows))]
    fn fsmonitor_helper_name() -> &'static str {
        "fsmonitor-helper.sh"
    }

    #[cfg(windows)]
    fn write_fsmonitor_helper(helper: &std::path::Path, sentinel: &std::path::Path) {
        fs::write(
            helper,
            format!(
                "@echo off\r\necho invoked>\"{}\"\r\nexit /b 0\r\n",
                sentinel.display()
            ),
        )
        .expect("fsmonitor helper should be written");
    }

    #[cfg(not(windows))]
    fn write_fsmonitor_helper(helper: &std::path::Path, sentinel: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(
            helper,
            format!(
                "#!/bin/sh\necho invoked > '{}'\nexit 0\n",
                sentinel.display()
            ),
        )
        .expect("fsmonitor helper should be written");
        let mut permissions = fs::metadata(helper)
            .expect("helper metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(helper, permissions).expect("helper should be executable");
    }
}
