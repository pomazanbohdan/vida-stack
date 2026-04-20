use crate::Cli;
use clap::Parser;
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

fn current_dir_lock() -> &'static RecoveringMutex {
    static LOCK: OnceLock<RecoveringMutex> = OnceLock::new();
    LOCK.get_or_init(|| RecoveringMutex(Mutex::new(())))
}

pub(crate) struct CurrentDirGuard {
    _lock: MutexGuard<'static, ()>,
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Self {
        let lock = current_dir_lock().lock();
        let original = env::current_dir().expect("current dir should resolve");
        env::set_current_dir(path).expect("current dir should change");
        Self {
            _lock: lock,
            original,
        }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.original).expect("current dir should restore");
    }
}

pub(crate) fn guard_current_dir(path: &Path) -> CurrentDirGuard {
    CurrentDirGuard::change_to(path)
}

pub(crate) struct EnvVarGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    pub(crate) fn unset(key: &'static str) -> Self {
        let original = env::var_os(key);
        env::remove_var(key);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => env::set_var(self.key, value),
            None => env::remove_var(self.key),
        }
    }
}

pub(crate) fn cli(args: &[&str]) -> Cli {
    let mut argv = vec!["vida"];
    argv.extend(args.iter().copied());
    Cli::parse_from(argv)
}
