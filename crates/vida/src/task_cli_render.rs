use crate::state_store::{
    BlockedTaskRecord, TaskCriticalPath, TaskDependencyRecord, TaskDependencyStatus,
    TaskDependencyTreeEdge, TaskDependencyTreeNode, TaskGraphIssue, TaskRecord,
};
use crate::{print_surface_header, print_surface_line, RenderMode};

fn print_task_record(render: RenderMode, title: &str, task: &TaskRecord) {
    print_surface_header(render, title);
    print_surface_line(render, "id", &task.id);
    if let Some(display_id) = task.display_id.as_deref() {
        if !display_id.is_empty() {
            print_surface_line(render, "display id", display_id);
        }
    }
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

pub(crate) fn print_task_list(render: RenderMode, tasks: &[TaskRecord], as_json: bool) {
    if crate::surface_render::print_surface_json(tasks, as_json, "task list should render as json")
    {
        return;
    }

    print_surface_header(render, "vida task");
    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

pub(crate) fn print_task_ready(
    render: RenderMode,
    scope_task_id: Option<&str>,
    tasks: &[TaskRecord],
    as_json: bool,
) {
    let payload = serde_json::json!({
        "surface": "vida task ready",
        "status": "pass",
        "scope_task_id": scope_task_id,
        "ready_count": tasks.len(),
        "tasks": tasks,
    });
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task ready payload should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task ready");
    if let Some(scope_task_id) = scope_task_id {
        print_surface_line(render, "scope task", scope_task_id);
    }
    print_surface_line(render, "ready count", &tasks.len().to_string());
    if tasks.is_empty() {
        print_surface_line(render, "ready tasks", "none");
        return;
    }

    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

pub(crate) fn print_task_show(render: RenderMode, task: &TaskRecord, as_json: bool) {
    if crate::surface_render::print_surface_json(task, as_json, "task should render as json") {
        return;
    }

    print_task_record(render, "vida task show", task);
}

pub(crate) fn print_task_mutation(
    render: RenderMode,
    title: &str,
    task: &TaskRecord,
    as_json: bool,
) {
    if crate::surface_render::print_surface_json(task, as_json, "task should render as json") {
        return;
    }

    print_task_record(render, title, task);
}

pub(crate) fn print_task_export_summary(
    render: RenderMode,
    exported_count: u64,
    target_path: &str,
    as_json: bool,
) {
    let payload = serde_json::json!({
        "status": "pass",
        "exported_count": exported_count,
        "target_path": target_path,
    });
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task export summary should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task export-jsonl");
    print_surface_line(render, "status", "pass");
    print_surface_line(render, "exported", &exported_count.to_string());
    print_surface_line(render, "target", target_path);
}

pub(crate) fn print_task_next_display_id(
    render: RenderMode,
    payload: &serde_json::Value,
    as_json: bool,
) {
    if crate::surface_render::print_surface_json(
        payload,
        as_json,
        "next display id payload should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task next-display-id");
    if payload
        .get("valid")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        print_surface_line(
            render,
            "parent_display_id",
            payload
                .get("parent_display_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        print_surface_line(
            render,
            "next_display_id",
            payload
                .get("next_display_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        );
        print_surface_line(
            render,
            "next_index",
            &payload
                .get("next_index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .to_string(),
        );
    } else {
        print_surface_line(
            render,
            "reason",
            payload
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("invalid_parent_display_id"),
        );
    }
}

pub(crate) fn print_task_dependencies(
    render: RenderMode,
    title: &str,
    task_id: &str,
    dependencies: &[TaskDependencyStatus],
    as_json: bool,
) {
    if crate::surface_render::print_surface_json(
        dependencies,
        as_json,
        "task dependencies should render as json",
    ) {
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
    if crate::surface_render::print_surface_json(
        tasks,
        as_json,
        "blocked tasks should render as json",
    ) {
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
    if crate::surface_render::print_surface_json(
        tree,
        as_json,
        "task dependency tree should render as json",
    ) {
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
    if crate::surface_render::print_surface_json(
        issues,
        as_json,
        "task graph issues should render as json",
    ) {
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
    if crate::surface_render::print_surface_json(
        dependency,
        as_json,
        "task dependency mutation should render as json",
    ) {
        return;
    }

    print_surface_header(render, title);
    print_surface_line(render, "task", &dependency.issue_id);
    print_surface_line(render, "depends_on", &dependency.depends_on_id);
    print_surface_line(render, "edge_type", &dependency.edge_type);
}

pub(crate) fn print_task_critical_path(render: RenderMode, path: &TaskCriticalPath, as_json: bool) {
    if crate::surface_render::print_surface_json(
        path,
        as_json,
        "critical path should render as json",
    ) {
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
