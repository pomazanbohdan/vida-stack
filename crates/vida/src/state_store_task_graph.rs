use super::*;

impl StateStore {
    pub async fn ready_tasks_scoped(
        &self,
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut rows = self.all_tasks().await?;
        rows.sort_by(task_sort_key);

        let by_id = rows
            .iter()
            .map(|task| (task.id.clone(), task.status.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(self.ready_scope_ids(&rows, scope_task_id)?)
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
            .filter(|task| task.status == "open" || task.status == "in_progress")
            .filter(|task| task.issue_type != "epic")
            .filter(|task| {
                task.dependencies.iter().all(|dependency| {
                    if dependency.edge_type == "parent-child" {
                        return true;
                    }
                    matches!(
                        by_id.get(&dependency.depends_on_id).map(String::as_str),
                        Some("closed")
                    )
                })
            })
            .collect::<Vec<_>>();

        ready.sort_by(task_ready_sort_key);
        Ok(ready)
    }

    fn ready_scope_ids(
        &self,
        rows: &[TaskRecord],
        scope_task_id: &str,
    ) -> Result<BTreeSet<String>, StateStoreError> {
        if !rows.iter().any(|task| task.id == scope_task_id) {
            return Err(StateStoreError::MissingTask {
                task_id: scope_task_id.to_string(),
            });
        }

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

    pub async fn task_dependency_tree(
        &self,
        task_id: &str,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let mut active = BTreeSet::new();
        Self::build_task_dependency_tree(&by_id, task_id, &mut active)
    }

    fn build_task_dependency_tree(
        by_id: &BTreeMap<String, TaskRecord>,
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
                let child_node = Self::build_task_dependency_tree(by_id, &child_id, active)?;
                edge.node = Some(Box::new(child_node));
            } else {
                edge.missing = true;
            }

            dependencies.push(edge);
        }
        active.remove(&task.id);

        dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });

        Ok(TaskDependencyTreeNode { task, dependencies })
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
