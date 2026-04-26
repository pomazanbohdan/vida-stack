use std::process::{Command, Output};
use std::thread;
use std::time::Duration;

const VIDA_TIMEOUT_ARGS: [&str; 3] = ["-k", "5s", "120s"];

pub(crate) fn bounded_binary_command(binary_path: &str) -> Command {
    let mut command = Command::new("timeout");
    command.args(VIDA_TIMEOUT_ARGS);
    command.arg(binary_path);
    command
}

#[allow(dead_code)]
pub(crate) fn retry_with_backoff<F, P>(mut op: F, attempts: usize, mut should_retry: P) -> Output
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
