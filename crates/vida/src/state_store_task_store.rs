use super::*;
use serde_json::Deserializer;

impl StateStore {
    pub async fn import_tasks_from_jsonl(
        &self,
        source_path: &Path,
    ) -> Result<TaskImportSummary, StateStoreError> {
        let raw = fs::read_to_string(source_path)?;
        let mut imported = 0usize;
        let mut unchanged = 0usize;
        let mut updated = 0usize;

        for (index, record) in Deserializer::from_str(&raw)
            .into_iter::<TaskJsonlRecord>()
            .enumerate()
        {
            let record = record.map_err(|error| StateStoreError::InvalidTaskJsonLine {
                line: index + 1,
                reason: error.to_string(),
            })?;
            let task_id = record.id.trim().to_string();
            if task_id.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!("line {} is missing task id", index + 1),
                });
            }

            let content = TaskContent::from(record);
            let existing: Option<TaskStorageRow> =
                self.db.select(("task", task_id.as_str())).await?;
            match existing {
                None => imported += 1,
                Some(current) if current == TaskStorageRow::from(content.clone()) => unchanged += 1,
                Some(_) => updated += 1,
            }

            let _: Option<TaskStorageRow> = self
                .db
                .upsert(("task", task_id.as_str()))
                .content(content.clone())
                .await?;

            let _ = self
                .db
                .query(format!(
                    "DELETE task_dependency WHERE issue_id = '{}';",
                    escape_surql_literal(&task_id)
                ))
                .await?;

            for dependency in &content.dependencies {
                let dep_id = format!(
                    "{}--{}--{}",
                    sanitize_record_id(&task_id),
                    sanitize_record_id(&dependency.depends_on_id),
                    sanitize_record_id(&dependency.edge_type)
                );
                let _: Option<TaskDependencyRecord> = self
                    .db
                    .upsert(("task_dependency", dep_id.as_str()))
                    .content(dependency.clone())
                    .await?;
            }
        }

        Ok(TaskImportSummary {
            source_path: source_path.display().to_string(),
            imported_count: imported,
            unchanged_count: unchanged,
            updated_count: updated,
        })
    }

    pub async fn export_tasks_to_jsonl(
        &self,
        target_path: &Path,
    ) -> Result<usize, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let mut body = String::new();
        for task in tasks {
            body.push_str(&serde_json::to_string(&task).map_err(|error| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!("failed to serialize task export row: {error}"),
                }
            })?);
            body.push('\n');
        }
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, body)?;
        Ok(self.all_tasks().await?.len())
    }

    pub async fn list_tasks(
        &self,
        status: Option<&str>,
        include_closed: bool,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut rows = self.all_tasks().await?;
        rows.retain(|task| {
            if !include_closed && task.status == "closed" {
                return false;
            }
            match status {
                Some(expected) => task.status == expected,
                None => true,
            }
        });
        rows.sort_by(task_sort_key);
        Ok(rows)
    }

    pub async fn show_task(&self, task_id: &str) -> Result<TaskRecord, StateStoreError> {
        let row: Option<TaskStorageRow> = self.db.select(("task", task_id)).await?;
        row.map(TaskRecord::from)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })
    }

    pub async fn ready_tasks(&self) -> Result<Vec<TaskRecord>, StateStoreError> {
        self.ready_tasks_scoped(None).await
    }

    pub async fn task_dependencies(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        let task = self.show_task(task_id).await?;
        let by_id = self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut dependencies = task
            .dependencies
            .into_iter()
            .map(|dependency| {
                let depends_on_id = dependency.depends_on_id.clone();
                let dependency_status = by_id
                    .get(&depends_on_id)
                    .map(|task| task.status.clone())
                    .unwrap_or_else(|| "missing".to_string());
                TaskDependencyStatus {
                    issue_id: dependency.issue_id,
                    depends_on_id,
                    edge_type: dependency.edge_type,
                    dependency_status,
                    dependency_issue_type: by_id
                        .get(&dependency.depends_on_id)
                        .map(|task| task.issue_type.clone()),
                }
            })
            .collect::<Vec<_>>();

        dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        Ok(dependencies)
    }

    pub async fn reverse_dependencies(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        let _ = self.show_task(task_id).await?;
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut reverse = tasks
            .into_iter()
            .flat_map(|task| {
                let issue_id = task.id.clone();
                let issue_status = task.status.clone();
                let issue_type = task.issue_type.clone();
                task.dependencies
                    .into_iter()
                    .filter(move |dependency| dependency.depends_on_id == task_id)
                    .map(move |dependency| TaskDependencyStatus {
                        issue_id: issue_id.clone(),
                        depends_on_id: dependency.depends_on_id,
                        edge_type: dependency.edge_type,
                        dependency_status: issue_status.clone(),
                        dependency_issue_type: Some(issue_type.clone()),
                    })
            })
            .collect::<Vec<_>>();

        reverse.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.issue_id.cmp(&right.issue_id))
        });

        for item in &mut reverse {
            item.dependency_issue_type = by_id
                .get(&item.issue_id)
                .map(|task| task.issue_type.clone());
            item.dependency_status = by_id
                .get(&item.issue_id)
                .map(|task| task.status.clone())
                .unwrap_or_else(|| "missing".to_string());
        }

        Ok(reverse)
    }

    pub async fn blocked_tasks(&self) -> Result<Vec<BlockedTaskRecord>, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut blocked = tasks
            .into_iter()
            .filter(|task| task.status == "open" || task.status == "in_progress")
            .filter(|task| task.issue_type != "epic")
            .filter_map(|task| {
                let blockers = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .filter_map(|dependency| {
                        let blocker_task = by_id.get(&dependency.depends_on_id)?;
                        if blocker_task.status == "closed" {
                            return None;
                        }
                        Some(TaskDependencyStatus {
                            issue_id: dependency.issue_id.clone(),
                            depends_on_id: dependency.depends_on_id.clone(),
                            edge_type: dependency.edge_type.clone(),
                            dependency_status: blocker_task.status.clone(),
                            dependency_issue_type: Some(blocker_task.issue_type.clone()),
                        })
                    })
                    .collect::<Vec<_>>();

                (!blockers.is_empty()).then_some(BlockedTaskRecord { task, blockers })
            })
            .collect::<Vec<_>>();

        blocked.sort_by(|left, right| task_ready_sort_key(&left.task, &right.task));
        Ok(blocked)
    }

    pub async fn add_task_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
        created_by: &str,
    ) -> Result<TaskDependencyRecord, StateStoreError> {
        let mut tasks = self.all_tasks().await?;
        let target_exists = tasks.iter().any(|task| task.id == depends_on_id);
        if !target_exists {
            return Err(StateStoreError::MissingTask {
                task_id: depends_on_id.to_string(),
            });
        }

        let task_index = tasks
            .iter()
            .position(|task| task.id == issue_id)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: issue_id.to_string(),
            })?;

        if tasks[task_index].dependencies.iter().any(|dependency| {
            dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency already exists: {} -> {} ({})",
                    issue_id, depends_on_id, edge_type
                ),
            });
        }

        let dependency = TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: edge_type.to_string(),
            created_at: unix_timestamp_nanos().to_string(),
            created_by: created_by.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        };
        tasks[task_index].dependencies.push(dependency.clone());
        tasks[task_index].updated_at = unix_timestamp_nanos().to_string();
        tasks[task_index].dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });

        let issues = Self::validate_task_graph_rows(&tasks);
        if !issues.is_empty() {
            let first = issues.first().expect("issues is not empty");
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency mutation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        self.persist_task_record(tasks[task_index].clone()).await?;
        Ok(dependency)
    }

    pub async fn remove_task_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
    ) -> Result<TaskDependencyRecord, StateStoreError> {
        let task = self.show_task(issue_id).await?;
        let removed = task
            .dependencies
            .iter()
            .find(|dependency| {
                dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type
            })
            .cloned()
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency does not exist: {} -> {} ({})",
                    issue_id, depends_on_id, edge_type
                ),
            })?;

        let mut updated = task;
        updated.dependencies.retain(|dependency| {
            !(dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type)
        });
        updated.updated_at = unix_timestamp_nanos().to_string();

        self.persist_task_record(updated).await?;
        Ok(removed)
    }

    pub async fn create_task(
        &self,
        request: CreateTaskRequest<'_>,
    ) -> Result<TaskRecord, StateStoreError> {
        let CreateTaskRequest {
            task_id,
            title,
            display_id,
            description,
            issue_type,
            status,
            priority,
            parent_id,
            labels,
            created_by,
            source_repo,
        } = request;

        let task_id = task_id.trim();
        let title = title.trim();
        if task_id.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "task id is empty".to_string(),
            });
        }
        if title.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task `{task_id}` title is empty"),
            });
        }
        if self.show_task(task_id).await.is_ok() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task already exists: {task_id}"),
            });
        }
        let normalized_parent_id = parent_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_display_id = display_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        if let Some(parent_id) = normalized_parent_id.as_deref() {
            if self.show_task(parent_id).await.is_err() {
                return Err(StateStoreError::MissingTask {
                    task_id: parent_id.to_string(),
                });
            }
        }

        let now = unix_timestamp_nanos().to_string();
        let mut normalized_labels = labels
            .iter()
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        normalized_labels.sort();
        normalized_labels.dedup();

        let mut dependencies = Vec::new();
        if let Some(parent_id) = normalized_parent_id.clone() {
            dependencies.push(TaskDependencyRecord {
                issue_id: task_id.to_string(),
                depends_on_id: parent_id.to_string(),
                edge_type: "parent-child".to_string(),
                created_at: now.clone(),
                created_by: created_by.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        }

        let mut task = TaskRecord {
            id: task_id.to_string(),
            display_id: normalized_display_id,
            title: title.to_string(),
            description: description.to_string(),
            status: status.to_string(),
            priority,
            issue_type: issue_type.to_string(),
            created_at: now.clone(),
            created_by: created_by.to_string(),
            updated_at: now.clone(),
            closed_at: None,
            close_reason: None,
            source_repo: source_repo.to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: normalized_labels,
            dependencies,
        };
        if status == "closed" {
            task.closed_at = Some(now);
        }

        let mut tasks = self.all_tasks().await?;
        tasks.push(task.clone());
        let issues = Self::validate_task_graph_rows(&tasks);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task creation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn update_task(
        &self,
        request: UpdateTaskRequest<'_>,
    ) -> Result<TaskRecord, StateStoreError> {
        let UpdateTaskRequest {
            task_id,
            status,
            notes,
            description,
            add_labels,
            remove_labels,
            set_labels,
        } = request;
        let mut task = self.show_task(task_id).await?;
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            task.status = status.to_string();
            if status == "closed" {
                if task.closed_at.is_none() {
                    task.closed_at = Some(unix_timestamp_nanos().to_string());
                }
            } else {
                task.closed_at = None;
                task.close_reason = None;
            }
        }
        if let Some(notes) = notes {
            task.notes = Some(notes.to_string());
        }
        if let Some(description) = description {
            task.description = description.to_string();
        }
        if let Some(set_labels) = set_labels {
            task.labels = set_labels
                .iter()
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty())
                .collect::<Vec<_>>();
        }
        for label in add_labels {
            let label = label.trim();
            if label.is_empty() || task.labels.iter().any(|existing| existing == label) {
                continue;
            }
            task.labels.push(label.to_string());
        }
        if !remove_labels.is_empty() {
            task.labels
                .retain(|label| !remove_labels.iter().any(|remove| remove == label));
        }
        task.labels.sort();
        task.labels.dedup();
        task.updated_at = unix_timestamp_nanos().to_string();
        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn close_task(
        &self,
        task_id: &str,
        reason: &str,
    ) -> Result<TaskRecord, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let open_children = tasks
            .iter()
            .filter(|task| {
                task.id != task_id
                    && task.status != "closed"
                    && task.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == task_id
                    })
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        if !open_children.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "cannot close task `{task_id}` while open child tasks exist: {}",
                    open_children.join(", ")
                ),
            });
        }

        let mut task = self.show_task(task_id).await?;
        let now = unix_timestamp_nanos().to_string();
        task.status = "closed".to_string();
        task.updated_at = now.clone();
        task.closed_at = Some(now);
        task.close_reason = Some(reason.to_string());
        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub(crate) async fn persist_task_record(
        &self,
        task: TaskRecord,
    ) -> Result<(), StateStoreError> {
        let task_id = task.id.clone();
        let row = TaskStorageRow::from(task.clone());
        let _: Option<TaskStorageRow> = self
            .db
            .upsert(("task", task_id.as_str()))
            .content(row)
            .await?;
        self.replace_task_dependency_rows(&task_id, &task.dependencies)
            .await?;
        Ok(())
    }

    async fn replace_task_dependency_rows(
        &self,
        task_id: &str,
        dependencies: &[TaskDependencyRecord],
    ) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query(format!(
                "DELETE task_dependency WHERE issue_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;

        for dependency in dependencies {
            let dep_id = format!(
                "{}--{}--{}",
                sanitize_record_id(task_id),
                sanitize_record_id(&dependency.depends_on_id),
                sanitize_record_id(&dependency.edge_type)
            );
            let _: Option<TaskDependencyRecord> = self
                .db
                .upsert(("task_dependency", dep_id.as_str()))
                .content(dependency.clone())
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn delete_task_record(&self, task_id: &str) -> Result<(), StateStoreError> {
        let _: Option<TaskStorageRow> = self.db.delete(("task", task_id)).await?;
        let _ = self
            .db
            .query(format!(
                "DELETE task_dependency WHERE issue_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;
        Ok(())
    }

    pub(crate) async fn all_tasks(&self) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut query = self
            .db
            .query("SELECT * FROM task ORDER BY priority ASC, id ASC;")
            .await?;
        let rows: Vec<TaskStorageRow> = query.take(0)?;
        Ok(rows.into_iter().map(TaskRecord::from).collect())
    }
}
