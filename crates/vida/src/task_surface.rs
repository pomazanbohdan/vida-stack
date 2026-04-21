use super::*;
use crate::task_cli_render::{print_task_bulk_reparent_result, print_task_direct_children};

#[derive(Debug, Clone)]
pub(crate) struct TaskReadMetadata {
    pub mode: &'static str,
    pub degraded: bool,
    pub snapshot_path: Option<String>,
    pub detail: &'static str,
}

impl TaskReadMetadata {
    fn authoritative_live() -> Self {
        Self {
            mode: "authoritative_live",
            degraded: false,
            snapshot_path: None,
            detail: "served from the authoritative state store",
        }
    }

    fn snapshot(path: &std::path::Path, detail: &'static str) -> Self {
        Self {
            mode: "snapshot",
            degraded: true,
            snapshot_path: Some(path.display().to_string()),
            detail,
        }
    }
}

fn task_json_success_status() -> &'static str {
    crate::contract_profile_adapter::release_contract_status(true)
}

fn canonical_json_string_array_entries(value: &serde_json::Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

fn normalize_task_json_contract_arrays(summary_json: &mut serde_json::Value) -> Result<(), String> {
    let Some(summary) = summary_json.as_object_mut() else {
        return Ok(());
    };
    for key in ["blocker_codes", "next_actions"] {
        if let Some(value) = summary.get(key) {
            let entries = canonical_json_string_array_entries(value).ok_or_else(|| {
                format!(
                    "task json contract inconsistency: `{key}` must contain canonical nonempty string entries"
                )
            })?;
            summary.insert(key.to_string(), serde_json::json!(entries));
        }
    }
    Ok(())
}

async fn open_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    if state_dir.exists() {
        StateStore::open_existing(state_dir).await
    } else {
        StateStore::open(state_dir).await
    }
}

pub(crate) async fn open_read_only_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    StateStore::open_existing_read_only(state_dir).await
}

fn is_authoritative_state_lock_error(error: &state_store::StateStoreError) -> bool {
    let message = error.to_string();
    message.contains("LOCK") || message.contains("lock")
}

fn load_task_snapshot_rows(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path)
}

async fn load_task_snapshot_rows_with_retry(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    for attempt in 0..80 {
        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
            Ok(rows) => return Ok(rows),
            Err(error @ state_store::StateStoreError::Io(_)) if attempt < 79 => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }
    load_task_snapshot_rows(state_dir)
}

async fn load_task_snapshot_rows_snapshot_first(
    state_dir: &std::path::Path,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
        Ok(rows) => Ok((
            rows,
            TaskReadMetadata::snapshot(
                &snapshot_path,
                "served from canonical task snapshot evidence",
            ),
        )),
        Err(snapshot_error) => match open_read_only_task_store(state_dir.to_path_buf()).await {
            Ok(store) => store
                .list_tasks(None, true)
                .await
                .map(|rows| (rows, TaskReadMetadata::authoritative_live())),
            Err(live_error) if is_authoritative_state_lock_error(&live_error) => Err(live_error),
            Err(live_error) => Err(live_error),
        }
        .map_err(|live_error| match snapshot_error {
            state_store::StateStoreError::Io(_) => live_error,
            other => other,
        }),
    }
}

fn resolve_task_from_rows(
    rows: &[state_store::TaskRecord],
    task_id_or_display_id: &str,
) -> Result<state_store::TaskRecord, state_store::StateStoreError> {
    if let Some(task) = rows.iter().find(|task| task.id == task_id_or_display_id) {
        return Ok(task.clone());
    }
    if let Some(task) = rows
        .iter()
        .find(|task| task.display_id.as_deref() == Some(task_id_or_display_id))
    {
        return Ok(task.clone());
    }
    Err(state_store::StateStoreError::MissingTask {
        task_id: task_id_or_display_id.to_string(),
    })
}

async fn refresh_task_snapshot_after_mutation(
    store: &StateStore,
    surface: &str,
) -> Result<(), ExitCode> {
    store
        .refresh_task_snapshot()
        .await
        .map(|_| ())
        .map_err(|error| {
            eprintln!("Failed to refresh canonical task snapshot after {surface}: {error}");
            ExitCode::from(1)
        })
}

