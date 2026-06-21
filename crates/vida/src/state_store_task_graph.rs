use super::*;
use crate::launcher_task_commands::shell_quote;
use crate::state_store::state_store_task_models::{
    task_is_spec_first_feature_parent, task_is_spec_pack_child, task_is_work_pool_pack_child,
    task_status_is_closed_like, task_status_is_open_like,
};
use taskflow_core::scheduling::scheduler_dispatch::{self, ParallelSafetyInput};
use taskflow_core::task::verify::all_structured_task_proof_targets_satisfied;

const TASK_TREE_MAX_DEPTH: usize = 64;
const TASK_TREE_MAX_NODE_VISITS: usize = 10_000;

#[derive(Debug, Clone)]
pub(crate) struct TaskGraphSnapshot {
    rows: Vec<TaskRecord>,
    index: TaskIndex,
    progress_rows: Vec<taskflow_core::task::progress::TaskProgressRow>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TaskIndex {
    by_id: BTreeMap<String, usize>,
    children_by_parent: BTreeMap<String, Vec<String>>,
    non_parent_dependencies: BTreeMap<String, Vec<(String, String)>>,
}

impl TaskIndex {
    fn from_sorted_rows(rows: &[TaskRecord]) -> Self {
        let mut by_id = BTreeMap::new();
        for (index, task) in rows.iter().enumerate() {
            by_id.insert(task.id.clone(), index);
        }

        let mut children_by_parent = BTreeMap::<String, Vec<String>>::new();
        let mut non_parent_dependencies = BTreeMap::<String, Vec<(String, String)>>::new();
        for task in rows {
            for dependency in &task.dependencies {
                if dependency.edge_type == "parent-child" {
                    children_by_parent
                        .entry(dependency.depends_on_id.clone())
                        .or_default()
                        .push(task.id.clone());
                } else if by_id.contains_key(&dependency.depends_on_id) {
                    non_parent_dependencies
                        .entry(task.id.clone())
                        .or_default()
                        .push((
                            dependency.depends_on_id.clone(),
                            dependency.edge_type.clone(),
                        ));
                }
            }
        }

        Self {
            by_id,
            children_by_parent,
            non_parent_dependencies,
        }
    }
}

impl TaskGraphSnapshot {
    pub(crate) fn from_rows(rows: &[TaskRecord]) -> Self {
        let mut rows = rows.to_vec();
        rows.sort_by(task_sort_key);
        let index = TaskIndex::from_sorted_rows(&rows);
        let progress_rows = rows.iter().map(task_progress_row_from_record).collect();

        Self {
            rows,
            index,
            progress_rows,
        }
    }

    fn rows(&self) -> &[TaskRecord] {
        &self.rows
    }

    fn task(&self, task_id: &str) -> Option<&TaskRecord> {
        self.index
            .by_id
            .get(task_id)
            .and_then(|index| self.rows.get(*index))
    }

    fn contains_task(&self, task_id: &str) -> bool {
        self.index.by_id.contains_key(task_id)
    }

    fn children_for(&self, task_id: &str) -> Option<&Vec<String>> {
        self.index.children_by_parent.get(task_id)
    }

    fn children_by_parent(&self) -> &BTreeMap<String, Vec<String>> {
        &self.index.children_by_parent
    }

    fn non_parent_dependencies(&self) -> &BTreeMap<String, Vec<(String, String)>> {
        &self.index.non_parent_dependencies
    }

    fn progress_rows(&self) -> &[taskflow_core::task::progress::TaskProgressRow] {
        &self.progress_rows
    }

    fn scope_ids(&self, scope_task_id: &str) -> Result<BTreeSet<String>, StateStoreError> {
        if !self.contains_task(scope_task_id) {
            return Err(StateStoreError::MissingTask {
                task_id: scope_task_id.to_string(),
            });
        }

        let mut scope_ids = BTreeSet::new();
        let mut stack = vec![scope_task_id.to_string()];
        while let Some(current) = stack.pop() {
            if !scope_ids.insert(current.clone()) {
                continue;
            }
            if let Some(descendants) = self.children_for(&current) {
                stack.extend(descendants.iter().cloned());
            }
        }

        Ok(scope_ids)
    }
}

fn task_progress_row_from_record(
    task: &TaskRecord,
) -> taskflow_core::task::progress::TaskProgressRow {
    taskflow_core::task::progress::TaskProgressRow {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        priority: task.priority,
        labels: task.labels.clone(),
        proof_targets: task.planner_metadata.proof_targets.clone(),
        proof_satisfied: all_structured_task_proof_targets_satisfied(
            task.notes.as_deref(),
            &task.planner_metadata.proof_targets,
        ),
        parent_id: task
            .dependencies
            .iter()
            .find(|dependency| dependency.edge_type == "parent-child")
            .map(|dependency| dependency.depends_on_id.clone()),
    }
}

fn task_progress_json_command(command: &str) -> String {
    if command.split_whitespace().any(|token| token == "--json") {
        command.to_string()
    } else {
        format!("{command} --json")
    }
}

impl StateStore {
    fn task_is_open_like(task: &TaskRecord) -> bool {
        task_status_is_open_like(&task.status) && !work_item_is_program_container(&task.issue_type)
    }

    fn task_blockers_from_snapshot(
        task: &TaskRecord,
        snapshot: &TaskGraphSnapshot,
    ) -> Vec<TaskDependencyStatus> {
        let mut blockers = task
            .dependencies
            .iter()
            .filter(|dependency| dependency.edge_type != "parent-child")
            .filter_map(
                |dependency| match snapshot.task(&dependency.depends_on_id) {
                    Some(blocker_task) if task_status_is_closed_like(&blocker_task.status) => None,
                    Some(blocker_task) => Some(TaskDependencyStatus {
                        issue_id: dependency.issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
                        dependency_status: blocker_task.status.clone(),
                        dependency_issue_type: Some(blocker_task.issue_type.clone()),
                    }),
                    None => Some(TaskDependencyStatus {
                        issue_id: dependency.issue_id.clone(),
                        depends_on_id: dependency.depends_on_id.clone(),
                        edge_type: dependency.edge_type.clone(),
                        dependency_status: "missing".to_string(),
                        dependency_issue_type: Some("missing_dependency_target".to_string()),
                    }),
                },
            )
            .collect::<Vec<_>>();
        blockers.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        blockers
    }

    fn parallel_safety_input(task: &TaskRecord) -> ParallelSafetyInput<'_> {
        ParallelSafetyInput {
            task_id: task.id.as_str(),
            execution_mode: task.execution_semantics.execution_mode.as_deref(),
            order_bucket: task.execution_semantics.order_bucket.as_deref(),
            parallel_group: task.execution_semantics.parallel_group.as_deref(),
            conflict_domain: task.execution_semantics.conflict_domain.as_deref(),
            owned_paths: task
                .planner_metadata
                .owned_paths
                .iter()
                .map(String::as_str)
                .collect(),
        }
    }

    fn parallel_blockers_against_current(
        task: &TaskRecord,
        current: Option<&TaskRecord>,
    ) -> Vec<String> {
        scheduler_dispatch::parallel_blockers_against_current(
            Self::parallel_safety_input(task),
            current.map(Self::parallel_safety_input),
        )
    }

    fn task_is_container_only(task: &TaskRecord) -> bool {
        task.execution_semantics.execution_mode.as_deref() == Some("container_only")
            || work_item_is_program_container(&task.issue_type)
    }

    pub(crate) fn ready_tasks_scoped_from_rows(
        rows: &[TaskRecord],
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let snapshot = TaskGraphSnapshot::from_rows(rows);
        Self::ready_tasks_scoped_from_snapshot(&snapshot, scope_task_id)
    }

    pub(crate) fn ready_tasks_scoped_from_snapshot(
        snapshot: &TaskGraphSnapshot,
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(snapshot.scope_ids(scope_task_id)?)
        } else {
            None
        };

        let mut ready = snapshot
            .rows()
            .iter()
            .filter(|task| {
                scope_ids
                    .as_ref()
                    .map(|ids| ids.contains(&task.id))
                    .unwrap_or(true)
            })
            .filter(|task| Self::task_is_open_like(task))
            .filter(|task| !Self::task_is_container_only(task))
            .filter(|task| Self::task_blockers_from_snapshot(task, snapshot).is_empty())
            .cloned()
            .collect::<Vec<_>>();

