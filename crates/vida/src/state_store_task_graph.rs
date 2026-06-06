use super::*;
use crate::launcher_task_commands::shell_quote;
use crate::state_store::state_store_task_models::{
    task_is_spec_first_feature_parent, task_is_spec_pack_child, task_is_work_pool_pack_child,
};

const TASK_TREE_MAX_DEPTH: usize = 64;
const TASK_TREE_MAX_NODE_VISITS: usize = 10_000;

impl StateStore {
    fn parent_child_reverse_index(rows: &[TaskRecord]) -> BTreeMap<String, Vec<String>> {
        let mut children = BTreeMap::<String, Vec<String>>::new();
        for task in rows {
            for dependency in &task.dependencies {
                if dependency.edge_type != "parent-child" {
                    continue;
                }
                children
                    .entry(dependency.depends_on_id.clone())
                    .or_default()
                    .push(task.id.clone());
            }
        }
        children
    }

    fn task_is_open_like(task: &TaskRecord) -> bool {
        (task.status == "open" || task.status == "in_progress")
            && !work_item_is_program_container(&task.issue_type)
    }

    fn task_blockers(
        task: &TaskRecord,
        by_id: &BTreeMap<String, TaskRecord>,
    ) -> Vec<TaskDependencyStatus> {
        let mut blockers = task
            .dependencies
            .iter()
            .filter(|dependency| dependency.edge_type != "parent-child")
            .filter_map(|dependency| match by_id.get(&dependency.depends_on_id) {
                Some(blocker_task) if Self::task_status_is_closed_like(&blocker_task.status) => {
                    None
                }
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
            })
            .collect::<Vec<_>>();
        blockers.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        blockers
    }

    fn compatible_parallel_group(task: &TaskRecord, current: &TaskRecord) -> Result<(), String> {
        match (
            task.execution_semantics.parallel_group.as_deref(),
            current.execution_semantics.parallel_group.as_deref(),
        ) {
            (None, None) => Ok(()),
            (Some(left), Some(right)) if left == right => Ok(()),
            _ => Err("parallel_group_mismatch".to_string()),
        }
    }

    fn parallel_blockers_against_current(
        task: &TaskRecord,
        current: Option<&TaskRecord>,
    ) -> Vec<String> {
        let Some(current) = current else {
            return vec!["no_current_task_reference".to_string()];
        };
        if task.id == current.id {
            return vec!["current_task_reference".to_string()];
        }

        let mut blockers = Vec::new();
        if task.execution_semantics.execution_mode.as_deref() != Some("parallel_safe") {
            blockers.push("execution_mode_not_parallel_safe".to_string());
        }
        if current.execution_semantics.execution_mode.as_deref() != Some("parallel_safe") {
            blockers.push("current_execution_mode_not_parallel_safe".to_string());
        }

        match (
            task.execution_semantics.order_bucket.as_deref(),
            current.execution_semantics.order_bucket.as_deref(),
        ) {
            (Some(left), Some(right)) if left == right => {}
            _ => blockers.push("order_bucket_mismatch_or_missing".to_string()),
        }

        match (
            task.execution_semantics.conflict_domain.as_deref(),
            current.execution_semantics.conflict_domain.as_deref(),
        ) {
            (Some(left), Some(right)) if left != right => {}
            (Some(_), Some(_)) => blockers.push("conflict_domain_collision".to_string()),
            _ => blockers.push("missing_conflict_domain".to_string()),
        }

        if let Err(blocker) = Self::compatible_parallel_group(task, current) {
            blockers.push(blocker);
        }

        blockers
    }

    fn task_is_container_only(task: &TaskRecord) -> bool {
        task.execution_semantics.execution_mode.as_deref() == Some("container_only")
            || work_item_is_program_container(&task.issue_type)
    }

    fn ready_scope_ids_from_rows(
        rows: &[TaskRecord],
        scope_task_id: &str,
    ) -> Result<BTreeSet<String>, StateStoreError> {
        if !rows.iter().any(|task| task.id == scope_task_id) {
            return Err(StateStoreError::MissingTask {
                task_id: scope_task_id.to_string(),
            });
        }

        let children = Self::parent_child_reverse_index(rows);

        let mut scope_ids = BTreeSet::new();
        let mut stack = vec![scope_task_id.to_string()];
        while let Some(current) = stack.pop() {
            if !scope_ids.insert(current.clone()) {
                continue;
            }
            if let Some(descendants) = children.get(&current) {
                stack.extend(descendants.iter().cloned());
            }
        }

        Ok(scope_ids)
    }

