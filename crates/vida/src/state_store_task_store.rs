use super::*;
use serde_json::Deserializer;

impl StateStore {
    pub(crate) fn canonical_task_snapshot_path_for_state_root(state_root: &Path) -> PathBuf {
        if let Some(project_root) =
            crate::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        {
            return project_root.join(".vida/exports/tasks.snapshot.jsonl");
        }

        if state_root.file_name().and_then(|value| value.to_str()) == Some("state") {
            if let Some(data_dir) = state_root.parent() {
                if let Some(vida_dir) = data_dir.parent() {
                    return vida_dir.join("exports/tasks.snapshot.jsonl");
                }
            }
        }

        state_root.join("exports/tasks.snapshot.jsonl")
    }

    pub(crate) fn read_tasks_from_jsonl_snapshot(
        source_path: &Path,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let raw = fs::read_to_string(source_path)?;
        let mut rows = Vec::new();

        for (index, record) in Deserializer::from_str(&raw)
            .into_iter::<TaskJsonlRecord>()
            .enumerate()
        {
            let record = record.map_err(|error| StateStoreError::InvalidTaskJsonLine {
                line: index + 1,
                reason: error.to_string(),
            })?;
            let content = TaskContent::from(record);
            rows.push(TaskRecord::from(TaskStorageRow::from(content)));
        }

        Ok(rows)
    }

    pub async fn refresh_task_snapshot(&self) -> Result<PathBuf, StateStoreError> {
        let snapshot_path = Self::canonical_task_snapshot_path_for_state_root(self.root());
        self.export_tasks_to_jsonl(&snapshot_path).await?;
        Ok(snapshot_path)
    }

