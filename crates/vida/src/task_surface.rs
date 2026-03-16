use super::*;

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
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "status": "ok",
                                    "source_path": summary.source_path,
                                    "imported_count": summary.imported_count,
                                    "unchanged_count": summary.unchanged_count,
                                    "updated_count": summary.updated_count,
                                }))
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
