use super::*;
use serde_json::Deserializer;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const TASK_SNAPSHOT_META_SCHEMA_VERSION: &str = "task-snapshot-meta-v1";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TaskSnapshotMeta {
    schema_version: String,
    snapshot_path: String,
    byte_len: u64,
    content_hash_blake3: String,
    task_count: usize,
    generated_at_unix_nanos: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClosedTaskRunReconciliation {
    pub(crate) run_id: String,
    pub(crate) task_id: String,
    pub(crate) previous_status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClosedTaskRunReconciliationSummary {
    pub(crate) scanned_count: usize,
    pub(crate) reconciled_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) reconciled_runs: Vec<ClosedTaskRunReconciliation>,
}

impl StateStore {
    pub(crate) fn task_status_is_closed_like(status: &str) -> bool {
        matches!(status, "closed" | "completed")
    }

    fn parent_id_for_task(task: &TaskRecord) -> Option<String> {
        task.dependencies
            .iter()
            .find(|dependency| dependency.edge_type == "parent-child")
            .map(|dependency| dependency.depends_on_id.clone())
    }

    fn reopen_closed_parent_chain_for_extension(
        tasks: &mut [TaskRecord],
        parent_id: Option<&str>,
        now: &str,
    ) -> Vec<TaskRecord> {
        let mut reopened = Vec::new();
        let mut current_parent_id = parent_id.map(ToOwned::to_owned);
        let mut visited = BTreeSet::new();

        while let Some(parent_id) = current_parent_id {
            if !visited.insert(parent_id.clone()) {
                break;
            }

            let Some(parent_index) = tasks.iter().position(|task| task.id == parent_id) else {
                break;
            };
            let next_parent_id = Self::parent_id_for_task(&tasks[parent_index]);
            if Self::task_status_is_closed_like(&tasks[parent_index].status) {
                tasks[parent_index].status = "in_progress".to_string();
                tasks[parent_index].updated_at = now.to_string();
                tasks[parent_index].closed_at = None;
                tasks[parent_index].close_reason = None;
                reopened.push(tasks[parent_index].clone());
            }
            current_parent_id = next_parent_id;
        }

        reopened
    }

    fn close_parent_chain_without_active_children(
        tasks: &mut [TaskRecord],
        parent_id: Option<&str>,
        now: &str,
        reason: &str,
        leaf_task_id: Option<&str>,
    ) -> Vec<TaskRecord> {
        let mut closed = Vec::new();
        let mut current_parent_id = parent_id.map(ToOwned::to_owned);
        let mut visited = BTreeSet::new();

        // Track all tasks being closed in this operation (leaf + all parents)
        let mut tasks_being_closed = BTreeSet::new();
        if let Some(leaf_id) = leaf_task_id {
            tasks_being_closed.insert(leaf_id.to_string());
        }

        while let Some(parent_id) = current_parent_id {
            if !visited.insert(parent_id.clone()) {
                break;
            }

            let Some(parent_index) = tasks.iter().position(|task| task.id == parent_id) else {
                break;
            };

            let child_indices = tasks
                .iter()
                .enumerate()
                .filter_map(|(index, task)| {
                    task.dependencies
                        .iter()
                        .any(|dependency| {
                            dependency.edge_type == "parent-child"
                                && dependency.depends_on_id == parent_id
                        })
                        .then_some(index)
                })
                .collect::<Vec<_>>();

            let has_non_closed_child_not_in_chain = child_indices.iter().any(|index| {
                let child = &tasks[*index];
                !Self::task_status_is_closed_like(&child.status)
                    && !tasks_being_closed.contains(&child.id)
            });

            if child_indices.is_empty() || has_non_closed_child_not_in_chain {
                break;
            }

            if !work_item_is_program_container(&tasks[parent_index].issue_type) {
                break;
            }

            let has_unresolved_non_parent_blockers = tasks[parent_index]
                .dependencies
                .iter()
                .filter(|dependency| dependency.edge_type != "parent-child")
                .any(|dependency| {
                    match tasks
                        .iter()
                        .find(|task| task.id == dependency.depends_on_id)
                    {
                        Some(blocker_task) => {
                            !Self::task_status_is_closed_like(&blocker_task.status)
                        }
                        None => true,
                    }
                });
            if has_unresolved_non_parent_blockers {
                break;
            }

            let next_parent_id = Self::parent_id_for_task(&tasks[parent_index]);
            if matches!(tasks[parent_index].status.as_str(), "open" | "in_progress") {
                tasks_being_closed.insert(parent_id.clone());
                tasks[parent_index].status = "closed".to_string();
                tasks[parent_index].updated_at = now.to_string();
                tasks[parent_index].closed_at = Some(now.to_string());
                tasks[parent_index].close_reason = Some(reason.to_string());
                closed.push(tasks[parent_index].clone());
            } else if !Self::task_status_is_closed_like(&tasks[parent_index].status) {
                break;
            }
            current_parent_id = next_parent_id;
        }

        closed
    }

    fn close_emptied_parent_chain_after_reparent(
        tasks: &mut [TaskRecord],
        parent_id: Option<&str>,
        now: &str,
        reason: &str,
    ) -> Vec<TaskRecord> {
        let mut closed = Vec::new();
        let mut current_parent_id = parent_id.map(ToOwned::to_owned);
        let mut visited = BTreeSet::new();

        while let Some(parent_id) = current_parent_id {
            if !visited.insert(parent_id.clone()) {
                break;
            }

            let Some(parent_index) = tasks.iter().position(|task| task.id == parent_id) else {
                break;
            };

            let has_non_closed_child = tasks.iter().any(|task| {
                !Self::task_status_is_closed_like(&task.status)
                    && task.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == parent_id
                    })
            });
            if has_non_closed_child {
                break;
            }

            let has_unresolved_non_parent_blockers = tasks[parent_index]
                .dependencies
                .iter()
                .filter(|dependency| dependency.edge_type != "parent-child")
                .any(|dependency| {
                    match tasks
                        .iter()
                        .find(|task| task.id == dependency.depends_on_id)
                    {
                        Some(blocker_task) => {
                            !Self::task_status_is_closed_like(&blocker_task.status)
                        }
                        None => true,
                    }
                });
            if has_unresolved_non_parent_blockers {
                break;
            }

            let next_parent_id = Self::parent_id_for_task(&tasks[parent_index]);
            if matches!(tasks[parent_index].status.as_str(), "open" | "in_progress") {
                tasks[parent_index].status = "closed".to_string();
                tasks[parent_index].updated_at = now.to_string();
                tasks[parent_index].closed_at = Some(now.to_string());
                tasks[parent_index].close_reason = Some(reason.to_string());
                closed.push(tasks[parent_index].clone());
            }
            current_parent_id = next_parent_id;
        }

        closed
    }

    async fn validate_task_display_id_alias(
        &self,
        task_id: &str,
        display_id: Option<&str>,
    ) -> Result<Option<String>, StateStoreError> {
        let normalized_display_id = display_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let Some(display_id) = normalized_display_id.as_deref() else {
            return Ok(None);
        };

        let tasks = self.all_tasks().await?;
        for task in tasks {
            if task.id == task_id {
                continue;
            }
            if task.id == display_id {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "task `{task_id}` display_id `{display_id}` conflicts with task id `{}`",
                        task.id
                    ),
                });
            }
            if task.display_id.as_deref() == Some(display_id) {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "task `{task_id}` display_id `{display_id}` conflicts with task `{}` display_id",
                        task.id
                    ),
                });
            }
        }

        Ok(Some(display_id.to_string()))
    }

    pub(crate) fn canonical_task_snapshot_path_for_state_root(state_root: &Path) -> PathBuf {
        if let Some(project_root) =
            crate::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        {
            return project_root.join(".vida/exports/tasks.snapshot.jsonl");
        }

        if state_root.file_name().and_then(|value| value.to_str()) == Some("state") {
            if let Some(data_dir) = state_root.parent() {
                if data_dir.file_name().and_then(|value| value.to_str()) == Some("data") {
                    let Some(vida_dir) = data_dir.parent() else {
                        return state_root.join("exports/tasks.snapshot.jsonl");
                    };
                    if vida_dir.file_name().and_then(|value| value.to_str()) != Some(".vida") {
                        return state_root.join("exports/tasks.snapshot.jsonl");
                    }
                    return vida_dir.join("exports/tasks.snapshot.jsonl");
                }
            }
        }

        state_root.join("exports/tasks.snapshot.jsonl")
    }

    pub(crate) fn canonical_task_snapshot_meta_path_for_state_root(state_root: &Path) -> PathBuf {
        Self::task_snapshot_meta_path_for_snapshot_path(
            &Self::canonical_task_snapshot_path_for_state_root(state_root),
        )
    }

    pub(crate) fn canonical_task_snapshot_marker_path_for_state_root(state_root: &Path) -> PathBuf {
        state_root.join(".task-snapshot-state-marker")
    }

    pub(crate) fn touch_task_snapshot_state_marker(state_root: &Path) {
        let marker_path = Self::canonical_task_snapshot_marker_path_for_state_root(state_root);
        if Self::path_is_symlink(&marker_path) {
            return;
        }
        if let Some(parent) = marker_path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let body = unix_timestamp_nanos().to_string();
        let _ = Self::write_jsonl_export_file(&marker_path, body.as_bytes());
    }

    fn task_snapshot_meta_path_for_snapshot_path(snapshot_path: &Path) -> PathBuf {
        snapshot_path.with_file_name("tasks.snapshot.meta.json")
    }

    fn path_is_symlink(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    }

    fn invalid_task_snapshot_reason(reason: impl Into<String>) -> StateStoreError {
        StateStoreError::InvalidTaskRecord {
            reason: reason.into(),
        }
    }

    pub(crate) fn read_fresh_tasks_from_jsonl_snapshot(
        state_root: &Path,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let snapshot_path = Self::canonical_task_snapshot_path_for_state_root(state_root);
        let meta_path = Self::canonical_task_snapshot_meta_path_for_state_root(state_root);
        if Self::path_is_symlink(&snapshot_path) || Self::path_is_symlink(&meta_path) {
            return Err(Self::invalid_task_snapshot_reason(
                "refusing to read task snapshot through symlink path",
            ));
        }
        let meta_raw = fs::read_to_string(&meta_path)?;
        let meta: TaskSnapshotMeta = serde_json::from_str(&meta_raw).map_err(|error| {
            Self::invalid_task_snapshot_reason(format!(
                "task snapshot metadata is invalid: {error}"
            ))
        })?;
        if meta.schema_version != TASK_SNAPSHOT_META_SCHEMA_VERSION {
            return Err(Self::invalid_task_snapshot_reason(format!(
                "unsupported task snapshot metadata schema_version `{}`",
                meta.schema_version
            )));
        }
        if meta.snapshot_path != snapshot_path.display().to_string() {
            return Err(Self::invalid_task_snapshot_reason(
                "task snapshot metadata path does not match canonical snapshot path",
            ));
        }

        let raw = fs::read_to_string(&snapshot_path)?;
        if meta.byte_len != raw.as_bytes().len() as u64 {
            return Err(Self::invalid_task_snapshot_reason(
                "task snapshot metadata byte_len does not match snapshot body",
            ));
        }
        let hash = blake3::hash(raw.as_bytes()).to_hex().to_string();
        if meta.content_hash_blake3 != hash {
            return Err(Self::invalid_task_snapshot_reason(
                "task snapshot metadata hash does not match snapshot body",
            ));
        }
        if let Some(marker_nanos) = Self::task_snapshot_state_marker_nanos(state_root) {
            let meta_nanos = meta
                .generated_at_unix_nanos
                .parse::<u128>()
                .map_err(|error| {
                    Self::invalid_task_snapshot_reason(format!(
                        "task snapshot metadata generated_at_unix_nanos is invalid: {error}"
                    ))
                })?;
            if meta_nanos < marker_nanos {
                return Err(Self::invalid_task_snapshot_reason(
                    "task snapshot metadata is older than latest state mutation marker",
                ));
            }
        } else {
            let marker_path = Self::canonical_task_snapshot_marker_path_for_state_root(state_root);
            if marker_path.exists() {
                let marker_modified = fs::metadata(&marker_path)?.modified()?;
                let meta_modified = fs::metadata(&meta_path)?.modified()?;
                if meta_modified < marker_modified {
                    return Err(Self::invalid_task_snapshot_reason(
                        "task snapshot metadata mtime is older than latest state mutation marker",
                    ));
                }
            }
        }

        let rows = Self::tasks_from_jsonl_snapshot_body(&raw)?;
        if rows.len() != meta.task_count {
            return Err(Self::invalid_task_snapshot_reason(
                "task snapshot metadata task_count does not match snapshot body",
            ));
        }
        Ok(rows)
    }

    fn task_snapshot_state_marker_nanos(state_root: &Path) -> Option<u128> {
        fs::read_to_string(Self::canonical_task_snapshot_marker_path_for_state_root(
            state_root,
        ))
        .ok()
        .and_then(|raw| raw.trim().parse::<u128>().ok())
    }

    pub(crate) fn read_tasks_from_jsonl_snapshot(
        source_path: &Path,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let raw = fs::read_to_string(source_path)?;
        Self::tasks_from_jsonl_snapshot_body(&raw)
    }

    fn tasks_from_jsonl_snapshot_body(raw: &str) -> Result<Vec<TaskRecord>, StateStoreError> {
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

    pub async fn refresh_task_snapshot_for_task(
        &self,
        task: &TaskRecord,
    ) -> Result<PathBuf, StateStoreError> {
        let snapshot_path = Self::canonical_task_snapshot_path_for_state_root(self.root());
        let mut tasks = if snapshot_path.exists() {
            Self::read_tasks_from_jsonl_snapshot(&snapshot_path)?
        } else {
            self.all_tasks().await?
        };
        let mut replaced = false;
        for existing in &mut tasks {
            if existing.id == task.id {
                *existing = task.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            tasks.push(task.clone());
        }
        tasks.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.id.cmp(&right.id))
        });
        let task_count = tasks.len();
        let mut body = String::new();
        for task in tasks {
            body.push_str(&serde_json::to_string(&task).map_err(|error| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!("failed to serialize task snapshot row: {error}"),
                }
            })?);
            body.push('\n');
        }
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Self::write_jsonl_export_file_with_meta(&snapshot_path, body.as_bytes(), task_count)?;
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
        let existing: Option<crate::state_store::RunGraphContinuationBinding> = self
            .db
            .select(("run_graph_continuation_binding", status.run_id.as_str()))
            .await?;
        if let Some(existing) = existing {
            let existing_task_id = existing.active_bounded_unit["task_id"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if existing.binding_source == "explicit_continuation_bind_task"
                && existing_task_id == Some(closed_task_id)
            {
                return Ok(Some(crate::state_store::RunGraphContinuationBinding {
                    run_id: status.run_id.clone(),
                    task_id: status.task_id.clone(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": status.task_id,
                        "run_id": status.run_id,
                        "active_node": status.active_node,
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "Closing the explicitly bound next task returned continuation to the owning run-graph task.".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only".to_string(),
                    request_text: None,
                    recorded_at: time::OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .expect("rfc3339 timestamp should render"),
                }));
            }
        }

        Ok(None)
    }

    pub(crate) fn run_graph_status_allows_task_close_closure_binding(
        status: &RunGraphStatus,
    ) -> bool {
        matches!(status.status.as_str(), "completed")
            && !matches!(
                status.lifecycle_stage.as_str(),
                "analysis_blocked"
                    | "implementation_blocked"
                    | "verification_blocked"
                    | "closure_blocked"
            )
            && status.next_node.is_none()
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
        if !Self::task_status_is_closed_like(&task.status) {
            return Ok(false);
        }

        if !self
            .raw_run_graph_state_allows_task_close_closure_binding(run_id)
            .await?
        {
            return Ok(false);
        }

        self.run_graph_dispatch_has_receipt_backed_execution_truth(run_id)
            .await
    }

    async fn task_close_reconcile_has_persisted_closure_receipt_truth(
        &self,
        run_id: &str,
        task_id: &str,
    ) -> Result<bool, StateStoreError> {
        let task = match self.show_task(task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        if !Self::task_status_is_closed_like(&task.status) {
            return Ok(false);
        }

        if !self
            .raw_run_graph_state_allows_task_close_closure_binding(run_id)
            .await?
        {
            return Ok(false);
        }

        self.run_graph_dispatch_has_receipt_backed_closure_truth(run_id)
            .await
    }

    async fn raw_run_graph_state_allows_task_close_closure_binding(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(plan): Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?
        else {
            return Ok(false);
        };
        let Some(route): Option<RoutedRunStateRow> =
            self.db.select(("routed_run_state", run_id)).await?
        else {
            return Ok(false);
        };
        let Some(governance): Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?
        else {
            return Ok(false);
        };
        let Some(resume): Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?
        else {
            return Ok(false);
        };

        let terminal_completion_evidence = matches!(plan.status.as_str(), "completed")
            || (route.lifecycle_stage == "closure_complete" && plan.active_node == "closure");

        Ok(terminal_completion_evidence
            && plan.next_node.is_none()
            && !matches!(
                route.lifecycle_stage.as_str(),
                "analysis_blocked"
                    | "implementation_blocked"
                    | "verification_blocked"
                    | "closure_blocked"
            )
            && governance.handoff_state == "none"
            && resume.resume_target == "none")
    }

    async fn run_graph_dispatch_has_receipt_backed_closure_truth(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        self.run_graph_dispatch_has_receipt_backed_execution_truth_with(run_id, |receipt| {
            crate::runtime_dispatch_state::dispatch_receipt_has_closure_execution_evidence(receipt)
        })
        .await
    }

    async fn run_graph_dispatch_has_receipt_backed_execution_truth(
        &self,
        run_id: &str,
    ) -> Result<bool, StateStoreError> {
        self.run_graph_dispatch_has_receipt_backed_execution_truth_with(run_id, |receipt| {
            crate::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(receipt)
        })
        .await
    }

    async fn run_graph_dispatch_has_receipt_backed_execution_truth_with(
        &self,
        run_id: &str,
        has_execution_evidence: impl FnOnce(&RunGraphDispatchReceipt) -> bool,
    ) -> Result<bool, StateStoreError> {
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
            && has_execution_evidence(&receipt))
    }

    fn task_close_retired_run_graph_status(
        mut status: RunGraphStatus,
        reason: &str,
    ) -> RunGraphStatus {
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = reason.to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        status
    }

    async fn filter_auto_closed_parents_ready_for_close(
        &self,
        parents: Vec<TaskRecord>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut ready = Vec::new();
        for parent in parents {
            if self
                .auto_closed_parent_has_unresolved_run_graph(&parent.id)
                .await?
            {
                continue;
            }
            ready.push(parent);
        }
        Ok(ready)
    }

    async fn auto_closed_parent_has_unresolved_run_graph(
        &self,
        task_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id_for_task(task_id).await? else {
            return Ok(false);
        };
        let status = match self.run_graph_status(&run_id).await {
            Ok(status) => status,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        if Self::run_graph_status_allows_task_close_closure_binding(&status) {
            return Ok(false);
        }
        Ok(!self
            .run_graph_dispatch_has_receipt_backed_execution_truth(&run_id)
            .await?)
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

    fn normalize_planner_metadata_list(values: Vec<String>) -> Vec<String> {
        let mut normalized = values
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_planner_metadata_text(value: Option<String>) -> Option<String> {
        value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn normalize_planner_metadata(metadata: TaskPlannerMetadata) -> TaskPlannerMetadata {
        TaskPlannerMetadata {
            owned_paths: Self::normalize_planner_metadata_list(metadata.owned_paths),
            acceptance_targets: Self::normalize_planner_metadata_list(metadata.acceptance_targets),
            proof_targets: Self::normalize_planner_metadata_list(metadata.proof_targets),
            risk: Self::normalize_planner_metadata_text(metadata.risk),
            estimate: Self::normalize_planner_metadata_text(metadata.estimate),
            lane_hint: Self::normalize_planner_metadata_text(metadata.lane_hint),
        }
    }

    fn normalize_stored_task_row(row: TaskStorageRowStored) -> TaskStorageRow {
        let mut normalized = TaskStorageRow::from(row);
        normalized.execution_semantics =
            Self::validate_execution_semantics(&normalized.task_id, normalized.execution_semantics)
                .unwrap_or_default();
        normalized.planner_metadata = Self::normalize_planner_metadata(normalized.planner_metadata);
        normalized
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
        let mut all_binding_query = self
            .db
            .query("SELECT * FROM run_graph_continuation_binding;")
            .await?;
        let all_bindings: Vec<crate::state_store::RunGraphContinuationBinding> =
            all_binding_query.take(0)?;
        for binding in all_bindings {
            if binding.active_bounded_unit["kind"].as_str() == Some("task_graph_task")
                && binding.active_bounded_unit["task_id"].as_str() == Some(task_id)
            {
                affected_run_ids.insert(binding.run_id);
            }
        }

        for run_id in affected_run_ids {
            let status = self.run_graph_status(&run_id).await?;
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
            if status.task_id == task_id {
                self.record_run_graph_status(&status).await?;
            }
            self.record_run_graph_continuation_binding(&binding).await?;
        }
        Ok(())
    }

    pub(crate) async fn reconcile_historical_closed_task_active_runs(
        &self,
        limit: usize,
    ) -> Result<ClosedTaskRunReconciliationSummary, StateStoreError> {
        #[derive(serde::Deserialize, SurrealValue)]
        struct HistoricalRunRow {
            run_id: String,
            task_id: String,
            status: String,
            #[allow(dead_code)]
            updated_at: String,
        }

        let limit = limit.max(1);
        let mut query = self
            .db
            .query(format!(
                "SELECT run_id, task_id, status, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT {limit};"
            ))
            .await?;
        let rows: Vec<HistoricalRunRow> = query.take(0)?;
        let scanned_count = rows.len();
        let mut reconciled_runs = Vec::new();
        let mut skipped_count = 0usize;

        for row in rows {
            if row.status == "completed" {
                skipped_count += 1;
                continue;
            }
            let task = match self.show_task(&row.task_id).await {
                Ok(task) => task,
                Err(StateStoreError::MissingTask { .. }) => {
                    skipped_count += 1;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !Self::task_status_is_closed_like(&task.status) {
                skipped_count += 1;
                continue;
            }
            if !self
                .task_close_reconcile_has_persisted_closure_receipt_truth(&row.run_id, &row.task_id)
                .await?
            {
                skipped_count += 1;
                continue;
            }
            let status = self.run_graph_status(&row.run_id).await?;
            let retired_status = Self::task_close_retired_run_graph_status(
                status,
                "historical_closed_task_stale_run_retired",
            );
            self.record_run_graph_status(&retired_status).await?;
            self.clear_run_graph_continuation_binding(&row.run_id)
                .await?;
            reconciled_runs.push(ClosedTaskRunReconciliation {
                run_id: row.run_id,
                task_id: row.task_id,
                previous_status: row.status,
            });
        }

        Ok(ClosedTaskRunReconciliationSummary {
            scanned_count,
            reconciled_count: reconciled_runs.len(),
            skipped_count,
            reconciled_runs,
        })
    }

    pub async fn import_tasks_from_jsonl(
        &self,
        source_path: &Path,
    ) -> Result<TaskImportSummary, StateStoreError> {
        let raw = fs::read_to_string(source_path)?;
        let mut imported = 0usize;
        let mut unchanged = 0usize;
        let mut updated = 0usize;
        let existing_tasks = self.all_tasks().await?;
        let mut provider_external_to_task_id = BTreeMap::new();
        for task in &existing_tasks {
            let Some(mapping) = task.provider_mapping.as_ref() else {
                continue;
            };
            let key = provider_external_key(mapping).map_err(|reason| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "stored task {} has invalid provider mapping: {reason}",
                        task.id
                    ),
                }
            })?;
            if let Some(previous_task_id) =
                provider_external_to_task_id.insert(key.clone(), task.id.clone())
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "duplicate provider mapping for provider={}, external_id={} between tasks {} and {}",
                        key.0, key.1, previous_task_id, task.id
                    ),
                });
            }
        }
        let mut records = Vec::new();

        for (index, record) in Deserializer::from_str(&raw)
            .into_iter::<TaskJsonlRecord>()
            .enumerate()
        {
            let mut record = record.map_err(|error| StateStoreError::InvalidTaskJsonLine {
                line: index + 1,
                reason: error.to_string(),
            })?;
            apply_provider_mapping_to_task_jsonl_record(&mut record).map_err(|reason| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!("line {} provider mapping blocked: {reason}", index + 1),
                }
            })?;
            if let Some(mapping) = record.provider_mapping.as_ref() {
                let key = provider_external_key(mapping).map_err(|reason| {
                    StateStoreError::InvalidTaskRecord {
                        reason: format!("line {} provider mapping blocked: {reason}", index + 1),
                    }
                })?;
                if let Some(previous_task_id) =
                    provider_external_to_task_id.insert(key.clone(), record.id.trim().to_string())
                {
                    return Err(StateStoreError::InvalidTaskRecord {
                        reason: format!(
                            "duplicate provider mapping for provider={}, external_id={} between tasks {} and {}",
                            key.0,
                            key.1,
                            previous_task_id,
                            record.id.trim()
                        ),
                    });
                }
            }
            records.push((index + 1, record));
        }

        let mut staged_contents = Vec::new();
        let mut staged_tasks_by_id = existing_tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let mut touched_task_ids = BTreeSet::new();

        for (line, mut record) in records {
            let task_id = record.id.trim().to_string();
            if task_id.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!("line {line} is missing task id"),
                });
            }
            if let Some(mapping) = record.provider_mapping.as_ref() {
                if let Some(external_parent_id) = mapping
                    .external_parent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    let provider = provider_external_key(mapping)
                        .map_err(|reason| StateStoreError::InvalidTaskRecord {
                            reason: format!("line {line} provider mapping blocked: {reason}"),
                        })?
                        .0;
                    let parent_key = (provider.clone(), external_parent_id.to_string());
                    let parent_task_id = provider_external_to_task_id
                        .get(&parent_key)
                        .cloned()
                        .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                            reason: format!(
                                "line {line} provider mapping blocked: unresolved external_parent_id: provider={provider}, external_parent_id={external_parent_id}"
                            ),
                        })?;
                    record
                        .dependencies
                        .retain(|dependency| dependency.edge_type != "parent-child");
                    record.dependencies.push(TaskDependencyJsonlRecord {
                        issue_id: task_id.clone(),
                        depends_on_id: parent_task_id,
                        edge_type: "parent-child".to_string(),
                        created_at: record.updated_at.clone(),
                        created_by: record.created_by.clone(),
                        metadata: format!(
                            "{{\"source\":\"provider_mapping\",\"external_parent_id\":\"{}\"}}",
                            external_parent_id
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"")
                        ),
                        thread_id: String::new(),
                    });
                }
            }

            let normalized_display_id = self
                .validate_task_display_id_alias(&task_id, record.display_id.as_deref())
                .await?;
            let mut content = TaskContent::from(record);
            content.display_id = normalized_display_id;

            touched_task_ids.insert(task_id.clone());
            for dependency in &content.dependencies {
                touched_task_ids.insert(dependency.depends_on_id.clone());
            }
            staged_tasks_by_id.insert(
                task_id.clone(),
                TaskRecord::from(TaskStorageRow::from(content.clone())),
            );
            staged_contents.push((task_id, content));
        }

        let staged_tasks = staged_tasks_by_id.into_values().collect::<Vec<_>>();
        let issues = Self::validate_task_graph_rows_for_mutation(
            &existing_tasks,
            &staged_tasks,
            &touched_task_ids,
        )
        .into_iter()
        // Preserve existing provider-import behavior for closed children under open parents
        // while still rejecting structural graph corruption before any rows are persisted.
        .filter(|issue| issue.issue_type != "open_parent_has_no_open_child")
        .collect::<Vec<_>>();
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task import would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        for (task_id, content) in staged_contents {
            let existing: Option<TaskStorageRowStored> =
                self.db.select(("task", task_id.as_str())).await?;
            match existing {
                None => imported += 1,
                Some(current) => {
                    if Self::normalize_stored_task_row(current)
                        == TaskStorageRow::from(content.clone())
                    {
                        unchanged += 1
                    } else {
                        updated += 1
                    }
                }
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
        let task_count = tasks.len();
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
        Self::write_jsonl_export_file_with_meta(target_path, body.as_bytes(), task_count)?;
        Ok(task_count)
    }

    fn write_jsonl_export_file_with_meta(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
    ) -> Result<(), StateStoreError> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Self::write_jsonl_export_file(target_path, body)?;
        Self::write_task_snapshot_meta_file(target_path, body, task_count)
    }

    fn write_task_snapshot_meta_file(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
    ) -> Result<(), StateStoreError> {
        let meta_path = Self::task_snapshot_meta_path_for_snapshot_path(target_path);
        let meta = TaskSnapshotMeta {
            schema_version: TASK_SNAPSHOT_META_SCHEMA_VERSION.to_string(),
            snapshot_path: target_path.display().to_string(),
            byte_len: body.len() as u64,
            content_hash_blake3: blake3::hash(body).to_hex().to_string(),
            task_count,
            generated_at_unix_nanos: unix_timestamp_nanos().to_string(),
        };
        let body = serde_json::to_vec_pretty(&meta).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!("failed to serialize task snapshot metadata: {error}"),
            }
        })?;
        Self::write_jsonl_export_file(&meta_path, &body)
    }

    fn write_jsonl_export_file(target_path: &Path, body: &[u8]) -> Result<(), StateStoreError> {
        if fs::symlink_metadata(target_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "refusing to write task export to symlink path: {}",
                    target_path.display()
                ),
            });
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options.open(target_path)?;
        std::io::Write::write_all(&mut file, body)?;
        Ok(())
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
        let row: Option<TaskStorageRowStored> = self.db.select(("task", task_id)).await?;
        row.map(Self::normalize_stored_task_row)
            .map(TaskRecord::from)
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

    pub(crate) fn task_dependencies_from_rows(
        rows: &[TaskRecord],
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        let by_id = rows
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();
        let task = by_id
            .get(task_id)
            .cloned()
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;

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
                    depends_on_id: depends_on_id.clone(),
                    edge_type: dependency.edge_type,
                    dependency_status,
                    dependency_issue_type: by_id
                        .get(&depends_on_id)
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

    pub(crate) fn reverse_dependencies_from_rows(
        rows: &[TaskRecord],
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        if !rows.iter().any(|task| task.id == task_id) {
            return Err(StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            });
        }

        let by_id = rows
            .iter()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut reverse = rows
            .iter()
            .flat_map(|task| {
                let issue_id = task.id.clone();
                let issue_status = task.status.clone();
                let issue_type = task.issue_type.clone();
                task.dependencies
                    .iter()
                    .filter(|dependency| dependency.depends_on_id == task_id)
                    .map(move |dependency| TaskDependencyStatus {
                        issue_id: issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
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
            .filter(|task| !work_item_is_program_container(&task.issue_type))
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

    pub(crate) fn blocked_tasks_from_rows(rows: &[TaskRecord]) -> Vec<BlockedTaskRecord> {
        let by_id = rows
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut blocked = rows
            .iter()
            .cloned()
            .filter(|task| task.status == "open" || task.status == "in_progress")
            .filter(|task| !work_item_is_program_container(&task.issue_type))
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
        blocked
    }

    pub async fn add_task_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
        created_by: &str,
    ) -> Result<TaskDependencyRecord, StateStoreError> {
        let mut tasks = self.all_tasks().await?;
        let original_tasks = tasks.clone();
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

        let touched_task_ids = BTreeSet::from([issue_id.to_string(), depends_on_id.to_string()]);
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
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

    pub async fn add_task_dependencies_bulk(
        &self,
        edges: &[TaskDependencyBulkAddInput],
        created_by: &str,
        dry_run: bool,
    ) -> Result<TaskDependencyBulkAddResult, StateStoreError> {
        let mut tasks = self.all_tasks().await?;
        let original_tasks = tasks.clone();
        let mut created = Vec::new();
        let mut existing = Vec::new();
        let mut failed = Vec::new();
        let mut unapplied = Vec::new();
        let mut touched_task_ids = BTreeSet::new();
        let now = unix_timestamp_nanos().to_string();

        for edge in edges {
            let issue_id = edge.issue_id.trim();
            let depends_on_id = edge.depends_on_id.trim();
            let edge_type = edge.edge_type.trim();
            let report = |reason: String| TaskDependencyBulkAddEdgeReport {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                edge_type: edge_type.to_string(),
                reason,
            };

            if issue_id.is_empty() || depends_on_id.is_empty() || edge_type.is_empty() {
                failed.push(report(
                    "issue_id, depends_on_id, and edge_type are required".to_string(),
                ));
                continue;
            }

            let target_exists = tasks.iter().any(|task| task.id == depends_on_id);
            if !target_exists {
                failed.push(report(format!(
                    "missing dependency target `{depends_on_id}`"
                )));
                continue;
            }

            let Some(task_index) = tasks.iter().position(|task| task.id == issue_id) else {
                failed.push(report(format!("missing source task `{issue_id}`")));
                continue;
            };

            if let Some(dependency) = tasks[task_index]
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type
                })
                .cloned()
            {
                existing.push(dependency);
                continue;
            }

            let dependency = TaskDependencyRecord {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                edge_type: edge_type.to_string(),
                created_at: now.clone(),
                created_by: created_by.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            };
            tasks[task_index].dependencies.push(dependency.clone());
            tasks[task_index].updated_at = now.clone();
            tasks[task_index].dependencies.sort_by(|left, right| {
                left.edge_type
                    .cmp(&right.edge_type)
                    .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
            });
            touched_task_ids.insert(issue_id.to_string());
            touched_task_ids.insert(depends_on_id.to_string());
            created.push(dependency);
        }

        if !failed.is_empty() {
            unapplied.extend(
                created
                    .iter()
                    .map(|dependency| TaskDependencyBulkAddEdgeReport {
                        issue_id: dependency.issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
                        reason: "bulk mutation aborted before persistence".to_string(),
                    }),
            );
            created.clear();
            return Ok(TaskDependencyBulkAddResult {
                dry_run,
                requested_count: edges.len(),
                created_count: 0,
                existing_count: existing.len(),
                failed_count: failed.len(),
                unapplied_count: unapplied.len(),
                created,
                existing,
                failed,
                unapplied,
            });
        }

        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
        if let Some(first) = issues.first() {
            unapplied.extend(
                created
                    .iter()
                    .map(|dependency| TaskDependencyBulkAddEdgeReport {
                        issue_id: dependency.issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
                        reason: "bulk mutation aborted before persistence".to_string(),
                    }),
            );
            failed.push(TaskDependencyBulkAddEdgeReport {
                issue_id: first.issue_id.clone(),
                depends_on_id: String::new(),
                edge_type: String::new(),
                reason: format!(
                    "dependency mutation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
            created.clear();
            return Ok(TaskDependencyBulkAddResult {
                dry_run,
                requested_count: edges.len(),
                created_count: 0,
                existing_count: existing.len(),
                failed_count: failed.len(),
                unapplied_count: unapplied.len(),
                created,
                existing,
                failed,
                unapplied,
            });
        }

        if !dry_run {
            let changed_task_ids = touched_task_ids
                .iter()
                .filter(|task_id| {
                    original_tasks
                        .iter()
                        .find(|task| task.id == **task_id)
                        .zip(tasks.iter().find(|task| task.id == **task_id))
                        .map(|(before, after)| before != after)
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            for task_id in changed_task_ids {
                let task = tasks
                    .iter()
                    .find(|task| task.id == task_id)
                    .cloned()
                    .expect("changed task should exist in staged task set");
                self.persist_task_record(task).await?;
            }
        }

        Ok(TaskDependencyBulkAddResult {
            dry_run,
            requested_count: edges.len(),
            created_count: created.len(),
            existing_count: existing.len(),
            failed_count: failed.len(),
            unapplied_count: unapplied.len(),
            created,
            existing,
            failed,
            unapplied,
        })
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
            planner_metadata,
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
        if work_item_requires_parent(issue_type) && parent_id.is_none() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task `{task_id}` of type `{}` cannot be created without parent_id. Only parent-optional work item kinds can have no parent.",
                    issue_type
                ),
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
        let normalized_display_id = self
            .validate_task_display_id_alias(task_id, display_id)
            .await?;
        let mut tasks = self.all_tasks().await?;
        let original_tasks = tasks.clone();
        if tasks.iter().any(|task| task.id == task_id) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task already exists: {task_id}"),
            });
        }
        if let Some(parent_id) = normalized_parent_id.as_deref() {
            if !tasks.iter().any(|task| task.id == parent_id) {
                return Err(StateStoreError::MissingTask {
                    task_id: parent_id.to_string(),
                });
            }
        }
        let execution_semantics = Self::validate_execution_semantics(task_id, execution_semantics)?;
        let planner_metadata = Self::normalize_planner_metadata(planner_metadata);

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
            planner_metadata,
            provider_mapping: None,
            dependencies,
        };
        if status == "closed" {
            task.closed_at = Some(now.clone());
        }

        let reopened_parents = if task.status != "closed" {
            Self::reopen_closed_parent_chain_for_extension(
                &mut tasks,
                normalized_parent_id.as_deref(),
                &now,
            )
        } else {
            Vec::new()
        };
        tasks.push(task.clone());
        let touched_task_ids = reopened_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(std::iter::once(task.id.clone()))
            .chain(normalized_parent_id.iter().cloned())
            .collect::<BTreeSet<_>>();
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task creation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        for parent in reopened_parents {
            self.persist_task_record(parent).await?;
        }
        self.persist_new_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn update_task(
        &self,
        request: UpdateTaskRequest<'_>,
    ) -> Result<TaskRecord, StateStoreError> {
        let UpdateTaskRequest {
            task_id,
            title,
            status,
            priority,
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
            planner_metadata,
        } = request;
        let mut task = self.show_task(task_id).await?;
        if let Some(title) = title {
            let trimmed = title.trim();
            if trimmed.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!("task `{task_id}` title cannot be empty"),
                });
            }
            task.title = trimmed.to_string();
        }
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            if status == "closed" {
                let tasks = self.all_tasks().await?;
                let non_closed_children = tasks
                    .iter()
                    .filter(|candidate| {
                        candidate.id != task_id
                            && !Self::task_status_is_closed_like(&candidate.status)
                            && candidate.dependencies.iter().any(|dependency| {
                                dependency.edge_type == "parent-child"
                                    && dependency.depends_on_id == task_id
                            })
                    })
                    .map(|candidate| candidate.id.clone())
                    .collect::<Vec<_>>();
                if !non_closed_children.is_empty() {
                    return Err(StateStoreError::InvalidTaskRecord {
                        reason: format!(
                            "cannot close task `{task_id}` while non-closed child tasks exist: {}",
                            non_closed_children.join(", ")
                        ),
                    });
                }
            }
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
        if let Some(priority) = priority {
            task.priority = priority;
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
        if let Some(planner_metadata) = planner_metadata {
            task.planner_metadata = Self::normalize_planner_metadata(planner_metadata);
        }
        task.labels.sort();
        task.labels.dedup();
        task.updated_at = unix_timestamp_nanos().to_string();
        let mut tasks = self.all_tasks().await?;
        let original_tasks = tasks.clone();
        let task_index = tasks
            .iter()
            .position(|existing| existing.id == task.id)
            .expect("updated task should exist in authoritative state");
        tasks[task_index] = task.clone();
        let (reopened_parents, closed_parents) = if task.status == "closed" {
            let parent_id = Self::parent_id_for_task(&task);
            (
                Vec::new(),
                Self::close_parent_chain_without_active_children(
                    &mut tasks,
                    parent_id.as_deref(),
                    &task.updated_at,
                    &format!("all direct child tasks closed after closing `{task_id}`"),
                    Some(task_id),
                ),
            )
        } else {
            let parent_id = Self::parent_id_for_task(&task);
            (
                Self::reopen_closed_parent_chain_for_extension(
                    &mut tasks,
                    parent_id.as_deref(),
                    &task.updated_at,
                ),
                Vec::new(),
            )
        };
        let touched_task_ids = reopened_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(closed_parents.iter().map(|parent| parent.id.clone()))
            .chain(std::iter::once(task.id.clone()))
            .collect::<BTreeSet<_>>();
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task update would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }
        for parent in reopened_parents {
            self.persist_task_record(parent).await?;
        }
        let closed_parents = self
            .filter_auto_closed_parents_ready_for_close(closed_parents)
            .await?;
        self.persist_task_record(task.clone()).await?;
        for parent in &closed_parents {
            self.persist_task_record(parent.clone()).await?;
            self.refresh_run_graph_continuation_after_task_close(&parent.id)
                .await?;
        }
        if task.status == "closed" {
            self.refresh_run_graph_continuation_after_task_close(task_id)
                .await?;
        }
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
        let original_tasks = tasks.clone();
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
        let closed_parents = Self::close_emptied_parent_chain_after_reparent(
            &mut tasks,
            Some(from_parent_id),
            &now,
            &format!("all direct child tasks moved from `{from_parent_id}` to `{to_parent_id}`"),
        );

        let touched_task_ids = moved_tasks
            .iter()
            .map(|task| task.id.clone())
            .chain(closed_parents.iter().map(|parent| parent.id.clone()))
            .chain([from_parent_id.to_string(), to_parent_id.to_string()])
            .collect::<BTreeSet<_>>();
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
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
            for parent in &closed_parents {
                self.persist_task_record(parent.clone()).await?;
                self.refresh_run_graph_continuation_after_task_close(&parent.id)
                    .await?;
            }
            for parent in &closed_parents {
                self.release_active_task_claims_for_task(&parent.id, "task_closed")
                    .await?;
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

    pub async fn defect_batch_rehome(
        &self,
        from_parent_id: &str,
        to_parent_id: &str,
        child_ids: &[String],
        pause_task_ids: &[String],
        start_task_ids: &[String],
        dry_run: bool,
    ) -> Result<TaskDefectBatchRehomeResult, StateStoreError> {
        self.show_task(from_parent_id).await?;
        self.show_task(to_parent_id).await?;
        if from_parent_id == to_parent_id {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "from_parent_id and to_parent_id must differ".to_string(),
            });
        }

        let pause_set = pause_task_ids
            .iter()
            .map(|task_id| task_id.trim())
            .filter(|task_id| !task_id.is_empty())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>();
        let start_set = start_task_ids
            .iter()
            .map(|task_id| task_id.trim())
            .filter(|task_id| !task_id.is_empty())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>();
        let overlapping = pause_set
            .intersection(&start_set)
            .cloned()
            .collect::<Vec<_>>();
        if !overlapping.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "tasks cannot be both paused and started in one defect batch rehome: {}",
                    overlapping.join(", ")
                ),
            });
        }

        let mut tasks = self.all_tasks().await?;
        let original_tasks = tasks.clone();
        let task_ids = tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let missing_status_targets = pause_set
            .union(&start_set)
            .filter(|task_id| !task_ids.contains(*task_id))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_status_targets.is_empty() {
            return Err(StateStoreError::MissingTask {
                task_id: missing_status_targets.join(", "),
            });
        }

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
        let mut changed_tasks = Vec::new();

        for task in &mut tasks {
            let mut changed = false;
            if moved_set.contains(&task.id) {
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
                    .unwrap_or_else(|| "vida task defect-batch-rehome".to_string());
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
                changed = true;
            }
            if pause_set.contains(&task.id) {
                task.status = "paused".to_string();
                task.closed_at = None;
                task.close_reason = None;
                changed = true;
            } else if start_set.contains(&task.id) {
                task.status = "in_progress".to_string();
                task.closed_at = None;
                task.close_reason = None;
                changed = true;
            }
            if changed {
                task.updated_at = now.clone();
                changed_tasks.push(task.clone());
            }
        }

        let touched_task_ids = changed_tasks
            .iter()
            .map(|task| task.id.clone())
            .chain([from_parent_id.to_string(), to_parent_id.to_string()])
            .collect::<BTreeSet<_>>();
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "defect batch rehome would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        if !dry_run {
            for task in &changed_tasks {
                self.persist_task_record(task.clone()).await?;
            }
        }

        Ok(TaskDefectBatchRehomeResult {
            from_parent_id: from_parent_id.to_string(),
            to_parent_id: to_parent_id.to_string(),
            requested_child_ids: requested_child_ids.clone(),
            moved_child_ids: requested_child_ids,
            paused_task_ids: pause_set.iter().cloned().collect(),
            started_task_ids: start_set.iter().cloned().collect(),
            moved_count: moved_set.len(),
            paused_count: pause_set.len(),
            started_count: start_set.len(),
            dry_run,
            tasks: changed_tasks,
        })
    }

    pub async fn close_task(
        &self,
        task_id: &str,
        reason: &str,
    ) -> Result<TaskRecord, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let non_closed_children = tasks
            .iter()
            .filter(|task| {
                task.id != task_id
                    && !Self::task_status_is_closed_like(&task.status)
                    && task.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == task_id
                    })
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        if !non_closed_children.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "cannot close task `{task_id}` while non-closed child tasks exist: {}",
                    non_closed_children.join(", ")
                ),
            });
        }

        let mut task = self.show_task(task_id).await?;
        let now = unix_timestamp_nanos().to_string();
        task.status = "closed".to_string();
        task.updated_at = now.clone();
        task.closed_at = Some(now);
        task.close_reason = Some(reason.to_string());
        let original_tasks = tasks.clone();
        let mut reconciled_tasks = tasks;
        let task_index = reconciled_tasks
            .iter()
            .position(|existing| existing.id == task.id)
            .expect("closed task should exist in authoritative state");
        reconciled_tasks[task_index] = task.clone();
        let parent_id = Self::parent_id_for_task(&task);
        let closed_parents = Self::close_parent_chain_without_active_children(
            &mut reconciled_tasks,
            parent_id.as_deref(),
            &task.updated_at,
            &format!("all direct child tasks closed after closing `{task_id}`"),
            Some(task_id),
        );
        let touched_task_ids = closed_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(std::iter::once(task.id.clone()))
            .collect::<BTreeSet<_>>();
        if let Some(first) = Self::validate_task_graph_rows_for_mutation(
            &original_tasks,
            &reconciled_tasks,
            &touched_task_ids,
        )
        .first()
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task close would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }
        let closed_parents = self
            .filter_auto_closed_parents_ready_for_close(closed_parents)
            .await?;
        self.persist_task_record(task.clone()).await?;
        for parent in &closed_parents {
            self.persist_task_record(parent.clone()).await?;
            self.refresh_run_graph_continuation_after_task_close(&parent.id)
                .await?;
        }
        self.release_active_task_claims_for_task(task_id, "task_closed")
            .await?;
        for parent in &closed_parents {
            self.release_active_task_claims_for_task(&parent.id, "task_closed")
                .await?;
        }
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
        Self::touch_task_snapshot_state_marker(self.root());
        Ok(())
    }

    async fn persist_new_task_record(&self, task: TaskRecord) -> Result<(), StateStoreError> {
        let task_id = task.id.clone();
        let row = TaskStorageRow::from(task.clone());
        let _: Option<TaskStorageRow> = self
            .db
            .upsert(("task", task_id.as_str()))
            .content(row)
            .await?;
        self.insert_task_dependency_rows(&task_id, &task.dependencies)
            .await?;
        Self::touch_task_snapshot_state_marker(self.root());
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

        self.insert_task_dependency_rows(task_id, dependencies)
            .await
    }

    async fn insert_task_dependency_rows(
        &self,
        task_id: &str,
        dependencies: &[TaskDependencyRecord],
    ) -> Result<(), StateStoreError> {
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
        Self::touch_task_snapshot_state_marker(self.root());
        Ok(())
    }

    pub(crate) async fn all_tasks(&self) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut query = self
            .db
            .query("SELECT * FROM task ORDER BY priority ASC, id ASC;")
            .await?;
        let rows: Vec<TaskStorageRowStored> = query.take(0)?;
        Ok(rows
            .into_iter()
            .map(Self::normalize_stored_task_row)
            .map(TaskRecord::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_task_store_temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
    }

    fn test_task_record(task_id: &str, issue_type: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority: 1,
            issue_type: issue_type.to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            updated_at: "1".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn canonical_task_snapshot_path_keeps_isolated_state_roots_local() {
        let state_root = unique_task_store_temp_root("vida-isolated-task-state").join("state");

        assert_eq!(
            StateStore::canonical_task_snapshot_path_for_state_root(&state_root),
            state_root.join("exports/tasks.snapshot.jsonl")
        );
    }

    #[test]
    fn canonical_task_snapshot_path_maps_project_state_layout_to_vida_exports() {
        let project_root = unique_task_store_temp_root("vida-project-task-state");
        let state_root = project_root.join(".vida").join("data").join("state");

        assert_eq!(
            StateStore::canonical_task_snapshot_path_for_state_root(&state_root),
            project_root
                .join(".vida")
                .join("exports/tasks.snapshot.jsonl")
        );
    }

    fn sample_snapshot_body() -> String {
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",",
            "\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",",
            "\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",",
            "\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",",
            "\"compaction_level\":0,\"original_size\":0,\"labels\":[],",
            "\"dependencies\":[]}\n"
        )
        .to_string()
    }

    #[test]
    fn fresh_task_snapshot_metadata_validates_hash_count_and_marker() {
        let state_root = unique_task_store_temp_root("vida-task-snapshot-meta-fresh")
            .join(".vida")
            .join("data")
            .join("state");
        let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_root);
        let body = sample_snapshot_body();
        StateStore::write_jsonl_export_file_with_meta(&snapshot_path, body.as_bytes(), 1)
            .expect("snapshot and metadata should write");

        let rows = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect("fresh snapshot should validate");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "vida-root");
        assert!(
            StateStore::canonical_task_snapshot_meta_path_for_state_root(&state_root).is_file()
        );
        let _ = fs::remove_dir_all(
            state_root
                .ancestors()
                .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("state"))
                .unwrap_or(&state_root),
        );
    }

    #[test]
    fn fresh_task_snapshot_metadata_rejects_hash_drift() {
        let state_root = unique_task_store_temp_root("vida-task-snapshot-meta-hash")
            .join(".vida")
            .join("data")
            .join("state");
        let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_root);
        let body = sample_snapshot_body();
        StateStore::write_jsonl_export_file_with_meta(&snapshot_path, body.as_bytes(), 1)
            .expect("snapshot and metadata should write");
        fs::write(&snapshot_path, "").expect("snapshot should be mutable for test");

        let error = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect_err("hash drift must reject snapshot");
        assert!(error
            .to_string()
            .contains("task snapshot metadata byte_len does not match snapshot body"));
        let _ = fs::remove_dir_all(
            state_root
                .ancestors()
                .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("state"))
                .unwrap_or(&state_root),
        );
    }

    #[test]
    fn fresh_task_snapshot_metadata_rejects_newer_task_marker() {
        let state_root = unique_task_store_temp_root("vida-task-snapshot-meta-marker")
            .join(".vida")
            .join("data")
            .join("state");
        let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_root);
        let body = sample_snapshot_body();
        StateStore::write_jsonl_export_file_with_meta(&snapshot_path, body.as_bytes(), 1)
            .expect("snapshot and metadata should write");
        fs::create_dir_all(&state_root).expect("state root should exist");
        fs::write(
            StateStore::canonical_task_snapshot_marker_path_for_state_root(&state_root),
            "999999999999999999999999999999",
        )
        .expect("marker should write");

        let error = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect_err("newer marker must reject snapshot");
        assert!(error
            .to_string()
            .contains("task snapshot metadata is older than latest state mutation marker"));
        let _ = fs::remove_dir_all(
            state_root
                .ancestors()
                .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("state"))
                .unwrap_or(&state_root),
        );
    }

    #[tokio::test]
    async fn persist_task_record_invalidates_preexisting_fresh_snapshot() {
        let root = unique_task_store_temp_root("vida-task-snapshot-persist-invalidates");
        let state_root = root.join(".vida").join("data").join("state");
        let store = StateStore::open(state_root.clone())
            .await
            .expect("store should open");
        let mut task = test_task_record("snapshot-active-task", "task", "in_progress");
        store
            .persist_task_record(task.clone())
            .await
            .expect("initial task should persist");
        store
            .refresh_task_snapshot()
            .await
            .expect("snapshot should refresh");
        let rows = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect("fresh snapshot should validate before later mutation");
        assert_eq!(rows[0].status, "in_progress");

        task.status = "closed".to_string();
        task.updated_at = "2".to_string();
        task.closed_at = Some("2".to_string());
        task.close_reason = Some("done".to_string());
        store
            .persist_task_record(task)
            .await
            .expect("closed task should persist");

        let error = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect_err("old snapshot must be invalid after direct store mutation");
        assert!(error
            .to_string()
            .contains("task snapshot metadata is older than latest state mutation marker"));

        store
            .refresh_task_snapshot()
            .await
            .expect("snapshot should refresh after close mutation");
        let rows = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect("refreshed snapshot should validate");
        assert_eq!(rows[0].status, "closed");

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn create_task_uses_work_item_taxonomy_for_parent_requirement() {
        let root = unique_task_store_temp_root("vida-work-item-parent-requirement");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let error = store
            .create_task(CreateTaskRequest {
                task_id: "bug-without-parent",
                title: "Bug without parent",
                display_id: None,
                description: "",
                issue_type: "bug",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect_err("bug alias must still require a parent");

        assert!(error
            .to_string()
            .contains("Only parent-optional work item kinds can have no parent"));
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn create_task_accepts_normalized_parent_optional_epic_kind() {
        let root = unique_task_store_temp_root("vida-work-item-epic-parent-optional");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let task = store
            .create_task(CreateTaskRequest {
                task_id: "capitalized-epic",
                title: "Capitalized Epic",
                display_id: None,
                description: "",
                issue_type: "Epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("normalized epic kind should be parent optional");

        assert_eq!(task.issue_type, "Epic");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_ignores_unrelated_existing_parentless_open_work_item() {
        let root = unique_task_store_temp_root("vida-close-ignores-existing-orphan");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "parent-epic",
                title: "Parent epic",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "child-task",
                title: "Child task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("parent-epic"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");
        store
            .persist_task_record(test_task_record("historical-orphan", "task", "open"))
            .await
            .expect("persist historical orphan");

        let closed = store
            .close_task("child-task", "proof passed")
            .await
            .expect("unrelated historical orphan should not block close");

        assert_eq!(closed.status, "closed");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn create_open_child_atomically_reopens_closed_parent_chain() {
        let root = unique_task_store_temp_root("vida-create-child-reopens-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "closed-parent",
                title: "Closed parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "closed-child",
                title: "Closed child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("closed-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");
        store
            .close_task("closed-child", "done")
            .await
            .expect("close child");
        store
            .close_task("closed-parent", "done")
            .await
            .expect("close parent");

        store
            .create_task(CreateTaskRequest {
                task_id: "new-child",
                title: "New child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("closed-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child under closed parent");

        let parent = store.show_task("closed-parent").await.expect("load parent");
        assert_eq!(parent.status, "in_progress");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn reparent_last_non_closed_child_atomically_closes_emptied_parent() {
        let root = unique_task_store_temp_root("vida-reparent-last-child-closes-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "source-parent",
                title: "Source parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create source parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "target-parent",
                title: "Target parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create target parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "moving-child",
                title: "Moving child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "blocked",
                priority: 1,
                parent_id: Some("source-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create moving child");

        store
            .reparent_children(
                "source-parent",
                "target-parent",
                &["moving-child".to_string()],
                false,
            )
            .await
            .expect("reparent should close emptied source parent atomically");

        let source_parent = store
            .show_task("source-parent")
            .await
            .expect("source parent should load");
        assert_eq!(source_parent.status, "closed");
        assert_eq!(
            source_parent.close_reason.as_deref(),
            Some("all direct child tasks moved from `source-parent` to `target-parent`")
        );
        let moving_child = store
            .show_task("moving-child")
            .await
            .expect("moving child should load");
        assert!(moving_child.dependencies.iter().any(|dependency| {
            dependency.edge_type == "parent-child" && dependency.depends_on_id == "target-parent"
        }));
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_releases_active_task_claims() {
        let root = unique_task_store_temp_root("vida-close-task-releases-claims");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "claim-parent",
                title: "Claim parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "claimed-task",
                title: "Claimed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: Some("claim-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "claim-claimed-task".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: "worktree".to_string(),
                orchestrator_session_id: "session-a".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("claimed-task".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:claimed-task".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
            .expect("claim task");

        store
            .close_task("claimed-task", "done")
            .await
            .expect("close task");

        assert!(store
            .active_orchestrator_claims()
            .await
            .expect("active claims")
            .iter()
            .all(|claim| claim.task_id.as_deref() != Some("claimed-task")));
        let claim = store
            .orchestrator_claim("claim-claimed-task")
            .await
            .expect("load claim")
            .expect("claim exists");
        assert_eq!(claim.status, "released");
        assert_eq!(claim.release_reason.as_deref(), Some("task_closed"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_closed_child_to_open_atomically_reopens_closed_parent() {
        let root = unique_task_store_temp_root("vida-update-child-reopens-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "closed-parent",
                title: "Closed parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "closed-child",
                title: "Closed child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("closed-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");
        store
            .close_task("closed-child", "done")
            .await
            .expect("close child");
        store
            .close_task("closed-parent", "done")
            .await
            .expect("close parent");

        store
            .update_task(UpdateTaskRequest {
                task_id: "closed-child",
                title: None,
                status: Some("open"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("reopen child under closed parent");

        let parent = store.show_task("closed-parent").await.expect("load parent");
        assert_eq!(parent.status, "in_progress");
        assert!(parent.closed_at.is_none());
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_status_closed_closes_parent_without_active_children() {
        let root = unique_task_store_temp_root("vida-update-child-closes-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("update-close-parent", "Update close parent", "epic", None),
            (
                "update-close-child",
                "Update close child",
                "task",
                Some("update-close-parent"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status: "open",
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task pair");
        }

        store
            .update_task(UpdateTaskRequest {
                task_id: "update-close-child",
                title: None,
                status: Some("closed"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("close child through update");

        let parent = store
            .show_task("update-close-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "closed");
        assert_eq!(
            parent.close_reason.as_deref(),
            Some("all direct child tasks closed after closing `update-close-child`")
        );
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_status_closed_keeps_parent_open_with_blocked_run_graph() {
        let root = unique_task_store_temp_root("vida-update-child-blocked-run-graph");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("run-graph-parent", "Run graph parent", "epic", None),
            (
                "run-graph-child",
                "Run graph child",
                "todo",
                Some("run-graph-parent"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status: "open",
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task tree");
        }

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-graph-parent",
            "implementation",
            "implementation",
        );
        status.task_id = "run-graph-parent".to_string();
        status.active_node = "test_author".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.handoff_state = "delegated_lane_blocked".to_string();
        status.recovery_ready = false;
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run graph status");

        store
            .update_task(UpdateTaskRequest {
                task_id: "run-graph-child",
                title: None,
                status: Some("closed"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("close child through update");

        let parent = store
            .show_task("run-graph-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "open");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_keeps_parent_open_with_blocked_run_graph() {
        let root = unique_task_store_temp_root("vida-close-child-blocked-run-graph");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            (
                "close-run-graph-parent",
                "Close run graph parent",
                "epic",
                None,
            ),
            (
                "close-run-graph-child",
                "Close run graph child",
                "todo",
                Some("close-run-graph-parent"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status: "open",
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task tree");
        }

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "close-run-graph-parent",
            "implementation",
            "implementation",
        );
        status.task_id = "close-run-graph-parent".to_string();
        status.active_node = "test_author".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.handoff_state = "delegated_lane_blocked".to_string();
        status.recovery_ready = false;
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run graph status");

        store
            .close_task("close-run-graph-child", "child completed")
            .await
            .expect("close child through task close");

        let parent = store
            .show_task("close-run-graph-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "open");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_close_auto_parent_validation_order_keeps_unready_ancestor_open() {
        let root = unique_task_store_temp_root("vida-close-leaf-unready-ancestor");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, status, parent_id) in [
            ("root-epic", "Root epic", "epic", "in_progress", None),
            (
                "runtime-epic",
                "Runtime epic",
                "epic",
                "in_progress",
                Some("root-epic"),
            ),
            (
                "route-parent",
                "Route parent",
                "task",
                "paused",
                Some("runtime-epic"),
            ),
            (
                "async-parent",
                "Async parent",
                "task",
                "paused",
                Some("route-parent"),
            ),
            (
                "timeout-leaf",
                "Timeout leaf",
                "defect",
                "paused",
                Some("async-parent"),
            ),
            (
                "active-sibling",
                "Active sibling",
                "defect",
                "in_progress",
                Some("runtime-epic"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status,
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task tree");
        }

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "runtime-epic",
            "implementation",
            "implementation",
        );
        status.task_id = "runtime-epic".to_string();
        status.active_node = "test_author".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "test_author_blocked".to_string();
        status.handoff_state = "delegated_lane_blocked".to_string();
        status.recovery_ready = false;
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked ancestor run graph status");

        store
            .close_task("timeout-leaf", "leaf completed")
            .await
            .expect("close leaf without speculative ancestor validation failure");

        let leaf = store.show_task("timeout-leaf").await.expect("load leaf");
        let async_parent = store
            .show_task("async-parent")
            .await
            .expect("load async parent");
        let route_parent = store
            .show_task("route-parent")
            .await
            .expect("load route parent");
        let runtime_epic = store
            .show_task("runtime-epic")
            .await
            .expect("load runtime epic");

        assert_eq!(leaf.status, "closed");
        assert_eq!(async_parent.status, "paused");
        assert_eq!(route_parent.status, "paused");
        assert_eq!(runtime_epic.status, "in_progress");
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_status_closed_keeps_parent_open_with_paused_sibling() {
        let root = unique_task_store_temp_root("vida-update-child-paused-sibling");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, status, parent_id) in [
            (
                "update-close-parent",
                "Update close parent",
                "epic",
                "open",
                None,
            ),
            (
                "update-close-child",
                "Update close child",
                "task",
                "open",
                Some("update-close-parent"),
            ),
            (
                "paused-sibling",
                "Paused sibling",
                "task",
                "paused",
                Some("update-close-parent"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status,
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task tree");
        }

        store
            .update_task(UpdateTaskRequest {
                task_id: "update-close-child",
                title: None,
                status: Some("closed"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("close child through update");

        let parent = store
            .show_task("update-close-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "open");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_status_closed_keeps_parent_open_when_blocked_by_non_parent_dependency() {
        let root = unique_task_store_temp_root("vida-update-child-parent-blocked");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("blocked-parent", "Blocked parent", "epic", None),
            ("blocking-task", "Blocking task", "epic", None),
            (
                "blocked-child",
                "Blocked child",
                "task",
                Some("blocked-parent"),
            ),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status: "open",
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create task");
        }

        store
            .add_task_dependency("blocked-parent", "blocking-task", "blocks", "test")
            .await
            .expect("add non-parent blocker dependency");

        store
            .update_task(UpdateTaskRequest {
                task_id: "blocked-child",
                title: None,
                status: Some("closed"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("close child through update");

        let parent = store
            .show_task("blocked-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "open");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        let _ = fs::remove_dir_all(&root);
    }

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
                issue_type: "epic",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
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
        let implementer_result_path = packet_dir.join("run-close-task-implementer-result.json");
        fs::write(
            &implementer_result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                },
                "execution_evidence": {
                    "status": "recorded",
                    "receipt_backed": true
                }
            }))
            .expect("encode implementer result"),
        )
        .expect("write implementer result");
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
                dispatch_result_path: Some(implementer_result_path.display().to_string()),
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

    #[cfg(unix)]
    #[test]
    fn export_writer_rejects_symlink_target() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-export-symlink-{}-{}",
            std::process::id(),
            nanos
        ));
        let exports_dir = root.join(".vida/exports");
        fs::create_dir_all(&exports_dir).expect("create exports dir");
        let target_path = exports_dir.join("tasks.snapshot.jsonl");
        let victim_path = root.join("victim");
        fs::write(&victim_path, "original").expect("write victim");
        symlink(&victim_path, &target_path).expect("create symlink");

        let error = StateStore::write_jsonl_export_file(&target_path, br#"{"id":"T-1"}"#)
            .expect_err("symlink write should be rejected");
        assert!(
            matches!(
                &error,
                StateStoreError::InvalidTaskRecord { reason }
                if reason.contains("refusing to write task export to symlink path")
            ) || matches!(
                &error,
                StateStoreError::Io(io_error) if io_error.raw_os_error() == Some(libc::ELOOP)
            )
        );
        let victim_after = fs::read_to_string(&victim_path).expect("read victim");
        assert_eq!(victim_after, "original");
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
                issue_type: "epic",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
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
                issue_type: "epic",
                status: "open",
                priority: 2,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
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
                issue_type: "epic",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
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

    #[tokio::test]
    async fn read_only_task_reads_default_planner_metadata_when_legacy_row_has_none() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-legacy-planner-metadata-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "legacy-planner-task",
                display_id: None,
                title: "Legacy planner task",
                description: "",
                status: "open",
                issue_type: "epic",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("legacy planner row should insert");
        let _ = store
            .db
            .query("UPDATE task:legacy-planner-task SET planner_metadata = NONE;")
            .await
            .expect("legacy row should downgrade planner metadata");

        drop(store);

        let reopened = StateStore::open_existing_read_only(root.clone())
            .await
            .expect("reopen store in read-only mode after legacy downgrade");
        let task = reopened
            .show_task("legacy-planner-task")
            .await
            .expect("legacy planner task should load");
        assert_eq!(task.planner_metadata, TaskPlannerMetadata::default());

        let tasks = reopened
            .all_tasks()
            .await
            .expect("legacy planner task should appear in all_tasks");
        let loaded = tasks
            .into_iter()
            .find(|task| task.id == "legacy-planner-task")
            .expect("legacy planner task should be present");
        assert_eq!(loaded.planner_metadata, TaskPlannerMetadata::default());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn open_existing_sanitizes_legacy_planner_metadata_nested_none_fields() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-legacy-planner-metadata-nested-none-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "legacy-planner-nested-none-task",
                display_id: None,
                title: "Legacy planner nested none task",
                description: "",
                status: "open",
                issue_type: "epic",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("legacy planner row should insert");
        let _ = store
            .db
            .query(
                "UPDATE task:legacy-planner-nested-none-task SET planner_metadata = { owned_paths: NONE, acceptance_targets: NONE, proof_targets: NONE, risk: NONE, estimate: NONE, lane_hint: NONE };",
            )
            .await
            .expect("legacy row should downgrade nested planner metadata");

        drop(store);

        let reopened = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store should sanitize nested planner metadata drift");
        let task = reopened
            .show_task("legacy-planner-nested-none-task")
            .await
            .expect("legacy planner nested none task should load");
        assert_eq!(task.planner_metadata, TaskPlannerMetadata::default());

        let tasks = reopened
            .all_tasks()
            .await
            .expect("legacy planner nested none task should appear in all_tasks");
        let loaded = tasks
            .into_iter()
            .find(|task| task.id == "legacy-planner-nested-none-task")
            .expect("legacy planner nested none task should be present");
        assert_eq!(loaded.planner_metadata, TaskPlannerMetadata::default());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn create_task_persists_structured_planner_metadata() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-planner-metadata-persist-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let planner_metadata = TaskPlannerMetadata {
            owned_paths: vec![
                "crates/vida/src/taskflow_plan_graph.rs".to_string(),
                "crates/vida/src/state_store_task_models.rs".to_string(),
            ],
            acceptance_targets: vec!["planner metadata is queryable".to_string()],
            proof_targets: vec!["cargo test -p vida taskflow_plan_graph".to_string()],
            risk: Some("medium".to_string()),
            estimate: Some("M".to_string()),
            lane_hint: Some("worker".to_string()),
        };
        let expected_planner_metadata = TaskPlannerMetadata {
            owned_paths: vec![
                "crates/vida/src/state_store_task_models.rs".to_string(),
                "crates/vida/src/taskflow_plan_graph.rs".to_string(),
            ],
            ..planner_metadata.clone()
        };

        store
            .create_task(CreateTaskRequest {
                task_id: "planner-metadata-task",
                title: "Planner metadata task",
                display_id: None,
                description: "structured planner metadata should persist",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: planner_metadata.clone(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create task with planner metadata");

        let loaded = store
            .show_task("planner-metadata-task")
            .await
            .expect("planner metadata task should load");
        assert_eq!(loaded.planner_metadata, expected_planner_metadata);

        let _ = fs::remove_dir_all(&root);
    }

    // ==================== Core Rule #12 Override Tests ====================
    // ==================== Cascading Closure Tests ====================
}
