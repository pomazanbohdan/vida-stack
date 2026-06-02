use crate::operator_contracts::{
    finalize_release1_operator_truth, shared_operator_output_contract_parity_error,
};
use crate::state_store::{
    BlockedTaskRecord, TaskBulkReparentResult, TaskCriticalPath, TaskDefectBatchRehomeResult,
    TaskDependencyBulkAddResult, TaskDependencyRecord, TaskDependencyStatus,
    TaskDependencyTreeChild, TaskDependencyTreeEdge, TaskDependencyTreeNode, TaskGraphIssue,
    TaskProgressSummary, TaskRecord,
};
use crate::{print_surface_header, print_surface_line, RenderMode};

pub(crate) fn task_read_metadata_value(
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

fn task_work_item_kind_value(issue_type: &str) -> serde_json::Value {
    serde_json::to_value(crate::state_store::task_work_item_kind(issue_type))
        .expect("work item kind should serialize")
}

fn task_record_value(task: &TaskRecord) -> serde_json::Value {
    let mut value = serde_json::to_value(task).expect("task record should serialize");
    value["work_item_kind"] = task_work_item_kind_value(&task.issue_type);
    value
}

fn optional_work_item_kind_value(issue_type: Option<&str>) -> serde_json::Value {
    issue_type.map_or(serde_json::Value::Null, task_work_item_kind_value)
}

fn build_operator_surface_payload(
    surface: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    let finalized = finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        serde_json::json!({
            "surface": surface,
        }),
    )
    .expect("task operator surface should finalize");
    let mut payload = serde_json::json!({
        "surface": surface,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
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

fn build_pass_operator_surface_payload(
    surface: &str,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    build_operator_surface_payload(surface, Vec::new(), Vec::new(), extra_fields)
}

pub(crate) fn print_task_update_graph_blocked(issue: &TaskGraphIssue, as_json: bool) {
    let quoted_issue_id = crate::shell_quote(issue.issue_id.trim());
    let next_actions = match issue.issue_type.as_str() {
        "open_parent_has_no_open_child" => vec![format!(
            "Repair emptied parent `{}` with `vida task update {} --status closed --json`, then rerun the original task update.",
            issue.issue_id, quoted_issue_id
        )],
        _ => vec![
            "Resolve task graph validation issues and rerun the original `vida task update ... --json` command."
                .to_string(),
        ],
    };
    let payload = build_operator_surface_payload(
        "vida task update",
        crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DependencyGraphIssues,
        )
        .into_iter()
        .collect(),
        next_actions,
        serde_json::json!({
            "graph_issue": issue,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task update graph blocked payload should render as json",
    ) {
        return;
    }
    println!(
        "blocked\t{}\t{}\t{}",
        issue.issue_type, issue.issue_id, issue.detail
    );
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

fn task_list_output_policy(summary_only: bool, explicit_full: bool) -> serde_json::Value {
    let max_inline_items = if summary_only {
        serde_json::json!(100)
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "mode": if summary_only { "summary" } else { "full" },
        "explicit_full": explicit_full,
        "max_inline_items": max_inline_items,
    })
}

fn task_parent_edge_value(task: &TaskRecord, full: bool) -> serde_json::Value {
    task.dependencies
        .iter()
        .find(|dependency| dependency.edge_type == "parent-child")
        .map(|dependency| {
            if full {
                serde_json::json!({
                    "parent_id": dependency.depends_on_id,
                    "edge_type": dependency.edge_type,
                    "metadata": dependency.metadata,
                    "thread_id": dependency.thread_id,
                    "created_at": dependency.created_at,
                    "created_by": dependency.created_by,
                })
            } else {
                serde_json::json!({
                    "parent_id": dependency.depends_on_id,
                    "edge_type": dependency.edge_type,
                })
            }
        })
        .unwrap_or(serde_json::Value::Null)
}

fn task_list_row_value(task: &TaskRecord, full: bool) -> serde_json::Value {
    let parent_edge = task_parent_edge_value(task, full);
    let parent_id = parent_edge
        .get("parent_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if full {
        let mut value = task_record_value(task);
        value["parent_id"] = parent_id;
        value["parent_edge"] = parent_edge;
        return value;
    }

    serde_json::json!({
        "id": task.id,
        "display_id": task.display_id,
        "status": task.status,
        "title": task.title,
        "priority": task.priority,
        "issue_type": task.issue_type,
        "work_item_kind": task_work_item_kind_value(&task.issue_type),
        "parent_id": parent_id,
        "parent_edge": parent_edge,
    })
}

pub(crate) fn print_task_list(
    render: RenderMode,
    tasks: &[TaskRecord],
    summary_only: bool,
    explicit_full: bool,
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let output_policy = task_list_output_policy(summary_only, explicit_full);
    let payload = if summary_only {
        build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "output_policy": output_policy,
                "view": "summary",
                "task_count": tasks.len(),
                "tasks": tasks.iter().map(|task| task_list_row_value(task, false)).collect::<Vec<_>>(),
            }),
        )
    } else {
        build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "output_policy": output_policy,
                "view": "full",
                "task_count": tasks.len(),
                "tasks": tasks.iter().map(|task| task_list_row_value(task, true)).collect::<Vec<_>>(),
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
    let payload = task_ready_payload(scope_task_id, tasks, read_metadata);
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

pub(crate) fn task_ready_payload(
    scope_task_id: Option<&str>,
    tasks: &[TaskRecord],
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) -> serde_json::Value {
    build_pass_operator_surface_payload(
        "vida task ready",
        serde_json::json!({
            "state_access": task_read_metadata_value(read_metadata),
            "scope_task_id": scope_task_id,
            "ready_count": tasks.len(),
            "tasks": tasks.iter().map(task_record_value).collect::<Vec<_>>(),
        }),
    )
}

pub(crate) fn task_show_payload(
    task: &TaskRecord,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) -> serde_json::Value {
    build_pass_operator_surface_payload(
        "vida task show",
        serde_json::json!({
            "state_access": task_read_metadata_value(read_metadata),
            "task_id": task.id,
            "task": task_record_value(task),
        }),
    )
}

pub(crate) fn print_task_show(
    render: RenderMode,
    task: &TaskRecord,
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let payload = task_show_payload(task, read_metadata);
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

pub(crate) fn task_progress_payload(summary: &TaskProgressSummary) -> serde_json::Value {
    build_pass_operator_surface_payload(
        "vida task progress",
        serde_json::json!({
            "task_id": summary.root_task.id,
            "root_work_item_kind": task_work_item_kind_value(&summary.root_task.issue_type),
            "progress": summary,
        }),
    )
}

pub(crate) fn print_task_progress(
    render: RenderMode,
    summary: &TaskProgressSummary,
    as_json: bool,
) {
    let payload = task_progress_payload(summary);
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
    print_surface_line(
        render,
        "closure candidate",
        &summary.closure_candidate.to_string(),
    );
    print_surface_line(render, "closure state", &summary.closure_candidate_state);
    print_surface_line(render, "next action", &summary.recommended_next_action);
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
    let payload = build_pass_operator_surface_payload(
        title,
        serde_json::json!({
            "task_id": task.id,
            "task": task_record_value(task),
        }),
    );
    if crate::surface_render::print_surface_json(&payload, as_json, "task should render as json") {
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
                    "issue_type": blocked.task.issue_type,
                    "work_item_kind": task_work_item_kind_value(&blocked.task.issue_type),
                    "blocker_count": blocked.blockers.len(),
                    "blockers": blocked.blockers.iter().map(|blocker| serde_json::json!({
                        "depends_on_id": blocker.depends_on_id,
                        "edge_type": blocker.edge_type,
                        "dependency_status": blocker.dependency_status,
                        "dependency_issue_type": blocker.dependency_issue_type,
                        "dependency_work_item_kind": optional_work_item_kind_value(blocker.dependency_issue_type.as_deref()),
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
        )
    } else {
        build_pass_operator_surface_payload(
            "vida task blocked",
            serde_json::json!({
                "blocked_count": tasks.len(),
                "tasks": tasks.iter().map(|blocked| serde_json::json!({
                    "task": task_record_value(&blocked.task),
                    "blockers": blocked.blockers.iter().map(|blocker| serde_json::json!({
                        "issue_id": blocker.issue_id,
                        "depends_on_id": blocker.depends_on_id,
                        "edge_type": blocker.edge_type,
                        "dependency_status": blocker.dependency_status,
                        "dependency_issue_type": blocker.dependency_issue_type,
                        "dependency_work_item_kind": optional_work_item_kind_value(blocker.dependency_issue_type.as_deref()),
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
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
    let dependencies = tree
        .dependencies
        .iter()
        .map(|edge| {
            serde_json::json!({
                "id": edge.depends_on_id,
                "status": edge.dependency_status,
                "issue_type": edge.dependency_issue_type,
                "work_item_kind": optional_work_item_kind_value(edge.dependency_issue_type.as_deref()),
                "edge_type": edge.edge_type,
                "missing": edge.missing,
                "cycle": edge.cycle,
            })
        })
        .collect::<Vec<_>>();
    let children = tree
        .children
        .iter()
        .map(|child| {
            serde_json::json!({
                "id": child.child_id,
                "display_id": child.child_display_id,
                "title": child.child_title,
                "status": child.child_status,
                "priority": child.child_priority,
                "issue_type": child.child_issue_type,
                "work_item_kind": optional_work_item_kind_value(child.child_issue_type.as_deref()),
                "labels": child.child_labels,
                "missing": child.missing,
                "cycle": child.cycle,
            })
        })
        .collect::<Vec<_>>();
    let payload = build_pass_operator_surface_payload(
        "vida task tree",
        serde_json::json!({
            "root": {
                "id": tree.task.id,
                "status": tree.task.status,
                "title": tree.task.title,
                "priority": tree.task.priority,
                "issue_type": tree.task.issue_type,
                "work_item_kind": task_work_item_kind_value(&tree.task.issue_type),
            },
            "root_task_id": tree.task.id,
            "dependency_count": tree.dependencies.len(),
            "child_count": tree.children.len(),
            "dependencies": dependencies,
            "children": children,
            "tree_depth": "immediate_edges_only",
            "drill_down": "run vida task tree <task-id> --json on a listed dependency or child for the next bounded slice",
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
            "children": tree.children.iter().map(|child| serde_json::json!({
                "child_id": child.child_id,
                "child_display_id": child.child_display_id,
                "child_title": child.child_title,
                "child_status": child.child_status,
                "child_priority": child.child_priority,
                "child_issue_type": child.child_issue_type,
                "child_work_item_kind": optional_work_item_kind_value(child.child_issue_type.as_deref()),
                "child_labels": child.child_labels,
                "node": child.node,
                "cycle": child.cycle,
                "missing": child.missing,
            })).collect::<Vec<_>>(),
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
        let title = child.child_title.as_deref().unwrap_or("");
        let priority = child
            .child_priority
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "child\t{}\t{}\t{}\t{}\t{}",
            child.child_id, state, issue_type, priority, title
        );
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
    let title = child.child_title.as_deref().unwrap_or("");
    let priority = child
        .child_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!(
        "{indent}child\t{}\t{}\t{}\t{}\t{}",
        child.child_id, state, issue_type, priority, title
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
    let payload = build_task_graph_issues_payload(issues);
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task graph issues payload should render as json",
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

fn build_task_graph_issues_payload(issues: &[TaskGraphIssue]) -> serde_json::Value {
    let blocker_codes = if issues.is_empty() {
        Vec::new()
    } else {
        crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DependencyGraphIssues,
        )
        .into_iter()
        .collect()
    };
    let next_actions = if issues.is_empty() {
        Vec::new()
    } else {
        vec![
            "Resolve task graph validation issues and rerun `vida task validate-graph --json`."
                .to_string(),
        ]
    };
    build_operator_surface_payload(
        "vida task validate-graph",
        blocker_codes,
        next_actions,
        serde_json::json!({
            "valid": issues.is_empty(),
            "issue_count": issues.len(),
            "issues": issues,
        }),
    )
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

pub(crate) fn print_task_dependency_bulk_add_result(
    render: RenderMode,
    result: &TaskDependencyBulkAddResult,
    as_json: bool,
) {
    let blocker_codes = if result.failed_count == 0 {
        Vec::new()
    } else {
        crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DependencyGraphIssues,
        )
        .into_iter()
        .collect()
    };
    let next_actions = if result.failed_count == 0 {
        Vec::new()
    } else {
        vec![
            "Inspect failed and unapplied edges, repair missing tasks or invalid graph edges, then rerun `vida task dep add-bulk --json` with only the missing edges."
                .to_string(),
        ]
    };
    let payload = build_operator_surface_payload(
        "vida task dep add-bulk",
        blocker_codes,
        next_actions,
        serde_json::json!({
            "result": result,
            "dry_run": result.dry_run,
            "requested_count": result.requested_count,
            "created_count": result.created_count,
            "existing_count": result.existing_count,
            "failed_count": result.failed_count,
            "unapplied_count": result.unapplied_count,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task dependency bulk add result should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task dep add-bulk");
    print_surface_line(
        render,
        "dry_run",
        if result.dry_run { "true" } else { "false" },
    );
    print_surface_line(render, "requested", &result.requested_count.to_string());
    print_surface_line(render, "created", &result.created_count.to_string());
    print_surface_line(render, "existing", &result.existing_count.to_string());
    print_surface_line(render, "failed", &result.failed_count.to_string());
    print_surface_line(render, "unapplied", &result.unapplied_count.to_string());
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

pub(crate) fn print_task_defect_batch_rehome_result(
    render: RenderMode,
    result: &TaskDefectBatchRehomeResult,
    as_json: bool,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task defect-batch-rehome",
        serde_json::json!({
            "result": result,
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task defect-batch rehome result should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task defect-batch-rehome");
    print_surface_line(render, "from_parent", &result.from_parent_id);
    print_surface_line(render, "to_parent", &result.to_parent_id);
    print_surface_line(
        render,
        "dry_run",
        if result.dry_run { "true" } else { "false" },
    );
    print_surface_line(render, "moved", &result.moved_count.to_string());
    print_surface_line(render, "paused", &result.paused_count.to_string());
    print_surface_line(render, "started", &result.started_count.to_string());
    if !result.moved_child_ids.is_empty() {
        print_surface_line(render, "children", &result.moved_child_ids.join(", "));
    }
    if !result.paused_task_ids.is_empty() {
        print_surface_line(render, "paused_tasks", &result.paused_task_ids.join(", "));
    }
    if !result.started_task_ids.is_empty() {
        print_surface_line(render, "started_tasks", &result.started_task_ids.join(", "));
    }
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
    use super::{
        build_pass_operator_surface_payload, build_task_graph_issues_payload, task_progress_payload,
    };
    use crate::operator_contracts::shared_operator_output_contract_parity_error;
    use crate::state_store::{
        TaskCriticalPathNode, TaskExecutionSemantics, TaskGraphIssue, TaskProgressSummary,
        TaskRecord,
    };
    use std::collections::BTreeMap;

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
            provider_mapping: None,
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
                "root": {
                    "id": "task-root",
                    "status": "open",
                    "title": "Sample task",
                    "priority": 1,
                    "issue_type": "task",
                },
                "root_task_id": "task-root",
                "dependency_count": 0,
                "child_count": 0,
                "dependencies": [],
                "children": [],
                "tree_depth": "immediate_edges_only",
                "drill_down": "run vida task tree <task-id> --json on a listed dependency or child for the next bounded slice",
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
                "output_policy": super::task_list_output_policy(true, false),
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
        assert_eq!(payload["output_policy"]["mode"], "summary");
        assert_eq!(payload["output_policy"]["explicit_full"], false);
        assert_eq!(payload["output_policy"]["max_inline_items"], 100);
        let row = payload["tasks"][0].as_object().expect("summary task row");
        assert_eq!(row.len(), 6);
        assert!(row.contains_key("id"));
        assert!(!row.contains_key("description"));
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_progress_payload_exposes_closure_candidate_action() {
        let mut root = sample_task("epic-ready");
        root.issue_type = "epic".to_string();
        let mut status_counts = BTreeMap::new();
        status_counts.insert("closed".to_string(), 2);
        let summary = TaskProgressSummary {
            root_task: root,
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: 2,
            descendant_count: 2,
            open_count: 0,
            in_progress_count: 0,
            closed_count: 2,
            epic_count: 0,
            status_counts,
            percent_closed: 100.0,
            closure_candidate: true,
            closure_candidate_state: "ready_to_close".to_string(),
            closure_candidate_reason: Some(
                "root container is open while all descendants are closed-like".to_string(),
            ),
            recommended_next_action:
                "Close container with `vida task close epic-ready --reason \"all descendants closed\" --json`."
                    .to_string(),
            canonical_commands: vec![
                "vida task close epic-ready --reason \"all descendants closed\" --json"
                    .to_string(),
            ],
        };

        let payload = task_progress_payload(&summary);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task progress");
        assert_eq!(payload["progress"]["closure_candidate"], true);
        assert_eq!(
            payload["progress"]["closure_candidate_state"],
            "ready_to_close"
        );
        assert_eq!(
            payload["progress"]["canonical_commands"][0],
            "vida task close epic-ready --reason \"all descendants closed\" --json"
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_list_full_payload_keeps_release1_operator_contract_parity() {
        let tasks = vec![sample_task("task-1")];
        let payload = build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "output_policy": super::task_list_output_policy(false, true),
                "view": "full",
                "task_count": tasks.len(),
                "tasks": tasks,
            }),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task list");
        assert_eq!(payload["output_policy"]["mode"], "full");
        assert_eq!(payload["output_policy"]["explicit_full"], true);
        assert!(payload["output_policy"]["max_inline_items"].is_null());
        let row = payload["tasks"][0].as_object().expect("full task row");
        assert!(row.contains_key("description"));
        assert!(row.contains_key("execution_semantics"));
        assert!(row.len() > 6);
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_list_row_exposes_canonical_work_item_kind_without_rewriting_issue_type() {
        let mut task = sample_task("bug-1");
        task.issue_type = "bug".to_string();

        let row = super::task_list_row_value(&task, false);

        assert_eq!(row["issue_type"], "bug");
        assert_eq!(row["work_item_kind"]["canonical_issue_type"], "defect");
        assert_eq!(row["work_item_kind"]["original_issue_type"], "bug");
        assert_eq!(row["work_item_kind"]["provider_issue_type"], "bug");
        assert_eq!(row["work_item_kind"]["schema_version"], 1);
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
    fn task_show_payload_exposes_canonical_work_item_kind() {
        let mut task = sample_task("pr-1");
        task.issue_type = "pr".to_string();

        let payload = super::task_show_payload(&task, None);

        assert_eq!(payload["task"]["issue_type"], "pr");
        assert_eq!(
            payload["task"]["work_item_kind"]["canonical_issue_type"],
            "pull_request"
        );
        assert_eq!(
            payload["task"]["work_item_kind"]["provider_issue_type"],
            "pr"
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn print_task_graph_issues_json_uses_release1_operator_envelope() {
        let pass_payload = build_task_graph_issues_payload(&[]);

        assert_eq!(pass_payload["surface"], "vida task validate-graph");
        assert_eq!(pass_payload["status"], "pass");
        assert!(pass_payload["trace_id"].is_null());
        assert!(pass_payload["workflow_class"].is_null());
        assert!(pass_payload["risk_tier"].is_null());
        assert_eq!(pass_payload["valid"], true);
        assert_eq!(pass_payload["issue_count"], 0);
        assert_eq!(
            pass_payload["shared_fields"]["trace_id"],
            pass_payload["operator_contracts"]["trace_id"]
        );
        assert_eq!(
            pass_payload["shared_fields"]["workflow_class"],
            pass_payload["operator_contracts"]["workflow_class"]
        );
        assert_eq!(
            pass_payload["shared_fields"]["risk_tier"],
            pass_payload["operator_contracts"]["risk_tier"]
        );
        assert_eq!(
            pass_payload["artifact_refs"]["surface"],
            "vida task validate-graph"
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&pass_payload),
            None
        );

        let blocked_payload = build_task_graph_issues_payload(&[TaskGraphIssue {
            issue_type: "missing_dependency".to_string(),
            issue_id: "task-a".to_string(),
            depends_on_id: Some("task-missing".to_string()),
            edge_type: Some("blocks".to_string()),
            detail: "dependency is not present".to_string(),
        }]);

        assert_eq!(blocked_payload["status"], "blocked");
        assert_eq!(blocked_payload["valid"], false);
        assert_eq!(blocked_payload["issue_count"], 1);
        assert_eq!(
            blocked_payload["blocker_codes"],
            serde_json::json!(["dependency_graph_issues"])
        );
        assert_eq!(
            blocked_payload["operator_contracts"]["status"],
            blocked_payload["status"]
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&blocked_payload),
            None
        );
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
