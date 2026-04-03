use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::release1_contracts::canonical_release1_contract_status_str;
use crate::state_store::{
    CreateTaskRequest, StateStore, StateStoreError, TaskRecord, UpdateTaskRequest,
};

pub(crate) fn taskflow_native_state_root(project_root: &Path) -> PathBuf {
    project_root.join(crate::state_store::default_state_dir())
}

pub(crate) fn proxy_state_dir() -> PathBuf {
    std::env::var_os("VIDA_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::resolve_runtime_project_root()
                .map(|project_root| taskflow_native_state_root(&project_root))
                .unwrap_or_else(|_| crate::state_store::default_state_dir())
        })
}

pub(crate) fn infer_project_root_from_state_root(state_root: &Path) -> Option<PathBuf> {
    state_root
        .ancestors()
        .find(|path| super::looks_like_project_root(path))
        .map(Path::to_path_buf)
}

fn read_runtime_consumption_snapshot(state_root: &Path) -> Result<serde_json::Value, String> {
    let summary = crate::runtime_consumption_summary(state_root)?;
    if summary.latest_kind.as_deref() != Some("final") {
        return Err(
            "execution_preparation_gate_blocked: latest runtime-consumption snapshot is not `final`"
                .to_string(),
        );
    }
    let snapshot_path = summary.latest_snapshot_path.ok_or_else(|| {
        "execution_preparation_gate_blocked: missing runtime-consumption snapshot".to_string()
    })?;
    let snapshot_body = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to read runtime-consumption snapshot: {error}"
        )
    })?;
    serde_json::from_str::<serde_json::Value>(&snapshot_body).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to parse runtime-consumption snapshot: {error}"
        )
    })
}

