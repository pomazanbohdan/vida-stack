use super::*;

impl StateStore {
    async fn write_task_reconciliation_summary(
        &self,
        input: TaskReconciliationSummaryInput,
    ) -> Result<TaskReconciliationSummary, StateStoreError> {
        let receipt_id = format!("task-reconciliation-{}", unix_timestamp_nanos());
        let recorded_at = unix_timestamp_nanos().to_string();
        let summary = TaskReconciliationSummary {
            receipt_id: receipt_id.clone(),
            operation: input.operation,
            source_kind: input.source_kind,
            source_path: input.source_path,
            task_count: input.task_count,
            dependency_count: input.dependency_count,
            stale_removed_count: input.stale_removed_count,
            recorded_at: recorded_at.clone(),
        };
        let _: Option<TaskReconciliationSummaryRow> = self
            .db
            .upsert(("task_reconciliation_summary", receipt_id.as_str()))
            .content(TaskReconciliationSummaryRow {
                receipt_id,
                operation: summary.operation.clone(),
                source_kind: summary.source_kind.clone(),
                source_path: summary.source_path.clone(),
                task_count: summary.task_count,
                dependency_count: summary.dependency_count,
                stale_removed_count: summary.stale_removed_count,
                recorded_at,
            })
            .await?;
        Ok(summary)
    }