    pub(crate) fn ready_tasks_scoped_from_rows(
        rows: &[TaskRecord],
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut rows = rows.to_vec();
        rows.sort_by(task_sort_key);

        let by_id = rows
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(Self::ready_scope_ids_from_rows(&rows, scope_task_id)?)
        } else {
            None
        };

        let mut ready = rows
            .into_iter()
            .filter(|task| {
                scope_ids
                    .as_ref()
                    .map(|ids| ids.contains(&task.id))
                    .unwrap_or(true)
            })
            .filter(Self::task_is_open_like)
            .filter(|task| !Self::task_is_container_only(task))
            .filter(|task| Self::task_blockers(task, &by_id).is_empty())
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
        let mut rows = self.all_tasks().await?;
        rows.sort_by(task_sort_key);
        let mut critical_path_ids = BTreeSet::new();
        if let Ok(path) = Self::critical_path_from_rows(&rows) {
            critical_path_ids.extend(path.nodes.into_iter().map(|node| node.id));
        }

        Self::scheduling_projection_scoped_from_rows(
            &rows,
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
        let mut rows = rows.to_vec();
        rows.sort_by(task_sort_key);

        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(Self::ready_scope_ids_from_rows(&rows, scope_task_id)?)
        } else {
            None
        };

        let by_id = rows
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let scoped_tasks = rows
            .into_iter()
            .filter(|task| {
                scope_ids
                    .as_ref()
                    .map(|ids| ids.contains(&task.id))
                    .unwrap_or(true)
            })
            .filter(Self::task_is_open_like)
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
                    .find(|task| Self::task_blockers(task, &by_id).is_empty())
                    .map(|task| task.id.clone())
            });
        let current_task = chosen_current
            .as_deref()
            .and_then(|task_id| by_id.get(task_id));

        let mut ready = Vec::new();
        let mut blocked = Vec::new();
        for task in scoped_tasks {
            let active_critical_path = critical_path_ids.contains(&task.id);
            let mut blocked_by = Self::task_blockers(&task, &by_id);
            if Self::task_is_container_only(&task) && blocked_by.is_empty() {
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
                Self::parallel_blockers_against_current(&task, current_task)
            } else {
                vec!["graph_blocked".to_string()]
            };
            let candidate = TaskSchedulingCandidate {
                task,
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
        let root_task = rows
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;
        let children_by_parent = Self::parent_child_reverse_index(&rows);
        let scope_ids = Self::ready_scope_ids_from_rows(&rows, task_id)?;
        let descendant_ids = scope_ids
            .into_iter()
            .filter(|candidate| candidate != task_id)
            .collect::<BTreeSet<_>>();

        let mut status_counts = BTreeMap::<String, usize>::new();
        let mut open_count = 0usize;
        let mut in_progress_count = 0usize;
        let mut closed_count = 0usize;
        let mut epic_count = 0usize;

        for task in rows.iter().filter(|task| descendant_ids.contains(&task.id)) {
            *status_counts.entry(task.status.clone()).or_insert(0) += 1;
            match task.status.as_str() {
                "open" => open_count += 1,
                "in_progress" => in_progress_count += 1,
                "closed" => closed_count += 1,
                _ => {}
            }
            if work_item_is_program_container(&task.issue_type) {
                epic_count += 1;
            }
        }

        let descendant_count = descendant_ids.len();
        let percent_closed = if descendant_count == 0 {
            0.0
        } else {
            (closed_count as f64 / descendant_count as f64) * 100.0
        };
        let all_descendants_closed_like = rows
            .iter()
            .filter(|task| descendant_ids.contains(&task.id))
            .all(|task| Self::task_status_is_closed_like(&task.status));
        let is_container = work_item_is_program_container(&root_task.issue_type);
        let root_closed = Self::task_status_is_closed_like(&root_task.status);
        let is_non_container_work_item = !is_container;
        let non_container_descendants_clear = descendant_count == 0 || all_descendants_closed_like;
        let proof_blocked_by_runtime = !root_closed
            && is_non_container_work_item
            && non_container_descendants_clear
            && !root_task.planner_metadata.proof_targets.is_empty()
            && root_task.labels.iter().any(|label| {
                label == "proof-blocked-by-runtime" || label == "runtime-proof-blocked"
            });
        let blocked_by_runtime = proof_blocked_by_runtime
            || (!root_closed
                && is_non_container_work_item
                && (root_task.status == "blocked"
                    || root_task
                        .labels
                        .iter()
                        .any(|label| label == "runtime-blocked" || label == "blocked-by-runtime")));
        let missing_proof = !root_closed
            && is_non_container_work_item
            && non_container_descendants_clear
            && !root_task.planner_metadata.proof_targets.is_empty()
            && !proof_blocked_by_runtime;
        let leaf_ready_for_close = !root_closed
            && is_non_container_work_item
            && non_container_descendants_clear
            && !missing_proof
            && !blocked_by_runtime
            && matches!(
                root_task.status.as_str(),
                "in_progress" | "review" | "verified" | "ready_for_close"
            );
        let closure_candidate =
            is_container && !root_closed && descendant_count > 0 && all_descendants_closed_like;
        let (
            closure_candidate_state,
            closure_candidate_reason,
            recommended_next_action,
            canonical_commands,
            next_required_command,
        ) = if closure_candidate {
            let quoted_root_task_id = shell_quote(&root_task.id);
            let close_command = format!(
                "vida task close {} --reason \"all descendants closed\" --json",
                quoted_root_task_id
            );
            (
                "ready_to_close".to_string(),
                Some(
                    "root container is open while all descendants are closed-like".to_string(),
                ),
                format!(
                    "Close container with `vida task close {} --reason \"all descendants closed\" --json`.",
                    quoted_root_task_id
                ),
                vec![close_command.clone()],
                Some(close_command),
            )
        } else if root_closed {
            (
                "already_closed".to_string(),
                Some("root task is already closed-like".to_string()),
                "No action; task is already closed.".to_string(),
                Vec::new(),
                None,
            )
        } else if is_non_container_work_item {
            let child_work_remaining = descendant_count > 0 && !all_descendants_closed_like;
            let next_required_command = if child_work_remaining {
                Some(
                    "Close or complete child work before closing the parent work item.".to_string(),
                )
            } else if missing_proof {
                Some(
                    "Run declared proof targets, then close the leaf task with explicit evidence."
                        .to_string(),
                )
            } else if proof_blocked_by_runtime {
                Some(
                    "Record or resolve the runtime proof blocker before closing the leaf task."
                        .to_string(),
                )
            } else if blocked_by_runtime {
                Some(
                    "Record or resolve the runtime blocker before closing the leaf task."
                        .to_string(),
                )
            } else if leaf_ready_for_close {
                let close_command = format!(
                    "vida task close {} --reason \"verified\" --json",
                    shell_quote(&root_task.id)
                );
                Some(close_command)
            } else {
                Some("Continue the leaf task until verification evidence is available.".to_string())
            };
            let closure_candidate_state = if child_work_remaining {
                "work_item_child_work_remaining"
            } else if missing_proof {
                "leaf_missing_proof"
            } else if proof_blocked_by_runtime {
                "leaf_proof_blocked_by_runtime"
            } else if blocked_by_runtime {
                "leaf_blocked_by_runtime"
            } else if leaf_ready_for_close {
                "leaf_ready_for_close"
            } else {
                "leaf_in_progress"
            };
            let closure_candidate_reason = if descendant_count == 0 {
                "leaf task uses proof readiness instead of container closure semantics"
            } else {
                "non-container work item uses proof readiness instead of container closure semantics"
            };
            (
                closure_candidate_state.to_string(),
                Some(closure_candidate_reason.to_string()),
                next_required_command
                    .clone()
                    .unwrap_or_else(|| "Continue normal leaf task execution.".to_string()),
                Vec::new(),
                next_required_command,
            )
        } else if descendant_count == 0 {
            (
                "container_without_descendants".to_string(),
                Some("container has no descendants to prove closure readiness".to_string()),
                "Add child work items or close with an explicit operator reason.".to_string(),
                Vec::new(),
                Some("Add child work items or close with an explicit operator reason.".to_string()),
            )
        } else {
            (
                "active_descendants_remaining".to_string(),
                Some("one or more descendants are not closed-like".to_string()),
                "Continue or close remaining descendant work before closing the container."
                    .to_string(),
                Vec::new(),
                Some(
                    "Continue or close remaining descendant work before closing the container."
                        .to_string(),
                ),
            )
        };

        Ok(TaskProgressSummary {
            root_task,
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: children_by_parent.get(task_id).map(Vec::len).unwrap_or(0),
            descendant_count,
            open_count,
            in_progress_count,
            closed_count,
            epic_count,
            status_counts,
            percent_closed,
            closure_candidate,
            closure_candidate_state,
            closure_candidate_reason,
            ready_for_close: closure_candidate || leaf_ready_for_close,
            missing_proof,
            proof_blocked_by_runtime,
            blocked_by_runtime,
            next_required_command,
            recommended_next_action,
            canonical_commands,
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
        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let tree_rows = by_id.values().cloned().collect::<Vec<_>>();
        let children_by_parent = Self::parent_child_reverse_index(&tree_rows);
        let mut active = BTreeSet::new();
        let mut expanded = BTreeSet::new();
        let mut node_visits = 0usize;
        Self::build_task_dependency_tree(
            &by_id,
            &children_by_parent,
            task_id,
            &mut active,
            &mut expanded,
            0,
            &mut node_visits,
        )
    }

    fn build_task_dependency_tree(
        by_id: &BTreeMap<String, TaskRecord>,
        children_by_parent: &BTreeMap<String, Vec<String>>,
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

        let task = by_id
            .get(task_id)
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
            } else if let Some(dependency_task) = by_id.get(&dependency.depends_on_id) {
                edge.dependency_status = dependency_task.status.clone();
                edge.dependency_issue_type = Some(dependency_task.issue_type.clone());
                if expanded.contains(&dependency.depends_on_id) {
                    edge.repeated = true;
                } else {
                    edge.node = Some(Box::new(Self::build_task_dependency_tree(
                        by_id,
                        children_by_parent,
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
        if let Some(child_ids) = children_by_parent.get(&task.id) {
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
                } else if let Some(child_task) = by_id.get(child_id) {
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
                            by_id,
                            children_by_parent,
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
        let issues = Self::validate_task_graph_rows(&tasks);
        if !issues.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "task graph is invalid; run `vida task validate-graph` first".to_string(),
            });
        }

        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let active_ids = tasks
            .iter()
            .filter(|task| {
                (task.status == "open" || task.status == "in_progress")
                    && !work_item_is_program_container(&task.issue_type)
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();

        let mut memo = BTreeMap::<String, Vec<String>>::new();
        let mut active = BTreeSet::new();
        let mut best = Vec::new();
        for task_id in active_ids {
            let path = Self::critical_path_for_task(&by_id, &task_id, &mut memo, &mut active)?;
            if compare_task_paths(&path, &best).is_gt() {
                best = path;
            }
        }

        let nodes = best
            .into_iter()
            .filter_map(|task_id| by_id.get(&task_id))
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
        let by_id = tasks
            .iter()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let mut issues = Vec::new();

        for task in tasks {
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
                && !Self::task_status_is_closed_like(&task.status)
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
                if !by_id.contains_key(&dependency.depends_on_id) {
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
                    if let Some(parent) = by_id.get(&dependency.depends_on_id) {
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

        let mut parent_children = BTreeMap::<String, Vec<String>>::new();
        for task in tasks {
            for dependency in &task.dependencies {
                if dependency.edge_type == "parent-child" {
                    parent_children
                        .entry(dependency.depends_on_id.clone())
                        .or_default()
                        .push(task.id.clone());
                }
            }
        }

        for task in tasks {
            let Some(children) = parent_children.get(&task.id) else {
                continue;
            };
            if task.status == "closed" {
                for child_id in children {
                    let Some(child) = by_id.get(child_id) else {
                        continue;
                    };
                    if !Self::task_status_is_closed_like(&child.status) {
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
            } else if (task.status == "open" || task.status == "in_progress")
                && work_item_is_program_container(&task.issue_type)
            {
                let has_non_closed_child = children.iter().any(|child_id| {
                    by_id
                        .get(child_id)
                        .map(|child| !Self::task_status_is_closed_like(&child.status))
                        .unwrap_or(false)
                });
                let has_unresolved_non_parent_dependency = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .any(|dependency| {
                        by_id
                            .get(&dependency.depends_on_id)
                            .map(|dependency_task| {
                                !Self::task_status_is_closed_like(&dependency_task.status)
                            })
                            .unwrap_or(true)
                    });
                let waiting_for_work_pool_handoff = task_is_spec_first_feature_parent(task)
                    && children
                        .iter()
                        .filter_map(|child_id| by_id.get(child_id))
                        .any(|child| task_is_spec_pack_child(child))
                    && !children
                        .iter()
                        .filter_map(|child_id| by_id.get(child_id))
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
        for task in tasks {
            Self::validate_parent_child_cycles(
                &task.id,
                &parent_children,
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
        by_id: &BTreeMap<String, TaskRecord>,
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

        let task = by_id
            .get(task_id)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;
        let mut best_dependency_path = Vec::new();
        for dependency in &task.dependencies {
            if dependency.edge_type == "parent-child" {
                continue;
            }
            let Some(dep_task) = by_id.get(&dependency.depends_on_id) else {
                continue;
            };
            if Self::task_status_is_closed_like(&dep_task.status)
                || work_item_is_program_container(&dep_task.issue_type)
            {
                continue;
            }

            let candidate = Self::critical_path_for_task(by_id, &dep_task.id, memo, active)?;
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
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
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
    fn validate_task_graph_flags_in_progress_parent_with_no_open_child() {
        let parent = task_record("parent", "in_progress");
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
