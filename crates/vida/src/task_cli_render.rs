use crate::operator_contracts::build_release1_operator_output_payload;
use crate::state_store::{
    BlockedTaskRecord, TaskBulkReparentResult, TaskCriticalPath, TaskDefectBatchRehomeResult,
    TaskDependencyBulkAddResult, TaskDependencyRecord, TaskDependencyStatus,
    TaskDependencyTreeChild, TaskDependencyTreeEdge, TaskDependencyTreeNode, TaskGraphIssue,
    TaskProgressSummary, TaskRecord,
};
use crate::{print_surface_header, print_surface_line, RenderMode};
use taskflow_core::task::import_export::{
    task_export_jsonl_success_fields, TaskExportJsonlSummary,
};

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

fn default_task_record_list_toon_rows(tasks: &[TaskRecord]) -> serde_json::Value {
    #[derive(serde::Serialize)]
    struct TaskRow<'a> {
        id: &'a str,
        status: &'a str,
        priority: u32,
        title: &'a str,
    }

    let rows = tasks
        .iter()
        .map(|task| TaskRow {
            id: &task.id,
            status: &task.status,
            priority: task.priority,
            title: &task.title,
        })
        .collect::<Vec<_>>();
    serde_json::json!(rows)
}

fn task_record_list_toon_text(surface: &str, tasks: &[TaskRecord], fields: Option<&str>) -> String {
    let rows = if fields.is_some() {
        tasks
            .iter()
            .map(|task| task_list_row_value(task, false))
            .map(|value| apply_json_field_selector(value, fields))
            .collect::<Vec<_>>()
            .into()
    } else {
        default_task_record_list_toon_rows(tasks)
    };
    let value = serde_json::json!({
        "task_count": tasks.len(),
        "tasks": rows,
    });
    taskflow_format_toon::render_value_section(surface, &value)
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
    build_release1_operator_output_payload(
        surface,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "surface": surface,
        }),
        extra_fields,
    )
    .expect("task operator surface should finalize release-1 operator output")
}

pub(crate) fn build_pass_operator_surface_payload(
    surface: &str,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    build_operator_surface_payload(surface, Vec::new(), Vec::new(), extra_fields)
}

