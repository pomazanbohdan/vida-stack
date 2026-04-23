use crate::operator_contracts::{
    finalize_release1_operator_truth, shared_operator_output_contract_parity_error,
};
use crate::state_store::{
    BlockedTaskRecord, TaskBulkReparentResult, TaskCriticalPath, TaskDependencyRecord,
    TaskDependencyStatus, TaskDependencyTreeChild, TaskDependencyTreeEdge, TaskDependencyTreeNode,
    TaskGraphIssue, TaskProgressSummary, TaskRecord,
};
use crate::{print_surface_header, print_surface_line, RenderMode};

fn task_read_metadata_value(
    metadata: Option<&crate::task_surface::TaskReadMetadata>,
) -> serde_json::Value {
    metadata.map_or_else(
        || serde_json::json!(null),
        |metadata| {
            serde_json::json!({
                "mode": metadata.mode,
                "degraded": metadata.degraded,
                "snapshot_path": metadata.snapshot_path,
                "detail": metadata.detail,
            })
        },
    )
}

fn print_task_read_metadata(
    render: RenderMode,
    metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let Some(metadata) = metadata else {
        return;
    };
    print_surface_line(render, "state access", metadata.mode);
    if metadata.degraded {
        print_surface_line(render, "degraded read", "yes");
    }
    print_surface_line(render, "read detail", metadata.detail);
    if let Some(snapshot_path) = metadata.snapshot_path.as_deref() {
        print_surface_line(render, "snapshot path", snapshot_path);
    }
}

fn build_pass_operator_surface_payload(
    surface: &str,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    let finalized = finalize_release1_operator_truth(
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "surface": surface,
        }),
    )
    .expect("task operator surface should finalize");
    let mut payload = serde_json::json!({
        "surface": surface,
        "status": finalized.status,
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
    });
    let extra_object = extra_fields
        .as_object()
        .expect("task operator surface extras must be an object")
        .clone();
    payload
        .as_object_mut()
        .expect("task operator surface payload should serialize to an object")
        .extend(extra_object);
    assert_eq!(
        shared_operator_output_contract_parity_error(&payload),
        None,
        "task operator surface payload should keep release-1 parity"
    );
    payload
}

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

pub(crate) fn print_task_list(
    render: RenderMode,
    tasks: &[TaskRecord],
    summary_only: bool,
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let payload = if summary_only {
        build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "view": "summary",
                "task_count": tasks.len(),
                "tasks": tasks.iter().map(|task| serde_json::json!({
                    "id": task.id,
                    "display_id": task.display_id,
                    "status": task.status,
                    "title": task.title,
                    "priority": task.priority,
                    "issue_type": task.issue_type,
                })).collect::<Vec<_>>(),
            }),
        )
    } else {
        build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "view": "full",
                "task_count": tasks.len(),
                "tasks": tasks,
            }),
        )
    };
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task list should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task");
    print_task_read_metadata(render, read_metadata);
    if summary_only {
        print_surface_line(render, "view", "summary");
    }
    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