        ready.sort_by(task_ready_sort_key);
        Ok(ready)
    }

    pub async fn ready_tasks_scoped(
        &self,
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let rows = self.all_tasks().await?;
        Self::ready_tasks_scoped_from_rows(&rows, scope_task_id)
    }

    pub async fn scheduling_projection_scoped(
        &self,
        scope_task_id: Option<&str>,
        current_task_id: Option<&str>,
    ) -> Result<TaskSchedulingProjection, StateStoreError> {
        let rows = self.all_tasks().await?;
        let snapshot = TaskGraphSnapshot::from_rows(&rows);
        let mut critical_path_ids = BTreeSet::new();
        if let Ok(path) = Self::critical_path_from_snapshot(&snapshot) {
            critical_path_ids.extend(path.nodes.into_iter().map(|node| node.id));
        }

        Self::scheduling_projection_scoped_from_snapshot(
            &snapshot,
            scope_task_id,
            current_task_id,
            &critical_path_ids,
        )
    }

    pub(crate) fn scheduling_projection_scoped_from_rows(
        rows: &[TaskRecord],
        scope_task_id: Option<&str>,
        current_task_id: Option<&str>,
        critical_path_ids: &BTreeSet<String>,
    ) -> Result<TaskSchedulingProjection, StateStoreError> {
        let snapshot = TaskGraphSnapshot::from_rows(rows);
        Self::scheduling_projection_scoped_from_snapshot(
            &snapshot,
            scope_task_id,
            current_task_id,
            critical_path_ids,
        )
    }

    pub(crate) fn scheduling_projection_scoped_from_snapshot(
        snapshot: &TaskGraphSnapshot,
        scope_task_id: Option<&str>,
        current_task_id: Option<&str>,
        critical_path_ids: &BTreeSet<String>,
    ) -> Result<TaskSchedulingProjection, StateStoreError> {
        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(snapshot.scope_ids(scope_task_id)?)
        } else {
            None
        };

        let scoped_tasks = snapshot
            .rows()
            .iter()
            .filter(|task| {
                scope_ids
                    .as_ref()
                    .map(|ids| ids.contains(&task.id))
                    .unwrap_or(true)
            })
            .filter(|task| Self::task_is_open_like(task))
            .collect::<Vec<_>>();

        let chosen_current = current_task_id
            .and_then(|task_id| {
                scoped_tasks
                    .iter()
                    .find(|task| task.id == task_id)
                    .filter(|task| !Self::task_is_container_only(task))
                    .map(|task| task.id.clone())
            })
            .or_else(|| {
                scoped_tasks
                    .iter()
                    .filter(|task| !Self::task_is_container_only(task))
                    .find(|task| Self::task_blockers_from_snapshot(task, snapshot).is_empty())
                    .map(|task| task.id.clone())
            });
        let current_task = chosen_current
            .as_deref()
            .and_then(|task_id| snapshot.task(task_id));

        let mut ready = Vec::new();
        let mut blocked = Vec::new();
        for task in scoped_tasks {
            let active_critical_path = critical_path_ids.contains(&task.id);
            let mut blocked_by = Self::task_blockers_from_snapshot(task, snapshot);
            if Self::task_is_container_only(task) && blocked_by.is_empty() {
                blocked_by.push(TaskDependencyStatus {
                    issue_id: task.id.clone(),
                    depends_on_id: task.id.clone(),
                    edge_type: "container-only".to_string(),
                    dependency_status: "container_only_task".to_string(),
                    dependency_issue_type: Some(task.issue_type.clone()),
                });
            }
            let ready_now = blocked_by.is_empty();
            let parallel_blockers = if ready_now {
                Self::parallel_blockers_against_current(task, current_task)
            } else {
                vec!["graph_blocked".to_string()]
            };
            let candidate = TaskSchedulingCandidate {
                task: task.clone(),
                ready_now,
                ready_parallel_safe: ready_now && parallel_blockers.is_empty(),
                blocked_by,
                active_critical_path,
                parallel_blockers,
            };
            if candidate.ready_now {
                ready.push(candidate);
            } else {
                blocked.push(candidate);
            }
        }
        ready.sort_by(|left, right| task_ready_sort_key(&left.task, &right.task));
        blocked.sort_by(|left, right| task_ready_sort_key(&left.task, &right.task));
        let parallel_candidates_after_current = ready
            .iter()
            .filter(|candidate| Some(candidate.task.id.as_str()) != chosen_current.as_deref())
            .filter(|candidate| candidate.ready_parallel_safe)
            .map(|candidate| candidate.task.clone())
            .collect::<Vec<_>>();

        Ok(TaskSchedulingProjection {
            current_task_id: chosen_current,
            ready,
            blocked,
            parallel_candidates_after_current,
        })
    }

    pub(crate) fn scheduling_projection_for_current_task_id(
        projection: &TaskSchedulingProjection,
        current_task_id: Option<&str>,
    ) -> TaskSchedulingProjection {
        let chosen_current = current_task_id
            .and_then(|task_id| {
                projection
                    .ready
                    .iter()
                    .chain(projection.blocked.iter())
                    .find(|candidate| candidate.task.id == task_id)
                    .map(|candidate| candidate.task.id.clone())
            })
            .or_else(|| projection.current_task_id.clone());
        let current_task = chosen_current.as_deref().and_then(|task_id| {
            projection
                .ready
                .iter()
                .chain(projection.blocked.iter())
                .find(|candidate| candidate.task.id == task_id)
                .map(|candidate| &candidate.task)
        });

        let ready = projection
            .ready
            .iter()
            .cloned()
            .map(|mut candidate| {
                candidate.parallel_blockers =
                    Self::parallel_blockers_against_current(&candidate.task, current_task);
                candidate.ready_parallel_safe =
                    candidate.ready_now && candidate.parallel_blockers.is_empty();
                candidate
            })
            .collect::<Vec<_>>();
        let parallel_candidates_after_current = ready
            .iter()
            .filter(|candidate| Some(candidate.task.id.as_str()) != chosen_current.as_deref())
            .filter(|candidate| candidate.ready_parallel_safe)
            .map(|candidate| candidate.task.clone())
            .collect::<Vec<_>>();

        TaskSchedulingProjection {
            current_task_id: chosen_current,
            ready,
            blocked: projection.blocked.clone(),
            parallel_candidates_after_current,
        }
    }

    pub async fn task_progress_summary(
        &self,
        task_id: &str,
    ) -> Result<TaskProgressSummary, StateStoreError> {
        let rows = self.all_tasks().await?;
        Self::task_progress_summary_from_rows(&rows, task_id)
    }

    pub(crate) fn task_progress_summary_from_rows(
        rows: &[TaskRecord],
        task_id: &str,
    ) -> Result<TaskProgressSummary, StateStoreError> {
        let snapshot = TaskGraphSnapshot::from_rows(rows);
        Self::task_progress_summary_from_snapshot(&snapshot, task_id)
    }

    pub(crate) fn task_progress_summary_from_snapshot(
        snapshot: &TaskGraphSnapshot,
        task_id: &str,
    ) -> Result<TaskProgressSummary, StateStoreError> {
        let core = taskflow_core::task::progress::task_progress_summary_from_rows(
            snapshot.progress_rows(),
            task_id,
            taskflow_core::task::progress::TaskProgressBasis::DescendantsExcludingRoot,
            shell_quote,
            task_progress_json_command,
        )
        .map_err(|_| StateStoreError::MissingTask {
            task_id: task_id.to_string(),
        })?;
        let root_task = snapshot.task(&core.root_task.id).cloned().ok_or_else(|| {
            StateStoreError::MissingTask {
                task_id: core.root_task.id.clone(),
            }
        })?;

        Ok(TaskProgressSummary {
            root_task,
            progress_basis: core.progress_basis,
            direct_child_count: core.direct_child_count,
            descendant_count: core.descendant_count,
            open_count: core.open_count,
            in_progress_count: core.in_progress_count,
            closed_count: core.closed_count,
            epic_count: core.epic_count,
            status_counts: core.status_counts,
            percent_closed: core.percent_closed,
            closure_candidate: core.closure_candidate,
            closure_candidate_state: core.closure_candidate_state,
            closure_candidate_reason: core.closure_candidate_reason,
            ready_for_close: core.ready_for_close,
            missing_proof: core.missing_proof,
            proof_blocked_by_runtime: core.proof_blocked_by_runtime,
            blocked_by_runtime: core.blocked_by_runtime,
            next_required_command: core.next_required_command,
            recommended_next_action: core.recommended_next_action,
            canonical_commands: core.canonical_commands,
        })
    }

    pub async fn task_dependency_tree(
        &self,
        task_id: &str,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let tasks = self.all_tasks().await?;
        Self::task_dependency_tree_from_rows(&tasks, task_id)
    }

    pub(crate) fn task_dependency_tree_from_rows(
        tasks: &[TaskRecord],
        task_id: &str,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let snapshot = TaskGraphSnapshot::from_rows(tasks);
        Self::task_dependency_tree_from_snapshot(&snapshot, task_id)
    }

    pub(crate) fn task_dependency_tree_from_snapshot(
        snapshot: &TaskGraphSnapshot,
        task_id: &str,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let mut active = BTreeSet::new();
        let mut expanded = BTreeSet::new();
        let mut node_visits = 0usize;
        Self::build_task_dependency_tree(
            snapshot,
            task_id,
            &mut active,
            &mut expanded,
            0,
            &mut node_visits,
        )
    }

    fn build_task_dependency_tree(
        snapshot: &TaskGraphSnapshot,
        task_id: &str,
        active: &mut BTreeSet<String>,
        expanded: &mut BTreeSet<String>,
        depth: usize,
        node_visits: &mut usize,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        if depth >= TASK_TREE_MAX_DEPTH {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task dependency tree exceeds max depth ({TASK_TREE_MAX_DEPTH}) at task {task_id}"
                ),
            });
        }
        if *node_visits >= TASK_TREE_MAX_NODE_VISITS {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task dependency tree exceeds max node visits ({TASK_TREE_MAX_NODE_VISITS})"
                ),
            });
        }
        *node_visits += 1;

        let task = snapshot
            .task(task_id)
            .cloned()
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;

        if !expanded.insert(task.id.clone()) {
            return Ok(TaskDependencyTreeNode {
                task,
                dependencies: Vec::new(),
                children: Vec::new(),
            });
        }

        active.insert(task.id.clone());
        let mut dependencies = Vec::new();
        for dependency in &task.dependencies {
            let mut edge = TaskDependencyTreeEdge {
                issue_id: dependency.issue_id.clone(),
                depends_on_id: dependency.depends_on_id.clone(),
                edge_type: dependency.edge_type.clone(),
                dependency_status: "missing".to_string(),
                dependency_issue_type: None,
                node: None,
                cycle: false,
                missing: false,
                repeated: false,
            };

            if active.contains(&dependency.depends_on_id) {
                edge.cycle = true;
            } else if let Some(dependency_task) = snapshot.task(&dependency.depends_on_id) {
                edge.dependency_status = dependency_task.status.clone();
                edge.dependency_issue_type = Some(dependency_task.issue_type.clone());
                if expanded.contains(&dependency.depends_on_id) {
                    edge.repeated = true;
                } else {
                    edge.node = Some(Box::new(Self::build_task_dependency_tree(
                        snapshot,
                        &dependency.depends_on_id,
                        active,
                        expanded,
                        depth + 1,
                        node_visits,
                    )?));
                }
            } else {
                edge.missing = true;
            }

            dependencies.push(edge);
        }
        let mut children = Vec::new();
        if let Some(child_ids) = snapshot.children_for(&task.id) {
            for child_id in child_ids {
                let mut child = TaskDependencyTreeChild {
                    child_id: child_id.clone(),
                    child_display_id: None,
                    child_title: None,
                    child_status: "missing".to_string(),
                    child_priority: None,
                    child_issue_type: None,
                    child_labels: Vec::new(),
                    node: None,
                    cycle: false,
                    missing: false,
                    repeated: false,
                };
                if active.contains(child_id) {
                    child.cycle = true;
                } else if let Some(child_task) = snapshot.task(child_id) {
                    child.child_display_id = child_task.display_id.clone();
                    child.child_title = Some(child_task.title.clone());
                    child.child_status = child_task.status.clone();
                    child.child_priority = Some(child_task.priority);
                    child.child_issue_type = Some(child_task.issue_type.clone());
                    child.child_labels = child_task.labels.clone();
                    if expanded.contains(child_id) {
                        child.repeated = true;
                    } else {
                        child.node = Some(Box::new(Self::build_task_dependency_tree(
                            snapshot,
                            child_id,
                            active,
                            expanded,
                            depth + 1,
                            node_visits,
                        )?));
                    }
                } else {
                    child.missing = true;
                }
                children.push(child);
            }
        }
        active.remove(&task.id);

        dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        children.sort_by(|left, right| left.child_id.cmp(&right.child_id));

        Ok(TaskDependencyTreeNode {
            task,
            dependencies,
            children,
        })
    }

    pub async fn validate_task_graph(&self) -> Result<Vec<TaskGraphIssue>, StateStoreError> {
        let tasks = self.all_tasks().await?;
        Ok(Self::validate_task_graph_rows(&tasks))
    }

    pub async fn critical_path(&self) -> Result<TaskCriticalPath, StateStoreError> {
        let tasks = self.all_tasks().await?;
        Self::critical_path_from_rows(&tasks)
    }

    pub(crate) fn critical_path_from_rows(
        tasks: &[TaskRecord],
    ) -> Result<TaskCriticalPath, StateStoreError> {
        let snapshot = TaskGraphSnapshot::from_rows(tasks);
        Self::critical_path_from_snapshot(&snapshot)
    }

    pub(crate) fn critical_path_from_snapshot(
        snapshot: &TaskGraphSnapshot,
    ) -> Result<TaskCriticalPath, StateStoreError> {
        let issues = Self::validate_task_graph_snapshot(snapshot);
        if !issues.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "task graph is invalid; run `vida task validate-graph` first".to_string(),
            });
        }

        let active_ids = snapshot
            .rows()
            .iter()
            .filter(|task| {
                task_status_is_open_like(&task.status)
                    && !work_item_is_program_container(&task.issue_type)
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();

        let mut memo = BTreeMap::<String, Vec<String>>::new();
        let mut active = BTreeSet::new();
        let mut best = Vec::new();
        for task_id in active_ids {
            let path = Self::critical_path_for_task(snapshot, &task_id, &mut memo, &mut active)?;
            if compare_task_paths(&path, &best).is_gt() {
                best = path;
            }
        }

        let nodes = best
            .into_iter()
            .filter_map(|task_id| snapshot.task(&task_id))
            .map(|task| TaskCriticalPathNode {
                id: task.id.clone(),
                title: task.title.clone(),
                status: task.status.clone(),
                issue_type: task.issue_type.clone(),
                priority: task.priority,
            })
            .collect::<Vec<_>>();

        Ok(TaskCriticalPath {
            length: nodes.len(),
            root_task_id: nodes.first().map(|node| node.id.clone()),
            terminal_task_id: nodes.last().map(|node| node.id.clone()),
            release_1_contract_steps: vec![TaskRelease1ContractStep {
                id: "doctor_run_graph_negative_control".to_string(),
                mode: "fail_closed".to_string(),
                blocker_code: crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::MissingRunGraphDispatchReceiptOperatorEvidence,
                )
                .to_string(),
                next_action: crate::status_surface_signals::missing_run_graph_dispatch_receipt_operator_evidence_next_action(),
            }],
            nodes,
        })
    }

    pub(crate) fn validate_task_graph_rows(tasks: &[TaskRecord]) -> Vec<TaskGraphIssue> {
        let snapshot = TaskGraphSnapshot::from_rows(tasks);
        Self::validate_task_graph_snapshot(&snapshot)
    }

    pub(crate) fn validate_task_graph_snapshot(
        snapshot: &TaskGraphSnapshot,
    ) -> Vec<TaskGraphIssue> {
        let mut issues = Vec::new();

        for task in snapshot.rows() {
            let parent_edges = task
                .dependencies
                .iter()
                .filter(|dependency| dependency.edge_type == "parent-child")
                .collect::<Vec<_>>();
            if parent_edges.len() > 1 {
                issues.push(TaskGraphIssue {
                    issue_type: "multiple_parent_edges".to_string(),
                    issue_id: task.id.clone(),
                    depends_on_id: None,
                    edge_type: Some("parent-child".to_string()),
                    detail: format!(
                        "task has {} parent-child edges; only one parent is allowed",
                        parent_edges.len()
                    ),
                });
            }
            if work_item_requires_parent(&task.issue_type)
                && !task_status_is_closed_like(&task.status)
                && parent_edges.is_empty()
            {
                issues.push(TaskGraphIssue {
                    issue_type: "missing_required_parent_edge".to_string(),
                    issue_id: task.id.clone(),
                    depends_on_id: None,
                    edge_type: Some("parent-child".to_string()),
                    detail: format!(
                        "non-closed work item kind `{}` requires one parent-child edge",
                        canonical_work_item_issue_type(&task.issue_type)
                    ),
                });
            }

            for dependency in &task.dependencies {
                if !snapshot.contains_task(&dependency.depends_on_id) {
                    issues.push(TaskGraphIssue {
                        issue_type: "missing_dependency_target".to_string(),
                        issue_id: task.id.clone(),
                        depends_on_id: Some(dependency.depends_on_id.clone()),
                        edge_type: Some(dependency.edge_type.clone()),
                        detail: "dependency target is missing from the authoritative runtime store"
                            .to_string(),
                    });
                }
                if dependency.depends_on_id == task.id {
                    issues.push(TaskGraphIssue {
                        issue_type: "self_dependency".to_string(),
                        issue_id: task.id.clone(),
                        depends_on_id: Some(dependency.depends_on_id.clone()),
                        edge_type: Some(dependency.edge_type.clone()),
                        detail: "task must not depend on itself".to_string(),
                    });
                }
                if dependency.edge_type == "parent-child" {
                    if let Some(parent) = snapshot.task(&dependency.depends_on_id) {
                        let child_kind = canonical_work_item_issue_type(&task.issue_type);
                        let parent_kind = canonical_work_item_issue_type(&parent.issue_type);
                        if child_kind == "epic" && parent_kind != "epic" {
                            issues.push(TaskGraphIssue {
                                issue_type: "invalid_parent_child_kind".to_string(),
                                issue_id: task.id.clone(),
                                depends_on_id: Some(parent.id.clone()),
                                edge_type: Some("parent-child".to_string()),
                                detail: format!(
                                    "epic work item `{}` can only be parented by another epic, got `{}`",
                                    task.id, parent.issue_type
                                ),
                            });
                        }
                    }
                }
            }
        }

        for task in snapshot.rows() {
            let Some(children) = snapshot.children_for(&task.id) else {
                continue;
            };
            if task_status_is_closed_like(&task.status) {
                for child_id in children {
                    let Some(child) = snapshot.task(child_id) else {
                        continue;
                    };
                    if !task_status_is_closed_like(&child.status) {
                        issues.push(TaskGraphIssue {
                            issue_type: "closed_parent_has_non_closed_child".to_string(),
                            issue_id: task.id.clone(),
                            depends_on_id: Some(child.id.clone()),
                            edge_type: Some("parent-child".to_string()),
                            detail: format!(
                                "closed parent has direct child {} with status {}",
                                child.id, child.status
                            ),
                        });
                    }
                }
            } else if task_status_is_open_like(&task.status)
                && work_item_is_program_container(&task.issue_type)
            {
                let has_non_closed_child = children.iter().any(|child_id| {
                    snapshot
                        .task(child_id)
                        .map(|child| !task_status_is_closed_like(&child.status))
                        .unwrap_or(false)
                });
                let has_unresolved_non_parent_dependency = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .any(|dependency| {
                        snapshot
                            .task(&dependency.depends_on_id)
                            .map(|dependency_task| {
                                !task_status_is_closed_like(&dependency_task.status)
                            })
                            .unwrap_or(true)
                    });
                let waiting_for_work_pool_handoff = task_is_spec_first_feature_parent(task)
                    && children
                        .iter()
                        .filter_map(|child_id| snapshot.task(child_id))
                        .any(|child| task_is_spec_pack_child(child))
                    && !children
                        .iter()
                        .filter_map(|child_id| snapshot.task(child_id))
                        .any(|child| task_is_work_pool_pack_child(child));
                if !has_non_closed_child
                    && !has_unresolved_non_parent_dependency
                    && !waiting_for_work_pool_handoff
                {
                    issues.push(TaskGraphIssue {
                        issue_type: "open_parent_has_no_open_child".to_string(),
                        issue_id: task.id.clone(),
                        depends_on_id: None,
                        edge_type: Some("parent-child".to_string()),
                        detail: "open or in-progress parent has no direct non-closed child"
                            .to_string(),
                    });
                }
            }
        }

        let mut visited = BTreeSet::new();
        let mut active = BTreeSet::new();
        for task in snapshot.rows() {
            Self::validate_parent_child_cycles(
                &task.id,
                snapshot.children_by_parent(),
                &mut visited,
                &mut active,
                &mut issues,
            );
        }

        let mut visited = BTreeSet::new();
        let mut active = BTreeSet::new();
        for task in snapshot.rows() {
            Self::validate_non_parent_dependency_cycles(
                &task.id,
                snapshot.non_parent_dependencies(),
                &mut visited,
                &mut active,
                &mut issues,
            );
        }

        issues.sort_by(|left, right| {
            left.issue_type
                .cmp(&right.issue_type)
                .then_with(|| left.issue_id.cmp(&right.issue_id))
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        issues.dedup();
        issues
    }

    fn task_graph_issue_key(
        issue: &TaskGraphIssue,
    ) -> (String, String, Option<String>, Option<String>) {
        (
            issue.issue_type.clone(),
            issue.issue_id.clone(),
            issue.depends_on_id.clone(),
            issue.edge_type.clone(),
        )
    }

    pub(crate) fn validate_task_graph_rows_for_mutation(
        before: &[TaskRecord],
        after: &[TaskRecord],
        touched_task_ids: &BTreeSet<String>,
    ) -> Vec<TaskGraphIssue> {
        let existing_issues = Self::validate_task_graph_rows(before)
            .into_iter()
            .map(|issue| Self::task_graph_issue_key(&issue))
            .collect::<BTreeSet<_>>();

        Self::validate_task_graph_rows(after)
            .into_iter()
            .filter(|issue| {
                touched_task_ids.contains(&issue.issue_id)
                    || issue
                        .depends_on_id
                        .as_ref()
                        .is_some_and(|id| touched_task_ids.contains(id))
                    || !existing_issues.contains(&Self::task_graph_issue_key(issue))
            })
            .collect()
    }

    fn critical_path_for_task(
        snapshot: &TaskGraphSnapshot,
        task_id: &str,
        memo: &mut BTreeMap<String, Vec<String>>,
        active: &mut BTreeSet<String>,
    ) -> Result<Vec<String>, StateStoreError> {
        if let Some(path) = memo.get(task_id) {
            return Ok(path.clone());
        }
        if !active.insert(task_id.to_string()) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("critical-path cycle detected at {task_id}"),
            });
        }

        let task = snapshot
            .task(task_id)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;
        let mut best_dependency_path = Vec::new();
        for dependency in &task.dependencies {
            if dependency.edge_type == "parent-child" {
                continue;
            }
            let Some(dep_task) = snapshot.task(&dependency.depends_on_id) else {
                continue;
            };
            if task_status_is_closed_like(&dep_task.status)
                || work_item_is_program_container(&dep_task.issue_type)
            {
                continue;
            }

            let candidate = Self::critical_path_for_task(snapshot, &dep_task.id, memo, active)?;
            if compare_task_paths(&candidate, &best_dependency_path).is_gt() {
                best_dependency_path = candidate;
            }
        }

        active.remove(task_id);
        best_dependency_path.push(task_id.to_string());
        memo.insert(task_id.to_string(), best_dependency_path.clone());
        Ok(best_dependency_path)
    }

    fn validate_parent_child_cycles(
        task_id: &str,
        parent_children: &BTreeMap<String, Vec<String>>,
        visited: &mut BTreeSet<String>,
        active: &mut BTreeSet<String>,
        issues: &mut Vec<TaskGraphIssue>,
    ) {
        if active.contains(task_id) {
            issues.push(TaskGraphIssue {
                issue_type: "parent_child_cycle".to_string(),
                issue_id: task_id.to_string(),
                depends_on_id: Some(task_id.to_string()),
                edge_type: Some("parent-child".to_string()),
                detail: "parent-child ancestry contains a cycle".to_string(),
            });
            return;
        }
        if visited.contains(task_id) {
            return;
        }

        visited.insert(task_id.to_string());
        active.insert(task_id.to_string());
        if let Some(children) = parent_children.get(task_id) {
            for child in children {
                Self::validate_parent_child_cycles(child, parent_children, visited, active, issues);
            }
        }
        active.remove(task_id);
    }

    fn validate_non_parent_dependency_cycles(
        task_id: &str,
        non_parent_dependencies: &BTreeMap<String, Vec<(String, String)>>,
        visited: &mut BTreeSet<String>,
        active: &mut BTreeSet<String>,
        issues: &mut Vec<TaskGraphIssue>,
    ) {
        if !visited.insert(task_id.to_string()) {
            return;
        }

        active.insert(task_id.to_string());
        if let Some(dependencies) = non_parent_dependencies.get(task_id) {
            for (depends_on_id, edge_type) in dependencies {
                if active.contains(depends_on_id) {
                    issues.push(TaskGraphIssue {
                        issue_type: "dependency_cycle".to_string(),
                        issue_id: task_id.to_string(),
                        depends_on_id: Some(depends_on_id.clone()),
                        edge_type: Some(edge_type.clone()),
                        detail: "non-parent dependency graph contains a cycle".to_string(),
                    });
                    continue;
                }
                Self::validate_non_parent_dependency_cycles(
                    depends_on_id,
                    non_parent_dependencies,
                    visited,
                    active,
                    issues,
                );
            }
        }
        active.remove(task_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher_task_commands::shell_quote;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vida-{label}-{}-{nanos}", std::process::id()))
    }

    async fn create_task_with_semantics(
        store: &StateStore,
        task_id: &str,
        execution_mode: Option<&str>,
        order_bucket: Option<&str>,
        parallel_group: Option<&str>,
        conflict_domain: Option<&str>,
    ) {
        let owned_paths = if execution_mode == Some("parallel_safe") {
            vec![format!("crates/test/{task_id}")]
        } else {
            Vec::new()
        };
        create_task_with_semantics_and_owned_paths(
            store,
            task_id,
            execution_mode,
            order_bucket,
            parallel_group,
            conflict_domain,
            owned_paths,
        )
        .await;
    }

    async fn create_task_with_semantics_and_owned_paths(
        store: &StateStore,
        task_id: &str,
        execution_mode: Option<&str>,
        order_bucket: Option<&str>,
        parallel_group: Option<&str>,
        conflict_domain: Option<&str>,
        owned_paths: Vec<String>,
    ) {
        let parent_id = format!("{task_id}-parent");
        store
            .create_task(CreateTaskRequest {
                task_id: &parent_id,
                title: &parent_id,
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("parent task should be created");
        store
            .create_task(CreateTaskRequest {
                task_id,
                title: task_id,
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some(&parent_id),
                labels: &[],
                execution_semantics: TaskExecutionSemantics {
                    execution_mode: execution_mode.map(ToOwned::to_owned),
                    order_bucket: order_bucket.map(ToOwned::to_owned),
                    parallel_group: parallel_group.map(ToOwned::to_owned),
                    conflict_domain: conflict_domain.map(ToOwned::to_owned),
                },
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths,
                    ..Default::default()
                },
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("task should be created");
    }

    fn task_record(task_id: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
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
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn proof_blocked_by_runtime_leaf_reports_runtime_proof_state() {
        let mut task = task_record("proof-runtime-blocked", "in_progress");
        task.issue_type = "defect".to_string();
        task.labels = vec!["proof-blocked-by-runtime".to_string()];
        task.planner_metadata.proof_targets =
            vec!["vida proof browser --route /blocked --json".to_string()];

        let summary = StateStore::task_progress_summary_from_rows(&[task], "proof-runtime-blocked")
            .expect("progress summary should build");

        assert!(!summary.ready_for_close);
        assert!(!summary.missing_proof);
        assert!(summary.proof_blocked_by_runtime);
        assert!(summary.blocked_by_runtime);
        assert_eq!(
            summary.closure_candidate_state,
            "leaf_proof_blocked_by_runtime"
        );
        assert_eq!(
            summary.next_required_command.as_deref(),
            Some("Record or resolve the runtime proof blocker before closing the leaf task.")
        );
    }

    fn parent_child_dependency(child_id: &str, parent_id: &str) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: child_id.to_string(),
            depends_on_id: parent_id.to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    fn dependency(issue_id: &str, depends_on_id: &str, edge_type: &str) -> TaskDependencyRecord {
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
    fn task_graph_snapshot_reuses_parent_child_and_dependency_indexes() {
        let mut parent = task_record("parent", "open");
        parent.issue_type = "epic".to_string();
        let blocker = task_record("blocker", "in_progress");
        let mut child = task_record("child", "open");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));
        child
            .dependencies
            .push(dependency("child", "blocker", "blocks"));

        let snapshot = TaskGraphSnapshot::from_rows(&[child, blocker, parent]);

        assert!(snapshot.contains_task("parent"));
        assert_eq!(snapshot.task("child").expect("child indexed").id, "child");
        assert_eq!(
            snapshot.children_for("parent").expect("parent children"),
            &vec!["child".to_string()]
        );
        assert_eq!(
            snapshot.scope_ids("parent").expect("scope ids"),
            BTreeSet::from(["child".to_string(), "parent".to_string()])
        );
        assert_eq!(
            snapshot
                .non_parent_dependencies()
                .get("child")
                .expect("child dependencies"),
            &vec![("blocker".to_string(), "blocks".to_string())]
        );

        let blockers = StateStore::task_blockers_from_snapshot(
            snapshot.task("child").expect("child task"),
            &snapshot,
        );
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].depends_on_id, "blocker");
        assert_eq!(blockers[0].dependency_status, "in_progress");
    }

    #[test]
    fn task_graph_snapshot_builds_reusable_task_index() {
        let mut epic = task_record("epic", "open");
        epic.issue_type = "epic".to_string();
        epic.priority = 3;

        let mut blocker = task_record("blocker", "closed");
        blocker.priority = 1;

        let mut child = task_record("child", "open");
        child.priority = 2;
        child
            .dependencies
            .push(parent_child_dependency("child", "epic"));
        child
            .dependencies
            .push(dependency("child", "blocker", "blocks"));

        let snapshot = TaskGraphSnapshot::from_rows(&[epic, child, blocker]);

        assert_eq!(
            snapshot
                .rows()
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["blocker", "child", "epic"]
        );
        assert!(snapshot.contains_task("child"));
        assert_eq!(
            snapshot.children_for("epic").cloned().unwrap_or_default(),
            vec!["child".to_string()]
        );
        assert_eq!(
            snapshot
                .non_parent_dependencies()
                .get("child")
                .cloned()
                .unwrap_or_default(),
            vec![("blocker".to_string(), "blocks".to_string())]
        );

        let scope_ids = snapshot.scope_ids("epic").expect("scope should resolve");
        assert_eq!(
            scope_ids.into_iter().collect::<Vec<_>>(),
            vec!["child".to_string(), "epic".to_string()]
        );
        let child_progress = snapshot
            .progress_rows()
            .iter()
            .find(|row| row.id == "child")
            .expect("child progress row should exist");
        assert_eq!(child_progress.parent_id.as_deref(), Some("epic"));
    }

    #[test]
    fn task_graph_snapshot_drives_key_graph_readers() {
        let mut epic = task_record("epic", "open");
        epic.issue_type = "epic".to_string();
        epic.priority = 0;

        let mut ready = task_record("ready", "open");
        ready.priority = 1;
        ready
            .dependencies
            .push(parent_child_dependency("ready", "epic"));
        ready
            .dependencies
            .push(dependency("ready", "blocker", "blocks"));

        let mut done = task_record("done", "closed");
        done.priority = 2;
        done.dependencies
            .push(parent_child_dependency("done", "epic"));

        let mut blocker = task_record("blocker", "closed");
        blocker.priority = 3;

        let snapshot = TaskGraphSnapshot::from_rows(&[epic, ready, done, blocker]);

        let progress = StateStore::task_progress_summary_from_snapshot(&snapshot, "epic")
            .expect("progress should reuse snapshot rows");
        assert_eq!(progress.descendant_count, 2);
        assert_eq!(progress.open_count, 1);
        assert_eq!(progress.closed_count, 1);

        let ready_tasks = StateStore::ready_tasks_scoped_from_snapshot(&snapshot, Some("epic"))
            .expect("ready tasks should reuse snapshot index");
        assert_eq!(
            ready_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ready"]
        );

        let critical_path = StateStore::critical_path_from_snapshot(&snapshot)
            .expect("critical path should reuse snapshot index");
        let critical_path_ids = critical_path
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            critical_path
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ready"]
        );

        let scheduling = StateStore::scheduling_projection_scoped_from_snapshot(
            &snapshot,
            Some("epic"),
            None,
            &critical_path_ids,
        )
        .expect("scheduling should reuse snapshot index");
        assert_eq!(scheduling.current_task_id.as_deref(), Some("ready"));
        assert_eq!(scheduling.ready.len(), 1);
        assert!(scheduling.blocked.is_empty());

        let tree = StateStore::task_dependency_tree_from_snapshot(&snapshot, "epic")
            .expect("tree should reuse snapshot index");
        assert_eq!(
            tree.children
                .iter()
                .map(|child| child.child_id.as_str())
                .collect::<Vec<_>>(),
            vec!["done", "ready"]
        );
        assert!(StateStore::validate_task_graph_snapshot(&snapshot).is_empty());
    }

    #[test]
    fn normalized_program_container_is_not_dispatchable_work() {
        let mut epic = task_record("mixed-case-epic", "open");
        epic.issue_type = "Epic".to_string();

        let ready = StateStore::ready_tasks_scoped_from_rows(&[epic.clone()], None)
            .expect("ready tasks should render");
        assert!(
            ready.is_empty(),
            "normalized epics must not be ready leaf work"
        );

        let projection = StateStore::scheduling_projection_scoped_from_rows(
            &[epic.clone()],
            None,
            None,
            &BTreeSet::new(),
        )
        .expect("scheduling projection should render");
        assert!(projection.ready.is_empty());
        assert!(projection.blocked.is_empty());
        assert_eq!(projection.current_task_id, None);

        let critical_path =
            StateStore::critical_path_from_rows(&[epic]).expect("critical path should render");
        assert!(critical_path.nodes.is_empty());
    }

    #[test]
    fn ready_tasks_exclude_container_only_execution_mode() {
        let mut work_pool = task_record("container-only-work-pool", "open");
        work_pool.issue_type = "task".to_string();
        work_pool.execution_semantics.execution_mode = Some("container_only".to_string());

        let ready = StateStore::ready_tasks_scoped_from_rows(&[work_pool], None)
            .expect("ready tasks should render");

        assert!(
            ready.is_empty(),
            "container_only execution-mode tasks must not be returned as executable ready work"
        );
    }

    #[test]
    fn validate_task_graph_flags_non_closed_parent_required_item_without_parent() {
        let child = task_record("child", "open");

        let issues = StateStore::validate_task_graph_rows(&[child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "missing_required_parent_edge"
                && issue.issue_id == "child"
                && issue.edge_type.as_deref() == Some("parent-child")
        }));
    }

    #[test]
    fn validate_task_graph_treats_external_closed_aliases_as_closed_like() {
        let merged_child = task_record("merged-child", "merged");
        let orphan_issues = StateStore::validate_task_graph_rows(&[merged_child]);
        assert!(
            orphan_issues
                .iter()
                .all(|issue| issue.issue_type != "missing_required_parent_edge"),
            "closed-like aliases must not be treated as open required-parent work: {orphan_issues:?}"
        );

        let parent = task_record("done-parent", "done");
        let mut child = task_record("open-child", "open");
        child
            .dependencies
            .push(parent_child_dependency("open-child", "done-parent"));
        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "closed_parent_has_non_closed_child"
                && issue.issue_id == "done-parent"
                && issue.depends_on_id.as_deref() == Some("open-child")
        }));
    }

    #[test]
    fn scheduling_projection_uses_closed_aliases_for_blockers_and_ready_work() {
        let done_task = task_record("done-task", "done");
        let blocker = task_record("resolved-blocker", "resolved");
        let mut ready = task_record("ready-task", "open");
        ready.dependencies.push(TaskDependencyRecord {
            issue_id: "ready-task".to_string(),
            depends_on_id: "resolved-blocker".to_string(),
            edge_type: "blocks".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });

        let projection = StateStore::scheduling_projection_scoped_from_rows(
            &[done_task, blocker, ready],
            None,
            None,
            &BTreeSet::new(),
        )
        .expect("scheduling projection should build");

        assert!(projection.ready.iter().any(|candidate| {
            candidate.task.id == "ready-task" && candidate.blocked_by.is_empty()
        }));
        assert!(
            projection
                .ready
                .iter()
                .all(|candidate| candidate.task.id != "done-task"),
            "done aliases must not be executable ready work"
        );
    }

    #[test]
    fn validate_task_graph_mutation_ignores_unrelated_existing_orphan() {
        let existing_orphan = task_record("existing-orphan", "open");
        let mut parent = task_record("parent", "open");
        parent.issue_type = "epic".to_string();
        let mut child = task_record("child", "open");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));
        let touched_task_ids = BTreeSet::from(["parent".to_string(), "child".to_string()]);

        let issues = StateStore::validate_task_graph_rows_for_mutation(
            &[existing_orphan.clone()],
            &[existing_orphan, parent, child],
            &touched_task_ids,
        );

        assert!(
            issues.is_empty(),
            "unrelated pre-existing graph debt should not block a valid mutation: {issues:?}"
        );
    }

    #[test]
    fn validate_task_graph_flags_epic_parented_by_non_epic() {
        let parent = task_record("parent", "open");
        let mut child = task_record("child-epic", "open");
        child.issue_type = "epic".to_string();
        child
            .dependencies
            .push(parent_child_dependency("child-epic", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "invalid_parent_child_kind"
                && issue.issue_id == "child-epic"
                && issue.depends_on_id.as_deref() == Some("parent")
        }));
    }

    #[test]
    fn validate_task_graph_flags_non_parent_dependency_cycle_with_stable_code() {
        let mut first = task_record("first", "open");
        first.issue_type = "epic".to_string();
        first
            .dependencies
            .push(dependency("first", "second", "blocks"));
        let mut second = task_record("second", "open");
        second.issue_type = "epic".to_string();
        second
            .dependencies
            .push(dependency("second", "first", "blocks"));

        let issues = StateStore::validate_task_graph_rows(&[first, second]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "dependency_cycle"
                && issue.edge_type.as_deref() == Some("blocks")
                && issue.detail == "non-parent dependency graph contains a cycle"
        }));
    }

    #[test]
    fn validate_task_graph_keeps_parent_child_and_blocks_edges_distinct() {
        let mut parent = task_record("parent", "open");
        parent.issue_type = "epic".to_string();
        parent
            .dependencies
            .push(dependency("parent", "child", "blocks"));
        let mut child = task_record("child", "open");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().all(|issue| {
            issue.issue_type != "dependency_cycle" && issue.issue_type != "parent_child_cycle"
        }));
    }

    #[test]
    fn validate_task_graph_mutation_reports_new_dependency_cycle() {
        let mut first = task_record("first", "open");
        first.issue_type = "epic".to_string();
        let mut second = task_record("second", "open");
        second.issue_type = "epic".to_string();
        let before = vec![first.clone(), second.clone()];
        first
            .dependencies
            .push(dependency("first", "second", "blocks"));
        second
            .dependencies
            .push(dependency("second", "first", "blocks"));
        let touched_task_ids = BTreeSet::from(["first".to_string(), "second".to_string()]);

        let issues = StateStore::validate_task_graph_rows_for_mutation(
            &before,
            &[first, second],
            &touched_task_ids,
        );

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "dependency_cycle" && issue.edge_type.as_deref() == Some("blocks")
        }));
    }

    #[test]
    fn validate_task_graph_flags_closed_parent_with_blocked_child() {
        let parent = task_record("parent", "closed");
        let mut child = task_record("child", "blocked");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "closed_parent_has_non_closed_child"
                && issue.issue_id == "parent"
                && issue.depends_on_id.as_deref() == Some("child")
                && issue.edge_type.as_deref() == Some("parent-child")
        }));
    }

    #[test]
    fn validate_task_graph_flags_closed_parent_with_paused_child() {
        let parent = task_record("parent", "closed");
        let mut child = task_record("child", "paused");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "closed_parent_has_non_closed_child"
                && issue.issue_id == "parent"
                && issue.depends_on_id.as_deref() == Some("child")
                && issue.edge_type.as_deref() == Some("parent-child")
        }));
    }

    #[test]
    fn validate_task_graph_flags_completed_parent_with_open_child() {
        let parent = task_record("parent", "completed");
        let mut child = task_record("child", "open");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "closed_parent_has_non_closed_child"
                && issue.issue_id == "parent"
                && issue.depends_on_id.as_deref() == Some("child")
                && issue.edge_type.as_deref() == Some("parent-child")
        }));
    }

    #[test]
    fn validate_task_graph_flags_in_progress_parent_with_no_open_child() {
        let mut parent = task_record("parent", "in_progress");
        parent.issue_type = "epic".to_string();
        let mut child = task_record("child", "closed");
        child
            .dependencies
            .push(parent_child_dependency("child", "parent"));

        let issues = StateStore::validate_task_graph_rows(&[parent, child]);

        assert!(issues.iter().any(|issue| {
            issue.issue_type == "open_parent_has_no_open_child"
                && issue.issue_id == "parent"
                && issue.depends_on_id.is_none()
                && issue.edge_type.as_deref() == Some("parent-child")
        }));
    }

    #[test]
    fn validate_task_graph_accepts_parent_child_closure_consistent_rows() {
        let open_parent = task_record("open-parent", "open");
        let mut open_child = task_record("open-child", "in_progress");
        open_child
            .dependencies
            .push(parent_child_dependency("open-child", "open-parent"));
        let closed_parent = task_record("closed-parent", "closed");
        let mut closed_child = task_record("closed-child", "closed");
        closed_child
            .dependencies
            .push(parent_child_dependency("closed-child", "closed-parent"));

        let issues = StateStore::validate_task_graph_rows(&[
            open_parent,
            open_child,
            closed_parent,
            closed_child,
        ]);

        assert!(issues.iter().all(|issue| {
            issue.issue_type != "closed_parent_has_non_closed_child"
                && issue.issue_type != "open_parent_has_no_open_child"
        }));
    }

    #[test]
    fn task_dependency_tree_rejects_excessive_depth() {
        let chain_len = TASK_TREE_MAX_DEPTH + 2;
        let mut rows = Vec::with_capacity(chain_len);
        for index in 0..chain_len {
            let task_id = format!("task-{index}");
            let mut task = task_record(&task_id, "open");
            if index > 0 {
                task.dependencies.push(TaskDependencyRecord {
                    issue_id: task_id.clone(),
                    depends_on_id: format!("task-{}", index - 1),
                    edge_type: "blocks".to_string(),
                    created_at: "1".to_string(),
                    created_by: "test".to_string(),
                    metadata: "{}".to_string(),
                    thread_id: String::new(),
                });
            }
            rows.push(task);
        }

        let result =
            StateStore::task_dependency_tree_from_rows(&rows, &format!("task-{}", chain_len - 1));
        match result {
            Err(StateStoreError::InvalidTaskRecord { reason }) => {
                assert!(reason.contains("exceeds max depth"));
            }
            other => panic!("expected InvalidTaskRecord depth error, got {other:?}"),
        }
    }

    #[test]
    fn task_dependency_tree_expands_shared_subtree_once() {
        let shared_chain_len = 50;
        let child_count = 250;
        let mut rows = Vec::with_capacity(shared_chain_len + child_count + 1);
        rows.push(task_record("root-task", "open"));

        for index in 0..shared_chain_len {
            let task_id = format!("shared-{index}");
            let mut task = task_record(&task_id, "open");
            if index > 0 {
                task.dependencies.push(TaskDependencyRecord {
                    issue_id: task_id.clone(),
                    depends_on_id: format!("shared-{}", index - 1),
                    edge_type: "blocks".to_string(),
                    created_at: "1".to_string(),
                    created_by: "test".to_string(),
                    metadata: "{}".to_string(),
                    thread_id: String::new(),
                });
            }
            rows.push(task);
        }

        for index in 0..child_count {
            let task_id = format!("child-{index}");
            let mut task = task_record(&task_id, "open");
            task.dependencies.push(TaskDependencyRecord {
                issue_id: task_id.clone(),
                depends_on_id: "root-task".to_string(),
                edge_type: "parent-child".to_string(),
                created_at: "1".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
            task.dependencies.push(TaskDependencyRecord {
                issue_id: task_id.clone(),
                depends_on_id: format!("shared-{}", shared_chain_len - 1),
                edge_type: "blocks".to_string(),
                created_at: "1".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
            rows.push(task);
        }

        let tree = StateStore::task_dependency_tree_from_rows(&rows, "root-task")
            .expect("shared dependency subtree should not exhaust node visits");

        assert_eq!(tree.children.len(), child_count);
        let repeated_shared_refs = tree
            .children
            .iter()
            .filter_map(|child| child.node.as_ref())
            .flat_map(|node| node.dependencies.iter())
            .filter(|edge| edge.depends_on_id == format!("shared-{}", shared_chain_len - 1))
            .filter(|edge| edge.repeated)
            .count();
        assert_eq!(repeated_shared_refs, child_count - 1);
    }

    #[test]
    fn task_dependency_tree_marks_cycles_without_recursive_expansion() {
        let mut root = task_record("root-task", "open");
        let mut child = task_record("child-task", "open");
        child
            .dependencies
            .push(parent_child_dependency("child-task", "root-task"));
        root.dependencies.push(TaskDependencyRecord {
            issue_id: "root-task".to_string(),
            depends_on_id: "child-task".to_string(),
            edge_type: "blocks".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });

        let tree = StateStore::task_dependency_tree_from_rows(&[root, child], "root-task")
            .expect("cycle should be represented, not recursively expanded");

        let dependency_node = tree.dependencies[0]
            .node
            .as_ref()
            .expect("child dependency should have one bounded node");
        let back_edge = dependency_node
            .dependencies
            .iter()
            .find(|edge| edge.depends_on_id == "root-task")
            .expect("back edge should be present");
        assert!(back_edge.cycle);
        assert!(back_edge.node.is_none());
    }

    #[tokio::test]
    async fn scheduling_projection_fail_closes_when_semantics_are_missing() {
        let root = temp_root("task-scheduling-fail-closed");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics(
            &store,
            "task-current",
            Some("parallel_safe"),
            Some("wave-1"),
            None,
            Some("backend"),
        )
        .await;
        create_task_with_semantics(&store, "task-legacy", None, None, None, None).await;

        let projection = store
            .scheduling_projection_scoped(None, Some("task-current"))
            .await
            .expect("projection should render");
        let legacy = projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == "task-legacy")
            .expect("legacy task should be ready");

        assert!(legacy.ready_now);
        assert!(!legacy.ready_parallel_safe);
        assert!(legacy
            .parallel_blockers
            .iter()
            .any(|value| value == "execution_mode_not_parallel_safe"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduling_projection_rejects_parallel_safe_tasks_without_owned_paths() {
        let root = temp_root("task-scheduling-missing-owned-paths");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics_and_owned_paths(
            &store,
            "task-current",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("current-domain"),
            vec!["crates/current".to_string()],
        )
        .await;
        create_task_with_semantics_and_owned_paths(
            &store,
            "task-empty-owned-paths",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("candidate-domain"),
            Vec::new(),
        )
        .await;

        let projection = store
            .scheduling_projection_scoped(None, Some("task-current"))
            .await
            .expect("projection should render");
        let candidate = projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == "task-empty-owned-paths")
            .expect("empty-owned task should remain graph-ready");

        assert!(candidate.ready_now);
        assert!(!candidate.ready_parallel_safe);
        assert!(candidate.parallel_blockers.iter().any(|value| value
            == scheduler_dispatch::PARALLEL_BLOCKER_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION));
        assert!(projection
            .parallel_candidates_after_current
            .iter()
            .all(|task| task.id != "task-empty-owned-paths"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduling_projection_rejects_parallel_safe_tasks_with_overlapping_owned_paths() {
        let root = temp_root("task-scheduling-overlapping-owned-paths");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics_and_owned_paths(
            &store,
            "task-current",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("current-domain"),
            vec!["crates/shared/src/lib.rs".to_string()],
        )
        .await;
        create_task_with_semantics_and_owned_paths(
            &store,
            "task-overlapping-owned-path",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("candidate-domain"),
            vec![" crates/shared/src/lib.rs ".to_string()],
        )
        .await;

        let projection = store
            .scheduling_projection_scoped(None, Some("task-current"))
            .await
            .expect("projection should render");
        let candidate = projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == "task-overlapping-owned-path")
            .expect("overlapping-owned-path task should remain graph-ready");

        assert!(candidate.ready_now);
        assert!(!candidate.ready_parallel_safe);
        assert!(candidate
            .parallel_blockers
            .iter()
            .any(|value| value == scheduler_dispatch::PARALLEL_BLOCKER_OWNED_PATH_COLLISION));
        assert!(projection
            .parallel_candidates_after_current
            .iter()
            .all(|task| task.id != "task-overlapping-owned-path"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduling_projection_blocks_container_only_tasks_from_execution() {
        let root = temp_root("task-scheduling-container-only");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics(
            &store,
            "task-current",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("backend"),
        )
        .await;
        create_task_with_semantics(
            &store,
            "task-work-pool",
            Some("container_only"),
            Some("wave-1"),
            Some("work-pool-pack"),
            Some("task-work-pool"),
        )
        .await;

        let projection = store
            .scheduling_projection_scoped(None, Some("task-current"))
            .await
            .expect("projection should render");

        assert_eq!(projection.current_task_id.as_deref(), Some("task-current"));
        assert!(projection
            .ready
            .iter()
            .all(|candidate| candidate.task.id != "task-work-pool"));
        let container = projection
            .blocked
            .iter()
            .find(|candidate| candidate.task.id == "task-work-pool")
            .expect("container-only task should be blocked from executable scheduling");
        assert!(!container.ready_now);
        assert!(!container.ready_parallel_safe);
        assert!(container
            .blocked_by
            .iter()
            .any(|blocker| blocker.dependency_status == "container_only_task"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scheduling_projection_allows_only_compatible_parallel_safe_tasks() {
        let root = temp_root("task-scheduling-compatible");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics(
            &store,
            "task-current",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("backend"),
        )
        .await;
        create_task_with_semantics(
            &store,
            "task-compatible",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("frontend"),
        )
        .await;
        create_task_with_semantics(
            &store,
            "task-collision",
            Some("parallel_safe"),
            Some("wave-1"),
            Some("writers"),
            Some("backend"),
        )
        .await;

        let projection = store
            .scheduling_projection_scoped(None, Some("task-current"))
            .await
            .expect("projection should render");
        assert_eq!(projection.current_task_id.as_deref(), Some("task-current"));
        assert_eq!(
            projection
                .parallel_candidates_after_current
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["task-compatible"]
        );

        let collision = projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == "task-collision")
            .expect("collision task should be present");
        assert!(!collision.ready_parallel_safe);
        assert!(collision
            .parallel_blockers
            .iter()
            .any(|value| value == "conflict_domain_collision"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn missing_dependency_targets_block_ready_and_scheduling() {
        let root = temp_root("task-missing-dependency-blocker");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics(
            &store,
            "task-with-missing-dependency",
            None,
            None,
            None,
            None,
        )
        .await;

        let mut task = store
            .show_task("task-with-missing-dependency")
            .await
            .expect("task should exist");
        task.dependencies.push(TaskDependencyRecord {
            issue_id: "task-with-missing-dependency".to_string(),
            depends_on_id: "task-missing".to_string(),
            edge_type: "blocks".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });
        store
            .persist_task_record(task)
            .await
            .expect("missing dependency should be persisted for readiness test");

        let ready = store
            .ready_tasks_scoped(None)
            .await
            .expect("ready tasks should render");
        assert!(ready
            .iter()
            .all(|task| task.id != "task-with-missing-dependency"));

        let projection = store
            .scheduling_projection_scoped(None, None)
            .await
            .expect("projection should render");
        let blocked = projection
            .blocked
            .iter()
            .find(|candidate| candidate.task.id == "task-with-missing-dependency")
            .expect("task should be blocked");
        assert!(!blocked.ready_now);
        assert!(blocked
            .blocked_by
            .iter()
            .any(|dependency| dependency.dependency_status == "missing"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn task_tree_payload_is_bounded_by_default() {
        let root = temp_root("task-tree-bounded-default");
        let store = StateStore::open(root.clone()).await.expect("open store");

        create_task_with_semantics(&store, "root-task", None, None, None, None).await;
        create_task_with_semantics(&store, "dependency-task", None, None, None, None).await;
        create_task_with_semantics(&store, "nested-dependency", None, None, None, None).await;
        create_task_with_semantics(&store, "child-task", None, None, None, None).await;
        create_task_with_semantics(&store, "grandchild-task", None, None, None, None).await;

        let mut root_task = store.show_task("root-task").await.expect("root exists");
        root_task.dependencies.push(TaskDependencyRecord {
            issue_id: "root-task".to_string(),
            depends_on_id: "dependency-task".to_string(),
            edge_type: "blocks".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });
        store
            .persist_task_record(root_task)
            .await
            .expect("persist root dependency");

        let mut dependency_task = store
            .show_task("dependency-task")
            .await
            .expect("dependency exists");
        dependency_task.dependencies.push(TaskDependencyRecord {
            issue_id: "dependency-task".to_string(),
            depends_on_id: "nested-dependency".to_string(),
            edge_type: "blocks".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });
        store
            .persist_task_record(dependency_task)
            .await
            .expect("persist nested dependency");

        let mut child_task = store.show_task("child-task").await.expect("child exists");
        child_task.dependencies.push(TaskDependencyRecord {
            issue_id: "child-task".to_string(),
            depends_on_id: "root-task".to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });
        store
            .persist_task_record(child_task)
            .await
            .expect("persist child edge");

        let mut grandchild_task = store
            .show_task("grandchild-task")
            .await
            .expect("grandchild exists");
        grandchild_task.dependencies.push(TaskDependencyRecord {
            issue_id: "grandchild-task".to_string(),
            depends_on_id: "child-task".to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "1".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });
        store
            .persist_task_record(grandchild_task)
            .await
            .expect("persist grandchild edge");

        let tree = store
            .task_dependency_tree("root-task")
            .await
            .expect("tree should render");

        assert_eq!(
            tree.dependencies
                .iter()
                .filter(|edge| edge.edge_type == "blocks")
                .count(),
            1
        );
        assert_eq!(tree.children.len(), 1);
        let dependency_node = tree
            .dependencies
            .iter()
            .find(|edge| edge.edge_type == "blocks")
            .expect("blocks dependency should be present")
            .node
            .as_ref()
            .expect("dependency node should be included");
        assert_eq!(dependency_node.task.id, "dependency-task");
        assert_eq!(
            dependency_node
                .dependencies
                .iter()
                .filter(|edge| edge.edge_type == "blocks")
                .count(),
            1
        );
        let nested_dependency = dependency_node
            .dependencies
            .iter()
            .find(|edge| edge.edge_type == "blocks")
            .expect("nested blocks dependency should be present");
        assert_eq!(nested_dependency.depends_on_id, "nested-dependency");
        let child_node = tree.children[0]
            .node
            .as_ref()
            .expect("child node should be included");
        assert_eq!(child_node.task.id, "child-task");
        assert_eq!(child_node.children.len(), 1);
        assert_eq!(child_node.children[0].child_id, "grandchild-task");

        let _ = fs::remove_dir_all(root);
    }
}