pub(crate) fn print_task_update_graph_blocked(issue: &TaskGraphIssue, as_json: bool) {
    let quoted_issue_id = crate::shell_quote(issue.issue_id.trim());
    let next_actions = match issue.issue_type.as_str() {
        "open_parent_has_no_open_child" => vec![format!(
            "Repair emptied parent `{}` with `{}`, then rerun the original task update.",
            issue.issue_id,
            crate::operator_command_text::human_command(&format!(
                "vida task update {} --status closed --json",
                quoted_issue_id
            ))
        )],
        _ => vec![
            "Resolve task graph validation issues and rerun the original `vida task update ...` command."
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

fn task_list_output_policy(view: &str, explicit_full: bool) -> serde_json::Value {
    let max_inline_items = match view {
        "compact" => serde_json::json!(25),
        "summary" => serde_json::json!(100),
        _ => serde_json::Value::Null,
    };
    serde_json::json!({
        "mode": view,
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
    surface: &str,
    render: RenderMode,
    tasks: &[TaskRecord],
    view: &str,
    explicit_full: bool,
    fields: Option<&str>,
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
) {
    let view = match view {
        "compact" => "compact",
        "full" => "full",
        _ => "summary",
    };
    let output_policy = task_list_output_policy(view, explicit_full);
    let summary_only = view != "full";
    let row_full = explicit_full && view == "full";
    let task_rows = tasks
        .iter()
        .map(|task| task_list_row_value(task, row_full))
        .map(|value| apply_json_field_selector(value, fields))
        .collect::<Vec<_>>();
    let payload = if summary_only {
        build_pass_operator_surface_payload(
            surface,
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "output_policy": output_policy,
                "fields": fields,
                "view": view,
                "task_count": tasks.len(),
                "tasks": task_rows,
            }),
        )
    } else {
        build_pass_operator_surface_payload(
            surface,
            serde_json::json!({
                "state_access": task_read_metadata_value(read_metadata),
                "output_policy": output_policy,
                "fields": fields,
                "view": view,
                "task_count": tasks.len(),
                "tasks": task_rows,
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

    if matches!(render, RenderMode::Plain) {
        println!("{}", task_record_list_toon_text(surface, tasks, fields));
        return;
    }

    print_surface_header(render, surface);
    print_task_read_metadata(render, read_metadata);
    if summary_only {
        print_surface_line(render, "view", "summary");
    }
    for task in tasks {
        println!("{}\t{}\t{}", task.id, task.status, task.title);
    }
}

fn apply_json_field_selector(value: serde_json::Value, fields: Option<&str>) -> serde_json::Value {
    crate::operator_toon_report::select_fields(value, fields)
}

pub(crate) fn print_task_ready(
    render: RenderMode,
    scope_task_id: Option<&str>,
    tasks: &[TaskRecord],
    as_json: bool,
    read_metadata: Option<&crate::task_surface::TaskReadMetadata>,
    view: &str,
    fields: Option<&str>,
) {
    let payload = task_ready_payload(scope_task_id, tasks, read_metadata, view, fields);
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task ready payload should render as json",
    ) {
        return;
    }

    if matches!(render, RenderMode::Plain) {
        println!(
            "{}",
            task_record_list_toon_text("vida task ready", tasks, fields)
        );
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
    view: &str,
    fields: Option<&str>,
) -> serde_json::Value {
    let view = match view {
        "compact" => "compact",
        "full" => "full",
        _ => "summary",
    };
    let output_policy = task_list_output_policy(view, view == "full");
    let row_full = view == "full";
    let task_rows = tasks
        .iter()
        .map(|task| task_list_row_value(task, row_full))
        .map(|value| apply_json_field_selector(value, fields))
        .collect::<Vec<_>>();
    build_pass_operator_surface_payload(
        "vida task ready",
        serde_json::json!({
            "state_access": task_read_metadata_value(read_metadata),
            "output_policy": output_policy,
            "fields": fields,
            "view": view,
            "scope_task_id": scope_task_id,
            "ready_count": tasks.len(),
            "tasks": task_rows,
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

pub(crate) fn task_progress_value(summary: &TaskProgressSummary) -> serde_json::Value {
    serde_json::json!({
        "root_task": {
            "id": summary.root_task.id,
            "title": summary.root_task.title,
            "status": summary.root_task.status,
            "issue_type": summary.root_task.issue_type,
            "priority": summary.root_task.priority,
        },
        "progress_basis": summary.progress_basis,
        "direct_child_count": summary.direct_child_count,
        "descendant_count": summary.descendant_count,
        "open_count": summary.open_count,
        "in_progress_count": summary.in_progress_count,
        "closed_count": summary.closed_count,
        "epic_count": summary.epic_count,
        "status_counts": summary.status_counts,
        "percent_closed": summary.percent_closed,
        "closure_candidate": summary.closure_candidate,
        "closure_candidate_state": summary.closure_candidate_state,
        "closure_candidate_reason": summary.closure_candidate_reason,
        "ready_for_close": summary.ready_for_close,
        "missing_proof": summary.missing_proof,
        "proof_blocked_by_runtime": summary.proof_blocked_by_runtime,
        "blocked_by_runtime": summary.blocked_by_runtime,
        "next_required_command": summary.next_required_command,
        "recommended_next_action": summary.recommended_next_action,
        "canonical_commands": summary.canonical_commands,
    })
}

pub(crate) fn task_progress_payload(summary: &TaskProgressSummary) -> serde_json::Value {
    build_pass_operator_surface_payload(
        "vida task progress",
        serde_json::json!({
            "task_id": summary.root_task.id,
            "root_work_item_kind": task_work_item_kind_value(&summary.root_task.issue_type),
            "progress": task_progress_value(summary),
        }),
    )
}

pub(crate) fn task_progress_payload_with_stage_ensemble(
    summary: &TaskProgressSummary,
    stage_ensemble: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut payload = task_progress_payload(summary);
    if let Some(stage_ensemble) = stage_ensemble {
        payload["progress"]["stage_ensemble"] = stage_ensemble.clone();
        payload["stage_ensemble"] = stage_ensemble;
    }
    payload
}

pub(crate) fn task_progress_toon_text(surface: &str, summary: &TaskProgressSummary) -> String {
    task_progress_toon_text_with_stage_ensemble(surface, summary, None)
}

pub(crate) fn task_progress_toon_text_with_stage_ensemble(
    surface: &str,
    summary: &TaskProgressSummary,
    stage_ensemble: Option<&serde_json::Value>,
) -> String {
    let toon_scalar = taskflow_format_toon::sanitize_toon_scalar;
    let mut lines = vec![
        format!("task: {}", toon_scalar(&summary.root_task.id)),
        format!("kind: {}", toon_scalar(&summary.root_task.issue_type)),
        format!("basis: {}", toon_scalar(&summary.progress_basis)),
        format!(
            "counts: closed={} open={} in_progress={} total={}",
            summary.closed_count,
            summary.open_count,
            summary.in_progress_count,
            summary.descendant_count
        ),
        format!("percent_closed: {:.2}", summary.percent_closed),
        format!("ready_for_close: {}", summary.ready_for_close),
        format!("state: {}", toon_scalar(&summary.closure_candidate_state)),
    ];
    if let Some(command) = summary.next_required_command.as_deref() {
        lines.push(format!("next: {}", toon_scalar(command)));
    } else {
        lines.push(format!(
            "next: {}",
            toon_scalar(&summary.recommended_next_action)
        ));
    }
    if let Some(stage_ensemble) = stage_ensemble {
        let active_stage = stage_ensemble["active_stage"].as_str().unwrap_or("none");
        let latest_receipt = stage_ensemble["latest_consolidation_receipt_id"]
            .as_str()
            .unwrap_or("none");
        let next_command = stage_ensemble["next_command"].as_str().unwrap_or("none");
        lines.push(format!(
            "stage_ensemble: active_stage={} stages={} attempts={} running={} produced={} accepted={} rejected={} stale={}",
            toon_scalar(active_stage),
            stage_ensemble["configured_stage_count"].as_u64().unwrap_or(0),
            stage_ensemble["configured_attempt_count"].as_u64().unwrap_or(0),
            stage_ensemble["running_count"].as_u64().unwrap_or(0),
            stage_ensemble["produced_count"].as_u64().unwrap_or(0),
            stage_ensemble["accepted_count"].as_u64().unwrap_or(0),
            stage_ensemble["rejected_count"].as_u64().unwrap_or(0),
            stage_ensemble["stale_count"].as_u64().unwrap_or(0)
        ));
        lines.push(format!(
            "latest_stage_receipt: {}",
            toon_scalar(latest_receipt)
        ));
        lines.push(format!("stage_next: {}", toon_scalar(next_command)));
    }
    taskflow_format_toon::render_section(surface, &lines.join("\n  "))
}

pub(crate) fn print_task_progress(
    render: RenderMode,
    summary: &TaskProgressSummary,
    as_json: bool,
) {
    print_task_progress_with_stage_ensemble(render, summary, None, as_json);
}

pub(crate) fn print_task_progress_with_stage_ensemble(
    render: RenderMode,
    summary: &TaskProgressSummary,
    stage_ensemble: Option<serde_json::Value>,
    as_json: bool,
) {
    let payload = task_progress_payload_with_stage_ensemble(summary, stage_ensemble.clone());
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task progress should render as json",
    ) {
        return;
    }

    if matches!(render, RenderMode::Plain) {
        println!(
            "{}",
            task_progress_toon_text_with_stage_ensemble(
                "vida task progress",
                summary,
                stage_ensemble.as_ref()
            )
        );
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
    print_surface_line(
        render,
        "ready for close",
        &summary.ready_for_close.to_string(),
    );
    print_surface_line(render, "missing proof", &summary.missing_proof.to_string());
    print_surface_line(
        render,
        "proof blocked by runtime",
        &summary.proof_blocked_by_runtime.to_string(),
    );
    print_surface_line(
        render,
        "blocked by runtime",
        &summary.blocked_by_runtime.to_string(),
    );
    print_surface_line(
        render,
        "next required command",
        summary.next_required_command.as_deref().unwrap_or("none"),
    );
    print_surface_line(render, "next action", &summary.recommended_next_action);
    if let Some(stage_ensemble) = stage_ensemble.as_ref() {
        print_surface_line(
            render,
            "stage ensemble active",
            stage_ensemble["active_stage"].as_str().unwrap_or("none"),
        );
        print_surface_line(
            render,
            "stage ensemble attempts",
            &stage_ensemble["configured_attempt_count"].to_string(),
        );
        if let Some(command) = stage_ensemble["next_command"].as_str() {
            print_surface_line(render, "stage next", command);
        }
    }
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
    let payload = task_mutation_payload(title, task);
    if crate::surface_render::print_surface_json(&payload, as_json, "task should render as json") {
        return;
    }

    print_task_record(render, title, task);
    print_surface_line(render, "changed tasks", "1");
    print_surface_line(
        render,
        "dependency edges",
        &task.dependencies.len().to_string(),
    );
}

pub(crate) fn task_mutation_payload(title: &str, task: &TaskRecord) -> serde_json::Value {
    let mutation_summary = serde_json::json!({
        "changed_task_count": 1,
        "changed_task_ids": [task.id.clone()],
        "changed_dependency_edge_count": task.dependencies.len(),
        "task_status": task.status.clone(),
        "task_issue_type": task.issue_type.clone(),
    });
    build_pass_operator_surface_payload(
        title,
        serde_json::json!({
            "task_id": task.id,
            "mutation_summary": mutation_summary,
            "task": task_record_value(task),
        }),
    )
}

pub(crate) fn print_task_export_summary(
    render: RenderMode,
    exported_count: u64,
    target_path: &str,
    as_json: bool,
) {
    let payload = build_pass_operator_surface_payload(
        "vida task export-jsonl",
        task_export_jsonl_success_fields(&TaskExportJsonlSummary {
            exported_count,
            target_path: target_path.to_string(),
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
    let dependency_cycle_count = tree.dependencies.iter().filter(|edge| edge.cycle).count();
    let child_cycle_count = tree.children.iter().filter(|child| child.cycle).count();
    let dependency_missing_count = tree.dependencies.iter().filter(|edge| edge.missing).count();
    let child_missing_count = tree.children.iter().filter(|child| child.missing).count();
    let dependency_repeated_count = tree
        .dependencies
        .iter()
        .filter(|edge| edge.repeated)
        .count();
    let child_repeated_count = tree.children.iter().filter(|child| child.repeated).count();
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
                "repeated": edge.repeated,
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
                "repeated": child.repeated,
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
            "diagnostics": {
                "cycle_count": dependency_cycle_count + child_cycle_count,
                "missing_count": dependency_missing_count + child_missing_count,
                "repeated_count": dependency_repeated_count + child_repeated_count,
                "bounded": true,
            },
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
    include_full: bool,
    as_json: bool,
) {
    let child_cycle_count = tree.children.iter().filter(|child| child.cycle).count();
    let child_missing_count = tree.children.iter().filter(|child| child.missing).count();
    let child_repeated_count = tree.children.iter().filter(|child| child.repeated).count();
    let payload = build_pass_operator_surface_payload(
        "vida task children",
        serde_json::json!({
            "root_task_id": tree.task.id,
            "child_count": tree.children.len(),
            "diagnostics": {
                "cycle_count": child_cycle_count,
                "missing_count": child_missing_count,
                "repeated_count": child_repeated_count,
                "bounded": true,
            },
            "view": if include_full { "full" } else { "brief" },
            "children": tree.children.iter().map(|child| task_direct_child_row_value(child, include_full)).collect::<Vec<_>>(),
        }),
    );
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task direct children should render as json",
    ) {
        return;
    }

    if matches!(render, RenderMode::Plain) {
        println!("{}", task_children_toon_text(tree));
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
        } else if child.repeated {
            "repeated"
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

fn task_direct_child_row_value(
    child: &TaskDependencyTreeChild,
    include_full: bool,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "child_id": child.child_id,
        "child_title": child.child_title,
        "child_status": child.child_status,
        "child_priority": child.child_priority,
        "child_issue_type": child.child_issue_type,
        "cycle": child.cycle,
        "missing": child.missing,
        "repeated": child.repeated,
    });
    if include_full {
        value["child_display_id"] = serde_json::json!(child.child_display_id);
        value["child_work_item_kind"] =
            optional_work_item_kind_value(child.child_issue_type.as_deref());
        value["child_labels"] = serde_json::json!(child.child_labels);
        value["node"] = serde_json::json!(child.node);
    }
    value
}

fn task_children_toon_text(tree: &TaskDependencyTreeNode) -> String {
    #[derive(serde::Serialize)]
    struct RootRow<'a> {
        id: &'a str,
        status: &'a str,
        title: &'a str,
    }

    #[derive(serde::Serialize)]
    struct ChildRow<'a> {
        id: &'a str,
        state: &'a str,
        issue_type: &'a str,
        priority: serde_json::Value,
        title: &'a str,
    }

    let children = tree
        .children
        .iter()
        .map(|child| {
            let issue_type = child.child_issue_type.as_deref().unwrap_or("unknown");
            let state = if child.cycle {
                "cycle"
            } else if child.missing {
                "missing"
            } else if child.repeated {
                "repeated"
            } else {
                child.child_status.as_str()
            };
            let priority = child
                .child_priority
                .map(|value| serde_json::json!(value))
                .unwrap_or_else(|| serde_json::json!("unknown"));
            let title = child.child_title.as_deref().unwrap_or("");
            ChildRow {
                id: &child.child_id,
                state,
                issue_type,
                priority,
                title,
            }
        })
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "root": RootRow {
            id: &tree.task.id,
            status: &tree.task.status,
            title: &tree.task.title,
        },
        "child_count": tree.children.len(),
        "children": children,
    });
    taskflow_format_toon::render_value_section("vida task children", &value)
}

fn print_task_dependency_tree_edge(edge: &TaskDependencyTreeEdge, depth: usize) {
    let indent = "  ".repeat(depth);
    let issue_type = edge.dependency_issue_type.as_deref().unwrap_or("unknown");
    let state = if edge.cycle {
        "cycle"
    } else if edge.missing {
        "missing"
    } else if edge.repeated {
        "repeated"
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
    } else if child.repeated {
        "repeated"
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
        vec![format!(
            "Resolve task graph validation issues and rerun `{}`.",
            crate::operator_command_text::human_command("vida task validate-graph --json")
        )]
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
    print_task_dependency_bulk_add_result_for_surface(
        render,
        result,
        as_json,
        "vida task dep add-bulk",
        "task dependency bulk add result should render as json",
    );
}

pub(crate) fn print_task_dependency_bulk_add_result_for_surface(
    render: RenderMode,
    result: &TaskDependencyBulkAddResult,
    as_json: bool,
    surface: &str,
    json_error_context: &str,
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
    } else if surface == "vida task dep ensure" {
        let retry_command = result
            .failed
            .iter()
            .chain(result.unapplied.iter())
            .find(|edge| {
                !edge.issue_id.trim().is_empty()
                    && !edge.depends_on_id.trim().is_empty()
                    && !edge.edge_type.trim().is_empty()
            })
            .map(|edge| {
                crate::operator_command_text::human_command(&format!(
                    "{} {} {} {} --json",
                    surface,
                    crate::shell_quote(edge.issue_id.trim()),
                    crate::shell_quote(edge.depends_on_id.trim()),
                    crate::shell_quote(edge.edge_type.trim())
                ))
            })
            .unwrap_or_else(|| format!("{surface} <task-id> <depends-on-id> <edge-type>"));
        vec![format!(
            "Inspect the failed dependency edge, repair missing tasks or invalid graph edges, then rerun `{retry_command}`."
        )]
    } else {
        vec![
            format!(
                "Inspect failed and unapplied edges, repair missing tasks or invalid graph edges, then rerun `{surface}` with only the missing edges."
            ),
        ]
    };
    let payload = build_operator_surface_payload(
        surface,
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
    if crate::surface_render::print_surface_json(&payload, as_json, json_error_context) {
        return;
    }

    print_surface_header(render, surface);
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
        build_pass_operator_surface_payload, build_task_graph_issues_payload,
        task_children_toon_text, task_mutation_payload, task_progress_payload,
        task_progress_toon_text, task_record_list_toon_text,
    };
    use crate::operator_contracts::shared_operator_output_contract_parity_error;
    use crate::state_store::{
        TaskCriticalPathNode, TaskDependencyTreeChild, TaskDependencyTreeNode,
        TaskExecutionSemantics, TaskGraphIssue, TaskProgressSummary, TaskRecord,
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
    fn task_mutation_payload_exposes_compact_summary() {
        let mut task = sample_task("task-1");
        task.status = "in_progress".to_string();
        task.dependencies
            .push(crate::state_store::TaskDependencyRecord {
                issue_id: "task-1".to_string(),
                depends_on_id: "dep-1".to_string(),
                edge_type: "blocks".to_string(),
                created_at: "2026-04-20T00:00:00Z".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });

        let payload = task_mutation_payload("vida task update", &task);

        assert_eq!(payload["mutation_summary"]["changed_task_count"], 1);
        assert_eq!(
            payload["mutation_summary"]["changed_task_ids"],
            serde_json::json!(["task-1"])
        );
        assert_eq!(
            payload["mutation_summary"]["changed_dependency_edge_count"],
            1
        );
        assert_eq!(payload["mutation_summary"]["task_status"], "in_progress");
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
                "output_policy": super::task_list_output_policy("summary", false),
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
    fn task_record_list_toon_text_is_compact_default_shape() {
        let mut task = sample_task("task-1");
        task.title = "Compact output task".to_string();

        let text = task_record_list_toon_text("vida task list", &[task], None);

        assert!(text.starts_with("vida task list\n  task_count: 1"));
        assert!(text.contains("\n  tasks[1]{id,status,priority,title}:"));
        assert!(text.contains("\n    \"task-1\",open,2,Compact output task"));
        assert!(!text.contains('\t'));
        assert!(!text.contains(" | "));
        assert!(!text.contains("description"));
        assert!(!text.contains("planner_metadata"));
    }

    #[test]
    fn task_children_toon_text_uses_tabular_headers() {
        let tree = TaskDependencyTreeNode {
            task: sample_task("root-task"),
            dependencies: Vec::new(),
            children: vec![TaskDependencyTreeChild {
                child_id: "child-1".to_string(),
                child_display_id: Some("vida-1".to_string()),
                child_title: Some("Child task".to_string()),
                child_status: "open".to_string(),
                child_priority: Some(2),
                child_issue_type: Some("task".to_string()),
                child_labels: vec!["runtime".to_string()],
                node: None,
                cycle: false,
                missing: false,
                repeated: false,
            }],
        };

        let text = task_children_toon_text(&tree);

        assert!(text.starts_with("vida task children\n"));
        assert!(text.contains("\n  child_count: 1"));
        assert!(text.contains("\n  children[1]{id,state,issue_type,priority,title}:"));
        assert!(text.contains("\n    \"child-1\",open,task,2,Child task"));
        assert!(text.contains("\n  root:"));
        assert!(!text.contains('\t'));
        assert!(!text.contains(" | "));
    }

    #[test]
    fn task_direct_child_row_value_is_brief_by_default() {
        let child = TaskDependencyTreeChild {
            child_id: "child-1".to_string(),
            child_display_id: Some("vida-1".to_string()),
            child_title: Some("Child task".to_string()),
            child_status: "open".to_string(),
            child_priority: Some(2),
            child_issue_type: Some("task".to_string()),
            child_labels: vec!["runtime".to_string()],
            node: None,
            cycle: false,
            missing: false,
            repeated: false,
        };

        let brief = super::task_direct_child_row_value(&child, false);
        let brief_object = brief.as_object().expect("brief child row");
        assert!(brief_object.contains_key("child_id"));
        assert!(brief_object.contains_key("child_status"));
        assert!(!brief_object.contains_key("node"));
        assert!(!brief_object.contains_key("child_labels"));
        assert!(!brief_object.contains_key("child_work_item_kind"));

        let full = super::task_direct_child_row_value(&child, true);
        let full_object = full.as_object().expect("full child row");
        assert!(full_object.contains_key("node"));
        assert!(full_object.contains_key("child_labels"));
        assert!(full_object.contains_key("child_work_item_kind"));
    }

    #[test]
    fn task_list_payload_applies_json_field_selector() {
        let tasks = vec![sample_task("task-1")];
        let rows = tasks
            .iter()
            .map(|task| super::task_list_row_value(task, false))
            .map(|value| super::apply_json_field_selector(value, Some("id,status,title")))
            .collect::<Vec<_>>();

        let row = rows[0].as_object().expect("selected task row");
        assert_eq!(row.len(), 3);
        assert_eq!(row["id"], "task-1");
        assert_eq!(row["status"], "open");
        assert_eq!(row["title"], "Task task-1");
        assert!(!row.contains_key("description"));
        assert!(!row.contains_key("priority"));
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
            ready_for_close: true,
            missing_proof: false,
            proof_blocked_by_runtime: false,
            blocked_by_runtime: false,
            next_required_command: Some(
                "vida task close epic-ready --reason \"all descendants closed\""
                    .to_string(),
            ),
            recommended_next_action:
                "Close container with `vida task close epic-ready --reason \"all descendants closed\"`."
                    .to_string(),
            canonical_commands: vec![
                "vida task close epic-ready --reason \"all descendants closed\""
                    .to_string(),
            ],
        };

        let payload = task_progress_payload(&summary);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task progress");
        let root_task = payload["progress"]["root_task"]
            .as_object()
            .expect("compact root task row");
        assert_eq!(root_task["id"], "epic-ready");
        assert_eq!(root_task["issue_type"], "epic");
        assert!(!root_task.contains_key("description"));
        assert!(!root_task.contains_key("planner_metadata"));
        assert!(!root_task.contains_key("notes"));
        assert!(!root_task.contains_key("labels"));
        assert_eq!(payload["progress"]["closure_candidate"], true);
        assert_eq!(
            payload["progress"]["closure_candidate_state"],
            "ready_to_close"
        );
        assert_eq!(
            payload["progress"]["canonical_commands"][0],
            "vida task close epic-ready --reason \"all descendants closed\""
        );
        assert_eq!(payload["progress"]["ready_for_close"], true);
        assert_eq!(payload["progress"]["missing_proof"], false);
        assert_eq!(payload["progress"]["proof_blocked_by_runtime"], false);
        assert_eq!(payload["progress"]["blocked_by_runtime"], false);
        assert_eq!(
            payload["progress"]["next_required_command"],
            "vida task close epic-ready --reason \"all descendants closed\""
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);

        let text = task_progress_toon_text("vida task progress", &summary);
        assert!(text.starts_with("vida task progress\n  task: epic-ready"));
        assert!(text.contains("\n  counts: closed=2 open=0 in_progress=0 total=2"));
        assert!(text.contains("\n  next: vida task close epic-ready"));
        assert!(!text.contains("planner_metadata"));
    }

    #[test]
    fn task_progress_toon_text_escapes_control_characters_in_scalars() {
        let mut root =
            sample_task("TASK-1\n  ready_for_close: true\n  next: forged-command\x1b[31m");
        root.issue_type = "epic\n  next: forged-from-kind\x1b[35m".to_string();
        let mut status_counts = BTreeMap::new();
        status_counts.insert("open".to_string(), 1);
        let summary = TaskProgressSummary {
            root_task: root,
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: 0,
            descendant_count: 0,
            open_count: 1,
            in_progress_count: 0,
            closed_count: 0,
            epic_count: 0,
            status_counts,
            percent_closed: 0.0,
            closure_candidate: false,
            closure_candidate_state: "open\n  ready_for_close: true".to_string(),
            closure_candidate_reason: None,
            ready_for_close: false,
            missing_proof: true,
            proof_blocked_by_runtime: false,
            blocked_by_runtime: false,
            next_required_command: Some(
                "vida task close injected\n  next: forged\x1b[0m".to_string(),
            ),
            recommended_next_action: "Inspect task progress".to_string(),
            canonical_commands: vec![],
        };

        let text = task_progress_toon_text("vida task progress", &summary);

        assert!(!text.contains('\x1b'));
        assert!(!text.contains("\n  next: forged-command"));
        assert!(!text.contains("\n  next: forged-from-kind"));
        assert!(!text.contains("\n  next: forged"));
        assert_eq!(text.matches("\n  ready_for_close:").count(), 1);
        assert!(text
            .contains(r"task: TASK-1\n  ready_for_close: true\n  next: forged-command\u{1b}[31m"));
        assert!(text.contains(r"kind: epic\n  next: forged-from-kind\u{1b}[35m"));
        assert!(text.contains(r"next: vida task close injected\n  next: forged\u{1b}[0m"));
    }

    #[test]
    fn task_progress_payload_exposes_leaf_readiness_fields() {
        let mut root = sample_task("leaf-defect");
        root.issue_type = "defect".to_string();
        root.planner_metadata.proof_targets =
            vec!["cargo test -p vida task_progress_summary -- --nocapture".to_string()];
        let summary = TaskProgressSummary {
            root_task: root,
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: 0,
            descendant_count: 0,
            open_count: 0,
            in_progress_count: 0,
            closed_count: 0,
            epic_count: 0,
            status_counts: BTreeMap::new(),
            percent_closed: 0.0,
            closure_candidate: false,
            closure_candidate_state: "leaf_missing_proof".to_string(),
            closure_candidate_reason: Some(
                "leaf task uses proof readiness instead of container closure semantics".to_string(),
            ),
            ready_for_close: false,
            missing_proof: true,
            proof_blocked_by_runtime: false,
            blocked_by_runtime: false,
            next_required_command: Some(
                "Run declared proof targets, then close the leaf task with explicit evidence."
                    .to_string(),
            ),
            recommended_next_action:
                "Run declared proof targets, then close the leaf task with explicit evidence."
                    .to_string(),
            canonical_commands: Vec::new(),
        };

        let payload = task_progress_payload(&summary);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["artifact_refs"]["surface"], "vida task progress");
        assert_eq!(payload["progress"]["closure_candidate"], false);
        assert_eq!(
            payload["progress"]["closure_candidate_state"],
            "leaf_missing_proof"
        );
        assert_eq!(payload["progress"]["ready_for_close"], false);
        assert_eq!(payload["progress"]["missing_proof"], true);
        assert_eq!(payload["progress"]["proof_blocked_by_runtime"], false);
        assert_eq!(payload["progress"]["blocked_by_runtime"], false);
        assert_eq!(
            payload["progress"]["next_required_command"],
            "Run declared proof targets, then close the leaf task with explicit evidence."
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_progress_payload_exposes_proof_blocked_by_runtime() {
        let mut root = sample_task("leaf-proof-runtime-blocked");
        root.issue_type = "defect".to_string();
        root.labels = vec!["proof-blocked-by-runtime".to_string()];
        root.planner_metadata.proof_targets =
            vec!["vida proof browser --route /blocked --json".to_string()];
        let summary = TaskProgressSummary {
            root_task: root,
            progress_basis: "descendants_excluding_root".to_string(),
            direct_child_count: 0,
            descendant_count: 0,
            open_count: 0,
            in_progress_count: 0,
            closed_count: 0,
            epic_count: 0,
            status_counts: BTreeMap::new(),
            percent_closed: 0.0,
            closure_candidate: false,
            closure_candidate_state: "leaf_proof_blocked_by_runtime".to_string(),
            closure_candidate_reason: Some(
                "leaf task uses proof readiness instead of container closure semantics".to_string(),
            ),
            ready_for_close: false,
            missing_proof: false,
            proof_blocked_by_runtime: true,
            blocked_by_runtime: true,
            next_required_command: Some(
                "Record or resolve the runtime proof blocker before closing the leaf task."
                    .to_string(),
            ),
            recommended_next_action:
                "Record or resolve the runtime proof blocker before closing the leaf task."
                    .to_string(),
            canonical_commands: Vec::new(),
        };

        let payload = task_progress_payload(&summary);

        assert_eq!(
            payload["progress"]["closure_candidate_state"],
            "leaf_proof_blocked_by_runtime"
        );
        assert_eq!(payload["progress"]["missing_proof"], false);
        assert_eq!(payload["progress"]["proof_blocked_by_runtime"], true);
        assert_eq!(payload["progress"]["blocked_by_runtime"], true);
        assert_eq!(
            payload["progress"]["next_required_command"],
            "Record or resolve the runtime proof blocker before closing the leaf task."
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
    }

    #[test]
    fn task_list_full_payload_keeps_release1_operator_contract_parity() {
        let tasks = vec![sample_task("task-1")];
        let payload = build_pass_operator_surface_payload(
            "vida task list",
            serde_json::json!({
                "output_policy": super::task_list_output_policy("full", true),
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