pub(crate) async fn ready_tasks_scoped_read_only(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.ready_tasks_scoped(scope_task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn task_dependency_tree_read_only(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<state_store::TaskDependencyTreeNode, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.task_dependency_tree(task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::task_dependency_tree_from_rows(&rows, task_id)
        }
        Err(error) => Err(error),
    }
}

async fn task_list_snapshot_first(
    state_dir: std::path::PathBuf,
    status: Option<&str>,
    include_all: bool,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let filtered = rows
        .into_iter()
        .filter(|task| include_all || task.status != "closed")
        .filter(|task| status.map(|wanted| task.status == wanted).unwrap_or(true))
        .collect();
    Ok((filtered, metadata))
}

async fn task_show_snapshot_first(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<(state_store::TaskRecord, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let task = resolve_task_from_rows(&rows, task_id)?;
    Ok((task, metadata))
}

async fn task_ready_snapshot_first(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let tasks = StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)?;
    Ok((tasks, metadata))
}

fn task_rows_as_values(
    tasks: &[state_store::TaskRecord],
) -> Result<Vec<serde_json::Value>, String> {
    tasks
        .iter()
        .map(|task| serde_json::to_value(task).map_err(|error| error.to_string()))
        .collect()
}

fn project_root_for_task_state(state_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir)
        .or_else(|| crate::resolve_runtime_project_root().ok())
}

fn resolve_optional_text_arg(
    label: &str,
    direct: Option<&str>,
    file_path: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    if direct.is_some() && file_path.is_some() {
        return Err(format!(
            "Use only one {label} source: --{label} <text> or --{label}-file <path>"
        ));
    }
    if let Some(path) = file_path {
        let value = std::fs::read_to_string(path).map_err(|error| {
            format!("Failed to read {label} file `{}`: {error}", path.display())
        })?;
        return Ok(Some(value));
    }
    Ok(direct.map(ToOwned::to_owned))
}

fn task_execution_semantics_from_create_args(
    command: &TaskCreateArgs,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: command.execution_mode.clone(),
        order_bucket: command.order_bucket.clone(),
        parallel_group: command.parallel_group.clone(),
        conflict_domain: command.conflict_domain.clone(),
    }
}

fn task_update_semantics_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err(
            "Use either the value flag or the matching clear flag for execution semantics, not both."
                .to_string(),
        );
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn task_update_parent_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err("Use either --parent-id or --clear-parent-id, not both.".to_string());
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn parse_label_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
}

fn parse_optional_label_value(value: Option<&str>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.to_string())
            .collect::<Vec<_>>()
    })
}

