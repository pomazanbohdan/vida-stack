use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use crate::taskflow_runtime::{
    dispatch_runtime_disabled_payload, task_runtime_mode_for_state_root,
    taskflow_dispatch_enabled_for_state_root, TaskExecutionBinding, TaskLifecycleMutationSource,
    TaskLifecycleService, TaskRuntimeMode,
};
use crate::taskflow_task_bridge::proxy_state_dir;

const ADOPTION_FILE: &str = "taskflow-dispatch-adoptions.jsonl";

pub(crate) async fn run_taskflow_dispatch(args: &[String]) -> ExitCode {
    let state_dir = proxy_state_dir();
    let mode = task_runtime_mode_for_state_root(&state_dir);
    match args.get(1).map(String::as_str) {
        None | Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("status") => run_status(&state_dir, mode, args).await,
        Some("adopt") => run_adopt(&state_dir, mode, args).await,
        Some(_) => {
            eprintln!("Usage: vida taskflow dispatch status [--json]\n       vida taskflow dispatch adopt [--dry-run|--apply] [--run-id <id>] [--task-id <id>] [--json]");
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "VIDA TaskFlow dispatch runtime\n\nUsage:\n  vida taskflow dispatch status [--json]\n  vida taskflow dispatch adopt --dry-run [--run-id <id>] [--task-id <id>] [--json]\n  vida taskflow dispatch adopt --apply --run-id <id> --task-id <id> [--json]\n\nThe dispatch runtime is opt-in through taskflow.dispatch.enabled."
    );
}

async fn run_status(state_dir: &Path, mode: TaskRuntimeMode, args: &[String]) -> ExitCode {
    let as_json = args.iter().any(|arg| arg == "--json");
    let enabled = taskflow_dispatch_enabled_for_state_root(state_dir);
    let adoption_path = state_dir.join(ADOPTION_FILE);
    let adopted_count = fs::read_to_string(&adoption_path)
        .ok()
        .map(|body| body.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0);
    let execution_bound_count =
        match crate::StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
            Ok(store) => store
                .task_store_summary()
                .await
                .map(|summary| summary.execution_bound_count)
                .unwrap_or(0),
            Err(_) => 0,
        };
    let unadopted_count = execution_bound_count.saturating_sub(adopted_count);
    if !enabled {
        let mut payload = dispatch_runtime_disabled_payload(
            "vida taskflow dispatch status",
            TaskRuntimeMode::ManagementOnly,
        );
        if let Some(object) = payload.as_object_mut() {
            object.insert("enabled".to_string(), serde_json::Value::Bool(false));
            object.insert(
                "management_runtime".to_string(),
                serde_json::Value::String("always_on".to_string()),
            );
            object.insert(
                "adopted_run_count".to_string(),
                serde_json::Value::from(adopted_count),
            );
            object.insert(
                "execution_bound_count".to_string(),
                serde_json::Value::from(execution_bound_count),
            );
            object.insert(
                "unadopted_run_count".to_string(),
                serde_json::Value::from(0),
            );
            object.insert(
                "artifact_refs".to_string(),
                serde_json::json!({"adoption_path": adoption_path}),
            );
        }
        emit_payload(&payload, as_json);
        return ExitCode::from(1);
    }
    let payload = serde_json::json!({
        "surface": "vida taskflow dispatch status",
        "status": "pass",
        "runtime": "task_dispatch",
        "mode": mode,
        "enabled": enabled,
        "management_runtime": "always_on",
        "execution_bound_count": execution_bound_count,
        "adopted_run_count": adopted_count,
        "unadopted_run_count": unadopted_count,
        "blocker_codes": [],
        "next_actions": if enabled { vec!["Dispatch runtime is enabled; use `vida taskflow dispatch adopt --dry-run` before adding existing runs."] } else { vec!["Management runtime remains available; set taskflow.dispatch.enabled: true to enable worker dispatch."] },
        "artifact_refs": {"adoption_path": adoption_path},
    });
    emit_payload(&payload, as_json);
    ExitCode::SUCCESS
}