    async fn record_snapshot_bridge_reconciliation_summary(
        &self,
        operation: &str,
        source_kind: &str,
        source_path: Option<String>,
        task_count: usize,
        dependency_count: usize,
        stale_removed_count: usize,
    ) -> Result<TaskReconciliationSummary, StateStoreError> {
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: operation.to_string(),
            source_kind: source_kind.to_string(),
            source_path,
            task_count,
            dependency_count,
            stale_removed_count,
        })
        .await
    }

    pub async fn task_store_summary(&self) -> Result<TaskStoreSummary, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let open_count = tasks.iter().filter(|task| task.status == "open").count();
        let in_progress_count = tasks
            .iter()
            .filter(|task| task.status == "in_progress")
            .count();
        let closed_count = tasks.iter().filter(|task| task.status == "closed").count();
        let epic_count = tasks
            .iter()
            .filter(|task| task.issue_type == "epic")
            .count();
        let ready_count = self.ready_tasks().await?.len();

        Ok(TaskStoreSummary {
            total_count: tasks.len(),
            open_count,
            in_progress_count,
            closed_count,
            epic_count,
            ready_count,
        })
    }

    #[allow(dead_code)]
    async fn build_taskflow_snapshot(&self) -> Result<TaskSnapshot, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let mut snapshot_tasks = Vec::with_capacity(tasks.len());
        let mut snapshot_dependencies = Vec::new();

        for task in tasks {
            snapshot_dependencies.extend(
                task.dependencies
                    .iter()
                    .map(task_dependency_to_canonical_edge),
            );
            snapshot_tasks.push(task_record_to_canonical_snapshot_row(&task)?);
        }

        snapshot_tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        snapshot_dependencies.sort_by(|left, right| {
            left.issue_id
                .0
                .cmp(&right.issue_id.0)
                .then_with(|| left.depends_on_id.0.cmp(&right.depends_on_id.0))
                .then_with(|| left.dependency_type.cmp(&right.dependency_type))
        });

        Ok(TaskSnapshot {
            tasks: snapshot_tasks,
            dependencies: snapshot_dependencies,
        })
    }

    #[allow(dead_code)]
    pub async fn export_taskflow_snapshot(&self) -> Result<TaskSnapshot, StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        self.record_snapshot_bridge_reconciliation_summary(
            "export_snapshot",
            "canonical_snapshot_object",
            None,
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            0,
        )
        .await?;
        Ok(snapshot)
    }

    #[allow(dead_code)]
    pub async fn export_taskflow_in_memory_store(
        &self,
    ) -> Result<InMemoryTaskStore, StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        let restored = restore_canonical_in_memory_store(&snapshot);
        self.record_snapshot_bridge_reconciliation_summary(
            "export_snapshot",
            "canonical_snapshot_memory",
            None,
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            0,
        )
        .await?;
        Ok(restored)
    }

    #[allow(dead_code)]
    pub async fn write_taskflow_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        let source_path = path.as_ref().display().to_string();
        write_canonical_snapshot(path, &snapshot)?;
        self.record_snapshot_bridge_reconciliation_summary(
            "export_snapshot",
            "canonical_snapshot_file",
            Some(source_path),
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            0,
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_taskflow_snapshot_into_memory(
        path: impl AsRef<Path>,
    ) -> Result<InMemoryTaskStore, StateStoreError> {
        Ok(read_canonical_snapshot_into_memory(path)?)
    }

    #[allow(dead_code)]
    pub async fn import_taskflow_snapshot(
        &self,
        snapshot: &TaskSnapshot,
    ) -> Result<(), StateStoreError> {
        let task_records = task_records_from_canonical_snapshot_for_additive_import(
            snapshot,
            &self.all_tasks().await?,
        )?;
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        self.record_snapshot_bridge_reconciliation_summary(
            "import_snapshot",
            "canonical_snapshot_memory",
            None,
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            0,
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn import_taskflow_snapshot_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let source_path = path.as_ref().display().to_string();
        let snapshot = taskflow_state_fs::read_snapshot(path)?;
        let task_records = task_records_from_canonical_snapshot_for_additive_import(
            &snapshot,
            &self.all_tasks().await?,
        )?;
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        self.record_snapshot_bridge_reconciliation_summary(
            "import_snapshot",
            "canonical_snapshot_file",
            Some(source_path),
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            0,
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn replace_with_taskflow_snapshot(
        &self,
        snapshot: &TaskSnapshot,
    ) -> Result<(), StateStoreError> {
        let task_records = task_records_from_canonical_snapshot(snapshot)?;
        let keep_ids = task_records
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();

        for task in task_records {
            self.persist_task_record(task).await?;
        }

        let mut stale_removed_count = 0usize;
        for task_id in self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
        {
            if !keep_ids.contains(&task_id) {
                self.delete_task_record(&task_id).await?;
                stale_removed_count += 1;
            }
        }

        self.record_snapshot_bridge_reconciliation_summary(
            "replace_snapshot",
            "canonical_snapshot_memory",
            None,
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            stale_removed_count,
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn replace_with_taskflow_snapshot_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let source_path = path.as_ref().display().to_string();
        let snapshot = taskflow_state_fs::read_snapshot(path)?;
        let task_records = task_records_from_canonical_snapshot(&snapshot)?;
        let keep_ids = task_records
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        let mut stale_removed_count = 0usize;
        for task_id in self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
        {
            if !keep_ids.contains(&task_id) {
                self.delete_task_record(&task_id).await?;
                stale_removed_count += 1;
            }
        }
        self.record_snapshot_bridge_reconciliation_summary(
            "replace_snapshot",
            "canonical_snapshot_file",
            Some(source_path),
            snapshot.tasks.len(),
            snapshot.dependencies.len(),
            stale_removed_count,
        )
        .await?;
        Ok(())
    }

    pub async fn latest_task_reconciliation_summary(
        &self,
    ) -> Result<Option<TaskReconciliationSummary>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT receipt_id, operation, source_kind, source_path, task_count, dependency_count, stale_removed_count, recorded_at FROM task_reconciliation_summary ORDER BY recorded_at DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<TaskReconciliationSummary> = query.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn record_runtime_consumption_final_task_reconciliation_summary(
        &self,
        source_path: Option<String>,
    ) -> Result<TaskReconciliationSummary, StateStoreError> {
        if let Some(latest) = self.latest_task_reconciliation_summary().await? {
            return Ok(latest);
        }

        self.record_snapshot_bridge_reconciliation_summary(
            "consume_final",
            "runtime_consumption_final_snapshot",
            source_path,
            0,
            0,
            0,
        )
        .await
    }

    pub async fn task_reconciliation_rollup(
        &self,
    ) -> Result<TaskReconciliationRollup, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT operation, source_kind, source_path, task_count, dependency_count, stale_removed_count, recorded_at FROM task_reconciliation_summary ORDER BY recorded_at DESC;",
            )
            .await?;
        let rows: Vec<TaskReconciliationRollupRow> = query.take(0)?;
        let mut by_operation = BTreeMap::<String, usize>::new();
        let mut by_source_kind = BTreeMap::<String, usize>::new();
        let latest_recorded_at = rows.first().map(|row| row.recorded_at.clone());
        let latest_source_path = rows.first().and_then(|row| row.source_path.clone());
        let mut total_task_rows = 0usize;
        let mut total_dependency_rows = 0usize;
        let mut total_stale_removed = 0usize;

        for row in &rows {
            *by_operation.entry(row.operation.clone()).or_insert(0) += 1;
            *by_source_kind.entry(row.source_kind.clone()).or_insert(0) += 1;
            total_task_rows += row.task_count;
            total_dependency_rows += row.dependency_count;
            total_stale_removed += row.stale_removed_count;
        }

        Ok(TaskReconciliationRollup {
            total_receipts: by_operation.values().sum(),
            latest_recorded_at,
            latest_source_path,
            total_task_rows,
            total_dependency_rows,
            total_stale_removed,
            by_operation,
            by_source_kind,
            rows,
        })
    }

    pub async fn taskflow_snapshot_bridge_summary(
        &self,
    ) -> Result<TaskflowSnapshotBridgeSummary, StateStoreError> {
        let latest_receipt = self.latest_task_reconciliation_summary().await?;
        let rollup = self.task_reconciliation_rollup().await?;
        Ok(TaskflowSnapshotBridgeSummary {
            total_receipts: rollup.total_receipts,
            export_receipts: *rollup.by_operation.get("export_snapshot").unwrap_or(&0),
            import_receipts: *rollup.by_operation.get("import_snapshot").unwrap_or(&0),
            replace_receipts: *rollup.by_operation.get("replace_snapshot").unwrap_or(&0),
            object_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_object"),
            ),
            memory_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            memory_import_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("import_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            memory_replace_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("replace_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            file_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            file_import_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("import_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            file_replace_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("replace_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            total_task_rows: rollup.total_task_rows,
            total_dependency_rows: rollup.total_dependency_rows,
            total_stale_removed: rollup.total_stale_removed,
            latest_operation: latest_receipt
                .as_ref()
                .map(|receipt| receipt.operation.clone()),
            latest_source_kind: latest_receipt
                .as_ref()
                .map(|receipt| receipt.source_kind.clone()),
            latest_source_path: rollup.latest_source_path,
            latest_recorded_at: rollup.latest_recorded_at,
        })
    }
}
