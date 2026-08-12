#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use taskflow_core::run_workflow::RunWorkflowCommand;
use taskflow_core::{canonical_task_status, path_policy::normalize_repo_relative_path};

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = normalize_repo_relative_path(&input);
    let _ = canonical_task_status(&input);
    if let Ok(value) = serde_json::from_str::<Value>(&input) {
        let _ = serde_json::from_value::<RunWorkflowCommand>(value);
    }
});
