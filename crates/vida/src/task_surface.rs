use super::*;

fn canonical_release1_task_status(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "pass" | "ok" => "pass",
        "blocked" => "blocked",
        _ => "blocked",
    }
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

pub(crate) async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::ImportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
                    Ok(summary) => {
                        if command.json {
                            let mut summary_json = serde_json::json!({
                                "status": canonical_release1_task_status("ok"),
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
                        print_task_list(command.render, &tasks, command.json);
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
                        eprintln!("Failed to show task: {error}");
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
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store.ready_tasks_scoped(command.scope.as_deref()).await {
                    Ok(tasks) => {
                        print_task_list(command.render, &tasks, command.json);
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
                        print_blocked_tasks(command.render, &tasks, command.json);
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
        canonical_json_string_array_entries, canonical_release1_task_status,
        normalize_task_json_contract_arrays,
    };

    #[test]
    fn canonical_release1_task_status_preserves_release1_vocabulary() {
        assert_eq!(canonical_release1_task_status("pass"), "pass");
        assert_eq!(canonical_release1_task_status("ok"), "pass");
        assert_eq!(canonical_release1_task_status("blocked"), "blocked");
    }

    #[test]
    fn canonical_release1_task_status_fails_closed_for_unknown_or_drifted_values() {
        assert_eq!(canonical_release1_task_status("block"), "blocked");
        assert_eq!(canonical_release1_task_status("unknown"), "blocked");
        assert_eq!(canonical_release1_task_status(" ok "), "pass");
    }

    #[test]
    fn canonical_release1_task_status_normalizes_case_and_whitespace_drift() {
        assert_eq!(canonical_release1_task_status(" PASS "), "pass");
        assert_eq!(canonical_release1_task_status(" BLOCKED "), "blocked");
        assert_eq!(canonical_release1_task_status(" Ok "), "pass");
    }

    #[test]
    fn normalize_task_json_contract_arrays_fail_closed_for_whitespace_only_entries() {
        let mut summary_json = serde_json::json!({
            "status": "pass",
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
