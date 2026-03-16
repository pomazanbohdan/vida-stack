use crate::state_store::{
    BlockedTaskRecord, TaskCriticalPath, TaskDependencyRecord, TaskDependencyStatus,
    TaskDependencyTreeEdge, TaskDependencyTreeNode, TaskGraphIssue, TaskRecord,
};
use crate::{print_surface_header, print_surface_line, RenderMode};

pub(crate) fn print_task_list(render: RenderMode, tasks: &[TaskRecord], as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tasks).expect("task list should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task");
    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

pub(crate) fn print_task_show(render: RenderMode, task: &TaskRecord, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(task).expect("task should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task show");
    print_surface_line(render, "id", &task.id);
    print_surface_line(render, "status", &task.status);
    print_surface_line(render, "title", &task.title);
    print_surface_line(render, "priority", &task.priority.to_string());
    print_surface_line(render, "issue type", &task.issue_type);
    if !task.labels.is_empty() {
        print_surface_line(render, "labels", &task.labels.join(", "));
    }
    if !task.dependencies.is_empty() {
        let summary = task
            .dependencies
            .iter()
            .map(|dependency| format!("{}:{}", dependency.edge_type, dependency.depends_on_id))
            .collect::<Vec<_>>()
            .join(", ");
        print_surface_line(render, "dependencies", &summary);
    }
}

pub(crate) fn print_task_dependencies(
    render: RenderMode,
    title: &str,
    task_id: &str,
    dependencies: &[TaskDependencyStatus],
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(dependencies)
                .expect("task dependencies should render as json")
        );
        return;
    }

    print_surface_header(render, title);
    print_surface_line(render, "task", task_id);
    if dependencies.is_empty() {
        print_surface_line(render, "dependencies", "none");
        return;
    }

    for dependency in dependencies {
        let issue_type = dependency
            .dependency_issue_type
            .as_deref()
            .unwrap_or("unknown");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            dependency.issue_id,
            dependency.edge_type,
            dependency.depends_on_id,
            dependency.dependency_status,
            issue_type
        );
    }
}

pub(crate) fn print_blocked_tasks(render: RenderMode, tasks: &[BlockedTaskRecord], as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tasks).expect("blocked tasks should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task blocked");
    if tasks.is_empty() {
        print_surface_line(render, "blocked tasks", "none");
        return;
    }

    for blocked in tasks {
        println!(
            "{}\t{}\t{}",
            blocked.task.id, blocked.task.status, blocked.task.title
        );
        for blocker in &blocked.blockers {
            println!(
                "  blocked-by\t{}\t{}\t{}",
                blocker.edge_type, blocker.depends_on_id, blocker.dependency_status
            );
        }
    }
}

pub(crate) fn print_task_dependency_tree(
    render: RenderMode,
    tree: &TaskDependencyTreeNode,
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(tree).expect("task dependency tree should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task tree");
    print_surface_line(
        render,
        "root",
        &format!(
            "{}\t{}\t{}",
            tree.task.id, tree.task.status, tree.task.title
        ),
    );
    if tree.dependencies.is_empty() {
        print_surface_line(render, "dependencies", "none");
        return;
    }

    for edge in &tree.dependencies {
        print_task_dependency_tree_edge(edge, 0);
    }
}

fn print_task_dependency_tree_edge(edge: &TaskDependencyTreeEdge, depth: usize) {
    let indent = "  ".repeat(depth);
    let issue_type = edge.dependency_issue_type.as_deref().unwrap_or("unknown");
    let state = if edge.cycle {
        "cycle"
    } else if edge.missing {
        "missing"
    } else {
        edge.dependency_status.as_str()
    };
    println!(
        "{indent}{} -> {}\t{}\t{}\t{}",
        edge.edge_type, edge.depends_on_id, state, issue_type, edge.issue_id
    );

    if let Some(node) = &edge.node {
        for child in &node.dependencies {
            print_task_dependency_tree_edge(child, depth + 1);
        }
    }
}

pub(crate) fn print_task_graph_issues(
    render: RenderMode,
    issues: &[TaskGraphIssue],
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(issues).expect("task graph issues should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task validate-graph");
    if issues.is_empty() {
        print_surface_line(render, "graph", "ok");
        return;
    }

    for issue in issues {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            issue.issue_type,
            issue.issue_id,
            issue.depends_on_id.as_deref().unwrap_or("-"),
            issue.edge_type.as_deref().unwrap_or("-"),
            issue.detail
        );
    }
}

pub(crate) fn print_task_dependency_mutation(
    render: RenderMode,
    title: &str,
    dependency: &TaskDependencyRecord,
    as_json: bool,
) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(dependency)
                .expect("task dependency mutation should render as json")
        );
        return;
    }

    print_surface_header(render, title);
    print_surface_line(render, "task", &dependency.issue_id);
    print_surface_line(render, "depends_on", &dependency.depends_on_id);
    print_surface_line(render, "edge_type", &dependency.edge_type);
}

pub(crate) fn print_task_critical_path(render: RenderMode, path: &TaskCriticalPath, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(path).expect("critical path should render as json")
        );
        return;
    }

    print_surface_header(render, "vida task critical-path");
    print_surface_line(render, "length", &path.length.to_string());
    print_surface_line(
        render,
        "root_task_id",
        path.root_task_id.as_deref().unwrap_or("none"),
    );
    print_surface_line(
        render,
        "terminal_task_id",
        path.terminal_task_id.as_deref().unwrap_or("none"),
    );
    for node in &path.nodes {
        println!(
            "{}\t{}\t{}\t{}",
            node.id, node.status, node.issue_type, node.title
        );
    }
}