async fn run_task_create_like(command: TaskCreateArgs, ensure_existing: bool) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let project_root = project_root_for_task_state(&state_dir).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    match open_task_store(state_dir).await {
        Ok(store) => {
            let mut parent_id = command.parent_id.clone();
            let mut display_id = command.display_id.clone().unwrap_or_default();
            let auto_display_from = command.auto_display_from.clone().unwrap_or_default();
            let parent_display_id = command.parent_display_id.clone().unwrap_or_default();
            if display_id.is_empty() && !auto_display_from.is_empty() && parent_id.is_some() {
                display_id = format!("{auto_display_from}.1");
            }
            if (display_id.is_empty() && !auto_display_from.is_empty())
                || (parent_id.is_none() && !parent_display_id.is_empty())
            {
                match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            if display_id.is_empty() && !auto_display_from.is_empty() {
                                let next = crate::taskflow_task_bridge::next_display_id_payload(
                                    &rows,
                                    &auto_display_from,
                                );
                                if !next
                                    .get("valid")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    print_task_next_display_id(command.render, &next, command.json);
                                    return ExitCode::from(1);
                                }
                                display_id = next
                                    .get("next_display_id")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                            }
                            if parent_id.is_none() && !parent_display_id.is_empty() {
                                let resolved =
                                    crate::taskflow_task_bridge::resolve_task_id_by_display_id(
                                        &rows,
                                        &parent_display_id,
                                    );
                                if !resolved
                                    .get("found")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    if command.json {
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
                                    return ExitCode::from(1);
                                }
                                parent_id = Some(
                                    resolved
                                        .get("task_id")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_string(),
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Failed to {} task: {error}",
                                if ensure_existing { "ensure" } else { "create" }
                            );
                            return ExitCode::from(1);
                        }
                    },
                    Err(error) => {
                        eprintln!(
                            "Failed to {} task: {error}",
                            if ensure_existing { "ensure" } else { "create" }
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            if ensure_existing {
                if let Ok(task) = store.show_task(&command.task_id).await {
                    print_task_mutation(command.render, "vida task ensure", &task, command.json);
                    return ExitCode::SUCCESS;
                }
            }
            let labels = parse_label_values(&command.labels);
            let source_repo = project_root.display().to_string();
            match store
                .create_task(state_store::CreateTaskRequest {
                    task_id: &command.task_id,
                    title: &command.title,
                    display_id: (!display_id.is_empty()).then_some(display_id.as_str()),
                    description: &command.description,
                    issue_type: &command.issue_type,
                    status: &command.status,
                    priority: command.priority,
                    parent_id: parent_id.as_deref(),
                    labels: &labels,
                    execution_semantics: task_execution_semantics_from_create_args(&command),
                    created_by: "vida task",
                    source_repo: &source_repo,
                })
                .await
            {
                Ok(task) => {
                    if let Err(code) =
                        refresh_task_snapshot_after_mutation(&store, "vida task create").await
                    {
                        return code;
                    }
                    print_task_mutation(
                        command.render,
                        if ensure_existing {
                            "vida task ensure"
                        } else {
                            "vida task create"
                        },
                        &task,
                        command.json,
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!(
                        "Failed to {} task: {error}",
                        if ensure_existing { "ensure" } else { "create" }
                    );
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::Help(command) => match command.topic.as_deref() {
            None | Some("task") => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some("parallelism" | "scheduling") => {
                print_taskflow_proxy_help(Some("parallelism"));
                ExitCode::SUCCESS
            }
            Some("next") => {
                print_taskflow_proxy_help(Some("next"));
                ExitCode::SUCCESS
            }
            Some("graph-summary") => {
                print_taskflow_proxy_help(Some("graph-summary"));
                ExitCode::SUCCESS
            }
            Some(
                "ready" | "deps" | "reverse-deps" | "blocked" | "children" | "reparent-children"
                | "move-children" | "tree" | "subtree" | "critical-path" | "next-display-id"
                | "create" | "ensure" | "update" | "close" | "list" | "show" | "import-jsonl"
                | "replace-jsonl" | "export-jsonl" | "validate-graph" | "dep",
            ) => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some(topic) => {
                eprintln!("Unsupported task help topic: {topic}");
                ExitCode::from(2)
            }
        },
        TaskCommand::ImportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
                    Ok(summary) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task import-jsonl")
                                .await
                        {
                            return code;
                        }
                        if command.json {
                            let mut summary_json = serde_json::json!({
                                "status": task_json_success_status(),
                                "source_path": summary.source_path,
                                "imported_count": summary.imported_count,
                                "unchanged_count": summary.unchanged_count,
                                "updated_count": summary.updated_count,
                            });
                            if let Err(error) =
                                normalize_task_json_contract_arrays(&mut summary_json)
                            {
                                eprintln!("Failed to render task import-jsonl json: {error}");
                                return ExitCode::from(1);
                            }
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&summary_json)
                                    .expect("json import summary should render")
                            );
                        } else {
                            print_surface_header(command.render, "vida task import-jsonl");
                            print_surface_line(command.render, "import", &summary.as_display());
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to import tasks from JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReplaceJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store
                    .replace_with_taskflow_snapshot_file(&command.path)
                    .await
                {
                    Ok(()) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task replace-jsonl")
                                .await
                        {
                            return code;
                        }
                        let source_path = command.path.display().to_string();
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": task_json_success_status(),
                                "operation": "replace_snapshot",
                                "source_path": source_path,
                            }));
                        } else {
                            print_surface_header(command.render, "vida task replace-jsonl");
                            print_surface_line(command.render, "status", "pass");
                            print_surface_line(command.render, "operation", "replace_snapshot");
                            print_surface_line(command.render, "source path", &source_path);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to replace tasks from snapshot file: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ExportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_task_store(state_dir).await {
                Ok(store) => match store.export_tasks_to_jsonl(&command.path).await {
                    Ok(exported_count) => {
                        print_task_export_summary(
                            command.render,
                            u64::try_from(exported_count)
                                .expect("task export count should fit u64"),
                            &command.path.display().to_string(),
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to export tasks to JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::List(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_list_snapshot_first(state_dir, command.status.as_deref(), command.all).await
            {
                Ok((tasks, metadata)) => {
                    print_task_list(
                        command.render,
                        &tasks,
                        command.summary,
                        command.json,
                        Some(&metadata),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to list tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_show_snapshot_first(state_dir, &command.task_id).await {
                Ok((task, metadata)) => {
                    print_task_show(command.render, &task, command.json, Some(&metadata));
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to show task: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Progress(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.task_progress_summary(&command.task_id).await {
                    Ok(summary) => {
                        print_task_progress(command.render, &summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute task progress: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Ready(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_ready_snapshot_first(state_dir, command.scope.as_deref()).await {
                Ok((tasks, metadata)) => {
                    print_task_ready(
                        command.render,
                        command.scope.as_deref(),
                        &tasks,
                        command.json,
                        Some(&metadata),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to compute ready tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Next(command) => {
            let mut proxy_args = vec!["next".to_string()];
            if let Some(scope) = command.scope.as_deref() {
                proxy_args.push("--scope".to_string());
                proxy_args.push(scope.to_string());
            }
            if let Some(state_dir) = command.state_dir.as_ref().and_then(|path| path.to_str()) {
                proxy_args.push("--state-dir".to_string());
                proxy_args.push(state_dir.to_string());
            }
            if command.json {
                proxy_args.push("--json".to_string());
            }
            crate::taskflow_proxy::run_taskflow_next_surface(&proxy_args).await
        }
        TaskCommand::NextDisplayId(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_task_store(state_dir).await {
                Ok(store) => match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            let payload = crate::taskflow_task_bridge::next_display_id_payload(
                                &rows,
                                &command.parent_display_id,
                            );
                            let valid = payload
                                .get("valid")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false);
                            print_task_next_display_id(command.render, &payload, command.json);
                            if valid {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to compute next display id: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to list tasks for next display id: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Create(command) => run_task_create_like(command, false).await,
        TaskCommand::Ensure(command) => run_task_create_like(command, true).await,
        TaskCommand::Update(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let notes = match resolve_optional_text_arg(
                "notes",
                command.notes.as_deref(),
                command.notes_file.as_deref(),
            ) {
                Ok(notes) => notes,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let add_labels = parse_label_values(&command.add_labels);
            let remove_labels = parse_label_values(&command.remove_labels);
            let set_labels = parse_optional_label_value(command.set_labels.as_deref());
            let execution_mode = match task_update_semantics_arg(
                command.execution_mode.as_deref(),
                command.clear_execution_mode,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let order_bucket = match task_update_semantics_arg(
                command.order_bucket.as_deref(),
                command.clear_order_bucket,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parallel_group = match task_update_semantics_arg(
                command.parallel_group.as_deref(),
                command.clear_parallel_group,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let conflict_domain = match task_update_semantics_arg(
                command.conflict_domain.as_deref(),
                command.clear_conflict_domain,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parent_id =
                match task_update_parent_arg(command.parent_id.as_deref(), command.clear_parent_id)
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .update_task(state_store::UpdateTaskRequest {
                        task_id: &command.task_id,
                        status: command.status.as_deref(),
                        notes: notes.as_deref(),
                        description: command.description.as_deref(),
                        parent_id,
                        add_labels: &add_labels,
                        remove_labels: &remove_labels,
                        set_labels: set_labels.as_deref(),
                        execution_mode,
                        order_bucket,
                        parallel_group,
                        conflict_domain,
                    })
                    .await
                {
                    Ok(task) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task update").await
                        {
                            return code;
                        }
                        print_task_mutation(
                            command.render,
                            "vida task update",
                            &task,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to update task: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Close(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let project_root = project_root_for_task_state(&state_dir);
            let feedback_source = command.source.as_deref().unwrap_or("vida task close");
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.close_task(&command.task_id, &command.reason).await {
                    Ok(task) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task close").await
                        {
                            return code;
                        }
                        if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_implementer_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                        let task_value = serde_json::to_value(&task)
                            .expect("task close payload should serialize");
                        let telemetry = match project_root.as_deref() {
                            Some(project_root) => {
                                crate::agent_feedback_surface::maybe_record_task_close_host_agent_feedback(
                                    project_root,
                                    &task_value,
                                    &command.reason,
                                    feedback_source,
                                )
                            }
                            None => serde_json::json!({
                                "status": "skipped",
                                "reason": "project_root_unavailable",
                            }),
                        };
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": "pass",
                                "task": task,
                                "host_agent_telemetry": telemetry,
                            }));
                        } else {
                            print_task_mutation(command.render, "vida task close", &task, false);
                            let telemetry_status = telemetry
                                .get("status")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown");
                            let telemetry_reason = telemetry
                                .get("reason")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("");
                            let telemetry_summary = if telemetry_reason.is_empty() {
                                telemetry_status.to_string()
                            } else {
                                format!("{telemetry_status}: {telemetry_reason}")
                            };
                            print_surface_line(
                                command.render,
                                "host agent telemetry",
                                &telemetry_summary,
                            );
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to close task: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Deps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.task_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReverseDeps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.reverse_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task reverse-deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read reverse dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Blocked(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.blocked_tasks().await {
                    Ok(tasks) => {
                        print_blocked_tasks(command.render, &tasks, command.summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute blocked tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Children(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_direct_children(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to read task direct children: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_dependency_tree(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to read task dependency tree: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReparentChildren(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .reparent_children(
                        &command.from_parent_id,
                        &command.to_parent_id,
                        &command.child_ids,
                        command.dry_run,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Err(code) = refresh_task_snapshot_after_mutation(
                            &store,
                            "vida task reparent-children",
                        )
                        .await
                        {
                            return code;
                        }
                        print_task_bulk_reparent_result(command.render, &result, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to bulk-reparent children: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ValidateGraph(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.validate_task_graph().await {
                    Ok(issues) => {
                        print_task_graph_issues(command.render, &issues, command.json);
                        if issues.is_empty() {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to validate task graph: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Dep(command) => match command.command {
            TaskDependencyCommand::Add(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependency(
                            &add.task_id,
                            &add.depends_on_id,
                            &add.edge_type,
                            &add.created_by,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep add")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                add.render,
                                "vida task dep add",
                                &dependency,
                                add.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Remove(remove) => {
                let state_dir = remove
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .remove_task_dependency(
                            &remove.task_id,
                            &remove.depends_on_id,
                            &remove.edge_type,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep remove")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                remove.render,
                                "vida task dep remove",
                                &dependency,
                                remove.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to remove task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::CriticalPath(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.critical_path().await {
                    Ok(path) => {
                        print_task_critical_path(command.render, &path, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute critical path: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_json_string_array_entries, normalize_task_json_contract_arrays,
        parse_label_values, parse_optional_label_value, task_json_success_status,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::cli;
    use crate::test_cli_support::guard_current_dir;
    use std::fs;

    #[test]
    fn task_json_success_status_defaults_to_release_contract_vocabulary() {
        assert_eq!(task_json_success_status(), "pass");
    }

    #[test]
    fn normalize_task_json_contract_arrays_fail_closed_for_whitespace_only_entries() {
        let mut summary_json = serde_json::json!({
            "status": task_json_success_status(),
            "blocker_codes": ["   "],
            "next_actions": ["Run `vida task import-jsonl --json`"],
        });

        assert!(normalize_task_json_contract_arrays(&mut summary_json).is_err());
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["pending"])),
            Some(vec!["pending".to_string()])
        );
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }

    #[test]
    fn parse_label_values_accepts_repeated_and_comma_separated_forms() {
        let labels = parse_label_values(&[
            "alpha,beta".to_string(),
            " gamma ".to_string(),
            "delta, ,epsilon".to_string(),
        ]);
        assert_eq!(labels, vec!["alpha", "beta", "gamma", "delta", "epsilon"]);
    }

    #[test]
    fn parse_optional_label_value_returns_none_for_absent_input() {
        assert_eq!(parse_optional_label_value(None), None);
        assert_eq!(
            parse_optional_label_value(Some("alpha, beta")),
            Some(vec!["alpha".to_string(), "beta".to_string()])
        );
    }

    #[test]
    #[ignore = "covered by binary integration smoke; in-process sequential SurrealKv opens keep the lock longer than this unit test assumes"]
    fn task_command_round_trip_succeeds() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let jsonl_path = harness.path().join("issues.jsonl");
        fs::write(
            &jsonl_path,
            concat!(
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write sample task jsonl");

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "import-jsonl",
                    jsonl_path.to_str().expect("jsonl path should render"),
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "list",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "ready",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );
    }
}