pub(crate) fn print_task_ready(
    render: RenderMode,
    scope_task_id: Option<&str>,
    tasks: &[TaskRecord],
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task ready",
        serde_json::json!({
            "state_access": task_read_metadata_value(read_metadata),
            "scope_task_id": scope_task_id,
            "ready_count": tasks.len(),
            "tasks": tasks,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task ready payload should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task ready");
    print_task_read_metadata(render, read_metadata);
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

pub(crate) fn print_task_show(
    render: RenderMode,
    task: &TaskRecord,
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task show",
        serde_json::json!({
            "state_access": task_read_metadata_value(read_metadata),
            "task_id": task.id,
            "task": task,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task show should render as json",
    ) {
        return;
    }

    print_task_record(render, "vida task show", task);
    print_task_read_metadata(render, read_metadata);
}

pub(crate) fn print_task_progress(
    render: RenderMode,
    summary: &TaskProgressSummary,
    as_json: bool,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task progress",
        serde_json::json!({
            "task_id": summary.root_task.id,
            "progress": summary,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task progress should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task progress");
    print_surface_line(render, "task", &summary.root_task.id);
    print_surface_line(render, "title", &summary.root_task.title);
    print_surface_line(render, "basis", &summary.progress_basis);
    print_surface_line(
        render,
        "direct children",
        &summary.direct_child_count.to_string(),
    );
    print_surface_line(render, "descendants", &summary.descendant_count.to_string());
    print_surface_line(render, "open", &summary.open_count.to_string());
    print_surface_line(
        render,
        "in progress",
        &summary.in_progress_count.to_string(),
    );
    print_surface_line(render, "closed", &summary.closed_count.to_string());
    print_surface_line(render, "epics", &summary.epic_count.to_string());
    print_surface_line(
        render,
        "percent closed",
        &format!("{:.2}", summary.percent_closed),
    );
    if summary.status_counts.is_empty() {
        print_surface_line(render, "status counts", "none");
        return;
    }

    let status_summary = summary
        .status_counts
        .iter()
        .map(|(status, count)| format!("{status}:{count}"))
        .collect::<Vec<_>>()
        .join(", ");
    print_surface_line(render, "status counts", &status_summary);
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
    let payload = build_pass_operator_surface_payload(
        "vida task export-jsonl",
        serde_json::json!({
            "exported_count": exported_count,
            "target_path": target_path,
        }),
    );
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
    let payload = build_pass_operator_surface_payload(
        title,
        serde_json::json!({
            "task_id": task_id,
            "dependency_count": dependencies.len(),
            "dependencies": dependencies,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
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

pub(crate) fn print_blocked_tasks(
    render: RenderMode,
    tasks: &[BlockedTaskRecord],
    summary_only: bool,
    as_json: bool,
) {
    let payload = if summary_only {
        build_pass_operator_surface_payload(
            "vida task blocked",
            serde_json::json!({
                "view": "summary",
                "blocked_count": tasks.len(),
                "tasks": tasks.iter().map(|blocked| serde_json::json!({
                    "id": blocked.task.id,
                    "display_id": blocked.task.display_id,
                    "status": blocked.task.status,
                    "title": blocked.task.title,
                    "blocker_count": blocked.blockers.len(),
                    "blockers": blocked.blockers.iter().map(|blocker| serde_json::json!({
                        "depends_on_id": blocker.depends_on_id,
                        "edge_type": blocker.edge_type,
                        "dependency_status": blocker.dependency_status,
                        "dependency_issue_type": blocker.dependency_issue_type,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
        )
    } else {
        build_pass_operator_surface_payload(
            "vida task blocked",
            serde_json::json!({
                "blocked_count": tasks.len(),
                "tasks": tasks,
            }),
        )
    };
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "blocked tasks should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task blocked");
    if summary_only {
        print_surface_line(render, "view", "summary");
    }
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
    let payload = build_pass_operator_surface_payload(
        "vida task tree",
        serde_json::json!({
            "root_task_id": tree.task.id,
            "dependency_count": tree.dependencies.len(),
            "child_count": tree.children.len(),
            "tree": tree,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
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
    } else {
        for edge in &tree.dependencies {
            print_task_dependency_tree_edge(edge, 0);
        }
    }

    if tree.children.is_empty() {
        print_surface_line(render, "children", "none");
        return;
    }

    print_surface_line(render, "children", &tree.children.len().to_string());
    for child in &tree.children {
        print_task_dependency_tree_child(child, 0);
    }
}

pub(crate) fn print_task_direct_children(
    render: RenderMode,
    tree: &TaskDependencyTreeNode,
    as_json: bool,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task children",
        serde_json::json!({
            "root_task_id": tree.task.id,
            "child_count": tree.children.len(),
            "children": tree.children,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task direct children should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task children");
    print_surface_line(
        render,
        "root",
        &format!(
            "{}\t{}\t{}",
            tree.task.id, tree.task.status, tree.task.title
        ),
    );
    if tree.children.is_empty() {
        print_surface_line(render, "children", "none");
        return;
    }

    print_surface_line(render, "children", &tree.children.len().to_string());
    for child in &tree.children {
        let issue_type = child.child_issue_type.as_deref().unwrap_or("unknown");
        let state = if child.cycle {
            "cycle"
        } else if child.missing {
            "missing"
        } else {
            child.child_status.as_str()
        };
        println!("child\t{}\t{}\t{}", child.child_id, state, issue_type);
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
        for child in &node.children {
            print_task_dependency_tree_child(child, depth + 1);
        }
    }
}

fn print_task_dependency_tree_child(child: &TaskDependencyTreeChild, depth: usize) {
    let indent = "  ".repeat(depth);
    let issue_type = child.child_issue_type.as_deref().unwrap_or("unknown");
    let state = if child.cycle {
        "cycle"
    } else if child.missing {
        "missing"
    } else {
        child.child_status.as_str()
    };
    println!(
        "{indent}child\t{}\t{}\t{}",
        child.child_id, state, issue_type
    );

    if let Some(node) = &child.node {
        for edge in &node.dependencies {
            print_task_dependency_tree_edge(edge, depth + 1);
        }
        for nested_child in &node.children {
            print_task_dependency_tree_child(nested_child, depth + 1);
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

pub(crate) fn print_task_bulk_reparent_result(
    render: RenderMode,
    result: &TaskBulkReparentResult,
    as_json: bool,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task reparent-children",
        serde_json::json!({
            "result": result,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task bulk reparent result should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task reparent-children");
    print_surface_line(render, "from_parent", &result.from_parent_id);
    print_surface_line(render, "to_parent", &result.to_parent_id);
    print_surface_line(
        render,
        "dry_run",
        if result.dry_run { "true" } else { "false" },
    );
    print_surface_line(render, "moved", &result.moved_count.to_string());
    if result.moved_child_ids.is_empty() {
        print_surface_line(render, "children", "none");
        return;
    }
    print_surface_line(render, "children", &result.moved_child_ids.join(", "));
}

pub(crate) fn print_task_critical_path(render: RenderMode, path: &TaskCriticalPath, as_json: bool) {
    let payload = build_pass_operator_surface_payload(
        "vida task critical-path",
        serde_json::json!({
            "length": path.length,
            "root_task_id": path.root_task_id,
            "terminal_task_id": path.terminal_task_id,
            "nodes": path.nodes,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
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

#[cfg(test)]
mod tests {
    use super::build_pass_operator_surface_payload;
    use crate::operator_contracts::shared_operator_output_contract_parity_error;
    use crate::state_store::{TaskCriticalPathNode, TaskExecutionSemantics, TaskRecord};

    fn sample_task(id: &str) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: Some(format!("vida-{id}")),
            title: format!("Task {id}"),
            description: "sample".to_string(),
            status: "open".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-20T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-20T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: "/tmp".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: vec!["operator-dx".to_string()],
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn task_ready_payload_keeps_release1_operator_contract_parity() {
        let tasks = vec![sample_task("task-1")];
        let payload = build_pass_operator_surface_payload(
            "vida task ready",
            serde_json::json!({
                "scope_task_id": "epic-1",
                "ready_count": tasks.len(),
                "tasks": tasks,
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task ready");
        assert_eq!(payload["ready_count"], 1);
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_tree_payload_keeps_release1_operator_contract_parity() {
        let payload = build_pass_operator_surface_payload(
            "vida task tree",
            serde_json::json!({
                "root_task_id": "task-root",
                "dependency_count": 0,
                "child_count": 0,
                "tree": {
                    "task": sample_task("task-root"),
                    "dependencies": [],
                    "children": [],
                },
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task tree");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_list_summary_payload_keeps_release1_operator_contract_parity() {
        let tasks = vec![sample_task("task-1")];
        let payload = build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "view": "summary",
                "task_count": tasks.len(),
                "tasks": [{
                    "id": tasks[0].id,
                    "display_id": tasks[0].display_id,
                    "status": tasks[0].status,
                    "title": tasks[0].title,
                    "priority": tasks[0].priority,
                    "issue_type": tasks[0].issue_type,
                }],
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task list");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_show_payload_keeps_release1_operator_contract_parity() {
        let task = sample_task("task-1");
        let payload = build_pass_operator_surface_payload(
            "vida task show",
            serde_json::json!({
                "task_id": task.id,
                "task": task,
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task show");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_export_payload_keeps_release1_operator_contract_parity() {
        let payload = build_pass_operator_surface_payload(
            "vida task export-jsonl",
            serde_json::json!({
                "exported_count": 2,
                "target_path": ".vida/exports/tasks.snapshot.jsonl",
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            "vida task export-jsonl"
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_critical_path_payload_keeps_release1_operator_contract_parity() {
        let payload = build_pass_operator_surface_payload(
            "vida task critical-path",
            serde_json::json!({
                "length": 1,
                "root_task_id": "task-root",
                "terminal_task_id": "task-root",
                "nodes": [TaskCriticalPathNode {
                    id: "task-root".to_string(),
                    status: "open".to_string(),
                    priority: 1,
                    issue_type: "task".to_string(),
                    title: "Task root".to_string(),
                }],
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(
            payload["artifact_refs"]["surface"],
            "vida task critical-path"
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }
}
