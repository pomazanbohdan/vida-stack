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
