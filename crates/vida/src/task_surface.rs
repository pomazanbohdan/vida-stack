use super::*;

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
                    labels: &command.labels,
                    created_by: "vida task",
                    source_repo: &source_repo,
                })
                .await
            {
                Ok(task) => {
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
            Some("next") => {
                print_taskflow_proxy_help(Some("next"));
                ExitCode::SUCCESS
            }
            Some("graph-summary") => {
                print_taskflow_proxy_help(Some("graph-summary"));
                ExitCode::SUCCESS
            }
            Some(
                "ready" | "deps" | "reverse-deps" | "blocked" | "tree" | "critical-path"
                | "next-display-id" | "create" | "ensure" | "update" | "close" | "list" | "show"
                | "import-jsonl" | "replace-jsonl" | "export-jsonl" | "validate-graph" | "dep",
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
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .list_tasks(command.status.as_deref(), command.all)
                    .await
                {
                    Ok(tasks) => {
                        print_task_list(command.render, &tasks, command.summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to list tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.show_task(&command.task_id).await {
                    Ok(task) => {
                        print_task_show(command.render, &task, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        if !command.task_id.starts_with("vida-") {
                            eprintln!("Failed to show task: {error}");
                            return ExitCode::from(1);
                        }
                        match store.list_tasks(None, true).await {
                            Ok(tasks) => match task_rows_as_values(&tasks) {
                                Ok(rows) => {
                                    let resolved =
                                        crate::taskflow_task_bridge::resolve_task_id_by_display_id(
                                            &rows,
                                            &command.task_id,
                                        );
                                    if !resolved
                                        .get("found")
                                        .and_then(serde_json::Value::as_bool)
                                        .unwrap_or(false)
                                    {
                                        eprintln!("Failed to show task: {error}");
                                        return ExitCode::from(1);
                                    }
                                    let resolved_id = resolved
                                        .get("task_id")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_string();
                                    match store.show_task(&resolved_id).await {
                                        Ok(task) => {
                                            print_task_show(command.render, &task, command.json);
                                            ExitCode::SUCCESS
                                        }
                                        Err(resolved_error) => {
                                            eprintln!("Failed to show task: {resolved_error}");
                                            ExitCode::from(1)
                                        }
                                    }
                                }
                                Err(render_error) => {
                                    eprintln!("Failed to show task: {render_error}");
                                    ExitCode::from(1)
                                }
                            },
                            Err(list_error) => {
                                eprintln!("Failed to show task: {list_error}");
                                ExitCode::from(1)
                            }
                        }
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
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.ready_tasks_scoped(command.scope.as_deref()).await {
                    Ok(tasks) => {
                        print_task_ready(
                            command.render,
                            command.scope.as_deref(),
                            &tasks,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute ready tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
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
            let set_labels = command.set_labels.as_ref().map(|labels| {
                labels
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            });
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .update_task(state_store::UpdateTaskRequest {
                        task_id: &command.task_id,
                        status: command.status.as_deref(),
                        notes: notes.as_deref(),
                        description: command.description.as_deref(),
                        add_labels: &command.add_labels,
                        remove_labels: &command.remove_labels,
                        set_labels: set_labels.as_deref(),
                    })
                    .await
                {
                    Ok(task) => {
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
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.task_dependency_tree(&command.task_id).await {
                    Ok(tree) => {
                        print_task_dependency_tree(command.render, &tree, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependency tree: {error}");
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
        task_json_success_status,
    };

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
}