    async fn build_task_close_reconciled_binding(
        &self,
        status: &RunGraphStatus,
        closed_task_id: &str,
    ) -> Result<Option<crate::state_store::RunGraphContinuationBinding>, StateStoreError> {
        if status.task_id == closed_task_id
            && self
                .task_close_reconcile_has_persisted_receipt_truth(&status.run_id, closed_task_id)
                .await?
        {
            return Ok(Some(crate::state_store::RunGraphContinuationBinding {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "downstream_dispatch_target",
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "dispatch_target": "closure",
                }),
                binding_source: "task_close_reconcile".to_string(),
                why_this_unit: "Closing the active task reconciled the run into a completed state and bound downstream closure as the next lawful bounded unit.".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only".to_string(),
                request_text: None,
                recorded_at: time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .expect("rfc3339 timestamp should render"),
            }));
        }

        Ok(None)
    }

    pub(crate) fn run_graph_status_allows_task_close_closure_binding(
        status: &RunGraphStatus,
    ) -> bool {
        matches!(
            status.status.as_str(),
            "ready" | "in_progress" | "completed"
        ) && !matches!(
            status.lifecycle_stage.as_str(),
            "analysis_blocked"
                | "implementation_blocked"
                | "verification_blocked"
                | "closure_blocked"
        ) && status.next_node.is_none()
            && status.handoff_state == "none"
            && status.resume_target == "none"
    }

    pub(crate) async fn task_close_reconcile_has_persisted_receipt_truth(
        &self,
        run_id: &str,
        task_id: &str,
    ) -> Result<bool, StateStoreError> {
        let task = match self.show_task(task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        if task.status != "closed" {
            return Ok(false);
        }

        let receipt: Option<RunGraphDispatchReceiptStored> = self
            .db
            .select(("run_graph_dispatch_receipt", run_id))
            .await?;
        let Some(receipt) = receipt.map(
            crate::state_store::state_store_run_graph_summary::normalize_legacy_downstream_preview_drift,
        ) else {
            return Ok(false);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        let receipt: RunGraphDispatchReceipt = receipt.into();
        Ok(receipt.run_id == run_id
            && receipt.blocker_code.is_none()
            && receipt
                .dispatch_packet_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            && crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(&receipt))
    }

    fn normalize_execution_semantics_value(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    fn validate_execution_mode(
        task_id: &str,
        value: Option<&str>,
    ) -> Result<Option<String>, StateStoreError> {
        let normalized = Self::normalize_execution_semantics_value(value);
        let Some(mode) = normalized else {
            return Ok(None);
        };
        match mode.as_str() {
            "sequential" | "parallel_safe" | "exclusive" => Ok(Some(mode)),
            _ => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task `{task_id}` execution_mode must be one of sequential, parallel_safe, exclusive"
                ),
            }),
        }
    }

    fn validate_execution_semantics(
        task_id: &str,
        semantics: TaskExecutionSemantics,
    ) -> Result<TaskExecutionSemantics, StateStoreError> {
        let normalized = TaskExecutionSemantics {
            execution_mode: Self::validate_execution_mode(
                task_id,
                semantics.execution_mode.as_deref(),
            )?,
            order_bucket: Self::normalize_execution_semantics_value(
                semantics.order_bucket.as_deref(),
            ),
            parallel_group: Self::normalize_execution_semantics_value(
                semantics.parallel_group.as_deref(),
            ),
            conflict_domain: Self::normalize_execution_semantics_value(
                semantics.conflict_domain.as_deref(),
            ),
        };
        Ok(normalized)
    }

    async fn materialize_task_close_closure_artifacts(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        let Some(mut receipt) = self.run_graph_dispatch_receipt(&status.run_id).await? else {
            return Ok(false);
        };
        let Some(dispatch_packet_path) = receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(false);
        };
        let completion_receipt_id = format!("task-close-{}", status.task_id);
        let completion_result_path =
            crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                self.root(),
                &status.run_id,
                "closure",
                &completion_receipt_id,
                dispatch_packet_path,
            )
            .map_err(|reason| StateStoreError::InvalidTaskRecord { reason })?;
        receipt.downstream_dispatch_target = Some("closure".to_string());
        receipt.downstream_dispatch_command = Some(format!(
            "vida taskflow consume continue --run-id {} --json",
            status.run_id
        ));
        receipt.downstream_dispatch_note =
            Some("task close reconciled the run into lawful closure".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_result_path = Some(completion_result_path);
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_active_target = Some("closure".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.lane_status = "lane_completed".to_string();
        self.record_run_graph_dispatch_receipt(&receipt).await?;
        Ok(true)
    }

    async fn refresh_run_graph_continuation_after_task_close(
        &self,
        task_id: &str,
    ) -> Result<(), StateStoreError> {
        #[derive(serde::Deserialize, SurrealValue)]
        struct RunIdRow {
            run_id: String,
        }

        let mut affected_run_ids = std::collections::BTreeSet::new();

        let mut query = self
            .db
            .query(format!(
                "SELECT run_id FROM execution_plan_state WHERE task_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;
        let rows: Vec<RunIdRow> = query.take(0)?;
        for row in rows {
            affected_run_ids.insert(row.run_id);
        }

        let mut explicit_binding_query = self
            .db
            .query(format!(
                "SELECT run_id FROM run_graph_continuation_binding \
                 WHERE active_bounded_unit.kind = 'task_graph_task' \
                 AND active_bounded_unit.task_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;
        let explicit_binding_rows: Vec<RunIdRow> = explicit_binding_query.take(0)?;
        for row in explicit_binding_rows {
            affected_run_ids.insert(row.run_id);
        }

        for run_id in affected_run_ids {
            let status = self.run_graph_status(&run_id).await?;
            if status.task_id == task_id {
                self.record_run_graph_status(&status).await?;
            }
            let Some(binding) = self
                .build_task_close_reconciled_binding(&status, task_id)
                .await?
            else {
                self.clear_run_graph_continuation_binding(&run_id).await?;
                continue;
            };
            let closure_bound = binding.active_bounded_unit["kind"] == "downstream_dispatch_target"
                && binding.active_bounded_unit["dispatch_target"] == "closure";
            if closure_bound
                && !self
                    .materialize_task_close_closure_artifacts(&status)
                    .await?
            {
                continue;
            }
            self.record_run_graph_continuation_binding(&binding).await?;
        }
        Ok(())
    }

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
            execution_semantics,
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
        let execution_semantics = Self::validate_execution_semantics(task_id, execution_semantics)?;

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
            execution_semantics,
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
            parent_id,
            add_labels,
            remove_labels,
            set_labels,
            execution_mode,
            order_bucket,
            parallel_group,
            conflict_domain,
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
        if let Some(parent_id) = parent_id {
            let normalized_parent_id = parent_id.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });
            if let Some(parent_id) = normalized_parent_id.as_deref() {
                if parent_id == task_id {
                    return Err(StateStoreError::InvalidTaskRecord {
                        reason: format!("task `{task_id}` cannot be its own parent"),
                    });
                }
                if self.show_task(parent_id).await.is_err() {
                    return Err(StateStoreError::MissingTask {
                        task_id: parent_id.to_string(),
                    });
                }
            }
            let created_at = task
                .dependencies
                .iter()
                .find(|dependency| dependency.edge_type == "parent-child")
                .map(|dependency| dependency.created_at.clone())
                .unwrap_or_else(|| unix_timestamp_nanos().to_string());
            let created_by = task
                .dependencies
                .iter()
                .find(|dependency| dependency.edge_type == "parent-child")
                .map(|dependency| dependency.created_by.clone())
                .unwrap_or_else(|| "vida task update".to_string());
            task.dependencies
                .retain(|dependency| dependency.edge_type != "parent-child");
            if let Some(parent_id) = normalized_parent_id {
                task.dependencies.push(TaskDependencyRecord {
                    issue_id: task_id.to_string(),
                    depends_on_id: parent_id,
                    edge_type: "parent-child".to_string(),
                    created_at,
                    created_by,
                    metadata: "{}".to_string(),
                    thread_id: String::new(),
                });
            }
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
        if let Some(execution_mode) = execution_mode {
            task.execution_semantics.execution_mode =
                Self::validate_execution_mode(task_id, execution_mode)?;
        }
        if let Some(order_bucket) = order_bucket {
            task.execution_semantics.order_bucket =
                Self::normalize_execution_semantics_value(order_bucket);
        }
        if let Some(parallel_group) = parallel_group {
            task.execution_semantics.parallel_group =
                Self::normalize_execution_semantics_value(parallel_group);
        }
        if let Some(conflict_domain) = conflict_domain {
            task.execution_semantics.conflict_domain =
                Self::normalize_execution_semantics_value(conflict_domain);
        }
        task.execution_semantics =
            Self::validate_execution_semantics(task_id, task.execution_semantics.clone())?;
        task.labels.sort();
        task.labels.dedup();
        task.updated_at = unix_timestamp_nanos().to_string();
        let mut tasks = self.all_tasks().await?;
        let task_index = tasks
            .iter()
            .position(|existing| existing.id == task.id)
            .expect("updated task should exist in authoritative state");
        tasks[task_index] = task.clone();
        let issues = Self::validate_task_graph_rows(&tasks);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task update would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }
        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn reparent_children(
        &self,
        from_parent_id: &str,
        to_parent_id: &str,
        child_ids: &[String],
        dry_run: bool,
    ) -> Result<TaskBulkReparentResult, StateStoreError> {
        self.show_task(from_parent_id).await?;
        self.show_task(to_parent_id).await?;
        if from_parent_id == to_parent_id {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "from_parent_id and to_parent_id must differ".to_string(),
            });
        }

        let mut tasks = self.all_tasks().await?;
        let direct_child_ids = tasks
            .iter()
            .filter(|task| {
                task.dependencies.iter().any(|dependency| {
                    dependency.edge_type == "parent-child"
                        && dependency.depends_on_id == from_parent_id
                })
            })
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();

        let requested_child_ids = if child_ids.is_empty() {
            direct_child_ids.iter().cloned().collect::<Vec<_>>()
        } else {
            let selected = child_ids
                .iter()
                .map(|child_id| child_id.trim())
                .filter(|child_id| !child_id.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>();
            let invalid = selected
                .iter()
                .filter(|child_id| !direct_child_ids.contains(*child_id))
                .cloned()
                .collect::<Vec<_>>();
            if !invalid.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "requested child ids are not direct children of `{from_parent_id}`: {}",
                        invalid.join(", ")
                    ),
                });
            }
            selected.iter().cloned().collect::<Vec<_>>()
        };

        let moved_set = requested_child_ids.iter().cloned().collect::<BTreeSet<_>>();
        let now = unix_timestamp_nanos().to_string();
        let mut moved_tasks = Vec::new();

        for task in &mut tasks {
            if !moved_set.contains(&task.id) {
                continue;
            }
            let created_at = task
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.edge_type == "parent-child"
                        && dependency.depends_on_id == from_parent_id
                })
                .map(|dependency| dependency.created_at.clone())
                .unwrap_or_else(|| now.clone());
            let created_by = task
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.edge_type == "parent-child"
                        && dependency.depends_on_id == from_parent_id
                })
                .map(|dependency| dependency.created_by.clone())
                .unwrap_or_else(|| "vida task reparent-children".to_string());
            task.dependencies
                .retain(|dependency| dependency.edge_type != "parent-child");
            task.dependencies.push(TaskDependencyRecord {
                issue_id: task.id.clone(),
                depends_on_id: to_parent_id.to_string(),
                edge_type: "parent-child".to_string(),
                created_at,
                created_by,
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
            task.dependencies.sort_by(|left, right| {
                left.edge_type
                    .cmp(&right.edge_type)
                    .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
            });
            task.updated_at = now.clone();
            moved_tasks.push(task.clone());
        }

        let issues = Self::validate_task_graph_rows(&tasks);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "bulk reparent would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        if !dry_run {
            for task in &moved_tasks {
                self.persist_task_record(task.clone()).await?;
            }
        }

        Ok(TaskBulkReparentResult {
            from_parent_id: from_parent_id.to_string(),
            to_parent_id: to_parent_id.to_string(),
            requested_child_ids: requested_child_ids.clone(),
            moved_child_ids: requested_child_ids,
            moved_count: moved_tasks.len(),
            dry_run,
            tasks: moved_tasks,
        })
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
        self.refresh_run_graph_continuation_after_task_close(task_id)
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn close_task_refreshes_run_graph_continuation_binding_to_closure() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-close-continuation-refresh-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "feature-close-dev",
                title: "Implement bounded fix",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-close-task",
            "implementation",
            "implementation",
        );
        status.task_id = "feature-close-dev".to_string();
        status.active_node = "implementer".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-close-task".to_string(),
                task_id: "feature-close-dev".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "run_graph_task",
                    "task_id": "feature-close-dev",
                    "run_id": "run-close-task",
                    "active_node": "implementer"
                }),
                binding_source: "test".to_string(),
                why_this_unit: "pre-close task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                request_text: None,
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist initial continuation binding");
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let implementer_packet_path = packet_dir.join("run-close-task-implementer.json");
        fs::write(
            &implementer_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": "run-close-task",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["implementation"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": "run-close-task" },
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": "run-close-task::implementer::delivery",
                    "goal": "Implement bounded fix",
                    "scope_in": ["dispatch_target:implementer"],
                    "owned_paths": ["crates/vida/src/state_store_task_store.rs"],
                    "definition_of_done": ["record bounded implementation result"],
                    "verification_command": "cargo test -p vida --bin vida 'state_store::state_store_task_store::tests::close_task_refreshes_run_graph_continuation_binding_to_closure' -- --exact --nocapture --test-threads=1",
                    "proof_target": "closure reconcile proof",
                    "stop_rules": ["stop after bounded result"],
                    "blocking_question": "What remains to complete the bounded fix?"
                }
            }))
            .expect("encode implementer packet"),
        )
        .expect("write implementer packet");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-close-task".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --dispatch-packet /tmp/implementer.json --execute-dispatch --json".to_string()),
                dispatch_packet_path: Some(implementer_packet_path.display().to_string()),
                dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("coach".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale coach handoff".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist dispatch receipt");

        store
            .close_task("feature-close-dev", "implemented and proven")
            .await
            .expect("close task");

        let binding = store
            .run_graph_continuation_binding("run-close-task")
            .await
            .expect("load continuation binding")
            .expect("continuation binding should exist");
        assert_eq!(binding.binding_source, "task_close_reconcile");
        assert_eq!(binding.task_id, "feature-close-dev");
        assert_eq!(
            binding.active_bounded_unit["kind"],
            "downstream_dispatch_target"
        );
        assert_eq!(binding.active_bounded_unit["dispatch_target"], "closure");
        assert_eq!(binding.sequential_vs_parallel_posture, "sequential_only");
        let receipt = store
            .run_graph_dispatch_receipt("run-close-task")
            .await
            .expect("load reconciled receipt")
            .expect("reconciled receipt should exist");
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("closure")
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("executed")
        );
        assert!(!receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_blockers.is_empty());
        assert!(receipt.downstream_dispatch_packet_path.is_none());
        assert!(receipt
            .downstream_dispatch_result_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        let resolved = crate::taskflow_consume_resume::resolve_runtime_consumption_resume_inputs(
            &store,
            Some("run-close-task"),
            None,
            None,
        )
        .await
        .expect("closure-bound run should resolve after task close reconcile");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "executed");
        let reconciled_status = store
            .run_graph_status("run-close-task")
            .await
            .expect("reconciled run status should load");
        assert_eq!(reconciled_status.checkpoint_kind, "none");
        let checkpoint_record = store
            .run_graph_projection_checkpoint_record("run-close-task")
            .await
            .expect("checkpoint record lookup should succeed");
        assert!(checkpoint_record.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_rebinds_explicit_next_task_binding_that_targets_closed_task() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-close-explicit-binding-refresh-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "run-owner-task",
                title: "Current active implementation task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create run owner task");
        store
            .create_task(CreateTaskRequest {
                task_id: "next-explicit-task",
                title: "Explicit next task target",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create explicit next task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-explicit-task-close",
            "implementation",
            "implementation",
        );
        status.task_id = "run-owner-task".to_string();
        status.active_node = "implementer".to_string();
        status.status = "in_progress".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "targeted_verification".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run-graph status");

        store
            .record_run_graph_continuation_binding(&RunGraphContinuationBinding {
                run_id: "run-explicit-task-close".to_string(),
                task_id: "next-explicit-task".to_string(),
                status: "bound".to_string(),
                active_bounded_unit: serde_json::json!({
                    "kind": "task_graph_task",
                    "task_id": "next-explicit-task",
                    "run_id": "run-explicit-task-close",
                    "task_status": "open",
                    "issue_type": "bug"
                }),
                binding_source: "explicit_continuation_bind_task".to_string(),
                why_this_unit: "test explicit next-task binding".to_string(),
                primary_path: "normal_delivery_path".to_string(),
                sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
                request_text: None,
                recorded_at: "2026-04-16T00:00:00Z".to_string(),
            })
            .await
            .expect("persist explicit continuation binding");

        store
            .close_task("next-explicit-task", "superseded by completed owner run")
            .await
            .expect("close explicit next task");

        let binding = store
            .run_graph_continuation_binding("run-explicit-task-close")
            .await
            .expect("load refreshed continuation binding")
            .expect("continuation binding should remain lawful");
        assert_eq!(binding.binding_source, "task_close_reconcile");
        assert_eq!(binding.task_id, "run-owner-task");
        assert_eq!(binding.active_bounded_unit["kind"], "run_graph_task");
        assert_eq!(binding.active_bounded_unit["task_id"], "run-owner-task");
        assert_eq!(
            binding.active_bounded_unit["run_id"],
            "run-explicit-task-close"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn show_task_defaults_execution_semantics_when_legacy_row_has_none() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-legacy-execution-semantics-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let _ = store
            .create_task(CreateTaskRequest {
                task_id: "legacy-task",
                display_id: None,
                title: "Legacy task",
                description: "",
                status: "open",
                issue_type: "task",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("legacy row should insert");
        let _ = store
            .db
            .query("UPDATE task:legacy-task SET execution_semantics = NONE;")
            .await
            .expect("legacy row should downgrade execution semantics");

        drop(store);

        let reopened = StateStore::open(root.clone())
            .await
            .expect("reopen store after legacy downgrade");
        let task = reopened
            .show_task("legacy-task")
            .await
            .expect("legacy task should load");
        assert_eq!(task.execution_semantics, TaskExecutionSemantics::default());

        let _ = fs::remove_dir_all(&root);
    }
}