async fn run_adopt(state_dir: &Path, mode: TaskRuntimeMode, args: &[String]) -> ExitCode {
    let as_json = args.iter().any(|arg| arg == "--json");
    let apply = args.iter().any(|arg| arg == "--apply");
    let dry_run = args.iter().any(|arg| arg == "--dry-run") || !apply;
    let run_id = option_value(args, "--run-id");
    let task_id = option_value(args, "--task-id");
    if !taskflow_dispatch_enabled_for_state_root(state_dir) {
        let payload = dispatch_runtime_disabled_payload(
            "vida taskflow dispatch adopt",
            TaskRuntimeMode::ManagementOnly,
        );
        emit_payload(&payload, as_json);
        return ExitCode::from(1);
    }
    if apply && (run_id.is_none() || task_id.is_none()) {
        let payload = serde_json::json!({
            "surface": "vida taskflow dispatch adopt",
            "status": "blocked",
            "runtime": "task_dispatch",
            "mode": mode,
            "blocker_codes": ["dispatch_adoption_requires_explicit_run_and_task"],
            "next_actions": ["Provide --run-id and --task-id, then rerun with --apply."],
        });
        emit_payload(&payload, as_json);
        return ExitCode::from(1);
    }

    let validated_receipt_id = if let (Some(run_id), Some(task_id)) =
        (run_id.as_deref(), task_id.as_deref())
    {
        let store = match crate::StateStore::open_existing_read_only(state_dir.to_path_buf()).await
        {
            Ok(store) => store,
            Err(error) => {
                let payload = serde_json::json!({
                    "surface": "vida taskflow dispatch adopt",
                    "status": "blocked",
                    "runtime": "task_dispatch",
                    "mode": mode,
                    "blocker_codes": ["dispatch_adoption_state_store_unavailable"],
                    "next_actions": [error.to_string()],
                    "run_id": run_id,
                    "task_id": task_id,
                });
                emit_payload(&payload, as_json);
                return ExitCode::from(1);
            }
        };
        let status = match store.run_graph_status(run_id).await {
            Ok(status) => status,
            Err(error) => {
                let payload = serde_json::json!({
                    "surface": "vida taskflow dispatch adopt",
                    "status": "blocked",
                    "runtime": "task_dispatch",
                    "mode": mode,
                    "blocker_codes": ["dispatch_adoption_run_unavailable"],
                    "next_actions": [error.to_string()],
                    "run_id": run_id,
                    "task_id": task_id,
                });
                emit_payload(&payload, as_json);
                return ExitCode::from(1);
            }
        };
        if status.task_id != task_id {
            let payload = serde_json::json!({
                "surface": "vida taskflow dispatch adopt",
                "status": "blocked",
                "runtime": "task_dispatch",
                "mode": mode,
                "blocker_codes": ["dispatch_adoption_binding_mismatch"],
                "next_actions": ["Use the task id bound to the run, then rerun adoption."],
                "run_id": run_id,
                "task_id": task_id,
                "bound_task_id": status.task_id,
            });
            emit_payload(&payload, as_json);
            return ExitCode::from(1);
        }
        if let Err(error) = store.show_task(task_id).await {
            let payload = serde_json::json!({
                "surface": "vida taskflow dispatch adopt",
                "status": "blocked",
                "runtime": "task_dispatch",
                "mode": mode,
                "blocker_codes": ["dispatch_adoption_task_unavailable"],
                "next_actions": [error.to_string()],
                "run_id": run_id,
                "task_id": task_id,
            });
            emit_payload(&payload, as_json);
            return ExitCode::from(1);
        }
        let receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                let payload = serde_json::json!({
                    "surface": "vida taskflow dispatch adopt",
                    "status": "blocked",
                    "runtime": "task_dispatch",
                    "mode": mode,
                    "blocker_codes": ["dispatch_adoption_receipt_required"],
                    "next_actions": ["Record a validated dispatch receipt for the run before adoption."],
                    "run_id": run_id,
                    "task_id": task_id,
                });
                emit_payload(&payload, as_json);
                return ExitCode::from(1);
            }
            Err(error) => {
                let payload = serde_json::json!({
                    "surface": "vida taskflow dispatch adopt",
                    "status": "blocked",
                    "runtime": "task_dispatch",
                    "mode": mode,
                    "blocker_codes": ["dispatch_adoption_receipt_unavailable"],
                    "next_actions": [error.to_string()],
                    "run_id": run_id,
                    "task_id": task_id,
                });
                emit_payload(&payload, as_json);
                return ExitCode::from(1);
            }
        };
        let receipt_id = receipt
            .exception_path_receipt_id
            .or(receipt.supersedes_receipt_id)
            .unwrap_or(receipt.run_id);
        if receipt_id.trim().is_empty() {
            let payload = serde_json::json!({
                "surface": "vida taskflow dispatch adopt",
                "status": "blocked",
                "runtime": "task_dispatch",
                "mode": mode,
                "blocker_codes": ["dispatch_adoption_receipt_required"],
                "next_actions": ["Use a dispatch receipt with a non-empty identity."],
                "run_id": run_id,
                "task_id": task_id,
            });
            emit_payload(&payload, as_json);
            return ExitCode::from(1);
        }
        Some(receipt_id)
    } else {
        None
    };

    if dry_run {
        let payload = serde_json::json!({
            "surface": "vida taskflow dispatch adopt",
            "status": "pass",
            "runtime": "task_dispatch",
            "mode": mode,
            "adoption_status": "dry_run",
            "would_adopt": run_id.as_ref().zip(task_id.as_ref()).map(|(run_id, task_id)| serde_json::json!({"run_id": run_id, "task_id": task_id, "receipt_id": validated_receipt_id, "binding_status": "adopted"})),
            "unadopted_run_count": if run_id.is_some() { 1 } else { 0 },
            "blocker_codes": [],
            "next_actions": ["Rerun with --apply and explicit --run-id/--task-id to persist the adoption binding."],
        });
        emit_payload(&payload, as_json);
        return ExitCode::SUCCESS;
    }

    let run_id = run_id.expect("validated run id");
    let task_id = task_id.expect("validated task id");
    let receipt_id = validated_receipt_id.expect("validated dispatch receipt id");
    if let Err(reason) = TaskLifecycleService::authorize(
        mode,
        TaskExecutionBinding::ExecutionBound,
        &TaskLifecycleMutationSource::DispatchReceipt {
            run_id: run_id.clone(),
            receipt_id: receipt_id.clone(),
        },
    ) {
        let payload = serde_json::json!({
            "surface": "vida taskflow dispatch adopt",
            "status": "blocked",
            "runtime": "task_dispatch",
            "mode": mode,
            "blocker_codes": [reason],
            "next_actions": ["Enable dispatch and provide a validated run/receipt binding."],
        });
        emit_payload(&payload, as_json);
        return ExitCode::from(1);
    }
    let path = state_dir.join(ADOPTION_FILE);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mismatched_binding = existing.lines().any(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .is_some_and(|row| {
                (row["run_id"] == run_id && row["task_id"] != task_id)
                    || (row["task_id"] == task_id && row["run_id"] != run_id)
            })
    });
    let mismatched_receipt = existing.lines().any(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .is_some_and(|row| {
                row["run_id"] == run_id
                    && row["task_id"] == task_id
                    && row
                        .get("receipt_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|existing_receipt| existing_receipt != receipt_id)
            })
    });
    if mismatched_binding || mismatched_receipt {
        let payload = serde_json::json!({
            "surface": "vida taskflow dispatch adopt",
            "status": "blocked",
            "runtime": "task_dispatch",
            "mode": mode,
            "blocker_codes": ["dispatch_adoption_binding_mismatch"],
            "next_actions": ["Use the existing run/task binding or reconcile it before retrying."],
            "run_id": run_id,
            "task_id": task_id,
        });
        emit_payload(&payload, as_json);
        return ExitCode::from(1);
    }
    let already_adopted = existing.lines().any(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .is_some_and(|row| row["run_id"] == run_id && row["task_id"] == task_id)
    });
    if !already_adopted {
        fs::create_dir_all(state_dir).ok();
        let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => file,
            Err(error) => {
                let payload = serde_json::json!({
                    "surface": "vida taskflow dispatch adopt",
                    "status": "blocked",
                    "runtime": "task_dispatch",
                    "mode": mode,
                    "blocker_codes": ["dispatch_adoption_persist_failed"],
                    "next_actions": [error.to_string()],
                });
                emit_payload(&payload, as_json);
                return ExitCode::from(1);
            }
        };
        let row = serde_json::json!({
            "schema_version": 1,
            "run_id": run_id,
            "task_id": task_id,
            "receipt_id": receipt_id,
            "binding_status": "adopted",
            "authority": "task_dispatch",
        });
        if writeln!(file, "{row}").is_err() {
            return ExitCode::from(1);
        }
    }
    let payload = serde_json::json!({
        "surface": "vida taskflow dispatch adopt",
        "status": "pass",
        "runtime": "task_dispatch",
        "mode": mode,
        "adoption_status": if already_adopted { "idempotent" } else { "adopted" },
        "run_id": run_id,
        "task_id": task_id,
        "receipt_id": receipt_id,
        "binding_status": "adopted",
        "artifact_refs": {"adoption_path": path},
        "blocker_codes": [],
    });
    emit_payload(&payload, as_json);
    ExitCode::SUCCESS
}

fn option_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .filter(|value| !value.starts_with('-') && !value.trim().is_empty())
        .cloned()
}

fn emit_payload(payload: &serde_json::Value, as_json: bool) {
    if as_json {
        crate::print_json_pretty(payload);
    } else if payload["status"] == "pass" {
        println!(
            "{}: pass",
            payload["surface"].as_str().unwrap_or("dispatch")
        );
    } else {
        eprintln!("{}", payload["blocker_codes"]);
    }
}

#[allow(dead_code)]
fn disabled_payload(mode: TaskRuntimeMode) -> serde_json::Value {
    dispatch_runtime_disabled_payload("vida taskflow dispatch", mode)
}
