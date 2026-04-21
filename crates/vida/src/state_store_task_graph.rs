use super::*;

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
        (task.status == "open" || task.status == "in_progress") && task.issue_type != "epic"
    }

    fn task_blockers(
        task: &TaskRecord,
        by_id: &BTreeMap<String, TaskRecord>,
    ) -> Vec<TaskDependencyStatus> {
        let mut blockers = task
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

        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(Self::ready_scope_ids_from_rows(&rows, scope_task_id)?)
        } else {
            None
        };
        let mut critical_path_ids = BTreeSet::new();
        if let Ok(path) = self.critical_path().await {
            critical_path_ids.extend(path.nodes.into_iter().map(|node| node.id));
        }

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
                    .map(|task| task.id.clone())
            })
            .or_else(|| {
                scoped_tasks
                    .iter()
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
            let blocked_by = Self::task_blockers(&task, &by_id);
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

    pub async fn task_progress_summary(
        &self,
        task_id: &str,
    ) -> Result<TaskProgressSummary, StateStoreError> {
        let rows = self.all_tasks().await?;
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
            if task.issue_type == "epic" {
                epic_count += 1;
            }
        }

        let descendant_count = descendant_ids.len();
        let percent_closed = if descendant_count == 0 {
            0.0
        } else {
            (closed_count as f64 / descendant_count as f64) * 100.0
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
        Self::build_task_dependency_tree(&by_id, &children_by_parent, task_id, &mut active)
    }

    fn build_task_dependency_tree(
        by_id: &BTreeMap<String, TaskRecord>,
        children_by_parent: &BTreeMap<String, Vec<String>>,
        task_id: &str,
        active: &mut BTreeSet<String>,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let task = by_id
            .get(task_id)
            .cloned()
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;

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
            };

            if active.contains(&dependency.depends_on_id) {
                edge.cycle = true;
            } else if let Some(child) = by_id.get(&dependency.depends_on_id) {
                edge.dependency_status = child.status.clone();
                edge.dependency_issue_type = Some(child.issue_type.clone());
                let child_id = child.id.clone();
                let child_node =
                    Self::build_task_dependency_tree(by_id, children_by_parent, &child_id, active)?;
                edge.node = Some(Box::new(child_node));
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
                    child_status: "missing".to_string(),
                    child_issue_type: None,
                    node: None,
                    cycle: false,
                    missing: false,
                };
                if active.contains(child_id) {
                    child.cycle = true;
                } else if let Some(child_task) = by_id.get(child_id) {
                    child.child_status = child_task.status.clone();
                    child.child_issue_type = Some(child_task.issue_type.clone());
                    let child_node = Self::build_task_dependency_tree(
                        by_id,
                        children_by_parent,
                        child_id,
                        active,
                    )?;
                    child.node = Some(Box::new(child_node));
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
                (task.status == "open" || task.status == "in_progress") && task.issue_type != "epic"
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
                next_action: "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.".to_string(),
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
            if dep_task.status == "closed" || dep_task.issue_type == "epic" {
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
        store
            .create_task(CreateTaskRequest {
                task_id,
                title: task_id,
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics {
                    execution_mode: execution_mode.map(ToOwned::to_owned),
                    order_bucket: order_bucket.map(ToOwned::to_owned),
                    parallel_group: parallel_group.map(ToOwned::to_owned),
                    conflict_domain: conflict_domain.map(ToOwned::to_owned),
                },
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("task should be created");
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
}
