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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn import_taskflow_snapshot_replaces_dependencies_for_updated_tasks_without_removing_unrelated_tasks(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-deps-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-blocker\",\"title\":\"Blocker\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep\",\"description\":\"keep\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-unrelated\",\"title\":\"Unrelated\",\"description\":\"unrelated\",\"status\":\"open\",\"priority\":4,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse keep timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-keep"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };

        store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect("additive import should succeed");

        let keep = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(keep.dependencies.len(), 1);
        assert_eq!(keep.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(keep.dependencies[0].edge_type, "parent-child");

        let unrelated = store
            .show_task("vida-unrelated")
            .await
            .expect("unrelated task should remain after additive import");
        assert_eq!(unrelated.title, "Unrelated");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 2);
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_memory")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn replace_with_taskflow_snapshot_removes_stale_dependencies_for_kept_tasks() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-replace-deps-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-blocker\",\"title\":\"Blocker\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep\",\"description\":\"keep\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse keep timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-keep"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };

        store
            .replace_with_taskflow_snapshot(&snapshot)
            .await
            .expect("replacement import should succeed");

        let keep = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(keep.dependencies.len(), 1);
        assert_eq!(keep.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(keep.dependencies[0].edge_type, "parent-child");

        let blockers = store
            .task_dependencies("vida-keep")
            .await
            .expect("dependencies should load");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].depends_on_id, "vida-root");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_allows_dependencies_on_existing_authoritative_tasks_outside_payload(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-existing-target-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-child"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };

        store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect("additive import should accept existing authoritative dependency target");

        let child = store
            .show_task("vida-child")
            .await
            .expect("child task should be imported");
        assert_eq!(child.dependencies.len(), 1);
        assert_eq!(child.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(child.dependencies[0].edge_type, "parent-child");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_memory")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_file_allows_dependencies_on_existing_authoritative_tasks_outside_payload(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-import-existing-target-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("snapshot.json");
        fs::write(
            &source,
            "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-child"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot).expect("write snapshot");

        store
            .import_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect(
            "file-backed additive import should accept existing authoritative dependency target",
        );

        let child = store
            .show_task("vida-child")
            .await
            .expect("child task should be imported");
        assert_eq!(child.dependencies.len(), 1);
        assert_eq!(child.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(child.dependencies[0].edge_type, "parent-child");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_file");
        assert_eq!(
            latest.source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_fails_closed_before_mutation_on_post_merge_parent_conflict() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-parent-conflict-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root-a\",\"title\":\"Root A\",\"description\":\"root a\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-root-b\",\"title\":\"Root B\",\"description\":\"root b\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-child\",\"title\":\"Child old\",\"description\":\"child\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-child\",\"depends_on_id\":\"vida-root-a\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let before_child = store
            .show_task("vida-child")
            .await
            .expect("child should exist before conflicting import");
        assert_eq!(before_child.title, "Child old");
        assert_eq!(before_child.dependencies.len(), 1);
        assert_eq!(before_child.dependencies[0].depends_on_id, "vida-root-a");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child new".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![
                CanonicalDependencyEdge {
                    issue_id: CanonicalTaskId::new("vida-child"),
                    depends_on_id: CanonicalTaskId::new("vida-root-a"),
                    dependency_type: "parent-child".to_string(),
                },
                CanonicalDependencyEdge {
                    issue_id: CanonicalTaskId::new("vida-child"),
                    depends_on_id: CanonicalTaskId::new("vida-root-b"),
                    dependency_type: "parent-child".to_string(),
                },
            ],
        };

        let error = store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect_err("post-merge multiple-parent conflict should fail");
        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("snapshot graph is invalid after additive merge"));
                assert!(reason.contains("multiple_parent_edges"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let after_child = store
            .show_task("vida-child")
            .await
            .expect("child should still exist after rejected import");
        assert_eq!(after_child.title, "Child old");
        assert_eq!(after_child.dependencies.len(), 1);
        assert_eq!(after_child.dependencies[0].depends_on_id, "vida-root-a");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load");
        assert!(
            latest.is_none(),
            "rejected import must not emit reconciliation receipt"
        );

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 0);
        assert_eq!(bridge.import_receipts, 0);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 0);
        assert!(bridge.latest_operation.is_none());
        assert!(bridge.latest_source_kind.is_none());

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let _ = fs::remove_dir_all(&root);
    }
}
