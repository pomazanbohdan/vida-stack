use super::*;
use crate::state_store::state_store_task_models::{
    task_is_dev_pack_child, task_is_spec_first_feature_parent, task_is_spec_pack_child,
    task_is_work_pool_pack_child,
};
use serde_json::Deserializer;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use taskflow_authority::task_transition::{
    admit_task_lifecycle, lifecycle_status_from_str, TaskLifecycleAdmissionStatus,
    TaskLifecycleRuntimeEvidence,
};
use taskflow_core::task::aggregate::{
    ensure_task_mutation_plan_covers_persistence, plan_add_task_dependency, plan_close_task,
    plan_create_task, plan_remove_task_dependency, plan_reparent_tasks, plan_update_task_metadata,
    plan_update_task_status, TaskAggregateTaskSnapshot, TaskCloseCommand, TaskCreateCommand,
    TaskDependencyMutationCommand, TaskMetadataUpdateCommand, TaskMutationPlan,
    TaskReparentCommand, TaskStatusUpdateCommand,
};
use taskflow_core::task::lifecycle::{TaskLifecycleEvent, TaskLifecycleInput, TaskLifecycleStatus};

const TASK_SNAPSHOT_META_SCHEMA_VERSION: &str = "task-snapshot-meta-v1";
const TASK_SNAPSHOT_STATE_GENERATION_FILE: &str = ".task-snapshot-state-generation";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TaskSnapshotMeta {
    schema_version: String,
    snapshot_path: String,
    byte_len: u64,
    content_hash_blake3: String,
    task_count: usize,
    generated_at_unix_nanos: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state_generation_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClosedTaskRunReconciliation {
    pub(crate) run_id: String,
    pub(crate) task_id: String,
    pub(crate) previous_status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClosedTaskRunReconciliationSkipped {
    pub(crate) run_id: String,
    pub(crate) task_id: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) inspect_command: String,
}

fn run_graph_status_command(run_id: &str) -> String {
    format!(
        "vida taskflow run-graph status {}",
        crate::shell_quote(run_id.trim())
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpecFirstDevHandoffGate {
    pub(crate) feature_id: String,
    pub(crate) spec_task_id: String,
    pub(crate) work_pool_task_id: String,
    pub(crate) dev_task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskChildStatusEvidence {
    id: String,
    status: String,
    updated_at: String,
    closed_at: Option<String>,
}

impl TaskChildStatusEvidence {
    fn from_task(task: &TaskRecord) -> Self {
        Self {
            id: task.id.clone(),
            status: task.status.clone(),
            updated_at: task.updated_at.clone(),
            closed_at: task.closed_at.clone(),
        }
    }

    fn render_compact(&self) -> String {
        format!(
            "{}(status={}, updated_at={}, closed_at={})",
            self.id,
            self.status,
            self.updated_at,
            self.closed_at.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ClosedTaskRunReconciliationSummary {
    pub(crate) scanned_count: usize,
    pub(crate) reconciled_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) reconciled_runs: Vec<ClosedTaskRunReconciliation>,
    pub(crate) skipped_runs: Vec<ClosedTaskRunReconciliationSkipped>,
}

impl StateStore {
    pub(crate) const TASK_UPDATE_CLOSE_AUTHORITY_BLOCKER_CODE: &'static str =
        "task_update_close_authority_required";
    pub(crate) const TASK_UPDATE_CLOSED_TASK_MUTATION_BLOCKER_CODE: &'static str =
        "task_update_closed_task_mutation_requires_reopen";
    const TASK_UPDATE_CLOSE_AUTHORITY_REASON_PREFIX: &'static str =
        "task update close authority required for `";
    const TASK_UPDATE_CLOSE_AUTHORITY_REASON_SUFFIX: &'static str =
        "`: configured proof targets require `vida task close`";
    const TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_PREFIX: &'static str =
        "task update closed task mutation requires reopen for `";
    const TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_SUFFIX: &'static str =
        "`: reopen the task before mutating metadata";

    pub(crate) fn task_status_is_closed_like(status: &str) -> bool {
        taskflow_core::task_status_is_closed_like(status)
    }

    pub(crate) fn task_status_matches_filter(actual: &str, expected: &str) -> bool {
        let expected = expected.trim();
        if let Some(expected_canonical) = taskflow_core::canonical_task_status(expected) {
            return taskflow_core::canonical_task_status(actual)
                .is_some_and(|actual_canonical| actual_canonical == expected_canonical);
        }
        actual == expected
    }

    fn task_is_execution_step(task: &TaskRecord) -> bool {
        taskflow_core::issue_type_is_execution_step(&task.issue_type)
    }

    pub(crate) fn run_graph_status_is_terminal_closure(status: &RunGraphStatus) -> bool {
        status.is_terminal_closure()
    }

    pub(crate) fn run_graph_status_is_reconciled_terminal_closure(status: &RunGraphStatus) -> bool {
        status.is_reconciled_terminal_closure()
    }

    fn task_lifecycle_status_for_authority(
        task_id: &str,
        status: &str,
    ) -> Result<TaskLifecycleStatus, StateStoreError> {
        lifecycle_status_from_str(status).map_err(|blocker_code| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task `{task_id}` lifecycle status `{}` rejected by authority: {blocker_code}",
                    status.trim()
                ),
            }
        })
    }

    fn admit_task_lifecycle_for_store(
        task_id: &str,
        event: TaskLifecycleEvent,
        current_status: Option<TaskLifecycleStatus>,
        requested_status: Option<TaskLifecycleStatus>,
        active_child_count: usize,
    ) -> taskflow_authority::task_transition::TaskLifecycleAdmission {
        let mut input = TaskLifecycleInput::new(task_id, event);
        input.current_status = current_status;
        input.requested_status = requested_status;
        admit_task_lifecycle(
            input,
            TaskLifecycleRuntimeEvidence {
                active_child_count,
                graph_issues: Vec::new(),
                defer_lifecycle_mutation: false,
            },
        )
    }

    fn ensure_task_lifecycle_admitted(
        task_id: &str,
        admission: taskflow_authority::task_transition::TaskLifecycleAdmission,
        active_children: &[TaskChildStatusEvidence],
    ) -> Result<(), StateStoreError> {
        if admission.status == TaskLifecycleAdmissionStatus::Admitted {
            return Ok(());
        }
        if !active_children.is_empty() {
            let evidence = active_children
                .iter()
                .map(TaskChildStatusEvidence::render_compact)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "cannot close task `{task_id}` while non-closed child tasks exist: {evidence}"
                ),
            });
        }
        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "task `{task_id}` lifecycle mutation rejected by authority: {}",
                admission.blocker_codes.join(", ")
            ),
        })
    }

    fn ensure_task_mutation_plan_covers_persistence(
        operation: &str,
        plan: &TaskMutationPlan,
        persisted_task_ids: &BTreeSet<String>,
    ) -> Result<(), StateStoreError> {
        ensure_task_mutation_plan_covers_persistence(plan, persisted_task_ids).map_err(|error| {
            StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "{operation} aggregate plan does not cover persistence: {} expected=[{}] actual=[{}]",
                    error.blocker_code,
                    error.expected_task_ids.join(","),
                    error.actual_task_ids.join(",")
                ),
            }
        })
    }

    pub(crate) fn task_update_close_authority_task_id_from_reason(reason: &str) -> Option<&str> {
        let rest = reason.strip_prefix(Self::TASK_UPDATE_CLOSE_AUTHORITY_REASON_PREFIX)?;
        let (task_id, suffix) = rest.split_once(Self::TASK_UPDATE_CLOSE_AUTHORITY_REASON_SUFFIX)?;
        if suffix.is_empty() && !task_id.trim().is_empty() {
            Some(task_id)
        } else {
            None
        }
    }

    pub(crate) fn task_update_closed_task_mutation_task_id_from_reason(
        reason: &str,
    ) -> Option<&str> {
        let rest = reason.strip_prefix(Self::TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_PREFIX)?;
        let (task_id, suffix) =
            rest.split_once(Self::TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_SUFFIX)?;
        if suffix.is_empty() && !task_id.trim().is_empty() {
            Some(task_id)
        } else {
            None
        }
    }

    fn ensure_closed_task_update_requires_reopen(
        task: &TaskRecord,
        metadata_update_requested: bool,
        parent_update_requested: bool,
    ) -> Result<(), StateStoreError> {
        if Self::task_status_is_closed_like(&task.status)
            && (metadata_update_requested || parent_update_requested)
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "{}{}{}",
                    Self::TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_PREFIX,
                    task.id,
                    Self::TASK_UPDATE_CLOSED_TASK_MUTATION_REASON_SUFFIX
                ),
            });
        }
        Ok(())
    }

    fn task_update_close_authority_required(
        task: &TaskRecord,
        requested_planner_metadata: Option<&TaskPlannerMetadata>,
    ) -> bool {
        !task.planner_metadata.proof_targets.is_empty()
            || requested_planner_metadata.is_some_and(|metadata| !metadata.proof_targets.is_empty())
    }

    fn ensure_task_update_close_authority(
        task: &TaskRecord,
        requested_planner_metadata: Option<&TaskPlannerMetadata>,
    ) -> Result<(), StateStoreError> {
        if Self::task_update_close_authority_required(task, requested_planner_metadata) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "{}{}{}",
                    Self::TASK_UPDATE_CLOSE_AUTHORITY_REASON_PREFIX,
                    task.id,
                    Self::TASK_UPDATE_CLOSE_AUTHORITY_REASON_SUFFIX
                ),
            });
        }
        Ok(())
    }

    fn non_closed_child_status_evidence_for_task(
        tasks: &[TaskRecord],
        task_id: &str,
    ) -> Vec<TaskChildStatusEvidence> {
        let closed_child_ids = tasks
            .iter()
            .filter(|candidate| {
                candidate.id != task_id
                    && Self::task_status_is_closed_like(&candidate.status)
                    && candidate.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == task_id
                    })
            })
            .map(|candidate| candidate.id.as_str())
            .collect::<BTreeSet<_>>();
        tasks
            .iter()
            .filter(|candidate| {
                candidate.id != task_id
                    && !closed_child_ids.contains(candidate.id.as_str())
                    && !Self::task_status_is_closed_like(&candidate.status)
                    && !Self::task_is_execution_step(candidate)
                    && candidate.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == task_id
                    })
            })
            .map(TaskChildStatusEvidence::from_task)
            .collect()
    }

    pub(crate) async fn non_closed_child_status_evidence_for_task_live(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskChildStatusEvidence>, StateStoreError> {
        let all_tasks = self.all_tasks().await?;
        Ok(Self::non_closed_child_status_evidence_for_task(
            &all_tasks, task_id,
        ))
    }

    pub(crate) async fn run_graph_terminal_closure_has_task_close_truth(
        &self,
        status: &RunGraphStatus,
    ) -> Result<bool, StateStoreError> {
        if !Self::run_graph_status_is_terminal_closure(status) {
            return Ok(false);
        }
        if Self::run_graph_status_is_reconciled_terminal_closure(status) {
            return Ok(true);
        }
        self.task_close_reconcile_has_persisted_receipt_truth(&status.run_id, &status.task_id)
            .await
    }

    fn task_has_canonical_close_truth(task: &TaskRecord) -> bool {
        Self::task_status_is_closed_like(&task.status)
            && task
                .closed_at
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            && task
                .close_reason
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
    }

    pub(crate) fn parent_id_for_task(task: &TaskRecord) -> Option<String> {
        task.dependencies
            .iter()
            .find(|dependency| dependency.edge_type == "parent-child")
            .map(|dependency| dependency.depends_on_id.clone())
    }

    pub(crate) fn spec_first_work_pool_handoff_gate_satisfied_for_task(
        tasks: &[TaskRecord],
        task_id: &str,
    ) -> Option<String> {
        let mut current_task_id = Some(task_id.to_string());
        let mut visited = BTreeSet::new();

        while let Some(task_id) = current_task_id {
            if !visited.insert(task_id.clone()) {
                return None;
            }
            let Some(task) = tasks.iter().find(|task| task.id == task_id) else {
                break;
            };
            if task_is_spec_first_feature_parent(task)
                && Self::spec_first_parent_has_closed_spec_before_finished_work_pool(tasks, task)
            {
                return Some(task.id.clone());
            }
            current_task_id = Self::parent_id_for_task(task);
        }

        let candidates = tasks
            .iter()
            .filter(|task| {
                task_is_spec_first_feature_parent(task)
                    && Self::spec_first_parent_has_closed_spec_before_finished_work_pool(
                        tasks, task,
                    )
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [feature_id] => Some(feature_id.clone()),
            _ => None,
        }
    }

    fn spec_first_parent_has_closed_spec_before_finished_work_pool(
        tasks: &[TaskRecord],
        parent: &TaskRecord,
    ) -> bool {
        let children = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(parent.id.as_str()))
            .collect::<Vec<_>>();
        let has_closed_spec_pack_child = children.iter().any(|task| {
            task_is_spec_pack_child(task) && Self::task_status_is_closed_like(&task.status)
        });
        let has_closed_work_pool_child = children.iter().any(|task| {
            task_is_work_pool_pack_child(task) && Self::task_status_is_closed_like(&task.status)
        });

        has_closed_spec_pack_child && !has_closed_work_pool_child
    }

    pub(crate) fn spec_first_dev_handoff_gate_satisfied_for_task(
        tasks: &[TaskRecord],
        task_id: &str,
    ) -> Option<SpecFirstDevHandoffGate> {
        let mut current_task_id = Some(task_id.to_string());
        let mut visited = BTreeSet::new();

        while let Some(task_id) = current_task_id {
            if !visited.insert(task_id.clone()) {
                return None;
            }
            let Some(task) = tasks.iter().find(|task| task.id == task_id) else {
                break;
            };
            if task_is_spec_first_feature_parent(task) {
                if let Some(gate) = Self::spec_first_parent_dev_handoff_gate(tasks, task) {
                    return Some(gate);
                }
            }
            current_task_id = Self::parent_id_for_task(task);
        }

        let candidates = tasks
            .iter()
            .filter(|task| task_is_spec_first_feature_parent(task))
            .filter_map(|task| Self::spec_first_parent_dev_handoff_gate(tasks, task))
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [gate] => Some(gate.clone()),
            _ => None,
        }
    }

    fn spec_first_parent_dev_handoff_gate(
        tasks: &[TaskRecord],
        parent: &TaskRecord,
    ) -> Option<SpecFirstDevHandoffGate> {
        let children = tasks
            .iter()
            .filter(|task| Self::parent_id_for_task(task).as_deref() == Some(parent.id.as_str()))
            .collect::<Vec<_>>();
        let spec_task = children
            .iter()
            .filter(|task| {
                task_is_spec_pack_child(task) && Self::task_status_is_closed_like(&task.status)
            })
            .min_by_key(|task| task.id.as_str())?;
        let work_pool_task = children
            .iter()
            .filter(|task| {
                task_is_work_pool_pack_child(task) && Self::task_status_is_closed_like(&task.status)
            })
            .min_by_key(|task| task.id.as_str())?;
        let dev_task = children
            .iter()
            .filter(|task| {
                task_is_dev_pack_child(task) && !Self::task_status_is_closed_like(&task.status)
            })
            .filter(|task| {
                task.planner_metadata
                    .owned_paths
                    .iter()
                    .any(|path| !path.trim().is_empty())
                    || !task.planner_metadata.acceptance_targets.is_empty()
                    || !task.planner_metadata.proof_targets.is_empty()
            })
            .min_by_key(|task| task.id.as_str())?;

        Some(SpecFirstDevHandoffGate {
            feature_id: parent.id.clone(),
            spec_task_id: spec_task.id.clone(),
            work_pool_task_id: work_pool_task.id.clone(),
            dev_task_id: dev_task.id.clone(),
        })
    }

    pub(crate) async fn repair_stale_spec_first_parent_auto_close_for_work_pool_handoff(
        &self,
        tasks: &[TaskRecord],
        feature_id: &str,
    ) -> Result<bool, StateStoreError> {
        let Some(parent) = tasks.iter().find(|task| task.id == feature_id) else {
            return Ok(false);
        };
        if !Self::task_status_is_closed_like(&parent.status)
            || !task_is_spec_first_feature_parent(parent)
            || !Self::spec_first_parent_has_closed_spec_before_finished_work_pool(tasks, parent)
            || !parent.close_reason.as_deref().is_some_and(|reason| {
                reason.starts_with("all direct child tasks closed after closing")
            })
        {
            return Ok(false);
        }

        let mut repaired = parent.clone();
        repaired.status = "open".to_string();
        repaired.closed_at = None;
        repaired.close_reason = None;
        repaired.updated_at = unix_timestamp_nanos().to_string();
        self.persist_task_record(repaired).await?;
        Ok(true)
    }

    fn spec_first_parent_waits_for_work_pool_handoff(
        tasks: &[TaskRecord],
        parent_index: usize,
        child_indices: &[usize],
    ) -> bool {
        let parent = &tasks[parent_index];
        if !task_is_spec_first_feature_parent(parent) {
            return false;
        }

        let has_spec_pack_child = child_indices
            .iter()
            .any(|index| task_is_spec_pack_child(&tasks[*index]));
        let has_work_pool_child = child_indices
            .iter()
            .any(|index| task_is_work_pool_pack_child(&tasks[*index]));

        has_spec_pack_child && !has_work_pool_child
    }

    fn reopen_closed_parent_chain_for_extension(
        tasks: &mut [TaskRecord],
        child_issue_type: &str,
        parent_id: Option<&str>,
        now: &str,
    ) -> Vec<TaskRecord> {
        if taskflow_core::issue_type_is_execution_step(child_issue_type) {
            return Vec::new();
        }

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
                let admission = Self::admit_task_lifecycle_for_store(
                    &tasks[parent_index].id,
                    TaskLifecycleEvent::ExtendParent,
                    Self::task_lifecycle_status_for_authority(
                        &tasks[parent_index].id,
                        &tasks[parent_index].status,
                    )
                    .ok(),
                    Some(TaskLifecycleStatus::Open),
                    0,
                );
                if !admission.admitted() {
                    break;
                }
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
                    && !Self::task_is_execution_step(child)
                    && !tasks_being_closed.contains(&child.id)
            });

            if child_indices.is_empty() || has_non_closed_child_not_in_chain {
                break;
            }

            if !work_item_is_program_container(&tasks[parent_index].issue_type) {
                break;
            }

            if Self::spec_first_parent_waits_for_work_pool_handoff(
                tasks,
                parent_index,
                &child_indices,
            ) {
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
                let admission = Self::admit_task_lifecycle_for_store(
                    &tasks[parent_index].id,
                    TaskLifecycleEvent::EmptyParent,
                    Self::task_lifecycle_status_for_authority(
                        &tasks[parent_index].id,
                        &tasks[parent_index].status,
                    )
                    .ok(),
                    Some(TaskLifecycleStatus::Closed),
                    0,
                );
                if !admission.admitted() {
                    break;
                }
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
                    && !Self::task_is_execution_step(task)
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
                let admission = Self::admit_task_lifecycle_for_store(
                    &tasks[parent_index].id,
                    TaskLifecycleEvent::EmptyParent,
                    Self::task_lifecycle_status_for_authority(
                        &tasks[parent_index].id,
                        &tasks[parent_index].status,
                    )
                    .ok(),
                    Some(TaskLifecycleStatus::Closed),
                    0,
                );
                if !admission.admitted() {
                    break;
                }
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

        if let Some(project_root) =
            crate::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
        {
            let native_state_root =
                crate::taskflow_task_bridge::taskflow_native_state_root(&project_root);
            if state_root == native_state_root {
                return project_root.join(".vida/exports/tasks.snapshot.jsonl");
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

    pub(crate) fn canonical_task_snapshot_state_generation_path_for_state_root(
        state_root: &Path,
    ) -> PathBuf {
        state_root.join(TASK_SNAPSHOT_STATE_GENERATION_FILE)
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

    fn task_snapshot_state_generation(
        state_root: &Path,
    ) -> Result<Option<String>, StateStoreError> {
        let generation_path =
            Self::canonical_task_snapshot_state_generation_path_for_state_root(state_root);
        if Self::path_is_symlink(&generation_path) {
            return Err(Self::invalid_task_snapshot_reason(
                "refusing to read task snapshot state generation through symlink path",
            ));
        }
        match fs::read_to_string(generation_path) {
            Ok(raw) => {
                let generation = raw.trim();
                if generation.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(generation.to_string()))
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn ensure_task_snapshot_state_generation(state_root: &Path) -> Result<String, StateStoreError> {
        if let Some(generation) = Self::task_snapshot_state_generation(state_root)? {
            return Ok(generation);
        }
        let generation_path =
            Self::canonical_task_snapshot_state_generation_path_for_state_root(state_root);
        fs::create_dir_all(state_root)?;
        let generation = format!("{}-{}", unix_timestamp_nanos(), std::process::id());
        Self::write_jsonl_export_file(&generation_path, generation.as_bytes())?;
        Ok(generation)
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

        if let (Some(snapshot_generation), Some(authoritative_generation)) = (
            meta.state_generation_id.as_deref(),
            Self::task_snapshot_state_generation(state_root)?.as_deref(),
        ) {
            if snapshot_generation != authoritative_generation {
                return Err(Self::invalid_task_snapshot_reason(
                    "task snapshot state generation does not match authoritative state generation",
                ));
            }
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
        Self::write_jsonl_export_file_with_meta_for_state_root(
            &snapshot_path,
            body.as_bytes(),
            task_count,
            self.root(),
        )?;
        Ok(snapshot_path)
    }

    async fn build_task_close_reconciled_binding(
        &self,
        status: &RunGraphStatus,
        closed_task_id: &str,
    ) -> Result<Option<crate::state_store::RunGraphContinuationBinding>, StateStoreError> {
        if status.task_id == closed_task_id
            && (self
                .task_close_reconcile_has_persisted_receipt_truth(&status.run_id, closed_task_id)
                .await?
                || self
                    .task_close_reconcile_has_closed_task_execution_truth(status, closed_task_id)
                    .await?)
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

    async fn task_close_reconcile_has_closed_task_execution_truth(
        &self,
        status: &RunGraphStatus,
        closed_task_id: &str,
    ) -> Result<bool, StateStoreError> {
        let active_resume_target = format!("dispatch.{}", status.active_node.replace('-', "_"));
        let active_lane_resume_target = format!("{active_resume_target}_lane");
        let resume_target_is_terminal_or_same_lane = status.resume_target == "none"
            || status.resume_target == active_resume_target
            || status.resume_target == active_lane_resume_target;
        if status.task_id != closed_task_id
            || status.next_node.is_some()
            || status.handoff_state != "none"
            || !resume_target_is_terminal_or_same_lane
        {
            return Ok(false);
        }
        let task = match self.show_task(closed_task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        if !Self::task_has_canonical_close_truth(&task) {
            return Ok(false);
        }
        self.run_graph_dispatch_has_receipt_backed_execution_truth(&status.run_id)
            .await
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

    async fn retire_canonical_task_close_active_run(
        &self,
        task: &TaskRecord,
    ) -> Result<(), StateStoreError> {
        if !Self::task_has_canonical_close_truth(task) {
            return Ok(());
        }
        let Some(run_id) = self.latest_run_graph_run_id_for_task(&task.id).await? else {
            return Ok(());
        };
        let status = match self.run_graph_status(&run_id).await {
            Ok(status) => status,
            Err(StateStoreError::MissingTask { .. }) => return Ok(()),
            Err(error) => return Err(error),
        };
        if status.task_id != task.id
            || Self::run_graph_status_is_reconciled_terminal_closure(&status)
        {
            return Ok(());
        }
        if !self
            .task_close_reconcile_has_persisted_receipt_truth(&run_id, &task.id)
            .await?
        {
            return Ok(());
        }
        let retired_status =
            Self::task_close_retired_run_graph_status(status, "closed_task_stale_run_retired");
        self.record_reconciled_terminal_closure_run_graph_status(&retired_status)
            .await?;
        self.clear_run_graph_continuation_binding(&run_id).await
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
            "sequential" | "parallel_safe" | "exclusive" | "container_only" => Ok(Some(mode)),
            _ => Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task `{task_id}` execution_mode must be one of sequential, parallel_safe, exclusive, container_only"
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
        crate::runtime_assignment_policy::canonical_sorted_nonempty_strings(values)
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

    fn normalize_task_record_defaults(
        task_id: &str,
        execution_semantics: TaskExecutionSemantics,
        planner_metadata: TaskPlannerMetadata,
    ) -> Result<(TaskExecutionSemantics, TaskPlannerMetadata), StateStoreError> {
        Ok((
            Self::validate_execution_semantics(task_id, execution_semantics)?,
            Self::normalize_planner_metadata(planner_metadata),
        ))
    }

    fn normalize_stored_task_row(row: TaskStorageRowStored) -> TaskStorageRow {
        let mut normalized = TaskStorageRow::from(row);
        let execution_semantics = std::mem::take(&mut normalized.execution_semantics);
        let planner_metadata = std::mem::take(&mut normalized.planner_metadata);
        normalized.execution_semantics =
            Self::validate_execution_semantics(&normalized.task_id, execution_semantics)
                .unwrap_or_default();
        normalized.planner_metadata = Self::normalize_planner_metadata(planner_metadata);
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
            "vida taskflow consume continue --run-id {}",
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

    async fn refresh_spec_post_design_handoff_after_task_close(
        &self,
        run_id: &str,
        closed_task_id: &str,
    ) -> Result<(), StateStoreError> {
        let task = match self.show_task(closed_task_id).await {
            Ok(task) => task,
            Err(StateStoreError::MissingTask { .. }) => return Ok(()),
            Err(error) => return Err(error),
        };
        if !Self::task_status_is_closed_like(&task.status) || !task_is_spec_pack_child(&task) {
            return Ok(());
        }
        let Some(mut receipt) = self.run_graph_dispatch_receipt(run_id).await? else {
            return Ok(());
        };
        if receipt.dispatch_target != "specification"
            || receipt.dispatch_status != "executed"
            || receipt.downstream_dispatch_target.as_deref() != Some("work-pool-pack")
            || !receipt.downstream_dispatch_blockers.iter().any(|blocker| {
                matches!(
                    blocker.as_str(),
                    "pending_design_finalize" | "pending_spec_task_close"
                )
            })
        {
            return Ok(());
        }

        receipt.downstream_dispatch_blockers.retain(|blocker| {
            !matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )
        });
        receipt.downstream_dispatch_ready = receipt.downstream_dispatch_blockers.is_empty();
        receipt.downstream_dispatch_status = Some(if receipt.downstream_dispatch_ready {
            "packet_ready".to_string()
        } else {
            "blocked".to_string()
        });
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt.downstream_dispatch_note = Some(
            "spec-pack task close cleared design/spec blockers; continue with work-pool handoff"
                .to_string(),
        );
        receipt.downstream_dispatch_active_target = Some("specification".to_string());
        receipt.downstream_dispatch_last_target = Some("specification".to_string());
        self.record_run_graph_dispatch_receipt(&receipt).await?;

        let mut status = self.run_graph_status(run_id).await?;
        status.active_node = "specification".to_string();
        status.next_node = Some("work_pool_pack".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "specification_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "awaiting_work_pool_pack".to_string();
        status.resume_target = "dispatch.work_pool_pack_lane".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.recovery_ready = true;
        self.record_run_graph_status(&status).await?;
        Ok(())
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
            if matches!(
                binding.active_bounded_unit["kind"].as_str(),
                Some("task_graph_task" | "run_graph_task")
            ) && binding.active_bounded_unit["task_id"].as_str() == Some(task_id)
            {
                affected_run_ids.insert(binding.run_id);
            }
        }

        for run_id in affected_run_ids {
            self.refresh_spec_post_design_handoff_after_task_close(&run_id, task_id)
                .await?;
            let mut status = self.run_graph_status(&run_id).await?;
            let Some(binding) = self
                .build_task_close_reconciled_binding(&status, task_id)
                .await?
            else {
                self.clear_run_graph_continuation_binding(&run_id).await?;
                continue;
            };
            let closure_bound = binding.active_bounded_unit["kind"] == "downstream_dispatch_target"
                && binding.active_bounded_unit["dispatch_target"] == "closure";
            if closure_bound {
                if !self
                    .materialize_task_close_closure_artifacts(&status)
                    .await?
                {
                    continue;
                }
                status = Self::task_close_retired_run_graph_status(
                    status,
                    "closed_task_stale_run_retired",
                );
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
        let mut skipped_runs = Vec::new();
        let mut skipped_count = 0usize;

        for row in rows {
            let task = match self.show_task(&row.task_id).await {
                Ok(task) => task,
                Err(StateStoreError::MissingTask { .. }) => {
                    skipped_count += 1;
                    skipped_runs.push(ClosedTaskRunReconciliationSkipped {
                        inspect_command: run_graph_status_command(&row.run_id),
                        run_id: row.run_id,
                        task_id: row.task_id,
                        status: row.status,
                        reason: "missing_task".to_string(),
                    });
                    continue;
                }
                Err(error) => return Err(error),
            };
            let has_canonical_close_truth = Self::task_has_canonical_close_truth(&task);
            let has_closure_receipt_truth = self
                .task_close_reconcile_has_persisted_closure_receipt_truth(
                    &row.run_id,
                    &row.task_id,
                )
                .await?;
            let status = if row.status == "completed"
                && Self::task_status_is_closed_like(&task.status)
                && (has_canonical_close_truth || has_closure_receipt_truth)
            {
                self.run_graph_raw_status_from_task_rows(&row.run_id)
                    .await?
                    .0
            } else {
                self.run_graph_status(&row.run_id).await?
            };
            if row.status == "completed" {
                if Self::run_graph_status_is_terminal_closure(&status)
                    && !Self::run_graph_status_is_reconciled_terminal_closure(&status)
                {
                    if has_canonical_close_truth || has_closure_receipt_truth {
                        let retired_status = Self::task_close_retired_run_graph_status(
                            status,
                            "historical_closed_task_stale_run_retired",
                        );
                        self.record_reconciled_terminal_closure_run_graph_status(&retired_status)
                            .await?;
                        self.clear_run_graph_continuation_binding(&row.run_id)
                            .await?;
                        reconciled_runs.push(ClosedTaskRunReconciliation {
                            run_id: row.run_id,
                            task_id: row.task_id,
                            previous_status: row.status,
                        });
                        continue;
                    }
                    skipped_count += 1;
                    skipped_runs.push(ClosedTaskRunReconciliationSkipped {
                        inspect_command: run_graph_status_command(&row.run_id),
                        run_id: row.run_id,
                        task_id: row.task_id,
                        status: row.status,
                        reason: format!(
                            "terminal_closure_missing_reconciliation_truth:canonical_close_truth={has_canonical_close_truth},closure_receipt_truth={has_closure_receipt_truth}"
                        ),
                    });
                    continue;
                }
                skipped_count += 1;
                skipped_runs.push(ClosedTaskRunReconciliationSkipped {
                    inspect_command: run_graph_status_command(&row.run_id),
                    run_id: row.run_id,
                    task_id: row.task_id,
                    status: row.status,
                    reason: "already_completed".to_string(),
                });
                continue;
            }
            if !Self::task_status_is_closed_like(&task.status) {
                skipped_count += 1;
                skipped_runs.push(ClosedTaskRunReconciliationSkipped {
                    inspect_command: run_graph_status_command(&row.run_id),
                    run_id: row.run_id,
                    task_id: row.task_id,
                    status: row.status,
                    reason: format!("task_status_not_closed_like:{}", task.status),
                });
                continue;
            }
            if !has_closure_receipt_truth && !has_canonical_close_truth {
                skipped_count += 1;
                skipped_runs.push(ClosedTaskRunReconciliationSkipped {
                    inspect_command: run_graph_status_command(&row.run_id),
                    run_id: row.run_id,
                    task_id: row.task_id,
                    status: row.status,
                    reason: "missing_receipt_backed_closure_truth".to_string(),
                });
                continue;
            }
            let retired_status = Self::task_close_retired_run_graph_status(
                status,
                "historical_closed_task_stale_run_retired",
            );
            self.record_reconciled_terminal_closure_run_graph_status(&retired_status)
                .await?;
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
            skipped_runs,
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
            record.issue_type = canonical_work_item_issue_type(&record.issue_type);
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
        let import_root_task_ids = records
            .iter()
            .filter(|(_, record)| {
                !work_item_requires_parent(&record.issue_type)
                    && !Self::task_status_is_closed_like(&record.status)
            })
            .map(|(_, record)| record.id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();

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
            let has_parent_edge = record
                .dependencies
                .iter()
                .any(|dependency| dependency.edge_type == "parent-child");
            if !has_parent_edge
                && !Self::task_status_is_closed_like(&record.status)
                && work_item_requires_parent(&record.issue_type)
            {
                if let [root_task_id] = import_root_task_ids.as_slice() {
                    if root_task_id != &task_id {
                        record.dependencies.push(TaskDependencyJsonlRecord {
                            issue_id: task_id.clone(),
                            depends_on_id: root_task_id.clone(),
                            edge_type: "parent-child".to_string(),
                            created_at: record.updated_at.clone(),
                            created_by: record.created_by.clone(),
                            metadata: "{\"source\":\"single_root_jsonl_import_compat\"}"
                                .to_string(),
                            thread_id: String::new(),
                        });
                    }
                }
            }

            let normalized_display_id = self
                .validate_task_display_id_alias(&task_id, record.display_id.as_deref())
                .await?;
            let mut content = TaskContent::from(record);
            content.display_id = normalized_display_id;
            let (execution_semantics, planner_metadata) = Self::normalize_task_record_defaults(
                &task_id,
                std::mem::take(&mut content.execution_semantics),
                std::mem::take(&mut content.planner_metadata),
            )?;
            content.execution_semantics = execution_semantics;
            content.planner_metadata = planner_metadata;

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
        if target_path == Self::canonical_task_snapshot_path_for_state_root(self.root()) {
            Self::write_jsonl_export_file_with_meta_for_state_root(
                target_path,
                body.as_bytes(),
                task_count,
                self.root(),
            )?;
        } else {
            Self::write_jsonl_export_file_with_meta(target_path, body.as_bytes(), task_count)?;
        }
        Ok(task_count)
    }

    fn write_jsonl_export_file_with_meta(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
    ) -> Result<(), StateStoreError> {
        Self::write_jsonl_export_file_with_meta_and_generation(target_path, body, task_count, None)
    }

    fn write_jsonl_export_file_with_meta_for_state_root(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
        state_root: &Path,
    ) -> Result<(), StateStoreError> {
        let generation = Self::ensure_task_snapshot_state_generation(state_root)?;
        Self::write_jsonl_export_file_with_meta_and_generation(
            target_path,
            body,
            task_count,
            Some(&generation),
        )
    }

    fn write_jsonl_export_file_with_meta_and_generation(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
        state_generation_id: Option<&str>,
    ) -> Result<(), StateStoreError> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Self::write_jsonl_export_file(target_path, body)?;
        Self::write_task_snapshot_meta_file(target_path, body, task_count, state_generation_id)
    }

    fn write_task_snapshot_meta_file(
        target_path: &Path,
        body: &[u8],
        task_count: usize,
        state_generation_id: Option<&str>,
    ) -> Result<(), StateStoreError> {
        let meta_path = Self::task_snapshot_meta_path_for_snapshot_path(target_path);
        let meta = TaskSnapshotMeta {
            schema_version: TASK_SNAPSHOT_META_SCHEMA_VERSION.to_string(),
            snapshot_path: target_path.display().to_string(),
            byte_len: body.len() as u64,
            content_hash_blake3: blake3::hash(body).to_hex().to_string(),
            task_count,
            generated_at_unix_nanos: unix_timestamp_nanos().to_string(),
            state_generation_id: state_generation_id.map(str::to_string),
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
            if !include_closed && Self::task_status_is_closed_like(&task.status) {
                return false;
            }
            match status {
                Some(expected) => Self::task_status_matches_filter(&task.status, expected),
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
            .filter(|task| taskflow_core::task_status_is_open_like(&task.status))
            .filter(|task| !work_item_is_program_container(&task.issue_type))
            .filter_map(|task| {
                let blockers = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .filter_map(|dependency| {
                        let blocker_task = by_id.get(&dependency.depends_on_id)?;
                        if Self::task_status_is_closed_like(&blocker_task.status) {
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
            .filter(|task| taskflow_core::task_status_is_open_like(&task.status))
            .filter(|task| !work_item_is_program_container(&task.issue_type))
            .filter_map(|task| {
                let blockers = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .filter_map(|dependency| {
                        let blocker_task = by_id.get(&dependency.depends_on_id)?;
                        if Self::task_status_is_closed_like(&blocker_task.status) {
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

        let now = unix_timestamp_nanos().to_string();
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

        let dependency_plan = plan_add_task_dependency(TaskDependencyMutationCommand {
            task_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: edge_type.to_string(),
            occurred_at: now,
        });
        Self::ensure_task_mutation_plan_covers_persistence(
            "add_task_dependency",
            &dependency_plan,
            &touched_task_ids,
        )?;

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
            let dependency_plans = created
                .iter()
                .map(|dependency| {
                    plan_add_task_dependency(TaskDependencyMutationCommand {
                        task_id: dependency.issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
                        occurred_at: dependency.created_at.clone(),
                    })
                })
                .collect::<Vec<_>>();
            let planned_touched_task_ids = dependency_plans
                .iter()
                .flat_map(|plan| plan.touched_task_ids.iter().cloned())
                .collect::<BTreeSet<_>>();
            if planned_touched_task_ids != touched_task_ids {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "add_task_dependencies_bulk aggregate plan does not cover operation touched set: expected=[{}] actual=[{}]",
                        touched_task_ids
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(","),
                        planned_touched_task_ids
                            .into_iter()
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                });
            }
            for plan in &dependency_plans {
                let plan_touched_task_ids = plan
                    .touched_task_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                Self::ensure_task_mutation_plan_covers_persistence(
                    "add_task_dependencies_bulk",
                    plan,
                    &plan_touched_task_ids,
                )?;
            }
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
        let now = unix_timestamp_nanos().to_string();
        updated.updated_at = now.clone();
        let touched_task_ids = BTreeSet::from([issue_id.to_string(), depends_on_id.to_string()]);
        let dependency_plan = plan_remove_task_dependency(TaskDependencyMutationCommand {
            task_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: edge_type.to_string(),
            occurred_at: now,
        });
        Self::ensure_task_mutation_plan_covers_persistence(
            "remove_task_dependency",
            &dependency_plan,
            &touched_task_ids,
        )?;

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
        let stored_issue_type = canonical_work_item_issue_type(issue_type);
        if work_item_requires_parent(&stored_issue_type) && parent_id.is_none() {
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
        let (execution_semantics, planner_metadata) =
            Self::normalize_task_record_defaults(task_id, execution_semantics, planner_metadata)?;
        let requested_status = Self::task_lifecycle_status_for_authority(task_id, status)?;
        Self::ensure_task_lifecycle_admitted(
            task_id,
            Self::admit_task_lifecycle_for_store(
                task_id,
                TaskLifecycleEvent::Create,
                None,
                Some(requested_status),
                0,
            ),
            &[],
        )?;

        let now = unix_timestamp_nanos().to_string();
        let normalized_labels = crate::runtime_assignment_policy::canonical_sorted_nonempty_strings(
            labels.iter().cloned(),
        );

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
            issue_type: stored_issue_type,
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
        if Self::task_status_is_closed_like(status) {
            task.closed_at = Some(now.clone());
        }

        let reopened_parents = if !Self::task_status_is_closed_like(&task.status) {
            Self::reopen_closed_parent_chain_for_extension(
                &mut tasks,
                &task.issue_type,
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
                    "task creation would create invalid graph: {} on {}: {}",
                    first.issue_type, first.issue_id, first.detail
                ),
            });
        }

        let create_plan = plan_create_task(TaskCreateCommand {
            task: TaskAggregateTaskSnapshot {
                id: task.id.clone(),
                status: task.status.clone(),
                updated_at: task.updated_at.clone(),
                closed_at: task.closed_at.clone(),
                close_reason: task.close_reason.clone(),
                parent_id: normalized_parent_id.clone(),
            },
            occurred_at: task.updated_at.clone(),
            auto_reopened_parents: reopened_parents
                .iter()
                .map(|parent| TaskAggregateTaskSnapshot {
                    id: parent.id.clone(),
                    status: parent.status.clone(),
                    updated_at: parent.updated_at.clone(),
                    closed_at: parent.closed_at.clone(),
                    close_reason: parent.close_reason.clone(),
                    parent_id: Self::parent_id_for_task(parent),
                })
                .collect(),
        });
        let persisted_task_ids = reopened_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(std::iter::once(task.id.clone()))
            .collect::<BTreeSet<_>>();
        Self::ensure_task_mutation_plan_covers_persistence(
            "create_task",
            &create_plan,
            &persisted_task_ids,
        )?;

        for parent in &reopened_parents {
            self.persist_task_record(parent.clone()).await?;
        }
        self.persist_new_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn append_task_notes(
        &self,
        task_id: &str,
        separator: &str,
        message: &str,
    ) -> Result<TaskRecord, StateStoreError> {
        let trimmed_message = message.trim();
        if trimmed_message.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task `{task_id}` note append message cannot be empty"),
            });
        }

        for _ in 0..16 {
            let current = self.show_task(task_id).await?;
            let expected_updated_at = current.updated_at.clone();
            let appended_notes = match current.notes.as_deref() {
                Some(notes) if !notes.trim().is_empty() => {
                    format!("{}{}{}", notes, separator, trimmed_message)
                }
                _ => trimmed_message.to_string(),
            };
            let mut updated_at = unix_timestamp_nanos().to_string();
            if updated_at <= expected_updated_at {
                updated_at = expected_updated_at
                    .parse::<u128>()
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| format!("{expected_updated_at}-append"));
            }

            let mut query = self
                .db
                .query(
                    "UPDATE task
                     SET notes = $notes, updated_at = $updated_at
                     WHERE task_id = $task_id AND updated_at = $expected_updated_at
                     RETURN AFTER;",
                )
                .bind(("notes", appended_notes.clone()))
                .bind(("updated_at", updated_at))
                .bind(("task_id", task_id.to_string()))
                .bind(("expected_updated_at", expected_updated_at))
                .await?;
            let rows: Vec<TaskStorageRowStored> = query.take(0)?;
            if let Some(row) = rows.into_iter().next() {
                let task = TaskRecord::from(Self::normalize_stored_task_row(row));
                Self::touch_task_snapshot_state_marker(self.root());
                return Ok(task);
            }
        }

        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "task `{task_id}` note append could not commit after concurrent update retries"
            ),
        })
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
        let base_task_for_update = task.clone();
        let base_updated_at = task.updated_at.clone();
        let explicit_notes_replacement = notes.is_some();
        let mut metadata_update_requested = false;
        let mut parent_update_requested = false;
        let mut status_update_requested = false;
        if let Some(title) = title {
            let trimmed = title.trim();
            if trimmed.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!("task `{task_id}` title cannot be empty"),
                });
            }
            task.title = trimmed.to_string();
            metadata_update_requested = true;
        }
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            status_update_requested = true;
            let current_status =
                Self::task_lifecycle_status_for_authority(task_id, &task.status).ok();
            let requested_lifecycle_status =
                Self::task_lifecycle_status_for_authority(task_id, status)?;
            let requested_status_is_closed =
                requested_lifecycle_status == TaskLifecycleStatus::Closed;
            if requested_status_is_closed {
                Self::ensure_task_update_close_authority(&task, planner_metadata.as_ref())?;
                let non_closed_children = self
                    .non_closed_child_status_evidence_for_task_live(task_id)
                    .await?;
                Self::ensure_task_lifecycle_admitted(
                    task_id,
                    Self::admit_task_lifecycle_for_store(
                        task_id,
                        TaskLifecycleEvent::Close,
                        current_status,
                        Some(TaskLifecycleStatus::Closed),
                        non_closed_children.len(),
                    ),
                    &non_closed_children,
                )?;
            } else {
                Self::ensure_task_lifecycle_admitted(
                    task_id,
                    Self::admit_task_lifecycle_for_store(
                        task_id,
                        TaskLifecycleEvent::UpdateStatus,
                        current_status,
                        Some(requested_lifecycle_status),
                        0,
                    ),
                    &[],
                )?;
            }
            task.status = status.to_string();
            if requested_status_is_closed {
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
            metadata_update_requested = true;
        }
        if let Some(notes) = notes {
            task.notes = Some(notes.to_string());
            metadata_update_requested = true;
        }
        if let Some(description) = description {
            task.description = description.to_string();
            metadata_update_requested = true;
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
            parent_update_requested =
                Self::parent_id_for_task(&base_task_for_update) != Self::parent_id_for_task(&task);
        }
        if let Some(set_labels) = set_labels {
            task.labels = crate::runtime_assignment_policy::canonical_sorted_nonempty_strings(
                set_labels.iter().cloned(),
            );
            metadata_update_requested = true;
        }
        for label in add_labels {
            let label = label.trim();
            if label.is_empty() || task.labels.iter().any(|existing| existing == label) {
                continue;
            }
            task.labels.push(label.to_string());
            metadata_update_requested = true;
        }
        if !remove_labels.is_empty() {
            task.labels
                .retain(|label| !remove_labels.iter().any(|remove| remove == label));
            metadata_update_requested = true;
        }
        if let Some(execution_mode) = execution_mode {
            task.execution_semantics.execution_mode =
                Self::validate_execution_mode(task_id, execution_mode)?;
            metadata_update_requested = true;
        }
        if let Some(order_bucket) = order_bucket {
            task.execution_semantics.order_bucket =
                Self::normalize_execution_semantics_value(order_bucket);
            metadata_update_requested = true;
        }
        if let Some(parallel_group) = parallel_group {
            task.execution_semantics.parallel_group =
                Self::normalize_execution_semantics_value(parallel_group);
            metadata_update_requested = true;
        }
        if let Some(conflict_domain) = conflict_domain {
            task.execution_semantics.conflict_domain =
                Self::normalize_execution_semantics_value(conflict_domain);
            metadata_update_requested = true;
        }
        if let Some(planner_metadata) = planner_metadata {
            task.planner_metadata = planner_metadata;
            metadata_update_requested = true;
        }
        Self::ensure_closed_task_update_requires_reopen(
            &base_task_for_update,
            metadata_update_requested,
            parent_update_requested,
        )?;
        let (execution_semantics, planner_metadata) = Self::normalize_task_record_defaults(
            task_id,
            std::mem::take(&mut task.execution_semantics),
            std::mem::take(&mut task.planner_metadata),
        )?;
        task.execution_semantics = execution_semantics;
        task.planner_metadata = planner_metadata;
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
        let task_is_closed = Self::task_status_is_closed_like(&task.status);
        let (reopened_parents, closed_parents) = if task_is_closed {
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
                    &task.issue_type,
                    parent_id.as_deref(),
                    &task.updated_at,
                ),
                Vec::new(),
            )
        };
        let closed_parents = self
            .filter_auto_closed_parents_ready_for_close(closed_parents)
            .await?;
        let mut touched_task_ids = reopened_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(closed_parents.iter().map(|parent| parent.id.clone()))
            .chain(std::iter::once(task.id.clone()))
            .collect::<BTreeSet<_>>();
        if parent_update_requested {
            if let Some(parent_id) = Self::parent_id_for_task(&base_task_for_update) {
                touched_task_ids.insert(parent_id);
            }
            if let Some(parent_id) = Self::parent_id_for_task(&task) {
                touched_task_ids.insert(parent_id);
            }
        }
        let issues =
            Self::validate_task_graph_rows_for_mutation(&original_tasks, &tasks, &touched_task_ids);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task update would create invalid graph: {} on {}: {}",
                    first.issue_type, first.issue_id, first.detail
                ),
            });
        }
        let mut update_plans = Vec::new();
        if status_update_requested {
            let status_plan = plan_update_task_status(TaskStatusUpdateCommand {
                task: TaskAggregateTaskSnapshot {
                    id: task.id.clone(),
                    status: task.status.clone(),
                    updated_at: task.updated_at.clone(),
                    closed_at: task.closed_at.clone(),
                    close_reason: task.close_reason.clone(),
                    parent_id: Self::parent_id_for_task(&task),
                },
                occurred_at: task.updated_at.clone(),
                auto_closed_parents: closed_parents
                    .iter()
                    .map(|parent| TaskAggregateTaskSnapshot {
                        id: parent.id.clone(),
                        status: parent.status.clone(),
                        updated_at: parent.updated_at.clone(),
                        closed_at: parent.closed_at.clone(),
                        close_reason: parent.close_reason.clone(),
                        parent_id: Self::parent_id_for_task(parent),
                    })
                    .collect(),
                auto_reopened_parents: reopened_parents
                    .iter()
                    .map(|parent| TaskAggregateTaskSnapshot {
                        id: parent.id.clone(),
                        status: parent.status.clone(),
                        updated_at: parent.updated_at.clone(),
                        closed_at: parent.closed_at.clone(),
                        close_reason: parent.close_reason.clone(),
                        parent_id: Self::parent_id_for_task(parent),
                    })
                    .collect(),
            });
            let plan_touched_task_ids = status_plan
                .touched_task_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            Self::ensure_task_mutation_plan_covers_persistence(
                "update_task_status",
                &status_plan,
                &plan_touched_task_ids,
            )?;
            update_plans.push(status_plan);
        }
        if parent_update_requested {
            let old_parent_id = Self::parent_id_for_task(&base_task_for_update);
            let new_parent_id = Self::parent_id_for_task(&task);
            if old_parent_id != new_parent_id {
                let reparent_plan = plan_reparent_tasks(TaskReparentCommand {
                    moved_tasks: vec![TaskAggregateTaskSnapshot {
                        id: task.id.clone(),
                        status: task.status.clone(),
                        updated_at: task.updated_at.clone(),
                        closed_at: task.closed_at.clone(),
                        close_reason: task.close_reason.clone(),
                        parent_id: new_parent_id.clone(),
                    }],
                    from_parent_id: old_parent_id.unwrap_or_default(),
                    to_parent_id: new_parent_id.unwrap_or_default(),
                    occurred_at: task.updated_at.clone(),
                    auto_closed_parents: Vec::new(),
                });
                let plan_touched_task_ids = reparent_plan
                    .touched_task_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                Self::ensure_task_mutation_plan_covers_persistence(
                    "update_task_parent",
                    &reparent_plan,
                    &plan_touched_task_ids,
                )?;
                update_plans.push(reparent_plan);
            }
        }
        if (metadata_update_requested || update_plans.is_empty()) && !status_update_requested {
            let metadata_plan = plan_update_task_metadata(TaskMetadataUpdateCommand {
                task_id: task.id.clone(),
                occurred_at: task.updated_at.clone(),
            });
            let plan_touched_task_ids = metadata_plan
                .touched_task_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            Self::ensure_task_mutation_plan_covers_persistence(
                "update_task_metadata",
                &metadata_plan,
                &plan_touched_task_ids,
            )?;
            update_plans.push(metadata_plan);
        }
        let planned_touched_task_ids = update_plans
            .iter()
            .flat_map(|plan| plan.touched_task_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        if planned_touched_task_ids != touched_task_ids {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "update_task aggregate plan does not cover operation touched set: expected=[{}] actual=[{}]",
                    touched_task_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                    planned_touched_task_ids
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            });
        }
        for parent in &reopened_parents {
            self.persist_task_record(parent.clone()).await?;
        }
        let task = if status_update_requested || explicit_notes_replacement {
            self.persist_task_record(task.clone()).await?;
            task
        } else {
            self.persist_task_update_record_preserving_latest_notes(
                task,
                base_updated_at,
                &base_task_for_update,
            )
            .await?
        };
        for parent in &closed_parents {
            self.persist_task_record(parent.clone()).await?;
            self.refresh_run_graph_continuation_after_task_close(&parent.id)
                .await?;
        }
        if task_is_closed {
            self.refresh_run_graph_continuation_after_task_close(task_id)
                .await?;
        }
        Ok(task)
    }

    fn task_records_match_except_notes_and_updated_at(
        left: &TaskRecord,
        right: &TaskRecord,
    ) -> bool {
        let mut left = left.clone();
        let mut right = right.clone();
        left.notes = None;
        right.notes = None;
        left.updated_at.clear();
        right.updated_at.clear();
        left == right
    }

    async fn persist_task_record_if_updated_at_matches(
        &self,
        task: &TaskRecord,
        expected_updated_at: &str,
    ) -> Result<Option<TaskRecord>, StateStoreError> {
        let task_id = task.id.clone();
        let row = TaskStorageRow::from(task.clone());
        let mut query = self
            .db
            .query(
                "UPDATE task
                 CONTENT $row
                 WHERE task_id = $task_id AND updated_at = $expected_updated_at
                 RETURN AFTER;",
            )
            .bind(("row", row))
            .bind(("task_id", task_id.clone()))
            .bind(("expected_updated_at", expected_updated_at.to_string()))
            .await?;
        let rows: Vec<TaskStorageRowStored> = query.take(0)?;
        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };
        let task = TaskRecord::from(Self::normalize_stored_task_row(row));
        self.replace_task_dependency_rows(&task_id, &task.dependencies)
            .await?;
        Self::touch_task_snapshot_state_marker(self.root());
        Ok(Some(task))
    }

    async fn persist_task_update_record_preserving_latest_notes(
        &self,
        mut task: TaskRecord,
        mut expected_updated_at: String,
        base_task_for_update: &TaskRecord,
    ) -> Result<TaskRecord, StateStoreError> {
        for _ in 0..16 {
            if let Some(persisted) = self
                .persist_task_record_if_updated_at_matches(&task, &expected_updated_at)
                .await?
            {
                return Ok(persisted);
            }
            let latest = self.show_task(&task.id).await?;
            if !Self::task_records_match_except_notes_and_updated_at(&latest, base_task_for_update)
            {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!(
                        "task `{}` changed during update; retry metadata update from latest authoritative row",
                        task.id
                    ),
                });
            }
            expected_updated_at = latest.updated_at.clone();
            task.notes = latest.notes.clone();
            task.updated_at = unix_timestamp_nanos().to_string();
            if task.updated_at <= expected_updated_at {
                task.updated_at = expected_updated_at
                    .parse::<u128>()
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| format!("{expected_updated_at}-update"));
            }
        }

        Err(StateStoreError::InvalidTaskRecord {
            reason: format!(
                "task `{}` metadata update could not commit after concurrent note append retries",
                task.id
            ),
        })
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
            let current_status =
                Self::task_lifecycle_status_for_authority(&task.id, &task.status).ok();
            Self::ensure_task_lifecycle_admitted(
                &task.id,
                Self::admit_task_lifecycle_for_store(
                    &task.id,
                    TaskLifecycleEvent::Reparent,
                    current_status,
                    None,
                    0,
                ),
                &[],
            )?;
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

        let reparent_plan = plan_reparent_tasks(TaskReparentCommand {
            moved_tasks: moved_tasks
                .iter()
                .map(|task| TaskAggregateTaskSnapshot {
                    id: task.id.clone(),
                    status: task.status.clone(),
                    updated_at: task.updated_at.clone(),
                    closed_at: task.closed_at.clone(),
                    close_reason: task.close_reason.clone(),
                    parent_id: Self::parent_id_for_task(task),
                })
                .collect(),
            from_parent_id: from_parent_id.to_string(),
            to_parent_id: to_parent_id.to_string(),
            occurred_at: now.clone(),
            auto_closed_parents: closed_parents
                .iter()
                .map(|parent| TaskAggregateTaskSnapshot {
                    id: parent.id.clone(),
                    status: parent.status.clone(),
                    updated_at: parent.updated_at.clone(),
                    closed_at: parent.closed_at.clone(),
                    close_reason: parent.close_reason.clone(),
                    parent_id: Self::parent_id_for_task(parent),
                })
                .collect(),
        });
        Self::ensure_task_mutation_plan_covers_persistence(
            "reparent_children",
            &reparent_plan,
            &touched_task_ids,
        )?;

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
                let current_status =
                    Self::task_lifecycle_status_for_authority(&task.id, &task.status).ok();
                Self::ensure_task_lifecycle_admitted(
                    &task.id,
                    Self::admit_task_lifecycle_for_store(
                        &task.id,
                        TaskLifecycleEvent::Reparent,
                        current_status,
                        None,
                        0,
                    ),
                    &[],
                )?;
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
                let current_status =
                    Self::task_lifecycle_status_for_authority(&task.id, &task.status).ok();
                Self::ensure_task_lifecycle_admitted(
                    &task.id,
                    Self::admit_task_lifecycle_for_store(
                        &task.id,
                        TaskLifecycleEvent::UpdateStatus,
                        current_status,
                        Some(TaskLifecycleStatus::Paused),
                        0,
                    ),
                    &[],
                )?;
                task.status = "paused".to_string();
                task.closed_at = None;
                task.close_reason = None;
                changed = true;
            } else if start_set.contains(&task.id) {
                let current_status =
                    Self::task_lifecycle_status_for_authority(&task.id, &task.status).ok();
                Self::ensure_task_lifecycle_admitted(
                    &task.id,
                    Self::admit_task_lifecycle_for_store(
                        &task.id,
                        TaskLifecycleEvent::UpdateStatus,
                        current_status,
                        Some(TaskLifecycleStatus::InProgress),
                        0,
                    ),
                    &[],
                )?;
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

        let reparent_plan = if moved_set.is_empty() {
            None
        } else {
            Some(plan_reparent_tasks(TaskReparentCommand {
                moved_tasks: changed_tasks
                    .iter()
                    .filter(|task| moved_set.contains(&task.id))
                    .map(|task| TaskAggregateTaskSnapshot {
                        id: task.id.clone(),
                        status: task.status.clone(),
                        updated_at: task.updated_at.clone(),
                        closed_at: task.closed_at.clone(),
                        close_reason: task.close_reason.clone(),
                        parent_id: Self::parent_id_for_task(task),
                    })
                    .collect(),
                from_parent_id: from_parent_id.to_string(),
                to_parent_id: to_parent_id.to_string(),
                occurred_at: now.clone(),
                auto_closed_parents: Vec::new(),
            }))
        };
        if let Some(reparent_plan) = &reparent_plan {
            let plan_touched_task_ids = reparent_plan
                .touched_task_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            Self::ensure_task_mutation_plan_covers_persistence(
                "defect_batch_rehome_reparent",
                reparent_plan,
                &plan_touched_task_ids,
            )?;
        }

        let status_plans = changed_tasks
            .iter()
            .filter(|task| pause_set.contains(&task.id) || start_set.contains(&task.id))
            .map(|task| {
                plan_update_task_status(TaskStatusUpdateCommand {
                    task: TaskAggregateTaskSnapshot {
                        id: task.id.clone(),
                        status: task.status.clone(),
                        updated_at: task.updated_at.clone(),
                        closed_at: task.closed_at.clone(),
                        close_reason: task.close_reason.clone(),
                        parent_id: Self::parent_id_for_task(task),
                    },
                    occurred_at: task.updated_at.clone(),
                    auto_closed_parents: Vec::new(),
                    auto_reopened_parents: Vec::new(),
                })
            })
            .collect::<Vec<_>>();
        for plan in &status_plans {
            let plan_touched_task_ids = plan
                .touched_task_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            Self::ensure_task_mutation_plan_covers_persistence(
                "defect_batch_rehome_status",
                plan,
                &plan_touched_task_ids,
            )?;
        }
        let planned_touched_task_ids = reparent_plan
            .iter()
            .flat_map(|plan| plan.touched_task_ids.iter().cloned())
            .chain(
                status_plans
                    .iter()
                    .flat_map(|plan| plan.touched_task_ids.iter().cloned()),
            )
            .collect::<BTreeSet<_>>();
        if planned_touched_task_ids != touched_task_ids {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "defect_batch_rehome aggregate plan does not cover operation touched set: expected=[{}] actual=[{}]",
                    touched_task_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                    planned_touched_task_ids
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(",")
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
        let non_closed_children = self
            .non_closed_child_status_evidence_for_task_live(task_id)
            .await?;
        let tasks = self.all_tasks().await?;

        let mut task = self.show_task(task_id).await?;
        let current_status = Self::task_lifecycle_status_for_authority(task_id, &task.status).ok();
        Self::ensure_task_lifecycle_admitted(
            task_id,
            Self::admit_task_lifecycle_for_store(
                task_id,
                TaskLifecycleEvent::Close,
                current_status,
                Some(TaskLifecycleStatus::Closed),
                non_closed_children.len(),
            ),
            &non_closed_children,
        )?;
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
        let close_plan = plan_close_task(TaskCloseCommand {
            task: TaskAggregateTaskSnapshot {
                id: task.id.clone(),
                status: task.status.clone(),
                updated_at: task.updated_at.clone(),
                closed_at: task.closed_at.clone(),
                close_reason: task.close_reason.clone(),
                parent_id,
            },
            reason: reason.to_string(),
            occurred_at: task.updated_at.clone(),
            auto_closed_parents: closed_parents
                .iter()
                .map(|parent| TaskAggregateTaskSnapshot {
                    id: parent.id.clone(),
                    status: parent.status.clone(),
                    updated_at: parent.updated_at.clone(),
                    closed_at: parent.closed_at.clone(),
                    close_reason: parent.close_reason.clone(),
                    parent_id: Self::parent_id_for_task(parent),
                })
                .collect(),
        });
        let persisted_task_ids = closed_parents
            .iter()
            .map(|parent| parent.id.clone())
            .chain(std::iter::once(task.id.clone()))
            .collect::<BTreeSet<_>>();
        Self::ensure_task_mutation_plan_covers_persistence(
            "close_task",
            &close_plan,
            &persisted_task_ids,
        )?;
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
        self.retire_canonical_task_close_active_run(&task).await?;
        for parent in &closed_parents {
            self.retire_canonical_task_close_active_run(parent).await?;
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

    async fn close_store_and_remove_root(store: StateStore, root: PathBuf) {
        store.close().await;
        let _ = fs::remove_dir_all(root);
    }

    struct TestProxyStateDirOverrideGuard;

    impl TestProxyStateDirOverrideGuard {
        fn install(path: PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for TestProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
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

    fn test_task_dependency(
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
    ) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: edge_type.to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    #[test]
    fn closed_child_row_wins_over_stale_in_progress_duplicate_for_close_admission() {
        let parent = test_task_record("parent", "epic", "in_progress");
        let mut closed = test_task_record("child", "task", "closed");
        closed.dependencies = vec![test_task_dependency("child", "parent", "parent-child")];
        let mut stale = test_task_record("child", "task", "in_progress");
        stale.updated_at = "2".to_string();
        stale.dependencies = vec![test_task_dependency("child", "parent", "parent-child")];

        let evidence = StateStore::non_closed_child_status_evidence_for_task(
            &[parent, stale, closed],
            "parent",
        );
        assert!(evidence.is_empty());
    }

    #[test]
    fn task_store_status_policy_uses_canonical_closed_aliases() {
        for alias in [
            "done",
            "closed",
            "complete",
            "completed",
            "resolved",
            "merged",
        ] {
            assert!(
                StateStore::task_status_is_closed_like(alias),
                "{alias} should be closed-like"
            );
            assert!(
                StateStore::task_status_matches_filter(alias, "closed"),
                "{alias} should match the canonical closed filter"
            );
        }

        for alias in ["open", "in_progress", "paused", "blocked", "cancelled"] {
            assert!(
                !StateStore::task_status_is_closed_like(alias),
                "{alias} should not be closed-like"
            );
        }
    }

    #[tokio::test]
    async fn list_tasks_excludes_and_filters_closed_aliases_canonically() {
        let root = unique_task_store_temp_root("vida-list-closed-aliases");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, status) in [
            ("open-epic", "open"),
            ("done-epic", "done"),
            ("resolved-epic", "resolved"),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title: task_id,
                    display_id: None,
                    description: "",
                    issue_type: "epic",
                    status,
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
        }

        let visible = store
            .list_tasks(None, false)
            .await
            .expect("list visible tasks")
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();
        assert_eq!(visible, vec!["open-epic"]);

        let closed = store
            .list_tasks(Some("closed"), true)
            .await
            .expect("list closed tasks")
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();
        assert_eq!(closed, vec!["done-epic", "resolved-epic"]);

        close_store_and_remove_root(store, root).await;
    }

    #[test]
    fn blocked_tasks_from_rows_treats_closed_alias_blockers_as_resolved() {
        let mut candidate = test_task_record("candidate", "task", "open");
        candidate.dependencies.push(test_task_dependency(
            "candidate",
            "resolved-blocker",
            "blocks",
        ));
        let resolved_blocker = test_task_record("resolved-blocker", "task", "resolved");

        assert!(
            StateStore::blocked_tasks_from_rows(&[candidate.clone(), resolved_blocker]).is_empty()
        );

        candidate.dependencies.clear();
        candidate.dependencies.push(test_task_dependency(
            "candidate",
            "unknown-blocker",
            "blocks",
        ));
        let unknown_blocker = test_task_record("unknown-blocker", "task", "cancelled");

        let blocked = StateStore::blocked_tasks_from_rows(&[candidate, unknown_blocker]);
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].blockers[0].dependency_status, "cancelled");
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

    #[test]
    fn fresh_task_snapshot_metadata_rejects_restored_state_generation_mismatch() {
        let state_root = unique_task_store_temp_root("vida-task-snapshot-state-generation")
            .join(".vida")
            .join("data")
            .join("state");
        let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_root);
        let generation_path =
            StateStore::canonical_task_snapshot_state_generation_path_for_state_root(&state_root);
        let body = sample_snapshot_body();
        fs::create_dir_all(&state_root).expect("state root should exist");
        fs::write(&generation_path, "checkpoint-generation-a")
            .expect("checkpoint generation should write");
        StateStore::write_jsonl_export_file_with_meta_for_state_root(
            &snapshot_path,
            body.as_bytes(),
            1,
            &state_root,
        )
        .expect("snapshot and generation metadata should write");
        fs::write(&generation_path, "checkpoint-generation-b")
            .expect("restored generation should write");

        let error = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect_err("snapshot from another state generation must reject");
        assert!(error.to_string().contains(
            "task snapshot state generation does not match authoritative state generation"
        ));
        let _ = fs::remove_dir_all(
            state_root
                .ancestors()
                .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("state"))
                .unwrap_or(&state_root),
        );
    }

    #[test]
    fn fresh_task_snapshot_metadata_preserves_missing_generation_marker_recovery() {
        let state_root = unique_task_store_temp_root("vida-task-snapshot-generation-recovery")
            .join(".vida")
            .join("data")
            .join("state");
        let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_root);
        let generation_path =
            StateStore::canonical_task_snapshot_state_generation_path_for_state_root(&state_root);
        let body = sample_snapshot_body();
        StateStore::write_jsonl_export_file_with_meta_for_state_root(
            &snapshot_path,
            body.as_bytes(),
            1,
            &state_root,
        )
        .expect("snapshot and generation metadata should write");
        fs::remove_file(&generation_path).expect("generation marker should be removable");

        let rows = StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_root)
            .expect("missing generation marker should use legacy freshness recovery");
        assert_eq!(rows.len(), 1);
        let _ = fs::remove_dir_all(
            state_root
                .ancestors()
                .find(|path| path.file_name().and_then(|name| name.to_str()) != Some("state"))
                .unwrap_or(&state_root),
        );
    }

    #[tokio::test]
    async fn refresh_task_snapshot_persists_state_generation_identity() {
        let root = unique_task_store_temp_root("vida-task-snapshot-generation-writer");
        let store = StateStore::open(root.clone())
            .await
            .expect("store should open");
        store
            .persist_task_record(test_task_record("snapshot-generation-task", "task", "open"))
            .await
            .expect("task should persist");

        let snapshot_path = store
            .refresh_task_snapshot()
            .await
            .expect("canonical snapshot should refresh");
        let meta_path = StateStore::task_snapshot_meta_path_for_snapshot_path(&snapshot_path);
        let meta: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(meta_path).expect("snapshot metadata"))
                .expect("snapshot metadata should be valid JSON");
        assert!(meta["state_generation_id"].as_str().is_some());

        let generation_path =
            StateStore::canonical_task_snapshot_state_generation_path_for_state_root(&root);
        fs::write(&generation_path, "restored-generation")
            .expect("restored generation should write");
        let error = StateStore::read_fresh_tasks_from_jsonl_snapshot(&root)
            .expect_err("restored generation must reject the pre-restore snapshot");
        assert!(error.to_string().contains(
            "task snapshot state generation does not match authoritative state generation"
        ));

        store.close().await;
        let _ = fs::remove_dir_all(root);
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
        close_store_and_remove_root(store, root).await;
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

        assert_eq!(task.issue_type, "epic");
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn append_task_notes_then_update_metadata_preserves_notes() {
        let root = unique_task_store_temp_root("vida-note-update-preserves-notes");
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
                task_id: "metadata-task",
                title: "Metadata task",
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
            .append_task_notes("metadata-task", "", "initial note")
            .await
            .expect("append initial notes");
        store
            .append_task_notes("metadata-task", "\n\n", "appended evidence")
            .await
            .expect("append notes");
        let mut planner_metadata = TaskPlannerMetadata::default();
        planner_metadata.owned_paths =
            vec!["crates/vida/src/state_store_task_store.rs".to_string()];
        let updated = store
            .update_task(UpdateTaskRequest {
                task_id: "metadata-task",
                title: None,
                status: None,
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
                planner_metadata: Some(planner_metadata),
            })
            .await
            .expect("metadata update should preserve notes");

        assert_eq!(
            updated.notes.as_deref(),
            Some("initial note\n\nappended evidence")
        );
        assert_eq!(
            updated.planner_metadata.owned_paths,
            vec!["crates/vida/src/state_store_task_store.rs".to_string()]
        );
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn stale_metadata_update_merges_append_only_notes_before_persist() {
        let root = unique_task_store_temp_root("vida-stale-note-update-preserves-notes");
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
                task_id: "stale-metadata-task",
                title: "Stale metadata task",
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
            .append_task_notes("stale-metadata-task", "", "initial note")
            .await
            .expect("append initial notes");
        let base_task = store
            .show_task("stale-metadata-task")
            .await
            .expect("read base task");
        let base_updated_at = base_task.updated_at.clone();
        let mut stale_update = base_task.clone();
        stale_update.planner_metadata.owned_paths =
            vec!["crates/vida/src/state_store_task_store.rs".to_string()];
        stale_update.updated_at = base_updated_at
            .parse::<u128>()
            .ok()
            .and_then(|value| value.checked_add(1))
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{base_updated_at}-update"));

        store
            .append_task_notes("stale-metadata-task", "\n\n", "late appended evidence")
            .await
            .expect("append notes after stale update read");
        let persisted = store
            .persist_task_update_record_preserving_latest_notes(
                stale_update,
                base_updated_at,
                &base_task,
            )
            .await
            .expect("stale metadata update should merge latest notes");

        assert_eq!(
            persisted.notes.as_deref(),
            Some("initial note\n\nlate appended evidence")
        );
        assert_eq!(
            persisted.planner_metadata.owned_paths,
            vec!["crates/vida/src/state_store_task_store.rs".to_string()]
        );
        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn close_parent_with_open_child_fails_without_mutating_parent() {
        let root = unique_task_store_temp_root("vida-close-parent-open-child-atomic");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "parent-with-open-child",
                title: "Parent with open child",
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
                task_id: "still-open-child",
                title: "Still open child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("parent-with-open-child"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");

        let error = store
            .close_task("parent-with-open-child", "done")
            .await
            .expect_err("open child must block parent close before persistence");

        assert!(error.to_string().contains(
            "cannot close task `parent-with-open-child` while non-closed child tasks exist"
        ));
        assert!(error.to_string().contains("still-open-child(status=open"));
        assert!(error.to_string().contains("updated_at="));
        assert!(error.to_string().contains("closed_at=none"));
        let parent = store
            .show_task("parent-with-open-child")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "open");
        assert!(parent.closed_at.is_none());
        assert!(parent.close_reason.is_none());
        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn create_open_step_under_closed_parent_preserves_parent_closure() {
        let root = unique_task_store_temp_root("vida-create-step-preserves-closed-parent");
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
            .expect("create epic");
        store
            .create_task(CreateTaskRequest {
                task_id: "closed-parent-task",
                title: "Closed parent task",
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
            .expect("create parent");
        store
            .close_task("closed-parent-task", "done")
            .await
            .expect("close parent");

        let closed_parent = store
            .show_task("closed-parent-task")
            .await
            .expect("load closed parent");
        let closed_at = closed_parent.closed_at.clone();
        let close_reason = closed_parent.close_reason.clone();

        store
            .create_task(CreateTaskRequest {
                task_id: "post-close-step",
                title: "Post-close evidence step",
                display_id: None,
                description: "",
                issue_type: "step",
                status: "in_progress",
                priority: 1,
                parent_id: Some("closed-parent-task"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create execution step under closed parent");

        let parent = store
            .show_task("closed-parent-task")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "closed");
        assert_eq!(parent.closed_at, closed_at);
        assert_eq!(parent.close_reason, close_reason);
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        close_store_and_remove_root(store, root).await;
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

        close_store_and_remove_root(store, root).await;
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

        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn update_task_allows_unchanged_parent_with_metadata_update() {
        let root = unique_task_store_temp_root("vida-update-unchanged-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "unchanged-parent",
                title: "Unchanged parent",
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
                task_id: "unchanged-child",
                title: "Unchanged child",
                display_id: None,
                description: "before",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("unchanged-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");

        store
            .update_task(UpdateTaskRequest {
                task_id: "unchanged-child",
                title: None,
                status: None,
                priority: None,
                notes: None,
                description: Some("after"),
                parent_id: Some(Some("unchanged-parent")),
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
            .expect("metadata update with unchanged parent should be accepted");

        let child = store
            .show_task("unchanged-child")
            .await
            .expect("load child");
        assert_eq!(child.description, "after");
        assert_eq!(
            StateStore::parent_id_for_task(&child).as_deref(),
            Some("unchanged-parent")
        );
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());

        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn update_task_status_closed_rejects_proof_protected_task_close_authority() {
        let root = unique_task_store_temp_root("vida-update-close-proof-authority");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "proof-parent",
                title: "Proof parent",
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
                task_id: "proof-child",
                title: "Proof child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("proof-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata {
                    proof_targets: vec!["cargo test proof-child".to_string()],
                    ..TaskPlannerMetadata::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create proof child");

        for requested_status in ["closed", "done"] {
            let error = store
                .update_task(UpdateTaskRequest {
                    task_id: "proof-child",
                    title: None,
                    status: Some(requested_status),
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
                .expect_err("generic update close should require close authority");

            match error {
                StateStoreError::InvalidTaskRecord { reason } => {
                    assert_eq!(
                        StateStore::task_update_close_authority_task_id_from_reason(&reason),
                        Some("proof-child")
                    );
                    assert!(reason.contains("configured proof targets require `vida task close`"));
                }
                other => panic!("expected InvalidTaskRecord, got {other}"),
            }
            let child = store.show_task("proof-child").await.expect("show child");
            assert_eq!(child.status, "open");
            assert!(child.closed_at.is_none());
            assert!(child.close_reason.is_none());
        }

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn append_task_notes_preserves_closed_task_status() {
        let root = unique_task_store_temp_root("vida-append-note-closed-task");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "note-parent",
                title: "Note parent",
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
                task_id: "note-child",
                title: "Note child",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("note-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create child");

        store
            .close_task("note-child", "initial close proof")
            .await
            .expect("close child");
        let closed_child = store.show_task("note-child").await.expect("show child");
        let closed_parent = store.show_task("note-parent").await.expect("show parent");

        let appended = store
            .append_task_notes("note-child", "\n", "post-close scorecard")
            .await
            .expect("append note to closed child");

        assert_eq!(appended.status, "closed");
        assert_eq!(appended.closed_at, closed_child.closed_at);
        assert_eq!(appended.close_reason, closed_child.close_reason);
        assert!(appended
            .notes
            .as_deref()
            .is_some_and(|notes| notes.contains("post-close scorecard")));

        let parent_after_append = store.show_task("note-parent").await.expect("show parent");
        assert_eq!(parent_after_append.status, closed_parent.status);
        assert_eq!(parent_after_append.closed_at, closed_parent.closed_at);
        assert_eq!(parent_after_append.close_reason, closed_parent.close_reason);
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn update_task_status_done_closes_parent_without_active_children() {
        let root = unique_task_store_temp_root("vida-update-child-done-closes-parent");
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("done-close-parent", "Done close parent", "epic", None),
            (
                "done-close-child",
                "Done close child",
                "task",
                Some("done-close-parent"),
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

        let child = store
            .update_task(UpdateTaskRequest {
                task_id: "done-close-child",
                title: None,
                status: Some("done"),
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
            .expect("close child through done alias update");

        assert_eq!(child.status, "done");
        assert!(child.closed_at.is_some());

        let parent = store
            .show_task("done-close-parent")
            .await
            .expect("load parent");
        assert_eq!(parent.status, "closed");
        assert_eq!(
            parent.close_reason.as_deref(),
            Some("all direct child tasks closed after closing `done-close-child`")
        );
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate")
            .is_empty());
        close_store_and_remove_root(store, root).await;
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
                "task",
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
        close_store_and_remove_root(store, root).await;
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
                "task",
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
        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_spec_close_keeps_feature_epic_open_until_work_pool_handoff() {
        let root = unique_task_store_temp_root("vida-tracked-flow-spec-close");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "feature-x",
                title: "Feature X",
                display_id: None,
                description: "tracked feature epic",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &["feature-request".to_string(), "spec-first".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create feature epic");
        store
            .create_task(CreateTaskRequest {
                task_id: "feature-x-spec",
                title: "Spec pack",
                display_id: None,
                description: "bounded spec packet",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("feature-x"),
                labels: &["spec-pack".to_string(), "documentation".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create spec task");
        store
            .create_task(CreateTaskRequest {
                task_id: "feature-x-work-pool",
                title: "Work-pool pack",
                display_id: None,
                description: "tracked work-pool packet",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("feature-x"),
                labels: &["work-pool-pack".to_string()],
                execution_semantics: TaskExecutionSemantics {
                    execution_mode: Some("container_only".to_string()),
                    order_bucket: Some("feature-x".to_string()),
                    parallel_group: Some("work-pool-pack".to_string()),
                    conflict_domain: Some("feature-x-work-pool".to_string()),
                },
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create work-pool task");

        store
            .close_task(
                "feature-x-spec",
                "design packet finalized and handed off into tracked work-pool shaping",
            )
            .await
            .expect("close spec task");

        let feature = store.show_task("feature-x").await.expect("load feature");
        let spec = store.show_task("feature-x-spec").await.expect("load spec");
        let work_pool = store
            .show_task("feature-x-work-pool")
            .await
            .expect("load work-pool");

        assert_eq!(spec.status, "closed");
        assert_eq!(work_pool.status, "open");
        assert_eq!(
            feature.status, "open",
            "tracked feature epic must stay open while its work-pool handoff child is open"
        );
        assert!(feature.closed_at.is_none());
        assert!(feature.close_reason.is_none());
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate graph")
            .is_empty());
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_spec_close_keeps_feature_epic_open_before_work_pool_exists() {
        let root = unique_task_store_temp_root("vida-tracked-flow-spec-close-no-work-pool");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "feature-y",
                title: "Feature Y",
                display_id: None,
                description: "tracked feature epic",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &["feature-request".to_string(), "spec-first".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create feature epic");
        store
            .create_task(CreateTaskRequest {
                task_id: "feature-y-spec",
                title: "Spec pack",
                display_id: None,
                description: "bounded spec packet",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("feature-y"),
                labels: &["spec-pack".to_string(), "documentation".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create spec task");

        store
            .close_task(
                "feature-y-spec",
                "design packet finalized and handed off into tracked work-pool shaping",
            )
            .await
            .expect("close spec task");

        let feature = store.show_task("feature-y").await.expect("load feature");
        let spec = store.show_task("feature-y-spec").await.expect("load spec");

        assert_eq!(spec.status, "closed");
        assert_eq!(
            feature.status, "open",
            "spec-first feature epic must stay open even when work-pool child has not been materialized yet"
        );
        assert!(feature.closed_at.is_none());
        assert!(feature.close_reason.is_none());
        assert!(store
            .validate_task_graph()
            .await
            .expect("validate graph")
            .is_empty());
        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_spec_close_clears_stale_design_and_spec_recovery_blockers() {
        let root = unique_task_store_temp_root("vida-tracked-flow-spec-close-clears-recovery");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "feature-z",
                title: "Feature Z",
                display_id: None,
                description: "tracked feature epic",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &["feature-request".to_string(), "spec-first".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create feature epic");
        store
            .create_task(CreateTaskRequest {
                task_id: "feature-z-spec",
                title: "Spec pack",
                display_id: None,
                description: "bounded spec packet",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("feature-z"),
                labels: &["spec-pack".to_string(), "documentation".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create spec task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "feature-z-spec",
            "specification",
            "specification",
        );
        status.task_id = "feature-z-spec".to_string();
        status.active_node = "specification".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "specification_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist spec status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "feature-z-spec".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some(
                    "vida task ensure feature-z-work-pool".to_string(),
                ),
                downstream_dispatch_note: Some(
                    "finalize the design doc and close spec-pack before work-pool shaping"
                        .to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-06-05T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale spec receipt");

        store
            .close_task(
                "feature-z-spec",
                "design packet finalized and handed off into tracked work-pool shaping",
            )
            .await
            .expect("close spec task");

        let receipt = store
            .run_graph_dispatch_receipt("feature-z-spec")
            .await
            .expect("load receipt")
            .expect("receipt should remain");
        assert!(receipt.downstream_dispatch_ready);
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(!receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )));

        let refreshed = store
            .run_graph_status("feature-z-spec")
            .await
            .expect("load refreshed status");
        assert_eq!(refreshed.status, "ready");
        assert_eq!(refreshed.active_node, "specification");
        assert_eq!(refreshed.next_node.as_deref(), Some("work_pool_pack"));
        assert_eq!(refreshed.lifecycle_stage, "specification_complete");
        assert_eq!(refreshed.handoff_state, "awaiting_work_pool_pack");
        assert_eq!(refreshed.resume_target, "dispatch.work_pool_pack_lane");
        assert!(refreshed.recovery_ready);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn tracked_flow_feature_run_reconciles_closed_spec_and_stale_parent_on_recovery_read() {
        let root = unique_task_store_temp_root("vida-tracked-flow-feature-run-clears-recovery");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "feature-live",
                title: "Feature live",
                display_id: None,
                description: "tracked feature epic",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &["feature-request".to_string(), "spec-first".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create feature epic");
        store
            .create_task(CreateTaskRequest {
                task_id: "feature-live-spec",
                title: "Spec pack",
                display_id: None,
                description: "bounded spec packet",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("feature-live"),
                labels: &["spec-pack".to_string(), "documentation".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create spec task");
        store
            .create_task(CreateTaskRequest {
                task_id: "activity-live-run",
                title: "Activity live run",
                display_id: None,
                description: "feature delivery run task",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: Some("feature-live"),
                labels: &["implementation".to_string()],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create feature run task");

        store
            .close_task(
                "feature-live-spec",
                "design packet finalized and handed off into tracked work-pool shaping",
            )
            .await
            .expect("close spec task");
        let mut stale_parent = store
            .show_task("feature-live")
            .await
            .expect("load feature parent");
        stale_parent.status = "closed".to_string();
        stale_parent.closed_at = Some("2".to_string());
        stale_parent.close_reason =
            Some("all direct child tasks closed after closing `feature-live-spec`".to_string());
        store
            .persist_task_record(stale_parent)
            .await
            .expect("persist stale auto-closed parent");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "activity-live-run",
            "specification",
            "specification",
        );
        status.task_id = "activity-live-run".to_string();
        status.active_node = "specification".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "specification_blocked".to_string();
        status.policy_gate = "blocked".to_string();
        status.handoff_state = "blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist feature-run status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "activity-live-run".to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some(
                    "vida task ensure feature-live-work-pool".to_string(),
                ),
                downstream_dispatch_note: Some(
                    "finalize the design doc and close spec-pack before work-pool shaping"
                        .to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-06-05T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale feature-run receipt");

        let refreshed = store
            .run_graph_status("activity-live-run")
            .await
            .expect("load reconciled feature-run status");
        assert_eq!(refreshed.status, "ready");
        assert_eq!(refreshed.active_node, "specification");
        assert_eq!(refreshed.next_node.as_deref(), Some("work_pool_pack"));
        assert_eq!(refreshed.lifecycle_stage, "specification_complete");
        assert_eq!(refreshed.handoff_state, "awaiting_work_pool_pack");
        assert_eq!(refreshed.resume_target, "dispatch.work_pool_pack_lane");
        assert!(refreshed.recovery_ready);

        let receipt = store
            .run_graph_dispatch_receipt("activity-live-run")
            .await
            .expect("load receipt")
            .expect("receipt should remain");
        let identity = store
            .run_graph_dispatch_task_identity("activity-live-run")
            .await
            .expect("load dispatch task identity")
            .expect("identity should be recorded");
        assert_eq!(identity.feature_epic_id.as_deref(), Some("feature-live"));
        assert_eq!(identity.spec_task_id.as_deref(), Some("feature-live-spec"));
        assert_eq!(identity.work_pool_task_id, None);
        assert_eq!(
            identity.source,
            "spec_first_work_pool_handoff_reconciliation"
        );
        assert!(receipt.downstream_dispatch_ready);
        assert!(!receipt
            .downstream_dispatch_blockers
            .iter()
            .any(|blocker| matches!(
                blocker.as_str(),
                "pending_design_finalize" | "pending_spec_task_close"
            )));
        let repaired_parent = store
            .show_task("feature-live")
            .await
            .expect("load repaired feature parent");
        assert_eq!(repaired_parent.status, "open");
        assert!(repaired_parent.closed_at.is_none());
        assert!(repaired_parent.close_reason.is_none());

        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
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
        close_store_and_remove_root(store, root).await;
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
        let _state_override = TestProxyStateDirOverrideGuard::install(root.clone());
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
            .acquire_current_session_run_graph_claim_for_test(
                "claim-close-task",
                "run-close-task",
                "feature-close-dev",
                "task:feature-close-dev",
                "crates/vida/src/state_store_task_store.rs",
            )
            .await
            .expect("seed current-session run-graph claim");

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
        let project_root = crate::resolve_runtime_project_root().expect("resolve project root");
        let config = crate::load_project_overlay_yaml().expect("load project overlay");
        let compiled_bundle =
            crate::build_compiled_agent_extension_bundle_for_root(&config, &project_root)
                .expect("compile agent extension bundle");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: compiled_bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        let execution_plan =
            crate::build_runtime_execution_plan_from_snapshot(&compiled_bundle, &role_selection);
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
                    "execution_plan": execution_plan,
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
                dispatch_command: Some(
                    "vida agent-init --dispatch-packet /tmp/implementer.json --execute-dispatch"
                        .to_string(),
                ),
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
        assert_eq!(reconciled_status.status, "completed");
        assert_eq!(reconciled_status.active_node, "closure");
        assert_eq!(reconciled_status.lifecycle_stage, "closure_complete");
        assert_eq!(
            reconciled_status.policy_gate,
            "closed_task_stale_run_retired"
        );
        assert_eq!(reconciled_status.checkpoint_kind, "none");
        assert!(reconciled_status.is_terminal_closure());
        let checkpoint_record = store
            .run_graph_projection_checkpoint_record("run-close-task")
            .await
            .expect("checkpoint record lookup should succeed");
        assert!(checkpoint_record.is_none());

        close_store_and_remove_root(store, root).await;
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
            .acquire_current_session_run_graph_claim_for_test(
                "claim-explicit-task-close",
                "run-explicit-task-close",
                "run-owner-task",
                "task:run-owner-task",
                "crates/vida/src/state_store_task_store.rs",
            )
            .await
            .expect("seed current-session run-graph claim");

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

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn reconcile_closed_runs_uses_completed_receipt_authority() {
        let root = unique_task_store_temp_root("vida-reconcile-closed-run-receipt-authority");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let parent_id = "closed-receipt-parent";
        let task_id = "closed-receipt-task";
        let run_id = "closed-receipt-run";

        store
            .create_task(CreateTaskRequest {
                task_id: parent_id,
                title: "Closed receipt parent",
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
                task_id,
                title: "Closed receipt task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: Some(parent_id),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        store
            .close_task(task_id, "receipt-backed closure proof passed")
            .await
            .expect("close task");

        let project_root = crate::resolve_runtime_project_root().expect("resolve project root");
        let config = crate::load_project_overlay_yaml().expect("load project overlay");
        let compiled_bundle = crate::build_compiled_agent_extension_bundle_for_root(
            &config,
            &project_root,
        )
        .expect("compile agent extension bundle");
        store
            .write_launcher_activation_snapshot(&crate::state_store::LauncherActivationSnapshot {
                source: "state_store".to_string(),
                source_config_path: project_root.join("vida.config.yaml").display().to_string(),
                source_config_digest: crate::launcher_activation_snapshot::config_file_digest(
                    &project_root.join("vida.config.yaml"),
                )
                .expect("config digest"),
                captured_at: "2026-01-01T00:00:00Z".to_string(),
                compiled_bundle: compiled_bundle.clone(),
                pack_router_keywords: serde_json::json!({}),
            })
            .await
            .expect("write launcher activation snapshot");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: compiled_bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        let mut legacy_execution_plan =
            crate::build_runtime_execution_plan_from_snapshot(&compiled_bundle, &role_selection);
        if let Some(plan) = legacy_execution_plan.as_object_mut() {
            plan.remove("team_flow_authority_selected_node_id");
            if let Some(dispatch_contract) = plan
                .get_mut("development_flow")
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|flow| flow.get_mut("dispatch_contract"))
                .and_then(serde_json::Value::as_object_mut)
            {
                dispatch_contract.remove("selected_node_id");
                dispatch_contract.remove("team_flow_authority_selected_node_id");
            }
        }
        let mut persisted_selection = serde_json::to_value(&role_selection)
            .expect("encode persisted role selection");
        persisted_selection["compiled_bundle"] = serde_json::Value::Null;
        persisted_selection["execution_plan"] = legacy_execution_plan;
        let packet_path = root.join("runtime-consumption/dispatch-packets/closed-receipt-run.json");
        fs::create_dir_all(packet_path.parent().expect("packet parent")).expect("packet dir");
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": persisted_selection.clone(),
                "run_graph_bootstrap": { "run_id": run_id },
            }))
            .expect("encode malformed legacy packet"),
        )
        .expect("write malformed legacy packet");
        let active_packet_path =
            root.join("runtime-consumption/dispatch-packets/active-legacy-run.json");
        fs::write(
            &active_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": "active-legacy-run",
                "role_selection_full": persisted_selection,
                "run_graph_bootstrap": { "run_id": "active-legacy-run" },
            }))
            .expect("encode active malformed legacy packet"),
        )
        .expect("write active malformed legacy packet");
        let active_result_path =
            root.join("runtime-consumption/dispatch-results/active-legacy-run.json");
        fs::create_dir_all(active_result_path.parent().expect("active result parent"))
            .expect("active result dir");
        fs::write(
            &active_result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "blocked",
                "execution_state": "blocked",
                "decision": "rework",
                "rework_target": "implementation",
                "allowed_next_node": "implementer",
                "execution_evidence": { "receipt_backed": true },
            }))
            .expect("encode active rework result"),
        )
        .expect("write active rework result");

        store
            .create_task(CreateTaskRequest {
                task_id: "active-legacy-task",
                title: "Active legacy task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "in_progress",
                priority: 1,
                parent_id: Some(parent_id),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create active legacy task");
        let mut active_status = crate::taskflow_run_graph::default_run_graph_status(
            "active-legacy-run",
            "implementation",
            "implementer",
        );
        active_status.task_id = "active-legacy-task".to_string();
        active_status.status = "executing".to_string();
        store
            .record_run_graph_status(&active_status)
            .await
            .expect("seed active legacy run");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "closure",
        );
        status.task_id = task_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "executing".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "closure_receipt_ready".to_string();
        status.handoff_state = "none".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("seed stale terminal closure run");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join(format!("{run_id}.json"));
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_lane_completion_result",
                "status": "pass",
                "execution_state": "executed",
                "completed_target": "closure",
                "closure_ready": true,
                "execution_evidence": {
                    "status": "recorded",
                    "receipt_backed": true
                }
            }))
            .expect("encode result"),
        )
        .expect("write result");
        let receipt = crate::state_store::RunGraphDispatchReceiptStored {
            run_id: run_id.to_string(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: Some("lane_completed".to_string()),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
            dispatch_packet_path: Some(
                root.join("runtime-consumption/dispatch-packets/closed-receipt-run.json")
                    .display()
                    .to_string(),
            ),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("test".to_string()),
            recorded_at: "2026-05-19T00:00:00Z".to_string(),
        };
        let _: Option<crate::state_store::RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", run_id))
            .content(receipt.clone())
            .await
            .expect("seed receipt without current-session owner evidence");
        let mut active_receipt = receipt;
        active_receipt.run_id = "active-legacy-run".to_string();
        active_receipt.dispatch_packet_path = Some(active_packet_path.display().to_string());
        active_receipt.dispatch_result_path = Some(active_result_path.display().to_string());
        active_receipt.recorded_at = "2026-05-18T00:00:00Z".to_string();
        let _: Option<crate::state_store::RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "active-legacy-run"))
            .content(active_receipt)
            .await
            .expect("seed active malformed receipt");
        store
            .acquire_orchestrator_claim(AcquireOrchestratorClaimRequest {
                claim_id: "foreign-closed-run-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: "foreign-worktree".to_string(),
                orchestrator_session_id: "foreign-session".to_string(),
                process_id: None,
                task_id: Some(task_id.to_string()),
                run_id: Some(run_id.to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some(format!("run:{run_id}")),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: LeaseMode::Exclusive,
                lease_seconds: 3600,
            })
            .await
            .expect("seed foreign run claim");

        let mut ordinary_update = status.clone();
        ordinary_update.policy_gate = "ordinary_update_should_still_require_ownership".to_string();
        let ordinary_result = store.record_run_graph_status(&ordinary_update).await;
        assert!(
            matches!(
                ordinary_result,
                Err(StateStoreError::InvalidTaskRecord { ref reason })
                    if reason.contains("current session does not own run")
            ),
            "ordinary run-graph mutation must remain ownership-guarded: {ordinary_result:?}"
        );

        let summary = store
            .reconcile_historical_closed_task_active_runs(1)
            .await
            .expect("reconcile receipt-backed closed run");
        assert_eq!(summary.reconciled_count, 1);
        assert_eq!(summary.skipped_count, 0);
        assert_eq!(summary.reconciled_runs[0].run_id, run_id);
        let reconciled = store
            .run_graph_status(run_id)
            .await
            .expect("load reconciled status");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.active_node, "closure");
        assert_eq!(reconciled.lifecycle_stage, "closure_complete");
        assert!(store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read binding")
            .is_none());
        let active_error = store
            .run_graph_status("active-legacy-run")
            .await
            .expect_err("active malformed role selection must remain fail-closed");
        assert!(
            active_error
                .to_string()
                .contains("team_flow_authority_selected_node_id_missing"),
            "active malformed run should expose the selected-node blocker: {active_error}"
        );
        let (active_raw_status, ..) = store
            .run_graph_raw_status_from_task_rows("active-legacy-run")
            .await
            .expect("read active raw execution state");
        assert_eq!(active_raw_status.status, "executing");

        close_store_and_remove_root(store, root).await;
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

        store.close().await;

        let reopened = StateStore::open(root.clone())
            .await
            .expect("reopen store after legacy downgrade");
        let task = reopened
            .show_task("legacy-task")
            .await
            .expect("legacy task should load");
        assert_eq!(task.execution_semantics, TaskExecutionSemantics::default());

        close_store_and_remove_root(reopened, root).await;
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

        store.close().await;

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

        close_store_and_remove_root(reopened, root).await;
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

        store.close().await;

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

        close_store_and_remove_root(reopened, root).await;
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

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn create_import_and_update_share_task_record_default_policy() {
        let root = unique_task_store_temp_root("vida-task-record-default-policy");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let messy_metadata = TaskPlannerMetadata {
            owned_paths: vec![
                " crates/vida/src/task_surface.rs ".to_string(),
                "crates/vida/src/state_store_task_store.rs".to_string(),
                "crates/vida/src/task_surface.rs".to_string(),
                " ".to_string(),
            ],
            acceptance_targets: vec![
                " lifecycle defaults converge ".to_string(),
                "lifecycle defaults converge".to_string(),
            ],
            proof_targets: vec![
                " cargo test -p vida task_record_default_policy ".to_string(),
                "cargo test -p vida task_record_default_policy".to_string(),
            ],
            risk: Some(" high ".to_string()),
            estimate: Some(" medium ".to_string()),
            lane_hint: Some(" core-boundary ".to_string()),
        };
        let expected_metadata = TaskPlannerMetadata {
            owned_paths: vec![
                "crates/vida/src/state_store_task_store.rs".to_string(),
                "crates/vida/src/task_surface.rs".to_string(),
            ],
            acceptance_targets: vec!["lifecycle defaults converge".to_string()],
            proof_targets: vec!["cargo test -p vida task_record_default_policy".to_string()],
            risk: Some("high".to_string()),
            estimate: Some("medium".to_string()),
            lane_hint: Some("core-boundary".to_string()),
        };
        let messy_semantics = TaskExecutionSemantics {
            execution_mode: Some(" parallel_safe ".to_string()),
            order_bucket: Some(" workflow ".to_string()),
            parallel_group: Some(" lifecycle ".to_string()),
            conflict_domain: Some(" defaults ".to_string()),
        };
        let expected_semantics = TaskExecutionSemantics {
            execution_mode: Some("parallel_safe".to_string()),
            order_bucket: Some("workflow".to_string()),
            parallel_group: Some("lifecycle".to_string()),
            conflict_domain: Some("defaults".to_string()),
        };

        store
            .create_task(CreateTaskRequest {
                task_id: "created-defaults",
                title: "Created defaults",
                display_id: None,
                description: "created path",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: messy_semantics.clone(),
                planner_metadata: messy_metadata.clone(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create task should normalize defaults");
        let created = store
            .show_task("created-defaults")
            .await
            .expect("created task should load");
        assert_eq!(created.execution_semantics, expected_semantics);
        assert_eq!(created.planner_metadata, expected_metadata);

        store
            .update_task(UpdateTaskRequest {
                task_id: "created-defaults",
                title: None,
                status: None,
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: Some(Some(" exclusive ")),
                order_bucket: Some(Some(" workflow-updated ")),
                parallel_group: Some(Some(" lifecycle-updated ")),
                conflict_domain: Some(Some(" defaults-updated ")),
                planner_metadata: Some(messy_metadata.clone()),
            })
            .await
            .expect("update task should normalize defaults");
        let updated = store
            .show_task("created-defaults")
            .await
            .expect("updated task should load");
        assert_eq!(
            updated.execution_semantics,
            TaskExecutionSemantics {
                execution_mode: Some("exclusive".to_string()),
                order_bucket: Some("workflow-updated".to_string()),
                parallel_group: Some("lifecycle-updated".to_string()),
                conflict_domain: Some("defaults-updated".to_string()),
            }
        );
        assert_eq!(updated.planner_metadata, expected_metadata);

        let jsonl_path = root.join("import-defaults.jsonl");
        fs::write(
            &jsonl_path,
            r#"{"id":"imported-defaults","title":"Imported defaults","description":"import path","status":"open","priority":1,"issue_type":"epic","created_at":"2026-06-19T00:00:00Z","created_by":"tester","updated_at":"2026-06-19T00:00:00Z","source_repo":".","labels":[],"execution_semantics":{"execution_mode":" parallel_safe ","order_bucket":" workflow ","parallel_group":" lifecycle ","conflict_domain":" defaults "},"planner_metadata":{"owned_paths":[" crates/vida/src/task_surface.rs ","crates/vida/src/state_store_task_store.rs","crates/vida/src/task_surface.rs"," "],"acceptance_targets":[" lifecycle defaults converge ","lifecycle defaults converge"],"proof_targets":[" cargo test -p vida task_record_default_policy ","cargo test -p vida task_record_default_policy"],"risk":" high ","estimate":" medium ","lane_hint":" core-boundary "},"dependencies":[]}"#,
        )
        .expect("import jsonl should write");
        store
            .import_tasks_from_jsonl(&jsonl_path)
            .await
            .expect("import should normalize defaults");
        let imported = store
            .show_task("imported-defaults")
            .await
            .expect("imported task should load");
        assert_eq!(imported.execution_semantics, expected_semantics);
        assert_eq!(imported.planner_metadata, expected_metadata);

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn import_tasks_replaces_parent_edges_with_single_root_rollup() {
        let root = unique_task_store_temp_root("vida-task-import-replace-parent-edge");
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "old-parent",
                title: "Old parent",
                display_id: None,
                description: "existing parent before import",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create old parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "imported-child",
                title: "Imported child",
                display_id: None,
                description: "existing child before import",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("old-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create child under old parent");

        let before = store
            .show_task("imported-child")
            .await
            .expect("existing child should load");
        assert_eq!(
            StateStore::parent_id_for_task(&before).as_deref(),
            Some("old-parent")
        );

        let jsonl_path = root.join("replace-parent-edge.jsonl");
        fs::write(
            &jsonl_path,
            concat!(
                r#"{"id":"new-parent","title":"New parent","description":"single import root","status":"open","priority":1,"issue_type":"epic","created_at":"2026-06-21T00:00:00Z","created_by":"tester","updated_at":"2026-06-21T00:00:00Z","source_repo":".","labels":[],"dependencies":[]}"#,
                "\n",
                r#"{"id":"imported-child","title":"Imported child","description":"replaced by import","status":"open","priority":1,"issue_type":"task","created_at":"2026-06-21T00:00:00Z","created_by":"tester","updated_at":"2026-06-21T00:00:00Z","source_repo":".","labels":[],"dependencies":[]}"#,
                "\n"
            ),
        )
        .expect("import jsonl should write");

        let summary = store
            .import_tasks_from_jsonl(&jsonl_path)
            .await
            .expect("single-root import should roll child under new parent");
        assert_eq!(summary.imported_count, 1);
        assert_eq!(summary.updated_count, 1);
        assert_eq!(summary.unchanged_count, 0);

        let imported = store
            .show_task("imported-child")
            .await
            .expect("imported child should load");
        assert_eq!(
            StateStore::parent_id_for_task(&imported).as_deref(),
            Some("new-parent")
        );
        assert!(
            imported
                .dependencies
                .iter()
                .all(|dependency| dependency.depends_on_id != "old-parent"),
            "{:?}",
            imported.dependencies
        );

        close_store_and_remove_root(store, root).await;
    }

    #[tokio::test]
    async fn import_tasks_rejects_invalid_execution_mode_without_persisting() {
        let root = unique_task_store_temp_root("vida-task-import-invalid-execution-mode");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let jsonl_path = root.join("invalid-execution-mode.jsonl");
        fs::write(
            &jsonl_path,
            r#"{"id":"invalid-execution-mode","title":"Invalid execution mode","description":"import path","status":"open","priority":1,"issue_type":"epic","created_at":"2026-06-19T00:00:00Z","created_by":"tester","updated_at":"2026-06-19T00:00:00Z","source_repo":".","labels":[],"execution_semantics":{"execution_mode":"unsafe_parallel"},"dependencies":[]}"#,
        )
        .expect("invalid import jsonl should write");

        let error = store
            .import_tasks_from_jsonl(&jsonl_path)
            .await
            .expect_err("invalid execution mode should block import");
        assert!(
            error
                .to_string()
                .contains("execution_mode must be one of sequential"),
            "{error}"
        );
        assert!(store.show_task("invalid-execution-mode").await.is_err());

        close_store_and_remove_root(store, root).await;
    }

    // ==================== Core Rule #12 Override Tests ====================
    // ==================== Cascading Closure Tests ====================
}
