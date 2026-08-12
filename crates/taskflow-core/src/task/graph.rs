//! Task graph algorithm helpers.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::{issue_type_is_execution_step, task_status_is_closed_like, task_status_is_open_like};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGraphDependencyRow {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGraphRow {
    pub id: String,
    pub status: String,
    pub issue_type: String,
    pub canonical_issue_type: String,
    pub parent_required: bool,
    pub program_container: bool,
    pub spec_first_feature_parent: bool,
    pub spec_pack_child: bool,
    pub work_pool_pack_child: bool,
    pub dependencies: Vec<TaskGraphDependencyRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskGraphIssue {
    pub issue_type: String,
    pub issue_id: String,
    pub depends_on_id: Option<String>,
    pub edge_type: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGraphView {
    rows: Vec<TaskGraphRow>,
    by_id: BTreeMap<String, usize>,
    children_by_parent: BTreeMap<String, Vec<String>>,
    non_parent_dependencies: BTreeMap<String, Vec<(String, String)>>,
}

impl TaskGraphView {
    #[must_use]
    pub fn from_rows(rows: impl IntoIterator<Item = TaskGraphRow>) -> Self {
        let mut rows = rows.into_iter().collect::<Vec<_>>();
        rows.sort_by(|left, right| left.id.cmp(&right.id));

        let mut by_id = BTreeMap::new();
        for (index, task) in rows.iter().enumerate() {
            by_id.insert(task.id.clone(), index);
        }

        let mut children_by_parent = BTreeMap::<String, Vec<String>>::new();
        let mut non_parent_dependencies = BTreeMap::<String, Vec<(String, String)>>::new();
        for task in &rows {
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
            rows,
            by_id,
            children_by_parent,
            non_parent_dependencies,
        }
    }

    #[must_use]
    pub fn rows(&self) -> &[TaskGraphRow] {
        &self.rows
    }

    #[must_use]
    pub fn task(&self, task_id: &str) -> Option<&TaskGraphRow> {
        self.by_id
            .get(task_id)
            .and_then(|index| self.rows.get(*index))
    }

    #[must_use]
    pub fn contains_task(&self, task_id: &str) -> bool {
        self.by_id.contains_key(task_id)
    }

    #[must_use]
    pub fn children_for(&self, task_id: &str) -> Option<&Vec<String>> {
        self.children_by_parent.get(task_id)
    }

    #[must_use]
    pub fn validate(&self) -> Vec<TaskGraphIssue> {
        let mut issues = Vec::new();

        for task in self.rows() {
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
            if task.parent_required
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
                        task.canonical_issue_type
                    ),
                });
            }

            for dependency in &task.dependencies {
                if !self.contains_task(&dependency.depends_on_id) {
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
                if dependency.edge_type == "parent-child"
                    && let Some(parent) = self.task(&dependency.depends_on_id)
                    && task.canonical_issue_type == "epic"
                    && parent.canonical_issue_type != "epic"
                {
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

        for task in self.rows() {
            let Some(children) = self.children_for(&task.id) else {
                continue;
            };
            if task_status_is_closed_like(&task.status) {
                for child_id in children {
                    let Some(child) = self.task(child_id) else {
                        continue;
                    };
                    if !task_status_is_closed_like(&child.status)
                        && !issue_type_is_execution_step(&child.canonical_issue_type)
                    {
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
            } else if task_status_is_open_like(&task.status) && task.program_container {
                let has_non_closed_child = children.iter().any(|child_id| {
                    self.task(child_id)
                        .map(|child| {
                            !task_status_is_closed_like(&child.status)
                                && !issue_type_is_execution_step(&child.canonical_issue_type)
                        })
                        .unwrap_or(false)
                });
                let has_unresolved_non_parent_dependency = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .any(|dependency| {
                        self.task(&dependency.depends_on_id)
                            .map(|dependency_task| {
                                !task_status_is_closed_like(&dependency_task.status)
                            })
                            .unwrap_or(true)
                    });
                let waiting_for_work_pool_handoff = task.spec_first_feature_parent
                    && children
                        .iter()
                        .filter_map(|child_id| self.task(child_id))
                        .any(|child| child.spec_pack_child)
                    && !children
                        .iter()
                        .filter_map(|child_id| self.task(child_id))
                        .any(|child| child.work_pool_pack_child);
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
        for task in self.rows() {
            validate_parent_child_cycles(
                &task.id,
                &self.children_by_parent,
                &mut visited,
                &mut active,
                &mut issues,
            );
        }

        let mut visited = BTreeSet::new();
        let mut active = BTreeSet::new();
        for task in self.rows() {
            validate_non_parent_dependency_cycles(
                &task.id,
                &self.non_parent_dependencies,
                &mut visited,
                &mut active,
                &mut issues,
            );
        }

        issues.sort();
        issues.dedup();
        issues
    }
}

#[must_use]
pub fn validate_task_graph_rows(
    rows: impl IntoIterator<Item = TaskGraphRow>,
) -> Vec<TaskGraphIssue> {
    TaskGraphView::from_rows(rows).validate()
}

#[must_use]
pub fn validate_task_graph_rows_for_mutation(
    before: impl IntoIterator<Item = TaskGraphRow>,
    after: impl IntoIterator<Item = TaskGraphRow>,
    touched_task_ids: &BTreeSet<String>,
) -> Vec<TaskGraphIssue> {
    let existing_issues = validate_task_graph_rows(before)
        .into_iter()
        .map(|issue| task_graph_issue_key(&issue))
        .collect::<BTreeSet<_>>();

    validate_task_graph_rows(after)
        .into_iter()
        .filter(|issue| {
            touched_task_ids.contains(&issue.issue_id)
                || issue
                    .depends_on_id
                    .as_ref()
                    .is_some_and(|id| touched_task_ids.contains(id))
                || !existing_issues.contains(&task_graph_issue_key(issue))
        })
        .collect()
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
            validate_parent_child_cycles(child, parent_children, visited, active, issues);
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
            validate_non_parent_dependency_cycles(
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectedDependencyAnalysis {
    pub topological_order: Vec<String>,
    pub cycle_node_id: Option<String>,
}

#[must_use]
pub fn analyze_directed_dependencies<'a>(
    nodes: impl IntoIterator<Item = &'a str>,
    edges: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> DirectedDependencyAnalysis {
    let mut node_ids = nodes
        .into_iter()
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())
        .collect::<Vec<_>>();
    node_ids.sort();
    node_ids.dedup();

    let mut graph = DiGraph::<&str, ()>::new();
    let mut node_indices = BTreeMap::<&str, NodeIndex>::new();
    for node_id in node_ids {
        let index = graph.add_node(node_id);
        node_indices.insert(node_id, index);
    }

    for (issue_id, depends_on_id) in edges {
        let (Some(issue_index), Some(depends_on_index)) = (
            node_indices.get(issue_id.trim()).copied(),
            node_indices.get(depends_on_id.trim()).copied(),
        ) else {
            continue;
        };
        graph.add_edge(issue_index, depends_on_index, ());
    }

    match toposort(&graph, None) {
        Ok(order) => DirectedDependencyAnalysis {
            topological_order: order
                .into_iter()
                .filter_map(|index| graph.node_weight(index).copied())
                .map(ToString::to_string)
                .collect(),
            cycle_node_id: None,
        },
        Err(cycle) => DirectedDependencyAnalysis {
            topological_order: Vec::new(),
            cycle_node_id: graph
                .node_weight(cycle.node_id())
                .copied()
                .map(ToString::to_string),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TaskGraphDependencyRow, TaskGraphIssue, TaskGraphRow, analyze_directed_dependencies,
        validate_task_graph_rows, validate_task_graph_rows_for_mutation,
    };
    use std::collections::BTreeSet;

    fn row(id: &str, status: &str, issue_type: &str) -> TaskGraphRow {
        let canonical_issue_type = issue_type.trim().to_ascii_lowercase();
        TaskGraphRow {
            id: id.to_string(),
            status: status.to_string(),
            issue_type: issue_type.to_string(),
            canonical_issue_type: canonical_issue_type.clone(),
            parent_required: canonical_issue_type != "epic",
            program_container: canonical_issue_type == "epic",
            spec_first_feature_parent: false,
            spec_pack_child: false,
            work_pool_pack_child: false,
            dependencies: Vec::new(),
        }
    }

    fn parent_child(issue_id: &str, depends_on_id: &str) -> TaskGraphDependencyRow {
        TaskGraphDependencyRow {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "parent-child".to_string(),
        }
    }

    fn blocks(issue_id: &str, depends_on_id: &str) -> TaskGraphDependencyRow {
        TaskGraphDependencyRow {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "blocks".to_string(),
        }
    }

    fn issue_types(issues: &[TaskGraphIssue]) -> Vec<&str> {
        issues
            .iter()
            .map(|issue| issue.issue_type.as_str())
            .collect()
    }

    #[test]
    fn task_graph_validation_reports_parent_and_target_invariants() {
        let parent = row("parent", "open", "epic");
        let mut child = row("child", "open", "task");
        child.dependencies = vec![
            parent_child("child", "parent"),
            parent_child("child", "missing-parent"),
            blocks("child", "missing-blocker"),
            blocks("child", "child"),
        ];

        let issues = validate_task_graph_rows([parent, child]);

        assert!(issue_types(&issues).contains(&"multiple_parent_edges"));
        assert!(issue_types(&issues).contains(&"missing_dependency_target"));
        assert!(issue_types(&issues).contains(&"self_dependency"));
    }

    #[test]
    fn task_graph_validation_reports_invalid_epic_parent_and_closed_parent_child() {
        let parent = row("parent-task", "closed", "task");
        let mut child = row("child-epic", "open", "epic");
        child.parent_required = false;
        child.program_container = true;
        child.dependencies = vec![parent_child("child-epic", "parent-task")];

        let issues = validate_task_graph_rows([parent, child]);

        assert!(issue_types(&issues).contains(&"invalid_parent_child_kind"));
        assert!(issue_types(&issues).contains(&"closed_parent_has_non_closed_child"));
    }

    #[test]
    fn task_graph_validation_reports_open_container_without_open_child_except_spec_first_handoff() {
        let mut parent = row("feature", "open", "epic");
        let mut spec = row("spec-pack", "closed", "task");
        spec.spec_pack_child = true;
        spec.dependencies = vec![parent_child("spec-pack", "feature")];

        let ordinary_issues = validate_task_graph_rows([parent.clone(), spec.clone()]);
        assert!(issue_types(&ordinary_issues).contains(&"open_parent_has_no_open_child"));

        parent.spec_first_feature_parent = true;
        let spec_first_issues = validate_task_graph_rows([parent, spec]);
        assert!(!issue_types(&spec_first_issues).contains(&"open_parent_has_no_open_child"));
    }

    #[test]
    fn task_graph_validation_reports_parent_and_dependency_cycles() {
        let mut task_a = row("task-a", "open", "task");
        let mut task_b = row("task-b", "open", "task");
        task_a.dependencies = vec![parent_child("task-a", "task-b"), blocks("task-a", "task-b")];
        task_b.dependencies = vec![parent_child("task-b", "task-a"), blocks("task-b", "task-a")];

        let issues = validate_task_graph_rows([task_a, task_b]);

        assert!(issue_types(&issues).contains(&"parent_child_cycle"));
        assert!(issue_types(&issues).contains(&"dependency_cycle"));
    }

    #[test]
    fn task_graph_mutation_validation_keeps_new_touched_issues_only() {
        let untouched = row("untouched", "open", "task");
        let mut before_touched = row("touched", "open", "task");
        before_touched.dependencies = vec![parent_child("touched", "parent")];
        let before = vec![untouched, before_touched.clone()];

        let mut after_touched = before_touched;
        after_touched
            .dependencies
            .push(blocks("touched", "touched"));
        let mut touched = BTreeSet::new();
        touched.insert("touched".to_string());

        let issues = validate_task_graph_rows_for_mutation(
            before,
            vec![row("untouched", "open", "task"), after_touched],
            &touched,
        );

        assert!(issues.iter().any(|issue| issue.issue_type == "self_dependency"));
        assert!(issues
            .iter()
            .all(|issue| issue.issue_id == "touched" || issue.depends_on_id.as_deref() == Some("touched")));
    }

    #[test]
    fn graph_analysis_reports_topological_order_for_acyclic_dependencies() {
        let analysis = analyze_directed_dependencies(
            ["task-a", "task-b", "task-c"],
            [("task-c", "task-b"), ("task-b", "task-a")],
        );

        assert!(analysis.cycle_node_id.is_none());
        assert_eq!(analysis.topological_order.len(), 3);
    }

    #[test]
    fn graph_analysis_reports_cycle_node_for_cyclic_dependencies() {
        let analysis = analyze_directed_dependencies(
            ["task-a", "task-b"],
            [("task-a", "task-b"), ("task-b", "task-a")],
        );

        assert!(matches!(
            analysis.cycle_node_id.as_deref(),
            Some("task-a" | "task-b")
        ));
        assert!(analysis.topological_order.is_empty());
    }
}
