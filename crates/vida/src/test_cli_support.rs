use crate::Cli;
use clap::Parser;
use std::cell::Cell;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

struct RecoveringMutex(Mutex<()>);

impl RecoveringMutex {
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn process_global_runtime_lock() -> &'static RecoveringMutex {
    static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
    LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
}

thread_local! {
    static PROCESS_GLOBAL_RUNTIME_GUARD_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn enter_process_global_runtime() -> Option<MutexGuard<'static, ()>> {
    PROCESS_GLOBAL_RUNTIME_GUARD_DEPTH.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        (current == 0).then(|| process_global_runtime_lock().lock())
    })
}

fn exit_process_global_runtime(lock: Option<MutexGuard<'static, ()>>) {
    drop(lock);
    PROCESS_GLOBAL_RUNTIME_GUARD_DEPTH.with(|depth| {
        depth.set(depth.get().saturating_sub(1));
    });
}

pub(crate) struct CurrentDirGuard {
    lock: Option<MutexGuard<'static, ()>>,
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Self {
        let lock = enter_process_global_runtime();
        let original = env::current_dir().expect("current dir should resolve");
        env::set_current_dir(path).expect("current dir should change");
        Self { lock, original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.original).expect("current dir should restore");
        exit_process_global_runtime(self.lock.take());
    }
}

pub(crate) fn guard_current_dir(path: &Path) -> CurrentDirGuard {
    CurrentDirGuard::change_to(path)
}

pub(crate) struct EnvVarGuard {
    lock: Option<MutexGuard<'static, ()>>,
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    pub(crate) fn set(key: &'static str, value: &str) -> Self {
        let lock = enter_process_global_runtime();
        let original = env::var_os(key);
        env::set_var(key, value);
        Self {
            lock,
            key,
            original,
        }
    }

    pub(crate) fn unset(key: &'static str) -> Self {
        let lock = enter_process_global_runtime();
        let original = env::var_os(key);
        env::remove_var(key);
        Self {
            lock,
            key,
            original,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => env::set_var(self.key, value),
            None => env::remove_var(self.key),
        }
        exit_process_global_runtime(self.lock.take());
    }
}

pub(crate) fn cli(args: &[&str]) -> Cli {
    let mut argv = vec!["vida"];
    argv.extend(args.iter().copied());
    Cli::parse_from(argv)
}

#[cfg(test)]
pub(crate) fn canonical_team_flow_test_project_root(root: &Path) {
    std::fs::create_dir_all(root.join(".vida/config")).expect("create project config dir");
    std::fs::create_dir_all(root.join(".vida/db")).expect("create project db dir");
    std::fs::create_dir_all(root.join(".vida/project")).expect("create project metadata dir");
    std::fs::write(root.join("AGENTS.md"), "# canonical TeamFlow fixture\n")
        .expect("write agents marker");
    std::fs::write(
        root.join("vida.config.yaml"),
        include_str!("../../../vida.config.yaml"),
    )
    .expect("write canonical project config");
}

#[cfg(test)]
mod tests {
    use super::{EnvVarGuard, cli, guard_current_dir};
    use std::path::PathBuf;

    #[test]
    fn cli_prepends_binary_name_and_parses_empty_or_status_commands() {
        assert!(cli(&[]).command.is_none());
        assert!(matches!(
            cli(&["status"]).command,
            Some(crate::Command::Status(_))
        ));
    }

    #[test]
    fn env_var_guard_restores_original_value_after_override() {
        const KEY: &str = "VIDA_TEST_CLI_SUPPORT_RESTORE";
        let _ = std::env::var_os(KEY).map(|_| std::env::remove_var(KEY));

        {
            let _guard = EnvVarGuard::set(KEY, "inside");
            assert_eq!(std::env::var(KEY).as_deref(), Ok("inside"));
        }

        assert!(std::env::var_os(KEY).is_none());
    }

    #[test]
    fn current_dir_guard_restores_directory_after_scoped_change() {
        let original = std::env::current_dir().expect("current dir should resolve");
        let target = std::env::temp_dir().join(format!(
            "vida-test-cli-support-current-dir-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&target).expect("temporary directory should exist");

        {
            let _guard = guard_current_dir(&target);
            assert_eq!(std::env::current_dir().unwrap(), target);
        }

        assert_eq!(std::env::current_dir().unwrap(), original);
        let _ = std::fs::remove_dir_all(PathBuf::from(target));
    }
}