fn has_execution_preparation_blocker(snapshot: &serde_json::Value) -> bool {
    let mut blockers: Vec<&str> = Vec::new();
    if let Some(rows) = snapshot["closure_admission"]["blockers"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(rows) = snapshot["operator_contracts"]["blocker_codes"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(code) = snapshot["dispatch_receipt"]["blocker_code"].as_str() {
        blockers.push(code);
    }
    blockers.iter().any(|value| {
        *value == "pending_execution_preparation_evidence"
            || *value == "missing_execution_preparation_contract"
    })
}

pub(crate) fn enforce_execution_preparation_contract_gate(state_root: &Path) -> Result<(), String> {
    let snapshot = read_runtime_consumption_snapshot(state_root)?;
    let contract = &snapshot["operator_contracts"];
    let contract_ready = contract["contract_id"].as_str() == Some("release-1-operator-contracts")
        && contract["schema_version"].as_str() == Some("release-1-v1")
        && contract["status"].is_string()
        && contract["blocker_codes"].is_array()
        && contract["next_actions"].is_array()
        && contract["artifact_refs"].is_object();
    if !contract_ready {
        return Err(
            "execution_preparation_gate_blocked: missing or invalid release-1 operator contract"
                .to_string(),
        );
    }
    if contract["status"]
        .as_str()
        .and_then(canonical_release1_contract_status_str)
        .is_none()
    {
        return Err(
            "execution_preparation_gate_blocked: release-1 operator contract has invalid status"
                .to_string(),
        );
    }
    if has_execution_preparation_blocker(&snapshot) {
        return Err(
            "execution_preparation_gate_blocked: pending_execution_preparation_evidence"
                .to_string(),
        );
    }
    Ok(())
}

async fn open_task_store_for_native_bridge(
    state_root: PathBuf,
) -> Result<StateStore, StateStoreError> {
    if state_root.exists() {
        StateStore::open_existing(state_root).await
    } else {
        StateStore::open(state_root).await
    }
}

fn is_datastore_lock_contention(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("database is locked")
        || normalized.contains("database at")
            && normalized.contains("lock")
            && normalized.contains("locked")
        || normalized.contains("problem with the datastore")
            && normalized.contains("lock")
            && normalized.contains("locked")
        || normalized.contains("already locked")
}

pub(crate) fn task_record_json(task: &TaskRecord) -> serde_json::Value {
    serde_json::to_value(task).expect("task record should serialize")
}

fn run_task_store_native_fallback(
    project_root: &Path,
    args: &[String],
) -> Result<serde_json::Value, String> {
    let state_root = taskflow_native_state_root(project_root);
    match args {
        [subcommand, tail @ ..] if subcommand == "list" => {
            let mut status = None::<String>;
            let mut include_all = false;
            let mut i = 0usize;
            while i < tail.len() {
                match tail[i].as_str() {
                    "--status" if i + 1 < tail.len() => {
                        status = Some(tail[i + 1].clone());
                        i += 2;
                    }
                    "--all" => {
                        include_all = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let tasks = crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.list_tasks(status.as_deref(), include_all).await
            })?;
            Ok(serde_json::to_value(tasks).expect("task list should serialize"))
        }
        [subcommand, task_id] if subcommand == "show" => {
            match crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.show_task(task_id).await
            }) {
                Ok(task) => Ok(serde_json::json!({
                    "status": "pass",
                    "task": task,
                })),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand] if subcommand == "ready" => {
            let tasks = crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.ready_tasks().await
            })?;
            Ok(serde_json::to_value(tasks).expect("ready task list should serialize"))
        }
        [subcommand, source] if subcommand == "import-jsonl" => {
            let summary = crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.import_tasks_from_jsonl(Path::new(source)).await
            })?;
            Ok(serde_json::json!({
                "status": "pass",
                "imported_count": summary.imported_count,
                "unchanged_count": summary.unchanged_count,
                "updated_count": summary.updated_count,
                "source_path": summary.source_path,
            }))
        }
        [subcommand, target] if subcommand == "export-jsonl" => {
            let exported_count = crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.export_tasks_to_jsonl(Path::new(target)).await
            })?;
            Ok(serde_json::json!({
                "status": "pass",
                "exported_count": exported_count,
                "target_path": target,
            }))
        }
        [subcommand, task_id, title, rest @ ..] if subcommand == "create" => {
            let mut issue_type = "task".to_string();
            let mut status = "open".to_string();
            let mut priority = 2u32;
            let mut parent_id = None::<String>;
            let mut display_id = String::new();
            let mut description = String::new();
            let mut labels = Vec::new();
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--type" if i + 1 < rest.len() => {
                        issue_type = rest[i + 1].clone();
                        i += 2;
                    }
                    "--status" if i + 1 < rest.len() => {
                        status = rest[i + 1].clone();
                        i += 2;
                    }
                    "--priority" if i + 1 < rest.len() => {
                        priority = rest[i + 1].parse::<u32>().unwrap_or(2);
                        i += 2;
                    }
                    "--parent-id" if i + 1 < rest.len() => {
                        parent_id = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = rest[i + 1].clone();
                        i += 2;
                    }
                    "--labels" if i + 1 < rest.len() => {
                        labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--display-id" if i + 1 < rest.len() => {
                        display_id = rest[i + 1].clone();
                        i += 2;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            match crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                let source_repo = project_root.display().to_string();
                store
                    .create_task(CreateTaskRequest {
                        task_id,
                        title,
                        display_id: (!display_id.is_empty()).then_some(display_id.as_str()),
                        description: &description,
                        issue_type: &issue_type,
                        status: &status,
                        priority,
                        parent_id: parent_id.as_deref(),
                        labels: &labels,
                        created_by: "vida taskflow",
                        source_repo: &source_repo,
                    })
                    .await
            }) {
                Ok(task) => Ok(task_record_json(&task)),
                Err(error) if error.contains("task already exists") => Ok(serde_json::json!({
                    "status": "error",
                    "reason": "task_already_exists",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand, task_id, rest @ ..] if subcommand == "update" => {
            let mut status = None::<String>;
            let mut notes = None::<String>;
            let mut description = None::<String>;
            let mut add_labels = Vec::new();
            let mut remove_labels = Vec::new();
            let mut set_labels = None::<Vec<String>>;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--status" if i + 1 < rest.len() => {
                        status = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--notes" if i + 1 < rest.len() => {
                        notes = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--add-label" if i + 1 < rest.len() => {
                        add_labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--remove-label" if i + 1 < rest.len() => {
                        remove_labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--set-labels" if i + 1 < rest.len() => {
                        set_labels = Some(
                            rest[i + 1]
                                .split(',')
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(|value| value.to_string())
                                .collect::<Vec<_>>(),
                        );
                        i += 2;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            match crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store
                    .update_task(UpdateTaskRequest {
                        task_id,
                        status: status.as_deref(),
                        notes: notes.as_deref(),
                        description: description.as_deref(),
                        add_labels: &add_labels,
                        remove_labels: &remove_labels,
                        set_labels: set_labels.as_deref(),
                    })
                    .await
            }) {
                Ok(task) => Ok(serde_json::json!({
                    "status": "pass",
                    "task": task,
                })),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        [subcommand, task_id, flag, reason] if subcommand == "close" && flag == "--reason" => {
            match crate::block_on_state_store(async {
                let store = open_task_store_for_native_bridge(state_root.clone()).await?;
                store.close_task(task_id, reason).await
            }) {
                Ok(task) => Ok(serde_json::json!({
                    "status": "pass",
                    "task": task,
                })),
                Err(error) if error.contains("missing task") => Ok(serde_json::json!({
                    "status": "missing",
                    "reason": "missing_task",
                    "task_id": task_id,
                })),
                Err(error) => Err(error),
            }
        }
        _ => Err("unsupported_taskflow_task_bridge".to_string()),
    }
}

const MAX_TASK_STORE_RETRY_ATTEMPTS: usize = 8;

fn run_task_store_with_backoff<T, F, S>(mut operation: F, mut sleep: S) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
    S: FnMut(Duration),
{
    let mut delay_ms = 25u64;
    let mut last_error = None::<String>;
    for attempt in 1..=MAX_TASK_STORE_RETRY_ATTEMPTS {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if attempt < MAX_TASK_STORE_RETRY_ATTEMPTS => {
                if is_datastore_lock_contention(&error) {
                    last_error = Some(error);
                    sleep(Duration::from_millis(delay_ms));
                    delay_ms = (delay_ms * 2).min(400);
                } else {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        "taskflow bridge exhausted datastore lock contention retries".to_string()
    }))
}

pub(crate) fn run_task_store_helper(
    project_root: &Path,
    args: &[String],
) -> Result<serde_json::Value, String> {
    run_task_store_with_backoff(
        || run_task_store_native_fallback(project_root, args),
        std::thread::sleep,
    )
}

pub(crate) fn helper_value_is_missing(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(|status| status == "missing")
        .unwrap_or(false)
}

pub(crate) fn helper_value_is_ok(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(|status| {
            crate::release1_contracts::canonical_release1_contract_status_str(status)
                == Some("pass")
        })
        .unwrap_or(false)
}

fn apply_status_override(payload: &mut serde_json::Value) {
    if let Ok(override_status) = env::var("VIDA_TASK_BRIDGE_STATUS_OVERRIDE") {
        if !override_status.is_empty() {
            payload["status"] = serde_json::Value::String(override_status);
        }
    }
}

fn canonicalize_helper_status(payload: &mut serde_json::Value) -> bool {
    let status_value = match payload.get_mut("status") {
        Some(value) => value,
        None => return false,
    };
    let status_str = match status_value.as_str() {
        Some(value) => value,
        None => return false,
    };
    if let Some(canonical_status) =
        crate::release1_contracts::canonical_release1_contract_status_str(status_str)
    {
        *status_value = serde_json::Value::String(canonical_status.to_string());
        true
    } else {
        false
    }
}

fn parse_display_path(display_id: &str) -> Option<(String, Vec<u32>)> {
    let trimmed = display_id.trim();
    if !trimmed.starts_with("vida-") {
        return None;
    }
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts[0].len() <= 5 {
        return None;
    }
    let mut levels = Vec::new();
    for part in parts.iter().skip(1) {
        levels.push(part.parse::<u32>().ok()?);
    }
    Some((parts[0].to_string(), levels))
}

pub(crate) fn next_display_id_payload(
    rows: &[serde_json::Value],
    parent_display_id: &str,
) -> serde_json::Value {
    let Some((parent_root, parent_levels)) = parse_display_path(parent_display_id) else {
        return serde_json::json!({
            "valid": false,
            "reason": "invalid_parent_display_id",
            "parent_display_id": parent_display_id,
        });
    };

    let mut max_child = 0u32;
    for row in rows {
        let display_id = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        let Some((child_root, child_levels)) = parse_display_path(display_id) else {
            continue;
        };
        if child_root != parent_root || child_levels.len() != parent_levels.len() + 1 {
            continue;
        }
        if !parent_levels.is_empty() && child_levels[..parent_levels.len()] != parent_levels[..] {
            continue;
        }
        max_child = max_child.max(*child_levels.last().unwrap_or(&0));
    }

    let next_index = max_child + 1;
    serde_json::json!({
        "valid": true,
        "parent_display_id": parent_display_id,
        "next_display_id": format!("{parent_display_id}.{next_index}"),
        "next_index": next_index,
    })
}

pub(crate) fn resolve_task_id_by_display_id(
    rows: &[serde_json::Value],
    display_id: &str,
) -> serde_json::Value {
    for row in rows {
        let current = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        if current == display_id {
            return serde_json::json!({
                "found": true,
                "display_id": display_id,
                "task_id": row.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(),
            });
        }
    }
    serde_json::json!({
        "found": false,
        "display_id": display_id,
        "reason": "parent_display_id_not_found",
    })
}

fn render_task_list_payload(payload: &serde_json::Value, as_json: bool) -> ExitCode {
    if as_json {
        crate::print_json_pretty(payload);
    } else if let Some(rows) = payload.as_array() {
        for row in rows {
            crate::print_jsonl_value(row);
        }
    } else {
        crate::print_json_pretty(payload);
    }
    ExitCode::SUCCESS
}

pub(crate) fn run_taskflow_task_bridge(
    project_root: &Path,
    args: &[String],
) -> Result<ExitCode, String> {
    match args {
        [head] if head == "task" => {
            crate::print_taskflow_proxy_help(Some("task"));
            Ok(ExitCode::SUCCESS)
        }
        [head, flag] if head == "task" && matches!(flag.as_str(), "--help" | "-h") => {
            crate::print_taskflow_proxy_help(Some("task"));
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, ..] if head == "task" && subcommand == "list" => {
            let mut helper_args = vec!["list".to_string()];
            let mut status = None::<String>;
            let mut include_all = false;
            let mut as_json = false;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--status" if i + 1 < args.len() => {
                        status = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--all" => {
                        include_all = true;
                        i += 1;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            if let Some(status) = status {
                helper_args.extend(["--status".to_string(), status]);
            }
            if include_all {
                helper_args.push("--all".to_string());
            }
            let payload = run_task_store_helper(project_root, &helper_args)?;
            Ok(render_task_list_payload(&payload, as_json))
        }
        [head, subcommand, task_id, tail @ ..] if head == "task" && subcommand == "show" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            let as_jsonl = tail.iter().any(|arg| arg == "--jsonl");
            if tail
                .iter()
                .any(|arg| !matches!(arg.as_str(), "--json" | "--jsonl"))
            {
                return Err("unsupported delegated task arguments".to_string());
            }
            let mut payload =
                run_task_store_helper(project_root, &["show".to_string(), task_id.clone()])?;
            if helper_value_is_missing(&payload) && task_id.starts_with("vida-") {
                let rows = run_task_store_helper(
                    project_root,
                    &["list".to_string(), "--all".to_string()],
                )?;
                if let Some(entries) = rows.as_array() {
                    let resolved = resolve_task_id_by_display_id(entries, task_id);
                    if resolved
                        .get("found")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        let resolved_id = resolved
                            .get("task_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        payload = run_task_store_helper(
                            project_root,
                            &["show".to_string(), resolved_id],
                        )?;
                    }
                }
            }
            if helper_value_is_missing(&payload) {
                if as_json {
                    crate::print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            if as_json {
                crate::print_json_pretty(&payload);
            } else if as_jsonl {
                crate::print_jsonl_value(&payload);
            } else {
                crate::print_json_pretty(&payload);
            }
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, ..] if head == "task" && subcommand == "ready" => {
            let as_json = args.iter().any(|arg| arg == "--json");
            if args
                .iter()
                .skip(2)
                .any(|arg| !matches!(arg.as_str(), "--json"))
            {
                return Err("unsupported delegated task arguments".to_string());
            }
            let payload = run_task_store_helper(project_root, &["ready".to_string()])?;
            Ok(render_task_list_payload(&payload, as_json))
        }
        [head, subcommand, source, tail @ ..] if head == "task" && subcommand == "import-jsonl" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let mut payload =
                run_task_store_helper(project_root, &["import-jsonl".to_string(), source.clone()])?;
            apply_status_override(&mut payload);
            if !canonicalize_helper_status(&mut payload) {
                let status = payload
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("missing");
                eprintln!(
                    "task import-jsonl helper reported non-canonical status `{}`",
                    status
                );
                return Err("task import-jsonl helper reported non-canonical status".to_string());
            }
            if as_json {
                crate::print_json_pretty(&payload);
            } else {
                println!(
                    "{}: imported={} unchanged={} updated={}",
                    payload
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("error"),
                    payload
                        .get("imported_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("unchanged_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("updated_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                );
            }
            Ok(if helper_value_is_ok(&payload) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, target, tail @ ..] if head == "task" && subcommand == "export-jsonl" => {
            let as_json = tail.iter().any(|arg| arg == "--json");
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let mut payload =
                run_task_store_helper(project_root, &["export-jsonl".to_string(), target.clone()])?;
            apply_status_override(&mut payload);
            if !canonicalize_helper_status(&mut payload) {
                let status = payload
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("missing");
                eprintln!(
                    "task export-jsonl helper reported non-canonical status `{}`",
                    status
                );
                return Err("task export-jsonl helper reported non-canonical status".to_string());
            }
            if as_json {
                crate::print_json_pretty(&payload);
            } else {
                println!(
                    "{}: exported={} target={}",
                    payload
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("error"),
                    payload
                        .get("exported_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    payload
                        .get("target_path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(target)
                );
            }
            Ok(if helper_value_is_ok(&payload) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, parent_display_id, tail @ ..]
            if head == "task" && subcommand == "next-display-id" =>
        {
            if tail.iter().any(|arg| !matches!(arg.as_str(), "--json")) {
                return Err("unsupported delegated task arguments".to_string());
            }
            let rows =
                run_task_store_helper(project_root, &["list".to_string(), "--all".to_string()])?;
            let entries = rows
                .as_array()
                .ok_or_else(|| "task list payload should be an array".to_string())?;
            let payload = next_display_id_payload(entries, parent_display_id);
            let valid = payload
                .get("valid")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            crate::print_json_pretty(&payload);
            Ok(if valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        [head, subcommand, task_id, title, rest @ ..]
            if head == "task" && subcommand == "create" =>
        {
            let mut issue_type = "task".to_string();
            let mut status = "open".to_string();
            let mut priority = "2".to_string();
            let mut display_id = String::new();
            let mut parent_id = String::new();
            let mut parent_display_id = String::new();
            let mut auto_display_from = String::new();
            let mut description = String::new();
            let mut labels = Vec::new();
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--type" if i + 1 < rest.len() => {
                        issue_type = rest[i + 1].clone();
                        i += 2;
                    }
                    "--status" if i + 1 < rest.len() => {
                        status = rest[i + 1].clone();
                        i += 2;
                    }
                    "--priority" if i + 1 < rest.len() => {
                        priority = rest[i + 1].clone();
                        i += 2;
                    }
                    "--display-id" if i + 1 < rest.len() => {
                        display_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--parent-id" if i + 1 < rest.len() => {
                        parent_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--parent-display-id" if i + 1 < rest.len() => {
                        parent_display_id = rest[i + 1].clone();
                        i += 2;
                    }
                    "--auto-display-from" if i + 1 < rest.len() => {
                        auto_display_from = rest[i + 1].clone();
                        i += 2;
                    }
                    "--description" if i + 1 < rest.len() => {
                        description = rest[i + 1].clone();
                        i += 2;
                    }
                    "--labels" if i + 1 < rest.len() => {
                        labels.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            if display_id.is_empty() && !auto_display_from.is_empty() && !parent_id.is_empty() {
                display_id = format!("{auto_display_from}.1");
            }
            if (display_id.is_empty()
                && !auto_display_from.is_empty()
                && parent_id.is_empty()
                && parent_display_id.is_empty())
                || (parent_id.is_empty() && !parent_display_id.is_empty())
            {
                let rows = run_task_store_helper(
                    project_root,
                    &["list".to_string(), "--all".to_string()],
                )?;
                let entries = rows
                    .as_array()
                    .ok_or_else(|| "task list payload should be an array".to_string())?;
                if display_id.is_empty() && !auto_display_from.is_empty() {
                    let next = next_display_id_payload(entries, &auto_display_from);
                    if !next
                        .get("valid")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        if as_json {
                            crate::print_json_pretty(&next);
                        } else {
                            eprintln!(
                                "{}",
                                next.get("reason")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("invalid_parent_display_id")
                            );
                        }
                        return Ok(ExitCode::from(1));
                    }
                    display_id = next
                        .get("next_display_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
                if parent_id.is_empty() && !parent_display_id.is_empty() {
                    let resolved = resolve_task_id_by_display_id(entries, &parent_display_id);
                    if !resolved
                        .get("found")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                    {
                        if as_json {
                            crate::print_json_pretty(&resolved);
                        } else {
                            eprintln!(
                                "{}",
                                resolved
                                    .get("reason")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("parent_display_id_not_found")
                            );
                        }
                        return Ok(ExitCode::from(1));
                    }
                    parent_id = resolved
                        .get("task_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                }
            }
            let mut helper_args = vec![
                "create".to_string(),
                task_id.clone(),
                title.clone(),
                "--type".to_string(),
                issue_type,
                "--status".to_string(),
                status,
                "--priority".to_string(),
                priority,
            ];
            if !display_id.is_empty() {
                helper_args.extend(["--display-id".to_string(), display_id]);
            }
            if !parent_id.is_empty() {
                helper_args.extend(["--parent-id".to_string(), parent_id]);
            }
            if !description.is_empty() {
                helper_args.extend(["--description".to_string(), description]);
            }
            for label in labels {
                helper_args.extend(["--labels".to_string(), label]);
            }
            let payload = run_task_store_helper(project_root, &helper_args)?;
            crate::print_json_pretty(&payload);
            Ok(
                if helper_value_is_missing(&payload)
                    || payload.get("reason").and_then(serde_json::Value::as_str)
                        == Some("task_already_exists")
                {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                },
            )
        }
        [head, subcommand, task_id, rest @ ..] if head == "task" && subcommand == "update" => {
            let mut helper_args = vec!["update".to_string(), task_id.clone()];
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--status" | "--notes" | "--description" | "--add-label" | "--remove-label"
                    | "--set-labels"
                        if i + 1 < rest.len() =>
                    {
                        helper_args.push(rest[i].clone());
                        helper_args.push(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let mut payload = run_task_store_helper(project_root, &helper_args)?;
            apply_status_override(&mut payload);
            if helper_value_is_missing(&payload) {
                if as_json {
                    crate::print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            if !canonicalize_helper_status(&mut payload) {
                let status = payload
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("missing");
                eprintln!(
                    "task update helper reported non-canonical status `{}`",
                    status
                );
                return Err("task update helper reported non-canonical status".to_string());
            }
            crate::print_json_pretty(&payload);
            Ok(ExitCode::SUCCESS)
        }
        [head, subcommand, task_id, rest @ ..] if head == "task" && subcommand == "close" => {
            let mut reason = None::<String>;
            let mut as_json = false;
            let mut i = 0usize;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--reason" if i + 1 < rest.len() => {
                        reason = Some(rest[i + 1].clone());
                        i += 2;
                    }
                    "--json" => {
                        as_json = true;
                        i += 1;
                    }
                    _ => return Err("unsupported delegated task arguments".to_string()),
                }
            }
            let reason = reason.ok_or_else(|| {
                "Usage: vida taskflow task close <task_id> --reason <reason> [--json]".to_string()
            })?;
            let close_reason = reason.clone();
            let payload = run_task_store_helper(
                project_root,
                &[
                    "close".to_string(),
                    task_id.clone(),
                    "--reason".to_string(),
                    reason,
                ],
            )?;
            if helper_value_is_missing(&payload) {
                if as_json {
                    crate::print_json_pretty(&payload);
                } else {
                    eprintln!("Missing task: {task_id}");
                }
                return Ok(ExitCode::from(1));
            }
            let mut render_payload = payload.clone();
            let telemetry_task = payload.get("task").cloned();
            render_payload["host_agent_telemetry"] = match telemetry_task.as_ref() {
                Some(task) => {
                    crate::agent_feedback_surface::maybe_record_task_close_host_agent_feedback(
                        project_root,
                        task,
                        &close_reason,
                        "vida taskflow task close",
                    )
                }
                None => serde_json::json!({
                    "status": "skipped",
                    "reason": "task_payload_unavailable_before_close"
                }),
            };
            crate::print_json_pretty(&render_payload);
            Ok(ExitCode::SUCCESS)
        }
        _ => Err("unsupported_taskflow_task_bridge".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_helper_status, enforce_execution_preparation_contract_gate,
        has_execution_preparation_blocker, helper_value_is_ok, is_datastore_lock_contention,
        run_task_store_with_backoff, Duration, MAX_TASK_STORE_RETRY_ATTEMPTS,
    };
    use crate::release1_contracts::canonical_release1_contract_status_str;
    use std::fs;

    #[test]
    fn execution_preparation_blocker_ignores_unrelated_operator_contract_blockers() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": [],
            },
            "operator_contracts": {
                "blocker_codes": [
                    "migration_required",
                    "protocol_binding_blocking_issues",
                ],
            },
            "dispatch_receipt": {},
        });

        assert!(!has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn execution_preparation_blocker_detects_pending_execution_preparation_evidence() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": [],
            },
            "operator_contracts": {
                "blocker_codes": [
                    "pending_execution_preparation_evidence",
                ],
            },
            "dispatch_receipt": {},
        });

        assert!(has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn release1_operator_contract_status_compatibility_normalizes_to_canonical_vocabulary() {
        assert_eq!(canonical_release1_contract_status_str("pass"), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str(" blocked "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("ok"), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str("block"),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("unknown"), None);
    }

    #[test]
    fn helper_value_is_ok_accepts_release1_pass_and_legacy_ok() {
        assert!(helper_value_is_ok(&serde_json::json!({"status": "pass"})));
        assert!(helper_value_is_ok(&serde_json::json!({"status": "ok"})));
        assert!(!helper_value_is_ok(
            &serde_json::json!({"status": "blocked"})
        ));
        assert!(!helper_value_is_ok(&serde_json::json!({"status": "block"})));
    }

    #[test]
    fn canonicalize_helper_status_replaces_legacy_ok() {
        let mut payload = serde_json::json!({"status": "ok"});
        assert!(canonicalize_helper_status(&mut payload));
        assert_eq!(payload["status"], "pass");
    }

    #[test]
    fn canonicalize_helper_status_leaves_non_release1_values() {
        let mut payload = serde_json::json!({"status": "error"});
        assert!(!canonicalize_helper_status(&mut payload));
        assert_eq!(payload["status"], "error");
    }

    #[test]
    fn execution_preparation_contract_gate_accepts_release1_canonical_and_compat_statuses() {
        let cases = [
            ("pass", "final-pass.json"),
            ("blocked", "final-blocked.json"),
            ("ok", "final-ok.json"),
            ("block", "final-block.json"),
        ];

        for (status, file_name) in cases {
            let root = std::env::temp_dir().join(format!(
                "vida-taskflow-bridge-release1-operator-contract-gate-{}-{}-{}",
                std::process::id(),
                status,
                file_name
            ));
            let snapshot_dir = root.join("runtime-consumption");
            fs::create_dir_all(&snapshot_dir).expect("create snapshot dir");
            let snapshot = serde_json::json!({
                "closure_admission": {
                    "blockers": [],
                },
                "operator_contracts": {
                    "contract_id": "release-1-operator-contracts",
                    "schema_version": "release-1-v1",
                    "status": status,
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {},
                },
                "dispatch_receipt": {
                    "blocker_code": null,
                },
            });
            let snapshot_path = snapshot_dir.join(file_name);
            fs::write(
                &snapshot_path,
                serde_json::to_string_pretty(&snapshot).expect("serialize snapshot"),
            )
            .expect("write runtime consumption snapshot");
            assert_eq!(
                enforce_execution_preparation_contract_gate(root.as_path()),
                Ok(())
            );

            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn run_task_store_with_backoff_stops_after_bounded_lock_contention_attempts() {
        let mut attempts = 0usize;
        let mut sleep_delays = Vec::new();
        let result: Result<(), String> = run_task_store_with_backoff(
            || {
                attempts += 1;
                Err("problem with the datastore lock: locked".to_string())
            },
            |delay| sleep_delays.push(delay),
        );

        let error = result.expect_err("lock contention should fail closed after retries");
        assert!(is_datastore_lock_contention(&error));
        assert_eq!(attempts, MAX_TASK_STORE_RETRY_ATTEMPTS);
        assert_eq!(sleep_delays.len(), MAX_TASK_STORE_RETRY_ATTEMPTS - 1);
        assert_eq!(
            sleep_delays,
            vec![
                Duration::from_millis(25),
                Duration::from_millis(50),
                Duration::from_millis(100),
                Duration::from_millis(200),
                Duration::from_millis(400),
                Duration::from_millis(400),
                Duration::from_millis(400),
            ]
        );
    }

    #[test]
    fn run_task_store_with_backoff_retries_on_already_locked_datastore_phrase() {
        let mut attempts = 0usize;
        let mut sleep_delays = Vec::new();
        let result: Result<(), String> = run_task_store_with_backoff(
            || {
                attempts += 1;
                Err("LOCK is already locked".to_string())
            },
            |delay| sleep_delays.push(delay),
        );

        let error = result.expect_err("already-locked contention should fail closed after retries");
        assert!(is_datastore_lock_contention(&error));
        assert_eq!(attempts, MAX_TASK_STORE_RETRY_ATTEMPTS);
        assert_eq!(sleep_delays.len(), MAX_TASK_STORE_RETRY_ATTEMPTS - 1);
    }
}
