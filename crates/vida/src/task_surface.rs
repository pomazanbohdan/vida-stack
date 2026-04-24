use super::*;
use crate::task_cli_render::{print_task_bulk_reparent_result, print_task_direct_children};

#[derive(Debug, Clone)]
pub(crate) struct TaskReadMetadata {
    pub mode: &'static str,
    pub degraded: bool,
    pub snapshot_path: Option<String>,
    pub detail: &'static str,
}

impl TaskReadMetadata {
    fn authoritative_live() -> Self {
        Self {
            mode: "authoritative_live",
            degraded: false,
            snapshot_path: None,
            detail: "served from the authoritative state store",
        }
    }

    fn snapshot(path: &std::path::Path, detail: &'static str) -> Self {
        Self {
            mode: "snapshot",
            degraded: true,
            snapshot_path: Some(path.display().to_string()),
            detail,
        }
    }
}

fn task_json_success_status() -> &'static str {
    crate::contract_profile_adapter::release_contract_status(true)
}

fn canonical_json_string_array_entries(value: &serde_json::Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

fn normalize_task_json_contract_arrays(summary_json: &mut serde_json::Value) -> Result<(), String> {
    let Some(summary) = summary_json.as_object_mut() else {
        return Ok(());
    };
    for key in ["blocker_codes", "next_actions"] {
        if let Some(value) = summary.get(key) {
            let entries = canonical_json_string_array_entries(value).ok_or_else(|| {
                format!(
                    "task json contract inconsistency: `{key}` must contain canonical nonempty string entries"
                )
            })?;
            summary.insert(key.to_string(), serde_json::json!(entries));
        }
    }
    Ok(())
}

async fn open_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    if state_dir.exists() {
        StateStore::open_existing(state_dir).await
    } else {
        StateStore::open(state_dir).await
    }
}

pub(crate) async fn open_read_only_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    StateStore::open_existing_read_only(state_dir).await
}

fn is_authoritative_state_lock_error(error: &state_store::StateStoreError) -> bool {
    let message = error.to_string();
    message.contains("LOCK") || message.contains("lock")
}

fn load_task_snapshot_rows(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path)
}

async fn load_task_snapshot_rows_with_retry(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    for attempt in 0..80 {
        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
            Ok(rows) => return Ok(rows),
            Err(error @ state_store::StateStoreError::Io(_)) if attempt < 79 => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }
    load_task_snapshot_rows(state_dir)
}

async fn load_task_snapshot_rows_snapshot_first(
    state_dir: &std::path::Path,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
        Ok(rows) => Ok((
            rows,
            TaskReadMetadata::snapshot(
                &snapshot_path,
                "served from canonical task snapshot evidence",
            ),
        )),
        Err(snapshot_error) => match open_read_only_task_store(state_dir.to_path_buf()).await {
            Ok(store) => store
                .list_tasks(None, true)
                .await
                .map(|rows| (rows, TaskReadMetadata::authoritative_live())),
            Err(live_error) if is_authoritative_state_lock_error(&live_error) => Err(live_error),
            Err(live_error) => Err(live_error),
        }
        .map_err(|live_error| match snapshot_error {
            state_store::StateStoreError::Io(_) => live_error,
            other => other,
        }),
    }
}

fn resolve_task_from_rows(
    rows: &[state_store::TaskRecord],
    task_id_or_display_id: &str,
) -> Result<state_store::TaskRecord, state_store::StateStoreError> {
    if let Some(task) = rows.iter().find(|task| task.id == task_id_or_display_id) {
        return Ok(task.clone());
    }
    if let Some(task) = rows
        .iter()
        .find(|task| task.display_id.as_deref() == Some(task_id_or_display_id))
    {
        return Ok(task.clone());
    }
    Err(state_store::StateStoreError::MissingTask {
        task_id: task_id_or_display_id.to_string(),
    })
}

async fn refresh_task_snapshot_after_mutation(
    store: &StateStore,
    surface: &str,
) -> Result<(), ExitCode> {
    store
        .refresh_task_snapshot()
        .await
        .map(|_| ())
        .map_err(|error| {
            eprintln!("Failed to refresh canonical task snapshot after {surface}: {error}");
            ExitCode::from(1)
        })
}

pub(crate) async fn ready_tasks_scoped_read_only(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.ready_tasks_scoped(scope_task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn task_dependency_tree_read_only(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<state_store::TaskDependencyTreeNode, state_store::StateStoreError> {
    match open_read_only_task_store(state_dir.clone()).await {
        Ok(store) => store.task_dependency_tree(task_id).await,
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::task_dependency_tree_from_rows(&rows, task_id)
        }
        Err(error) => Err(error),
    }
}

async fn task_list_snapshot_first(
    state_dir: std::path::PathBuf,
    status: Option<&str>,
    include_all: bool,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let filtered = rows
        .into_iter()
        .filter(|task| include_all || task.status != "closed")
        .filter(|task| status.map(|wanted| task.status == wanted).unwrap_or(true))
        .collect();
    Ok((filtered, metadata))
}

async fn task_show_snapshot_first(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<(state_store::TaskRecord, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let task = resolve_task_from_rows(&rows, task_id)?;
    Ok((task, metadata))
}

async fn task_ready_snapshot_first(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_snapshot_first(&state_dir).await?;
    let tasks = StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)?;
    Ok((tasks, metadata))
}

fn task_rows_as_values(
    tasks: &[state_store::TaskRecord],
) -> Result<Vec<serde_json::Value>, String> {
    tasks
        .iter()
        .map(|task| serde_json::to_value(task).map_err(|error| error.to_string()))
        .collect()
}

fn project_root_for_task_state(state_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir)
        .or_else(|| crate::resolve_runtime_project_root().ok())
}

fn task_close_uses_isolated_state_dir(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
) -> bool {
    explicit_state_dir
        && crate::taskflow_task_bridge::infer_project_root_from_state_root(state_dir).is_none()
}

fn task_close_host_agent_telemetry(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
    project_root: Option<&std::path::Path>,
    task_value: &serde_json::Value,
    close_reason: &str,
    feedback_source: &str,
) -> serde_json::Value {
    if task_close_uses_isolated_state_dir(state_dir, explicit_state_dir) {
        return serde_json::json!({
            "status": "skipped",
            "reason": "isolated_state_dir",
            "state_dir": state_dir.display().to_string(),
            "feedback_store": "not_recorded",
        });
    }

    match project_root {
        Some(project_root) => {
            crate::agent_feedback_surface::maybe_record_task_close_host_agent_feedback(
                project_root,
                task_value,
                close_reason,
                feedback_source,
            )
        }
        None => serde_json::json!({
            "status": "skipped",
            "reason": "project_root_unavailable",
        }),
    }
}

fn resolve_optional_text_arg(
    label: &str,
    direct: Option<&str>,
    file_path: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    if direct.is_some() && file_path.is_some() {
        return Err(format!(
            "Use only one {label} source: --{label} <text> or --{label}-file <path>"
        ));
    }
    if let Some(path) = file_path {
        let value = std::fs::read_to_string(path).map_err(|error| {
            format!("Failed to read {label} file `{}`: {error}", path.display())
        })?;
        return Ok(Some(value));
    }
    Ok(direct.map(ToOwned::to_owned))
}

fn task_execution_semantics_from_create_args(
    command: &TaskCreateArgs,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: command.execution_mode.clone(),
        order_bucket: command.order_bucket.clone(),
        parallel_group: command.parallel_group.clone(),
        conflict_domain: command.conflict_domain.clone(),
    }
}

fn task_update_semantics_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err(
            "Use either the value flag or the matching clear flag for execution semantics, not both."
                .to_string(),
        );
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn task_update_parent_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    if value.is_some() && clear {
        return Err("Use either --parent-id or --clear-parent-id, not both.".to_string());
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(value.map(Some))
    }
}

fn parse_label_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
}

fn parse_optional_label_value(value: Option<&str>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.to_string())
            .collect::<Vec<_>>()
    })
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationPlannedTask {
    task_id: String,
    title: String,
    description: String,
    issue_type: String,
    status: String,
    priority: u32,
    parent_id: Option<String>,
    labels: Vec<String>,
    execution_semantics: state_store::TaskExecutionSemantics,
    planner_metadata: state_store::TaskPlannerMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationPlannedDependency {
    issue_id: String,
    depends_on_id: String,
    edge_type: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationValidationSummary {
    status: String,
    issue_count: usize,
    blocker_codes: Vec<String>,
    issues: Vec<state_store::TaskGraphIssue>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskGraphMutationValidationReceipt {
    receipt_kind: String,
    schema_version: String,
    receipt_id: String,
    mutation_kind: String,
    surface: String,
    source_task_id: String,
    dry_run: bool,
    applied: bool,
    reason: String,
    before_validation: TaskMutationValidationSummary,
    after_validation: TaskMutationValidationSummary,
    before_task_count: usize,
    after_task_count: usize,
    planned_task_ids: Vec<String>,
    planned_dependency_edges: Vec<TaskMutationPlannedDependency>,
    validation_scope: String,
    operator_truth: serde_json::Value,
}

#[allow(dead_code)]
pub(crate) const ADAPTIVE_REPLAN_FINDING_KINDS: &[&str] = &[
    "verification_finding",
    "proof_gap",
    "scope_drift",
    "oversized_task",
];

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct AdaptiveReplanFindingInput {
    schema_version: String,
    input_kind: String,
    finding_kind: String,
    source_task_id: String,
    summary: String,
    evidence_refs: Vec<String>,
    operator_truth: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct AdaptiveReplanFindingInputError {
    status: String,
    blocker_codes: Vec<String>,
    reason: String,
    field: Option<String>,
    supported_finding_kinds: Vec<String>,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct AdaptiveReplanFindingPreview {
    status: String,
    surface: String,
    dry_run: bool,
    applied: bool,
    planned_mutation_category: String,
    planned_mutation_kind: String,
    source_task_id: String,
    finding: AdaptiveReplanFindingInput,
    preview_receipt: AdaptiveReplanFindingPreviewReceipt,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct AdaptiveReplanFindingPreviewReceipt {
    receipt_kind: String,
    schema_version: String,
    receipt_id: String,
    surface: String,
    source_task_id: String,
    finding_kind: String,
    planned_mutation_category: String,
    planned_mutation_kind: String,
    dry_run: bool,
    applied: bool,
    graph_state_opened: bool,
    graph_state_mutated: bool,
    operator_truth: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskMutationResult {
    status: String,
    surface: String,
    mutation_kind: String,
    source_task_id: String,
    dry_run: bool,
    applied: bool,
    reason: String,
    planned_tasks: Vec<TaskMutationPlannedTask>,
    planned_dependencies: Vec<TaskMutationPlannedDependency>,
    created_task_ids: Vec<String>,
    validation: TaskMutationValidationSummary,
    graph_mutation_receipt: TaskGraphMutationValidationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSplitChildSpec {
    task_id: String,
    title: String,
}

fn task_mutation_validation_summary(
    issues: Vec<state_store::TaskGraphIssue>,
) -> TaskMutationValidationSummary {
    let blocker_codes = if issues.is_empty() {
        Vec::new()
    } else {
        vec!["invalid_task_graph".to_string()]
    };
    TaskMutationValidationSummary {
        status: if issues.is_empty() {
            task_json_success_status().to_string()
        } else {
            "blocked".to_string()
        },
        issue_count: issues.len(),
        blocker_codes,
        issues,
    }
}

#[allow(dead_code)]
pub(crate) fn adaptive_replan_finding_input_operator_truth() -> serde_json::Value {
    serde_json::json!({
        "input_model": "adaptive_replan_finding_input",
        "schema_version": "1",
        "accepted_finding_kinds": ADAPTIVE_REPLAN_FINDING_KINDS,
        "parsing_and_validation_only": true,
        "adaptive_mutation_execution_loop_implemented": false,
        "adaptive_mutation_execution_loop_truth": "not_implemented_in_this_slice",
        "valid_input_does_not_mutate_task_graph": true,
    })
}

#[allow(dead_code)]
fn adaptive_replan_finding_input_error(
    reason: impl Into<String>,
    field: Option<&str>,
) -> AdaptiveReplanFindingInputError {
    AdaptiveReplanFindingInputError {
        status: "blocked".to_string(),
        blocker_codes: vec!["invalid_adaptive_replan_finding_input".to_string()],
        reason: reason.into(),
        field: field.map(str::to_string),
        supported_finding_kinds: ADAPTIVE_REPLAN_FINDING_KINDS
            .iter()
            .map(|kind| kind.to_string())
            .collect(),
        operator_truth: adaptive_replan_finding_input_operator_truth(),
    }
}

#[allow(dead_code)]
fn required_non_empty_json_string(
    input: &serde_json::Value,
    field: &str,
) -> Result<String, AdaptiveReplanFindingInputError> {
    input
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            adaptive_replan_finding_input_error(
                format!("`{field}` must be a non-empty string"),
                Some(field),
            )
        })
}

#[allow(dead_code)]
fn optional_json_string_list(
    input: &serde_json::Value,
    field: &str,
) -> Result<Vec<String>, AdaptiveReplanFindingInputError> {
    let Some(value) = input.get(field) else {
        return Ok(Vec::new());
    };
    let rows = value.as_array().ok_or_else(|| {
        adaptive_replan_finding_input_error(format!("`{field}` must be an array"), Some(field))
    })?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(entry) = row
            .as_str()
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        else {
            return Err(adaptive_replan_finding_input_error(
                format!("`{field}` entries must be non-empty strings"),
                Some(field),
            ));
        };
        values.push(entry.to_string());
    }
    values.sort();
    values.dedup();
    Ok(values)
}

#[allow(dead_code)]
pub(crate) fn parse_adaptive_replan_finding_input(
    input: &serde_json::Value,
) -> Result<AdaptiveReplanFindingInput, AdaptiveReplanFindingInputError> {
    if !input.is_object() {
        return Err(adaptive_replan_finding_input_error(
            "adaptive replan finding input must be a JSON object",
            None,
        ));
    }
    let finding_kind = required_non_empty_json_string(input, "finding_kind")?;
    if !ADAPTIVE_REPLAN_FINDING_KINDS.contains(&finding_kind.as_str()) {
        return Err(adaptive_replan_finding_input_error(
            format!("unsupported adaptive replan finding kind `{finding_kind}`"),
            Some("finding_kind"),
        ));
    }
    Ok(AdaptiveReplanFindingInput {
        schema_version: input
            .get("schema_version")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("1")
            .to_string(),
        input_kind: "adaptive_replan_finding_input".to_string(),
        finding_kind,
        source_task_id: required_non_empty_json_string(input, "source_task_id")?,
        summary: required_non_empty_json_string(input, "summary")?,
        evidence_refs: optional_json_string_list(input, "evidence_refs")?,
        operator_truth: adaptive_replan_finding_input_operator_truth(),
    })
}

fn adaptive_replan_preview_operator_truth() -> serde_json::Value {
    serde_json::json!({
        "surface": "vida task adaptive-preview",
        "schema_version": "1",
        "preview_only": true,
        "finding_json_parsed": true,
        "planned_mutation_category_only": true,
        "preview_receipt_emitted": true,
        "graph_state_opened": false,
        "graph_state_mutated": false,
        "adaptive_mutation_execution_loop_implemented": false,
        "adaptive_mutation_execution_loop_truth": "not_implemented_in_this_slice",
    })
}

fn planned_mutation_for_finding_kind(finding_kind: &str) -> (&'static str, &'static str) {
    match finding_kind {
        "verification_finding" | "proof_gap" => ("blocker_resolution", "spawn_blocker_task"),
        "scope_drift" => ("scope_replan", "replan_scope_review"),
        "oversized_task" => ("task_decomposition", "split_task"),
        _ => ("unsupported", "blocked"),
    }
}

fn adaptive_replan_preview_receipt_id(
    finding: &AdaptiveReplanFindingInput,
    planned_mutation_category: &str,
    planned_mutation_kind: &str,
) -> String {
    let evidence_fingerprint = if finding.evidence_refs.is_empty() {
        "none".to_string()
    } else {
        finding.evidence_refs.join("+")
    };
    format!(
        "adaptive-replan-preview:{}:{}:{}:{}:evidence={}",
        finding.source_task_id,
        finding.finding_kind,
        planned_mutation_category,
        planned_mutation_kind,
        evidence_fingerprint
    )
}

fn build_adaptive_replan_finding_preview_receipt(
    finding: &AdaptiveReplanFindingInput,
    surface: &str,
    planned_mutation_category: &str,
    planned_mutation_kind: &str,
) -> AdaptiveReplanFindingPreviewReceipt {
    AdaptiveReplanFindingPreviewReceipt {
        receipt_kind: "adaptive_replan_finding_preview_receipt".to_string(),
        schema_version: "1".to_string(),
        receipt_id: adaptive_replan_preview_receipt_id(
            finding,
            planned_mutation_category,
            planned_mutation_kind,
        ),
        surface: surface.to_string(),
        source_task_id: finding.source_task_id.clone(),
        finding_kind: finding.finding_kind.clone(),
        planned_mutation_category: planned_mutation_category.to_string(),
        planned_mutation_kind: planned_mutation_kind.to_string(),
        dry_run: true,
        applied: false,
        graph_state_opened: false,
        graph_state_mutated: false,
        operator_truth: adaptive_replan_preview_operator_truth(),
    }
}

fn build_adaptive_replan_finding_preview(
    finding_json: &serde_json::Value,
    surface: &str,
) -> Result<AdaptiveReplanFindingPreview, AdaptiveReplanFindingInputError> {
    let finding = parse_adaptive_replan_finding_input(finding_json)?;
    let (planned_mutation_category, planned_mutation_kind) =
        planned_mutation_for_finding_kind(&finding.finding_kind);
    let preview_receipt = build_adaptive_replan_finding_preview_receipt(
        &finding,
        surface,
        planned_mutation_category,
        planned_mutation_kind,
    );
    Ok(AdaptiveReplanFindingPreview {
        status: task_json_success_status().to_string(),
        surface: surface.to_string(),
        dry_run: true,
        applied: false,
        planned_mutation_category: planned_mutation_category.to_string(),
        planned_mutation_kind: planned_mutation_kind.to_string(),
        source_task_id: finding.source_task_id.clone(),
        finding,
        preview_receipt,
        operator_truth: adaptive_replan_preview_operator_truth(),
    })
}

fn print_adaptive_replan_finding_preview(
    render: RenderMode,
    result: &AdaptiveReplanFindingPreview,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(result)
            .expect("adaptive replan finding preview should serialize to json");
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, &result.surface);
    print_surface_line(render, "status", &result.status);
    print_surface_line(
        render,
        "planned_mutation_category",
        &result.planned_mutation_category,
    );
    print_surface_line(
        render,
        "planned_mutation_kind",
        &result.planned_mutation_kind,
    );
    print_surface_line(render, "source_task_id", &result.source_task_id);
    print_surface_line(render, "dry_run", "true");
    print_surface_line(render, "applied", "false");
    print_surface_line(render, "graph_state_mutated", "false");
    print_surface_line(
        render,
        "preview_receipt_id",
        &result.preview_receipt.receipt_id,
    );
}

fn print_adaptive_replan_finding_input_error(
    error: &AdaptiveReplanFindingInputError,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(error)
            .expect("adaptive replan finding input error should serialize to json");
        crate::print_json_pretty(&payload);
    } else {
        eprintln!("{}", error.reason);
    }
}

fn parse_adaptive_preview_finding_json_text(
    finding_text: &str,
    field: Option<&str>,
) -> Result<serde_json::Value, AdaptiveReplanFindingInputError> {
    match serde_json::from_str::<serde_json::Value>(finding_text) {
        Ok(value) => Ok(value),
        Err(error) => Err(adaptive_replan_finding_input_error(
            format!("finding input must be valid JSON: {error}"),
            field,
        )),
    }
}

fn load_adaptive_preview_finding_json(
    finding_json: Option<&str>,
    finding_file: Option<&std::path::Path>,
) -> Result<serde_json::Value, AdaptiveReplanFindingInputError> {
    match (finding_json, finding_file) {
        (Some(_), Some(_)) => Err(adaptive_replan_finding_input_error(
            "Use only one finding source: --finding-json <json> or --finding-file <path>",
            None,
        )),
        (Some(value), None) => parse_adaptive_preview_finding_json_text(value, None),
        (None, Some(path)) => {
            let value = std::fs::read_to_string(path).map_err(|error| {
                adaptive_replan_finding_input_error(
                    format!("Failed to read finding file `{}`: {error}", path.display()),
                    Some("finding_file"),
                )
            })?;
            parse_adaptive_preview_finding_json_text(&value, Some("finding_file"))
        }
        (None, None) => Err(adaptive_replan_finding_input_error(
            "Provide --finding-json <json> or --finding-file <path>",
            None,
        )),
    }
}

async fn run_task_adaptive_preview(command: TaskAdaptivePreviewArgs) -> ExitCode {
    let finding_json = match load_adaptive_preview_finding_json(
        command.finding_json.as_deref(),
        command.finding_file.as_deref(),
    ) {
        Ok(value) => value,
        Err(error) => {
            print_adaptive_replan_finding_input_error(&error, command.json);
            return ExitCode::from(2);
        }
    };
    match build_adaptive_replan_finding_preview(&finding_json, "vida task adaptive-preview") {
        Ok(result) => {
            print_adaptive_replan_finding_preview(command.render, &result, command.json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            print_adaptive_replan_finding_input_error(&error, command.json);
            ExitCode::from(2)
        }
    }
}

fn graph_mutation_receipt_id(
    mutation_kind: &str,
    source_task_id: &str,
    planned_tasks: &[TaskMutationPlannedTask],
    planned_dependencies: &[TaskMutationPlannedDependency],
) -> String {
    let planned_task_ids = planned_tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let dependency_fingerprint = planned_dependencies
        .iter()
        .map(|dependency| {
            format!(
                "{}>{}:{}",
                dependency.issue_id, dependency.depends_on_id, dependency.edge_type
            )
        })
        .collect::<Vec<_>>()
        .join("+");
    format!(
        "task-graph-mutation:{mutation_kind}:{source_task_id}:tasks={planned_task_ids}:edges={dependency_fingerprint}"
    )
}

struct GraphMutationReceiptInput<'a> {
    mutation_kind: &'a str,
    surface: &'a str,
    source_task_id: &'a str,
    dry_run: bool,
    applied: bool,
    reason: &'a str,
    before_validation: TaskMutationValidationSummary,
    after_validation: TaskMutationValidationSummary,
    before_task_count: usize,
    after_task_count: usize,
    planned_tasks: &'a [TaskMutationPlannedTask],
    planned_dependencies: &'a [TaskMutationPlannedDependency],
}

fn build_graph_mutation_receipt(
    input: GraphMutationReceiptInput<'_>,
) -> TaskGraphMutationValidationReceipt {
    let planned_task_ids = input
        .planned_tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    TaskGraphMutationValidationReceipt {
        receipt_kind: "task_graph_mutation_receipt".to_string(),
        schema_version: "1".to_string(),
        receipt_id: graph_mutation_receipt_id(
            input.mutation_kind,
            input.source_task_id,
            input.planned_tasks,
            input.planned_dependencies,
        ),
        mutation_kind: input.mutation_kind.to_string(),
        surface: input.surface.to_string(),
        source_task_id: input.source_task_id.to_string(),
        dry_run: input.dry_run,
        applied: input.applied,
        reason: input.reason.to_string(),
        before_validation: input.before_validation,
        after_validation: input.after_validation,
        before_task_count: input.before_task_count,
        after_task_count: input.after_task_count,
        planned_task_ids,
        planned_dependency_edges: input.planned_dependencies.to_vec(),
        validation_scope:
            "before=current_authoritative_task_rows; after=planned_simulated_task_rows".to_string(),
        operator_truth: serde_json::json!({
            "receipt_records_graph_mutation_shape": true,
            "records_before_after_validation": true,
            "adaptive_replanner_loop_implemented": false,
            "adaptive_replanner_loop_truth": "not_implemented_in_this_slice",
            "applied_mutation_requires_after_validation_pass": true,
        }),
    }
}

fn task_parent_id(task: &state_store::TaskRecord) -> Option<String> {
    task.dependencies
        .iter()
        .find(|dependency| dependency.edge_type == "parent-child")
        .map(|dependency| dependency.depends_on_id.clone())
}

fn open_child_ids_for_task(rows: &[state_store::TaskRecord], task_id: &str) -> Vec<String> {
    let mut child_ids = rows
        .iter()
        .filter(|task| {
            task.status != "closed"
                && task.dependencies.iter().any(|dependency| {
                    dependency.edge_type == "parent-child" && dependency.depends_on_id == task_id
                })
        })
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    child_ids.sort();
    child_ids
}

fn inherited_split_execution_semantics(
    task: &state_store::TaskRecord,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: Some("sequential".to_string()),
        order_bucket: task.execution_semantics.order_bucket.clone(),
        parallel_group: None,
        conflict_domain: task
            .execution_semantics
            .conflict_domain
            .clone()
            .or_else(|| Some(task.id.clone())),
    }
}

fn blocker_execution_semantics(
    task: &state_store::TaskRecord,
) -> state_store::TaskExecutionSemantics {
    state_store::TaskExecutionSemantics {
        execution_mode: Some("sequential".to_string()),
        order_bucket: task.execution_semantics.order_bucket.clone(),
        parallel_group: None,
        conflict_domain: task.execution_semantics.conflict_domain.clone(),
    }
}

fn parse_split_child_specs(values: &[String]) -> Result<Vec<ParsedSplitChildSpec>, String> {
    if values.len() < 2 {
        return Err(
            "Use at least two `--child <task-id>:<title>` entries for `vida task split`."
                .to_string(),
        );
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let Some((task_id, title)) = value.split_once(':') else {
            return Err(format!(
                "Invalid `--child` value `{value}`. Expected `<task-id>:<title>`."
            ));
        };
        let task_id = task_id.trim();
        let title = title.trim();
        if task_id.is_empty() || title.is_empty() {
            return Err(format!(
                "Invalid `--child` value `{value}`. Both task id and title are required."
            ));
        }
        if !seen.insert(task_id.to_string()) {
            return Err(format!("Duplicate split child task id `{task_id}`."));
        }
        parsed.push(ParsedSplitChildSpec {
            task_id: task_id.to_string(),
            title: title.to_string(),
        });
    }
    Ok(parsed)
}

fn build_split_mutation_preview(
    rows: &[state_store::TaskRecord],
    source: &state_store::TaskRecord,
    child_specs: &[ParsedSplitChildSpec],
    reason: &str,
    surface: &str,
    dry_run: bool,
) -> Result<(TaskMutationResult, Vec<state_store::TaskRecord>), String> {
    if source.status == "closed" {
        return Err(format!(
            "Cannot split closed task `{}`; reopen it or choose another source task.",
            source.id
        ));
    }
    if source.issue_type == "epic" {
        return Err(format!(
            "Cannot split epic `{}` through `vida task split`; choose a bounded non-epic task.",
            source.id
        ));
    }
    let existing_children = open_child_ids_for_task(rows, &source.id);
    if !existing_children.is_empty() {
        return Err(format!(
            "Cannot split task `{}` while open child tasks already exist: {}",
            source.id,
            existing_children.join(", ")
        ));
    }
    if let Some(existing) = child_specs
        .iter()
        .find(|spec| rows.iter().any(|task| task.id == spec.task_id))
    {
        return Err(format!(
            "Cannot split task `{}` because child task id `{}` already exists.",
            source.id, existing.task_id
        ));
    }

    let non_parent_dependencies = source
        .dependencies
        .iter()
        .filter(|dependency| dependency.edge_type != "parent-child")
        .cloned()
        .collect::<Vec<_>>();
    let parent_id = Some(source.id.clone());
    let inherited_semantics = inherited_split_execution_semantics(source);
    let mut planned_tasks = Vec::with_capacity(child_specs.len());
    let mut planned_dependencies = Vec::new();
    let mut simulated_rows = rows.to_vec();
    let source_index = simulated_rows
        .iter()
        .position(|task| task.id == source.id)
        .ok_or_else(|| {
            format!(
                "Source task `{}` is missing from current task rows.",
                source.id
            )
        })?;

    let mut previous_child_id = None::<String>;
    for (index, spec) in child_specs.iter().enumerate() {
        let description = if source.description.trim().is_empty() {
            format!("Split from `{}`: {reason}", source.id)
        } else {
            source.description.clone()
        };
        let mut dependencies = vec![state_store::TaskDependencyRecord {
            issue_id: spec.task_id.clone(),
            depends_on_id: source.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        if index == 0 {
            for dependency in &non_parent_dependencies {
                dependencies.push(state_store::TaskDependencyRecord {
                    issue_id: spec.task_id.clone(),
                    depends_on_id: dependency.depends_on_id.clone(),
                    edge_type: dependency.edge_type.clone(),
                    created_at: source.updated_at.clone(),
                    created_by: surface.to_string(),
                    metadata: "{}".to_string(),
                    thread_id: String::new(),
                });
                planned_dependencies.push(TaskMutationPlannedDependency {
                    issue_id: spec.task_id.clone(),
                    depends_on_id: dependency.depends_on_id.clone(),
                    edge_type: dependency.edge_type.clone(),
                    reason: "inherit_source_dependency".to_string(),
                });
            }
        }

        if let Some(previous_child_id) = previous_child_id.as_ref() {
            dependencies.push(state_store::TaskDependencyRecord {
                issue_id: spec.task_id.clone(),
                depends_on_id: previous_child_id.clone(),
                edge_type: "depends-on".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
            planned_dependencies.push(TaskMutationPlannedDependency {
                issue_id: spec.task_id.clone(),
                depends_on_id: previous_child_id.clone(),
                edge_type: "depends-on".to_string(),
                reason: "sequential_split_chain".to_string(),
            });
        }

        simulated_rows.push(state_store::TaskRecord {
            id: spec.task_id.clone(),
            display_id: None,
            title: spec.title.clone(),
            description: description.clone(),
            status: "open".to_string(),
            priority: source.priority,
            issue_type: source.issue_type.clone(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            updated_at: source.updated_at.clone(),
            closed_at: None,
            close_reason: None,
            source_repo: source.source_repo.clone(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: source.labels.clone(),
            planner_metadata: source.planner_metadata.clone(),
            execution_semantics: inherited_semantics.clone(),
            dependencies,
        });
        planned_tasks.push(TaskMutationPlannedTask {
            task_id: spec.task_id.clone(),
            title: spec.title.clone(),
            description,
            issue_type: source.issue_type.clone(),
            status: "open".to_string(),
            priority: source.priority,
            parent_id: parent_id.clone(),
            labels: source.labels.clone(),
            execution_semantics: inherited_semantics.clone(),
            planner_metadata: source.planner_metadata.clone(),
        });
        previous_child_id = Some(spec.task_id.clone());
    }

    if let Some(last_child_id) = previous_child_id {
        simulated_rows[source_index]
            .dependencies
            .push(state_store::TaskDependencyRecord {
                issue_id: source.id.clone(),
                depends_on_id: last_child_id.clone(),
                edge_type: "depends-on".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        planned_dependencies.push(TaskMutationPlannedDependency {
            issue_id: source.id.clone(),
            depends_on_id: last_child_id,
            edge_type: "depends-on".to_string(),
            reason: "block_source_until_split_children_complete".to_string(),
        });
    }

    let before_validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(rows));
    let validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(&simulated_rows));
    let status = if validation.issue_count > 0 {
        "blocked".to_string()
    } else if dry_run {
        "dry_run".to_string()
    } else {
        task_json_success_status().to_string()
    };
    let created_task_ids = if dry_run || validation.issue_count > 0 {
        Vec::new()
    } else {
        planned_tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect()
    };
    let applied = !dry_run && validation.issue_count == 0;
    let graph_mutation_receipt = build_graph_mutation_receipt(GraphMutationReceiptInput {
        mutation_kind: "split_task",
        surface,
        source_task_id: &source.id,
        dry_run,
        applied,
        reason,
        before_validation,
        after_validation: validation.clone(),
        before_task_count: rows.len(),
        after_task_count: simulated_rows.len(),
        planned_tasks: &planned_tasks,
        planned_dependencies: &planned_dependencies,
    });
    Ok((
        TaskMutationResult {
            status,
            surface: surface.to_string(),
            mutation_kind: "split_task".to_string(),
            source_task_id: source.id.clone(),
            dry_run,
            applied,
            reason: reason.to_string(),
            planned_tasks,
            planned_dependencies,
            created_task_ids,
            validation,
            graph_mutation_receipt,
        },
        simulated_rows,
    ))
}

fn build_spawn_blocker_preview(
    rows: &[state_store::TaskRecord],
    source: &state_store::TaskRecord,
    command: &TaskSpawnBlockerArgs,
    surface: &str,
) -> Result<(TaskMutationResult, Vec<state_store::TaskRecord>), String> {
    if source.status == "closed" {
        return Err(format!(
            "Cannot spawn blocker for closed task `{}`.",
            source.id
        ));
    }
    if rows.iter().any(|task| task.id == command.blocker_task_id) {
        return Err(format!(
            "Cannot create blocker task `{}` because it already exists.",
            command.blocker_task_id
        ));
    }

    let mut blocker_labels = source.labels.clone();
    blocker_labels.extend(parse_label_values(&command.labels));
    blocker_labels.sort();
    blocker_labels.dedup();

    let blocker_priority = command.priority.unwrap_or(source.priority);
    let blocker_description = command
        .description
        .clone()
        .unwrap_or_else(|| format!("Blocker for `{}`: {}", source.id, command.reason));
    let blocker_parent_id = task_parent_id(source);
    let blocker_semantics = blocker_execution_semantics(source);

    let mut simulated_rows = rows.to_vec();
    let source_index = simulated_rows
        .iter()
        .position(|task| task.id == source.id)
        .ok_or_else(|| {
            format!(
                "Source task `{}` is missing from current task rows.",
                source.id
            )
        })?;
    simulated_rows.push(state_store::TaskRecord {
        id: command.blocker_task_id.clone(),
        display_id: None,
        title: command.title.clone(),
        description: blocker_description.clone(),
        status: command.status.clone(),
        priority: blocker_priority,
        issue_type: command.issue_type.clone(),
        created_at: source.updated_at.clone(),
        created_by: surface.to_string(),
        updated_at: source.updated_at.clone(),
        closed_at: None,
        close_reason: None,
        source_repo: source.source_repo.clone(),
        compaction_level: 0,
        original_size: 0,
        notes: None,
        labels: blocker_labels.clone(),
        planner_metadata: source.planner_metadata.clone(),
        execution_semantics: blocker_semantics.clone(),
        dependencies: blocker_parent_id
            .iter()
            .map(|parent_id| state_store::TaskDependencyRecord {
                issue_id: command.blocker_task_id.clone(),
                depends_on_id: parent_id.clone(),
                edge_type: "parent-child".to_string(),
                created_at: source.updated_at.clone(),
                created_by: surface.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            })
            .collect(),
    });
    simulated_rows[source_index]
        .dependencies
        .push(state_store::TaskDependencyRecord {
            issue_id: source.id.clone(),
            depends_on_id: command.blocker_task_id.clone(),
            edge_type: "blocks".to_string(),
            created_at: source.updated_at.clone(),
            created_by: surface.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        });

    let before_validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(rows));
    let validation =
        task_mutation_validation_summary(StateStore::validate_task_graph_rows(&simulated_rows));
    let dry_run = command.dry_run;
    let status = if validation.issue_count > 0 {
        "blocked".to_string()
    } else if dry_run {
        "dry_run".to_string()
    } else {
        task_json_success_status().to_string()
    };
    let planned_tasks = vec![TaskMutationPlannedTask {
        task_id: command.blocker_task_id.clone(),
        title: command.title.clone(),
        description: blocker_description.clone(),
        issue_type: command.issue_type.clone(),
        status: command.status.clone(),
        priority: blocker_priority,
        parent_id: blocker_parent_id,
        labels: blocker_labels,
        execution_semantics: blocker_semantics,
        planner_metadata: source.planner_metadata.clone(),
    }];
    let planned_dependencies = vec![TaskMutationPlannedDependency {
        issue_id: source.id.clone(),
        depends_on_id: command.blocker_task_id.clone(),
        edge_type: "blocks".to_string(),
        reason: "spawn_blocker_dependency".to_string(),
    }];
    let created_task_ids = if dry_run || validation.issue_count > 0 {
        Vec::new()
    } else {
        vec![command.blocker_task_id.clone()]
    };
    let applied = !dry_run && validation.issue_count == 0;
    let graph_mutation_receipt = build_graph_mutation_receipt(GraphMutationReceiptInput {
        mutation_kind: "spawn_blocker_task",
        surface,
        source_task_id: &source.id,
        dry_run,
        applied,
        reason: &command.reason,
        before_validation,
        after_validation: validation.clone(),
        before_task_count: rows.len(),
        after_task_count: simulated_rows.len(),
        planned_tasks: &planned_tasks,
        planned_dependencies: &planned_dependencies,
    });
    Ok((
        TaskMutationResult {
            status,
            surface: surface.to_string(),
            mutation_kind: "spawn_blocker_task".to_string(),
            source_task_id: source.id.clone(),
            dry_run,
            applied: !dry_run && validation.issue_count == 0,
            reason: command.reason.clone(),
            planned_tasks,
            planned_dependencies,
            created_task_ids,
            validation,
            graph_mutation_receipt,
        },
        simulated_rows,
    ))
}

fn print_task_mutation_preview(render: RenderMode, result: &TaskMutationResult, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(result).expect("task mutation preview should serialize to json");
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, &result.surface);
    print_surface_line(render, "status", &result.status);
    print_surface_line(render, "mutation_kind", &result.mutation_kind);
    print_surface_line(render, "source_task_id", &result.source_task_id);
    print_surface_line(
        render,
        "dry_run",
        if result.dry_run { "true" } else { "false" },
    );
    print_surface_line(
        render,
        "applied",
        if result.applied { "true" } else { "false" },
    );
    print_surface_line(
        render,
        "planned_task_count",
        &result.planned_tasks.len().to_string(),
    );
    print_surface_line(
        render,
        "planned_dependency_count",
        &result.planned_dependencies.len().to_string(),
    );
    if !result.created_task_ids.is_empty() {
        print_surface_line(
            render,
            "created_task_ids",
            &result.created_task_ids.join(", "),
        );
    }
    if !result.validation.blocker_codes.is_empty() {
        print_surface_line(
            render,
            "blocker_codes",
            &result.validation.blocker_codes.join(", "),
        );
    }
}

async fn run_task_split_like(command: TaskSplitArgs, surface: &str) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let child_specs = match parse_split_child_specs(&command.children) {
        Ok(specs) => specs,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let store = match open_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let source = match store.show_task(&command.task_id).await {
        Ok(task) => task,
        Err(error) => {
            eprintln!("Failed to load split source task: {error}");
            return ExitCode::from(1);
        }
    };
    let rows = match store.all_tasks().await {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Failed to read current task graph before split: {error}");
            return ExitCode::from(1);
        }
    };
    let (result, _) = match build_split_mutation_preview(
        &rows,
        &source,
        &child_specs,
        &command.reason,
        surface,
        command.dry_run,
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if result.validation.issue_count > 0 {
        print_task_mutation_preview(command.render, &result, command.json);
        return ExitCode::from(1);
    }

    if !command.dry_run {
        let source_repo = source.source_repo.clone();
        for task in &result.planned_tasks {
            if let Err(error) = store
                .create_task(state_store::CreateTaskRequest {
                    task_id: &task.task_id,
                    title: &task.title,
                    display_id: None,
                    description: &task.description,
                    issue_type: &task.issue_type,
                    status: &task.status,
                    priority: task.priority,
                    parent_id: task.parent_id.as_deref(),
                    labels: &task.labels,
                    execution_semantics: task.execution_semantics.clone(),
                    planner_metadata: task.planner_metadata.clone(),
                    created_by: surface,
                    source_repo: &source_repo,
                })
                .await
            {
                eprintln!(
                    "Failed to create split child task `{}`: {error}",
                    task.task_id
                );
                return ExitCode::from(1);
            }
        }
        for dependency in &result.planned_dependencies {
            if let Err(error) = store
                .add_task_dependency(
                    &dependency.issue_id,
                    &dependency.depends_on_id,
                    &dependency.edge_type,
                    surface,
                )
                .await
            {
                eprintln!(
                    "Failed to add split dependency `{}` -> `{}`: {error}",
                    dependency.issue_id, dependency.depends_on_id
                );
                return ExitCode::from(1);
            }
        }
        if let Err(code) = refresh_task_snapshot_after_mutation(&store, surface).await {
            return code;
        }
    }

    print_task_mutation_preview(command.render, &result, command.json);
    ExitCode::SUCCESS
}

async fn run_task_spawn_blocker_like(command: TaskSpawnBlockerArgs, surface: &str) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let store = match open_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let source = match store.show_task(&command.task_id).await {
        Ok(task) => task,
        Err(error) => {
            eprintln!("Failed to load blocker source task: {error}");
            return ExitCode::from(1);
        }
    };
    let rows = match store.all_tasks().await {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Failed to read current task graph before blocker mutation: {error}");
            return ExitCode::from(1);
        }
    };
    let (result, _) = match build_spawn_blocker_preview(&rows, &source, &command, surface) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if result.validation.issue_count > 0 {
        print_task_mutation_preview(command.render, &result, command.json);
        return ExitCode::from(1);
    }

    if !command.dry_run {
        let planned_task = result
            .planned_tasks
            .first()
            .expect("spawn blocker preview should include one planned task");
        if let Err(error) = store
            .create_task(state_store::CreateTaskRequest {
                task_id: &planned_task.task_id,
                title: &planned_task.title,
                display_id: None,
                description: &planned_task.description,
                issue_type: &planned_task.issue_type,
                status: &planned_task.status,
                priority: planned_task.priority,
                parent_id: planned_task.parent_id.as_deref(),
                labels: &planned_task.labels,
                execution_semantics: planned_task.execution_semantics.clone(),
                planner_metadata: planned_task.planner_metadata.clone(),
                created_by: surface,
                source_repo: &source.source_repo,
            })
            .await
        {
            eprintln!(
                "Failed to create blocker task `{}`: {error}",
                planned_task.task_id
            );
            return ExitCode::from(1);
        }
        let dependency = result
            .planned_dependencies
            .first()
            .expect("spawn blocker preview should include one dependency");
        if let Err(error) = store
            .add_task_dependency(
                &dependency.issue_id,
                &dependency.depends_on_id,
                &dependency.edge_type,
                surface,
            )
            .await
        {
            eprintln!(
                "Failed to attach blocker task `{}` to source `{}`: {error}",
                dependency.depends_on_id, dependency.issue_id
            );
            return ExitCode::from(1);
        }
        if let Err(code) = refresh_task_snapshot_after_mutation(&store, surface).await {
            return code;
        }
    }

    print_task_mutation_preview(command.render, &result, command.json);
    ExitCode::SUCCESS
}

async fn run_task_create_like(command: TaskCreateArgs, ensure_existing: bool) -> ExitCode {
    let title = match task_create_title(&command) {
        Ok(title) => title,
        Err(error) => {
            if command.json {
                crate::print_json_pretty(&serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": ["invalid_task_title_input"],
                    "reason": error,
                    "usage": "vida task create <task-id> <title> --json OR vida task create <task-id> --title <title> --json",
                }));
            } else {
                eprintln!("{error}");
                eprintln!(
                    "Usage: vida task create <task-id> <title> --json OR vida task create <task-id> --title <title> --json"
                );
            }
            return ExitCode::from(2);
        }
    };
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let project_root = project_root_for_task_state(&state_dir).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    match open_task_store(state_dir).await {
        Ok(store) => {
            let mut parent_id = command.parent_id.clone();
            let mut display_id = command.display_id.clone().unwrap_or_default();
            let auto_display_from = command.auto_display_from.clone().unwrap_or_default();
            let parent_display_id = command.parent_display_id.clone().unwrap_or_default();
            if display_id.is_empty() && !auto_display_from.is_empty() && parent_id.is_some() {
                display_id = format!("{auto_display_from}.1");
            }
            if (display_id.is_empty() && !auto_display_from.is_empty())
                || (parent_id.is_none() && !parent_display_id.is_empty())
            {
                match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            if display_id.is_empty() && !auto_display_from.is_empty() {
                                let next = crate::taskflow_task_bridge::next_display_id_payload(
                                    &rows,
                                    &auto_display_from,
                                );
                                if !next
                                    .get("valid")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    print_task_next_display_id(command.render, &next, command.json);
                                    return ExitCode::from(1);
                                }
                                display_id = next
                                    .get("next_display_id")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                            }
                            if parent_id.is_none() && !parent_display_id.is_empty() {
                                let resolved =
                                    crate::taskflow_task_bridge::resolve_task_id_by_display_id(
                                        &rows,
                                        &parent_display_id,
                                    );
                                if !resolved
                                    .get("found")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    if command.json {
                                        crate::print_json_pretty(&resolved);
                                    } else {
                                        eprintln!(
                                            "{}",
                                            resolved
                                                .get("reason")
                                                .and_then(serde_json::Value::as_str)
                                                .unwrap_or("parent_display_id_not_found")
                                        );
                                    }
                                    return ExitCode::from(1);
                                }
                                parent_id = Some(
                                    resolved
                                        .get("task_id")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_string(),
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Failed to {} task: {error}",
                                if ensure_existing { "ensure" } else { "create" }
                            );
                            return ExitCode::from(1);
                        }
                    },
                    Err(error) => {
                        eprintln!(
                            "Failed to {} task: {error}",
                            if ensure_existing { "ensure" } else { "create" }
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            if ensure_existing {
                if let Ok(task) = store.show_task(&command.task_id).await {
                    print_task_mutation(command.render, "vida task ensure", &task, command.json);
                    return ExitCode::SUCCESS;
                }
            }
            let labels = parse_label_values(&command.labels);
            let source_repo = project_root.display().to_string();
            match store
                .create_task(state_store::CreateTaskRequest {
                    task_id: &command.task_id,
                    title: &title,
                    display_id: (!display_id.is_empty()).then_some(display_id.as_str()),
                    description: &command.description,
                    issue_type: &command.issue_type,
                    status: &command.status,
                    priority: command.priority,
                    parent_id: parent_id.as_deref(),
                    labels: &labels,
                    execution_semantics: task_execution_semantics_from_create_args(&command),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "vida task",
                    source_repo: &source_repo,
                })
                .await
            {
                Ok(task) => {
                    if let Err(code) =
                        refresh_task_snapshot_after_mutation(&store, "vida task create").await
                    {
                        return code;
                    }
                    print_task_mutation(
                        command.render,
                        if ensure_existing {
                            "vida task ensure"
                        } else {
                            "vida task create"
                        },
                        &task,
                        command.json,
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!(
                        "Failed to {} task: {error}",
                        if ensure_existing { "ensure" } else { "create" }
                    );
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn task_create_title(command: &TaskCreateArgs) -> Result<String, String> {
    let positional = command
        .positional_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let option = command
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (positional, option) {
        (Some(_), Some(_)) => Err(
            "Provide only one task title source: positional <TITLE> or --title <TITLE>."
                .to_string(),
        ),
        (Some(title), None) | (None, Some(title)) => Ok(title.to_string()),
        (None, None) => {
            Err("Missing task title. Use positional <TITLE> or --title <TITLE>.".to_string())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskCloseAutomationReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    release_build: Option<crate::release_surface::ReleaseBuildReceipt>,
    release_install: Option<crate::release_surface::ReleaseInstallReceipt>,
    git: Option<TaskCloseGitAutomationReceipt>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskCloseGitAutomationReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    commit_exit_code: Option<i32>,
    push_exit_code: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskOwnedStatusReceipt {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task_id: String,
    ownership_source: String,
    owned_paths: Vec<String>,
    dirty_files: Vec<String>,
    owned_files: Vec<String>,
    unowned_files: Vec<String>,
    stageable_files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskHandoffAcceptReceipt {
    status: String,
    task_id: String,
    agent_id: String,
    accepted_at: String,
    changed_files: Vec<String>,
    proof_commands: Vec<String>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    receipt_path: String,
    receipt_root: String,
    isolation: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskContinuationCandidate {
    task_id: String,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
    ready_parallel_safe: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskNextLawfulReceipt {
    status: String,
    active_bounded_unit: serde_json::Value,
    why_this_unit: String,
    sequential_vs_parallel_posture: String,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    source_surfaces: Vec<String>,
}

fn task_close_automation_requested(command: &TaskCloseArgs) -> bool {
    command.release || command.install || command.commit || command.push || command.stage_owned
}

fn task_close_automation_receipt(
    command: &TaskCloseArgs,
    project_root: Option<&std::path::Path>,
    task: Option<&state_store::TaskRecord>,
) -> TaskCloseAutomationReceipt {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();

    let release_install = if command.install {
        let receipt = crate::release_surface::release_install_receipt(&crate::ReleaseInstallArgs {
            target: command.install_target.clone(),
            skip_build: command.skip_release_build,
            source_binary: command.source_binary.clone(),
            install_root: command.install_root.clone(),
            json: true,
        });
        if receipt.status != "pass" {
            blocker_codes.extend(receipt.blocker_codes.iter().cloned());
            next_actions.extend(receipt.next_actions.iter().cloned());
        }
        Some(receipt)
    } else {
        None
    };

    let release_build = if command.release && !command.install {
        let receipt = crate::release_surface::release_build_receipt(false);
        if receipt.status != "pass" {
            blocker_codes.push("release_build_failed".to_string());
            next_actions.push(
                "Fix release build failures, then rerun `vida task close --release --json`."
                    .to_string(),
            );
        }
        Some(receipt)
    } else {
        None
    };

    let git = if command.commit || command.push || command.stage_owned {
        let receipt = task_close_git_automation_receipt(command, project_root, task);
        if receipt.status != "pass" {
            blocker_codes.extend(receipt.blocker_codes.iter().cloned());
            next_actions.extend(receipt.next_actions.iter().cloned());
        }
        Some(receipt)
    } else {
        None
    };

    TaskCloseAutomationReceipt {
        status: if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        blocker_codes,
        next_actions,
        release_build,
        release_install,
        git,
    }
}

fn task_close_git_automation_receipt(
    command: &TaskCloseArgs,
    project_root: Option<&std::path::Path>,
    task: Option<&state_store::TaskRecord>,
) -> TaskCloseGitAutomationReceipt {
    let explicit_files = task_close_commit_file_strings(command, task);
    let commit_message = command.commit_message.clone().or_else(|| {
        command
            .commit
            .then(|| format!("Close {}: {}", command.task_id, command.reason))
    });

    if command.push && !command.commit {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "push_requires_commit",
            "Pass `--commit --commit-file <path>` with `--push` so the pushed change is explicit.",
        );
    }
    if command.stage_owned && !command.commit {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "stage_owned_requires_commit",
            "Pass `--commit --stage-owned` so owned-path staging is tied to an explicit commit request.",
        );
    }
    if command.commit && explicit_files.is_empty() {
        return blocked_task_close_git_receipt(
            explicit_files,
            commit_message,
            "dirty_ownership_ambiguous",
            "Pass one or more `--commit-file <path>` values, or pass `--stage-owned` when the task has planner_metadata.owned_paths.",
        );
    }

    let repo_root = project_root
        .map(std::path::Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    if command.commit {
        match dirty_paths_for_repo(&repo_root) {
            Ok(dirty_paths) => {
                let ambiguous: Vec<String> = dirty_paths
                    .into_iter()
                    .filter(|path| !path_is_explicitly_owned(path, &explicit_files))
                    .collect();
                if !ambiguous.is_empty() {
                    return blocked_task_close_git_receipt(
                        explicit_files,
                        commit_message,
                        "dirty_ownership_ambiguous",
                        "Clean unrelated dirty files or include only the owned paths with repeated `--commit-file` values.",
                    );
                }
            }
            Err(_) => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "git_status_failed",
                    "Run the command from a git worktree or resolve git status errors before committing.",
                );
            }
        }

        let stage_files: Vec<std::path::PathBuf> = explicit_files
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let add_status = std::process::Command::new("git")
            .arg("add")
            .arg("--")
            .args(&stage_files)
            .current_dir(&repo_root)
            .status();
        match add_status {
            Ok(status) if status.success() => {}
            _ => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "git_stage_failed",
                    "Verify the explicit commit files exist and can be staged.",
                );
            }
        }

        let diff_status = std::process::Command::new("git")
            .args(["diff", "--cached", "--quiet", "--"])
            .args(&stage_files)
            .current_dir(&repo_root)
            .status();
        match diff_status {
            Ok(status) if status.success() => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "no_explicit_commit_changes",
                    "Ensure at least one explicit `--commit-file` has a staged content change.",
                );
            }
            Ok(status) if status.code() == Some(1) => {}
            _ => {
                return blocked_task_close_git_receipt(
                    explicit_files,
                    commit_message,
                    "git_status_failed",
                    "Resolve git diff errors before committing.",
                );
            }
        }

        let message = commit_message
            .as_deref()
            .unwrap_or("Close task with post-close automation");
        let commit_status = std::process::Command::new("git")
            .args(["commit", "-m", message, "--"])
            .args(&stage_files)
            .current_dir(&repo_root)
            .status();
        match commit_status {
            Ok(status) if status.success() => {
                if command.push {
                    let push_status = std::process::Command::new("git")
                        .arg("push")
                        .current_dir(&repo_root)
                        .status();
                    match push_status {
                        Ok(push) if push.success() => TaskCloseGitAutomationReceipt {
                            status: "pass".to_string(),
                            blocker_codes: Vec::new(),
                            next_actions: Vec::new(),
                            explicit_files,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: push.code(),
                        },
                        Ok(push) => TaskCloseGitAutomationReceipt {
                            status: "blocked".to_string(),
                            blocker_codes: vec!["git_push_failed".to_string()],
                            next_actions: vec![
                                "Fix git push configuration or remote state, then push manually."
                                    .to_string(),
                            ],
                            explicit_files,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: push.code(),
                        },
                        Err(_) => TaskCloseGitAutomationReceipt {
                            status: "blocked".to_string(),
                            blocker_codes: vec!["git_push_failed".to_string()],
                            next_actions: vec![
                                "Ensure `git push` can run in this worktree, then push manually."
                                    .to_string(),
                            ],
                            explicit_files,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: None,
                        },
                    }
                } else {
                    TaskCloseGitAutomationReceipt {
                        status: "pass".to_string(),
                        blocker_codes: Vec::new(),
                        next_actions: Vec::new(),
                        explicit_files,
                        commit_message,
                        commit_exit_code: status.code(),
                        push_exit_code: None,
                    }
                }
            }
            Ok(status) => TaskCloseGitAutomationReceipt {
                status: "blocked".to_string(),
                blocker_codes: vec!["git_commit_failed".to_string()],
                next_actions: vec![
                    "Inspect git commit output and resolve commit blockers before retrying."
                        .to_string(),
                ],
                explicit_files,
                commit_message,
                commit_exit_code: status.code(),
                push_exit_code: None,
            },
            Err(_) => TaskCloseGitAutomationReceipt {
                status: "blocked".to_string(),
                blocker_codes: vec!["git_commit_failed".to_string()],
                next_actions: vec![
                    "Ensure `git commit` can run in this worktree before retrying.".to_string(),
                ],
                explicit_files,
                commit_message,
                commit_exit_code: None,
                push_exit_code: None,
            },
        }
    } else {
        TaskCloseGitAutomationReceipt {
            status: "pass".to_string(),
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
            explicit_files,
            commit_message,
            commit_exit_code: None,
            push_exit_code: None,
        }
    }
}

fn blocked_task_close_git_receipt(
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    blocker_code: &str,
    next_action: &str,
) -> TaskCloseGitAutomationReceipt {
    TaskCloseGitAutomationReceipt {
        status: "blocked".to_string(),
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        explicit_files,
        commit_message,
        commit_exit_code: None,
        push_exit_code: None,
    }
}

fn task_close_commit_file_strings(
    command: &TaskCloseArgs,
    task: Option<&state_store::TaskRecord>,
) -> Vec<String> {
    let mut files: Vec<String> = command
        .commit_files
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    if command.stage_owned {
        if let Some(task) = task {
            files.extend(task.planner_metadata.owned_paths.iter().cloned());
        }
    }
    canonical_owned_paths(files)
}

fn canonical_owned_paths(paths: Vec<String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for path in paths {
        let trimmed = path.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
}

fn task_owned_status_receipt(
    task_id: &str,
    metadata_owned_paths: Vec<String>,
    override_files: Vec<String>,
    dirty_files: Vec<String>,
) -> TaskOwnedStatusReceipt {
    let override_files = canonical_owned_paths(override_files);
    let metadata_owned_paths = canonical_owned_paths(metadata_owned_paths);
    let (owned_paths, ownership_source) = if !override_files.is_empty() {
        (override_files, "explicit_file_overrides".to_string())
    } else if !metadata_owned_paths.is_empty() {
        (
            metadata_owned_paths,
            "planner_metadata.owned_paths".to_string(),
        )
    } else {
        (Vec::new(), "missing".to_string())
    };

    if owned_paths.is_empty() {
        return TaskOwnedStatusReceipt {
            status: "blocked".to_string(),
            blocker_codes: vec!["missing_owned_paths".to_string()],
            next_actions: vec![
                "Add planner_metadata.owned_paths to the task or rerun with repeated `--file <path>` overrides.".to_string(),
            ],
            task_id: task_id.to_string(),
            ownership_source,
            owned_paths,
            dirty_files,
            owned_files: Vec::new(),
            unowned_files: Vec::new(),
            stageable_files: Vec::new(),
        };
    }

    let mut owned_files = Vec::new();
    let mut unowned_files = Vec::new();
    for path in &dirty_files {
        if path_is_explicitly_owned(path, &owned_paths) {
            owned_files.push(path.clone());
        } else {
            unowned_files.push(path.clone());
        }
    }
    let stageable_files = owned_files.clone();
    let blocked = !unowned_files.is_empty();

    TaskOwnedStatusReceipt {
        status: if blocked { "blocked" } else { "pass" }.to_string(),
        blocker_codes: if blocked {
            vec!["dirty_ownership_ambiguous".to_string()]
        } else {
            Vec::new()
        },
        next_actions: if blocked {
            vec![
                "Commit/stash unrelated dirty files or expand the explicit owned path set before staging.".to_string(),
            ]
        } else if stageable_files.is_empty() {
            vec!["No dirty files are covered by the selected ownership source.".to_string()]
        } else {
            vec!["Stage only `stageable_files` before committing this task.".to_string()]
        },
        task_id: task_id.to_string(),
        ownership_source,
        owned_paths,
        dirty_files,
        owned_files,
        unowned_files,
        stageable_files,
    }
}

fn task_handoff_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render")
}

fn task_handoff_receipt_filename_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos()
        .to_string()
}

fn task_handoff_project_receipt_root(project_root: &std::path::Path) -> std::path::PathBuf {
    project_root.join(".vida").join("receipts")
}

fn task_handoff_isolated_receipt_root(state_dir: &std::path::Path) -> std::path::PathBuf {
    state_dir.join("receipts")
}

fn task_handoff_receipt_dir(receipt_root: &std::path::Path) -> std::path::PathBuf {
    receipt_root.join("task-handoffs")
}

fn task_handoff_receipt_root(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
) -> (std::path::PathBuf, &'static str) {
    if task_close_uses_isolated_state_dir(state_dir, explicit_state_dir) {
        return (
            task_handoff_isolated_receipt_root(state_dir),
            "isolated_state_dir",
        );
    }
    let project_root = project_root_for_task_state(state_dir)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    (
        task_handoff_project_receipt_root(&project_root),
        "project_state_dir",
    )
}

fn sanitize_task_handoff_receipt_component(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.to_string()
    }
}

fn task_handoff_receipt_path(
    receipt_root: &std::path::Path,
    task_id: &str,
    filename_timestamp: &str,
) -> std::path::PathBuf {
    task_handoff_receipt_dir(receipt_root).join(format!(
        "{}-{}.json",
        sanitize_task_handoff_receipt_component(task_id),
        filename_timestamp
    ))
}

fn canonical_nonempty_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !canonical.contains(&trimmed) {
            canonical.push(trimmed);
        }
    }
    canonical
}

fn blocked_task_handoff_accept_receipt(
    task_id: &str,
    agent_id: &str,
    blocker_code: &str,
    next_action: &str,
) -> TaskHandoffAcceptReceipt {
    TaskHandoffAcceptReceipt {
        status: "blocked".to_string(),
        task_id: task_id.to_string(),
        agent_id: agent_id.to_string(),
        accepted_at: task_handoff_timestamp(),
        changed_files: Vec::new(),
        proof_commands: Vec::new(),
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        receipt_path: "not_persisted".to_string(),
        receipt_root: "not_persisted".to_string(),
        isolation: "not_persisted".to_string(),
    }
}

fn task_handoff_accept_receipt(
    command: &TaskHandoffAcceptArgs,
    receipt_path: &std::path::Path,
    receipt_root: &std::path::Path,
    isolation: &str,
    accepted_at: String,
) -> TaskHandoffAcceptReceipt {
    let changed_files = canonical_owned_paths(
        command
            .files
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    );
    let proof_commands = canonical_nonempty_strings(command.proofs.clone());
    let blocker_codes = canonical_nonempty_strings(command.blockers.clone());
    let next_actions = canonical_nonempty_strings(command.next_actions.clone());
    TaskHandoffAcceptReceipt {
        status: command.status.as_str().to_string(),
        task_id: command.task_id.trim().to_string(),
        agent_id: command.agent.as_deref().unwrap_or("").trim().to_string(),
        accepted_at,
        changed_files,
        proof_commands,
        blocker_codes,
        next_actions,
        receipt_path: receipt_path.display().to_string(),
        receipt_root: receipt_root.display().to_string(),
        isolation: isolation.to_string(),
    }
}

fn validate_task_handoff_accept_receipt(
    receipt: &TaskHandoffAcceptReceipt,
) -> Result<(), (&'static str, &'static str)> {
    if receipt.agent_id.trim().is_empty() {
        return Err((
            "missing_agent_id",
            "Pass `--agent <id>` with the delegated agent or carrier id.",
        ));
    }
    if receipt.status == "blocked"
        && receipt.blocker_codes.is_empty()
        && receipt.proof_commands.is_empty()
    {
        return Err((
            "blocked_handoff_requires_detail",
            "Pass `--blocker <code>` or `--proof <command>` when accepting a blocked handoff.",
        ));
    }
    Ok(())
}

fn persist_task_handoff_accept_receipt(
    receipt: &TaskHandoffAcceptReceipt,
    receipt_path: &std::path::Path,
) -> Result<(), String> {
    if let Some(parent) = receipt_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create task handoff receipt directory `{}`: {error}",
                parent.display()
            )
        })?;
    }
    let rendered = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("failed to render task handoff receipt json: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(receipt_path)
        .map_err(|error| {
            format!(
                "failed to create task handoff receipt `{}` without overwrite: {error}",
                receipt_path.display()
            )
        })?;
    use std::io::Write;
    file.write_all(&rendered).map_err(|error| {
        format!(
            "failed to write task handoff receipt `{}`: {error}",
            receipt_path.display()
        )
    })?;
    file.write_all(b"\n").map_err(|error| {
        format!(
            "failed to finish task handoff receipt `{}`: {error}",
            receipt_path.display()
        )
    })
}

fn task_continuation_candidate(
    task: &state_store::TaskRecord,
    ready_parallel_safe: bool,
) -> TaskContinuationCandidate {
    TaskContinuationCandidate {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
        ready_parallel_safe,
    }
}

fn task_continuation_active_unit(task: &state_store::TaskRecord) -> serde_json::Value {
    serde_json::json!({
        "task_id": task.id,
        "title": task.title,
        "status": task.status,
        "issue_type": task.issue_type,
    })
}

fn task_continuation_source_surfaces() -> Vec<String> {
    vec![
        "vida task next-lawful".to_string(),
        "StateStore::latest_explicit_run_graph_continuation_binding".to_string(),
        "StateStore::latest_run_graph_status".to_string(),
        "StateStore::run_graph_continuation_binding(latest_run_id)".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida task ready --json".to_string(),
        "vida status --json continuation_binding".to_string(),
        "vida taskflow consume continue --json projection_truth.continuation_binding".to_string(),
    ]
}

fn continuation_binding_active_kind(binding: &state_store::RunGraphContinuationBinding) -> &str {
    binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}

fn continuation_binding_requires_open_task(
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    continuation_binding_active_kind(binding) != "downstream_dispatch_target"
}

fn task_status_for_binding<'a>(
    tasks: &'a [state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> Option<&'a str> {
    tasks
        .iter()
        .find(|task| task.id == binding.task_id)
        .map(|task| task.status.as_str())
}

fn task_exists_for_binding(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    tasks.iter().any(|task| task.id == binding.task_id)
}

fn continuation_bindings_same_unit(
    left: &state_store::RunGraphContinuationBinding,
    right: &state_store::RunGraphContinuationBinding,
) -> bool {
    left.run_id == right.run_id
        && left.task_id == right.task_id
        && left.active_bounded_unit == right.active_bounded_unit
}

fn continuation_binding_is_historical_task_close_reconcile(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "task_close_reconcile" && explicit.run_id != current.run_id
}

fn select_task_next_lawful_binding<'a>(
    tasks: &[state_store::TaskRecord],
    explicit_binding: Option<&'a state_store::RunGraphContinuationBinding>,
    current_binding: Option<&'a state_store::RunGraphContinuationBinding>,
) -> Result<Option<&'a state_store::RunGraphContinuationBinding>, TaskNextLawfulReceipt> {
    match (explicit_binding, current_binding) {
        (Some(explicit), Some(current)) if !continuation_bindings_same_unit(explicit, current) => {
            if continuation_binding_is_historical_task_close_reconcile(explicit, current) {
                return Ok(Some(current));
            }
            let explicit_status = task_status_for_binding(tasks, explicit);
            if continuation_binding_requires_open_task(explicit)
                && matches!(explicit_status, Some("closed"))
            {
                return Ok(Some(current));
            }
            Err(blocked_task_next_lawful_receipt(
                explicit.active_bounded_unit.clone(),
                Vec::new(),
                "continuation_source_drift",
                &format!(
                    "Continuation sources disagree: explicit binding `{}`/`{}` points to `{}`, while current latest-run binding `{}`/`{}` from `{}` points to `{}`. Reconcile with `vida status --json` and `vida taskflow consume continue --json` before continuing.",
                    explicit.run_id,
                    explicit.binding_source,
                    explicit.task_id,
                    current.run_id,
                    current.binding_source,
                    current.binding_source,
                    current.task_id
                ),
            ))
        }
        (Some(explicit), Some(_current)) => Ok(Some(explicit)),
        (Some(explicit), None) => Ok(Some(explicit)),
        (None, Some(current)) => Ok(Some(current)),
        (None, None) => Ok(None),
    }
}

fn blocked_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_code: &str,
    next_action: &str,
) -> TaskNextLawfulReceipt {
    TaskNextLawfulReceipt {
        status: "blocked".to_string(),
        active_bounded_unit,
        why_this_unit: "blocked_until_unique_lawful_continuation_is_evidenced".to_string(),
        sequential_vs_parallel_posture: "unknown_until_explicit_binding".to_string(),
        ready_task_candidates,
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        source_surfaces: task_continuation_source_surfaces(),
    }
}

fn pass_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    why_this_unit: &str,
    sequential_vs_parallel_posture: &str,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    next_action: String,
) -> TaskNextLawfulReceipt {
    TaskNextLawfulReceipt {
        status: task_json_success_status().to_string(),
        active_bounded_unit,
        why_this_unit: why_this_unit.to_string(),
        sequential_vs_parallel_posture: sequential_vs_parallel_posture.to_string(),
        ready_task_candidates,
        blocker_codes: Vec::new(),
        next_actions: vec![next_action],
        source_surfaces: task_continuation_source_surfaces(),
    }
}

fn task_next_lawful_receipt(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    runtime_binding: Option<&state_store::RunGraphContinuationBinding>,
) -> TaskNextLawfulReceipt {
    let active_tasks = tasks
        .iter()
        .filter(|task| task.status == "in_progress" && task.issue_type != "epic")
        .collect::<Vec<_>>();

    if let Some(binding) = runtime_binding {
        let binding_task = tasks.iter().find(|task| task.id == binding.task_id);
        let conflicting_active = active_tasks
            .iter()
            .find(|task| task.id != binding.task_id)
            .map(|task| task.id.clone());
        if let Some(conflicting_task_id) = conflicting_active {
            return blocked_task_next_lawful_receipt(
                binding.active_bounded_unit.clone(),
                ready_task_candidates,
                "runtime_taskflow_active_conflict",
                &format!(
                    "Runtime binding points to `{}` but TaskFlow has active `{}`; reconcile or close the stale active task before continuing.",
                    binding.task_id, conflicting_task_id
                ),
            );
        }
        if continuation_binding_requires_open_task(binding) {
            let Some(task) = binding_task else {
                return blocked_task_next_lawful_receipt(
                    binding.active_bounded_unit.clone(),
                    ready_task_candidates,
                    "runtime_binding_task_missing",
                    "Refresh runtime evidence or bind the intended TaskFlow task explicitly before continuing.",
                );
            };
            if task.status == "closed" {
                return blocked_task_next_lawful_receipt(
                    binding.active_bounded_unit.clone(),
                    ready_task_candidates,
                    "runtime_binding_task_closed",
                    "Refresh continuation evidence after close/release automation before continuing.",
                );
            }
        }
        let ready_conflict = ready_task_candidates
            .iter()
            .any(|candidate| candidate.task_id != binding.task_id);
        if ready_conflict
            && !ready_task_candidates
                .iter()
                .any(|candidate| candidate.task_id == binding.task_id)
        {
            return blocked_task_next_lawful_receipt(
                binding.active_bounded_unit.clone(),
                ready_task_candidates,
                "runtime_ready_candidate_conflict",
                "Runtime binding and TaskFlow ready candidates differ; inspect `vida status --json` and bind/close the intended task explicitly.",
            );
        }
        return pass_task_next_lawful_receipt(
            binding.active_bounded_unit.clone(),
            &binding.why_this_unit,
            &binding.sequential_vs_parallel_posture,
            ready_task_candidates,
            format!(
                "Continue `{}` via the bound runtime path: {}.",
                binding.task_id, binding.primary_path
            ),
        );
    }

    match active_tasks.as_slice() {
        [active] => {
            let ready_conflict = ready_task_candidates
                .iter()
                .any(|candidate| candidate.task_id != active.id);
            if ready_conflict {
                return blocked_task_next_lawful_receipt(
                    task_continuation_active_unit(active),
                    ready_task_candidates,
                    "taskflow_active_ready_conflict",
                    "TaskFlow has one active task and different ready candidates; close/reconcile active work or bind the intended continuation explicitly.",
                );
            }
            pass_task_next_lawful_receipt(
                task_continuation_active_unit(active),
                "single TaskFlow in_progress task is the lawful continuation",
                "sequential_only_active_task",
                ready_task_candidates,
                format!("Continue active task `{}`.", active.id),
            )
        }
        [] => match ready_task_candidates.as_slice() {
            [candidate] => pass_task_next_lawful_receipt(
                serde_json::json!({
                    "task_id": candidate.task_id,
                    "title": candidate.title,
                    "status": candidate.status,
                    "issue_type": candidate.issue_type,
                }),
                "single ready TaskFlow candidate after close/release automation",
                if candidate.ready_parallel_safe {
                    "parallel_safe_single_candidate"
                } else {
                    "sequential_only_single_candidate"
                },
                ready_task_candidates.clone(),
                format!("Continue ready task `{}`.", candidate.task_id),
            ),
            [] => blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "no_ready_task_candidates",
                "Create/import the next task or refresh TaskFlow state before continuing.",
            ),
            _ => blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "ambiguous_ready_task_candidates",
                "Multiple ready tasks are available; choose and bind the intended bounded unit explicitly before implementation.",
            ),
        },
        _ => blocked_task_next_lawful_receipt(
            serde_json::Value::Null,
            ready_task_candidates,
            "multiple_active_tasks",
            "Close or reconcile extra in_progress tasks before selecting a continuation item.",
        ),
    }
}

fn dirty_paths_for_repo(repo_root: &std::path::Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(porcelain_status_path)
        .collect::<Vec<_>>())
}

fn porcelain_status_path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    if path.is_empty() {
        None
    } else {
        Some(
            path.rsplit_once(" -> ")
                .map(|(_, destination)| destination)
                .unwrap_or(path)
                .to_string(),
        )
    }
}

fn path_is_explicitly_owned(path: &str, explicit_files: &[String]) -> bool {
    explicit_files.iter().any(|explicit| {
        path == explicit
            || path
                .strip_prefix(explicit)
                .map(|suffix| suffix.starts_with('/'))
                .unwrap_or(false)
    })
}

pub(crate) async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::Help(command) => match command.topic.as_deref() {
            None | Some("task") => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some("parallelism" | "scheduling") => {
                print_taskflow_proxy_help(Some("parallelism"));
                ExitCode::SUCCESS
            }
            Some("next") => {
                print_taskflow_proxy_help(Some("next"));
                ExitCode::SUCCESS
            }
            Some("graph-summary") => {
                print_taskflow_proxy_help(Some("graph-summary"));
                ExitCode::SUCCESS
            }
            Some(
                "ready" | "deps" | "reverse-deps" | "blocked" | "children" | "reparent-children"
                | "move-children" | "tree" | "subtree" | "critical-path" | "next-display-id"
                | "create" | "ensure" | "update" | "close" | "split" | "spawn-blocker" | "list"
                | "adaptive-preview" | "show" | "import-jsonl" | "replace-jsonl" | "export-jsonl"
                | "validate-graph" | "dep" | "handoff" | "next-lawful",
            ) => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some(topic) => {
                eprintln!("Unsupported task help topic: {topic}");
                ExitCode::from(2)
            }
        },
        TaskCommand::ImportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
                    Ok(summary) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task import-jsonl")
                                .await
                        {
                            return code;
                        }
                        if command.json {
                            let mut summary_json = serde_json::json!({
                                "status": task_json_success_status(),
                                "source_path": summary.source_path,
                                "imported_count": summary.imported_count,
                                "unchanged_count": summary.unchanged_count,
                                "updated_count": summary.updated_count,
                            });
                            if let Err(error) =
                                normalize_task_json_contract_arrays(&mut summary_json)
                            {
                                eprintln!("Failed to render task import-jsonl json: {error}");
                                return ExitCode::from(1);
                            }
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&summary_json)
                                    .expect("json import summary should render")
                            );
                        } else {
                            print_surface_header(command.render, "vida task import-jsonl");
                            print_surface_line(command.render, "import", &summary.as_display());
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to import tasks from JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReplaceJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open(state_dir).await {
                Ok(store) => match store
                    .replace_with_taskflow_snapshot_file(&command.path)
                    .await
                {
                    Ok(()) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task replace-jsonl")
                                .await
                        {
                            return code;
                        }
                        let source_path = command.path.display().to_string();
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": task_json_success_status(),
                                "operation": "replace_snapshot",
                                "source_path": source_path,
                            }));
                        } else {
                            print_surface_header(command.render, "vida task replace-jsonl");
                            print_surface_line(command.render, "status", "pass");
                            print_surface_line(command.render, "operation", "replace_snapshot");
                            print_surface_line(command.render, "source path", &source_path);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to replace tasks from snapshot file: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ExportJsonl(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_read_only_task_store(state_dir).await {
                Ok(store) => match store.export_tasks_to_jsonl(&command.path).await {
                    Ok(exported_count) => {
                        print_task_export_summary(
                            command.render,
                            u64::try_from(exported_count)
                                .expect("task export count should fit u64"),
                            &command.path.display().to_string(),
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to export tasks to JSONL: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::List(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_list_snapshot_first(state_dir, command.status.as_deref(), command.all).await
            {
                Ok((tasks, metadata)) => {
                    print_task_list(
                        command.render,
                        &tasks,
                        command.summary,
                        command.json,
                        Some(&metadata),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to list tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_show_snapshot_first(state_dir, &command.task_id).await {
                Ok((task, metadata)) => {
                    print_task_show(command.render, &task, command.json, Some(&metadata));
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to show task: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::OwnedStatus(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let repo_root = project_root_for_task_state(&state_dir)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            match task_show_snapshot_first(state_dir, &command.task_id).await {
                Ok((task, _metadata)) => {
                    let dirty_files = match dirty_paths_for_repo(&repo_root) {
                        Ok(paths) => paths,
                        Err(error) => {
                            let receipt = TaskOwnedStatusReceipt {
                                status: "blocked".to_string(),
                                blocker_codes: vec!["git_status_failed".to_string()],
                                next_actions: vec![
                                    "Run the command from a git worktree or resolve git status errors before staging.".to_string(),
                                ],
                                task_id: command.task_id.clone(),
                                ownership_source: "unresolved".to_string(),
                                owned_paths: Vec::new(),
                                dirty_files: Vec::new(),
                                owned_files: Vec::new(),
                                unowned_files: Vec::new(),
                                stageable_files: Vec::new(),
                            };
                            if command.json {
                                let mut value = serde_json::to_value(&receipt)
                                    .expect("owned status receipt should serialize");
                                value["git_error"] = serde_json::json!(error);
                                crate::print_json_pretty(&value);
                            } else {
                                eprintln!("Failed to inspect git status: {error}");
                            }
                            return ExitCode::from(1);
                        }
                    };
                    let receipt = task_owned_status_receipt(
                        &task.id,
                        task.planner_metadata.owned_paths.clone(),
                        command
                            .files
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect(),
                        dirty_files,
                    );
                    if command.json {
                        crate::print_json_pretty(
                            &serde_json::to_value(&receipt)
                                .expect("owned status receipt should serialize"),
                        );
                    } else {
                        print_surface_line(command.render, "owned status", &receipt.status);
                        if !receipt.blocker_codes.is_empty() {
                            print_surface_line(
                                command.render,
                                "blockers",
                                &receipt.blocker_codes.join(", "),
                            );
                        }
                        print_surface_line(
                            command.render,
                            "stageable files",
                            &receipt.stageable_files.len().to_string(),
                        );
                    }
                    if receipt.status == "pass" {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to inspect task owned status: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Handoff(command) => match command.command {
            TaskHandoffCommand::Accept(command) => {
                let explicit_state_dir = command.state_dir.is_some();
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let (receipt_root, isolation) =
                    task_handoff_receipt_root(&state_dir, explicit_state_dir);
                let accepted_at = task_handoff_timestamp();
                let receipt_path = task_handoff_receipt_path(
                    &receipt_root,
                    &command.task_id,
                    &task_handoff_receipt_filename_timestamp(),
                );
                let mut receipt = task_handoff_accept_receipt(
                    &command,
                    &receipt_path,
                    &receipt_root,
                    isolation,
                    accepted_at,
                );
                match task_show_snapshot_first(state_dir, &command.task_id).await {
                    Ok((_task, _metadata)) => {}
                    Err(error) => {
                        receipt = blocked_task_handoff_accept_receipt(
                            &command.task_id,
                            command.agent.as_deref().unwrap_or(""),
                            "missing_task",
                            "Create or import the task before accepting delegated handoff evidence.",
                        );
                        if command.json {
                            crate::print_json_pretty(
                                &serde_json::to_value(&receipt)
                                    .expect("task handoff blocked receipt should serialize"),
                            );
                        } else {
                            eprintln!("Failed to accept task handoff: {error}");
                        }
                        return ExitCode::from(1);
                    }
                }
                if let Err((blocker_code, next_action)) =
                    validate_task_handoff_accept_receipt(&receipt)
                {
                    receipt = blocked_task_handoff_accept_receipt(
                        &command.task_id,
                        command.agent.as_deref().unwrap_or(""),
                        blocker_code,
                        next_action,
                    );
                    if command.json {
                        crate::print_json_pretty(
                            &serde_json::to_value(&receipt)
                                .expect("task handoff blocked receipt should serialize"),
                        );
                    } else {
                        eprintln!("Failed to accept task handoff: {blocker_code}");
                    }
                    return ExitCode::from(1);
                }
                if let Err(error) = persist_task_handoff_accept_receipt(&receipt, &receipt_path) {
                    let blocked = blocked_task_handoff_accept_receipt(
                        &command.task_id,
                        command.agent.as_deref().unwrap_or(""),
                        "task_handoff_receipt_write_failed",
                        "Resolve receipt directory permissions and rerun handoff acceptance.",
                    );
                    if command.json {
                        let mut value = serde_json::to_value(&blocked)
                            .expect("task handoff blocked receipt should serialize");
                        value["write_error"] = serde_json::json!(error);
                        crate::print_json_pretty(&value);
                    } else {
                        eprintln!("Failed to persist task handoff receipt: {error}");
                    }
                    return ExitCode::from(1);
                }
                if command.json {
                    crate::print_json_pretty(
                        &serde_json::to_value(&receipt)
                            .expect("task handoff receipt should serialize"),
                    );
                } else {
                    print_surface_line(command.render, "handoff", &receipt.status);
                    print_surface_line(command.render, "receipt", &receipt.receipt_path);
                }
                if receipt.status == "pass" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
        },
        TaskCommand::Progress(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.task_progress_summary(&command.task_id).await {
                    Ok(summary) => {
                        print_task_progress(command.render, &summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute task progress: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read task progress from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::task_progress_summary_from_rows(&rows, &command.task_id) {
                        Ok(summary) => {
                            print_task_progress(command.render, &summary, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to compute task progress from snapshot: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Ready(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_ready_snapshot_first(state_dir, command.scope.as_deref()).await {
                Ok((tasks, metadata)) => {
                    print_task_ready(
                        command.render,
                        command.scope.as_deref(),
                        &tasks,
                        command.json,
                        Some(&metadata),
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to compute ready tasks: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Next(command) => {
            let mut proxy_args = vec!["next".to_string()];
            if let Some(scope) = command.scope.as_deref() {
                proxy_args.push("--scope".to_string());
                proxy_args.push(scope.to_string());
            }
            if let Some(state_dir) = command.state_dir.as_ref().and_then(|path| path.to_str()) {
                proxy_args.push("--state-dir".to_string());
                proxy_args.push(state_dir.to_string());
            }
            if command.json {
                proxy_args.push("--json".to_string());
            }
            crate::taskflow_proxy::run_taskflow_next_surface(&proxy_args).await
        }
        TaskCommand::NextLawful(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => {
                    let tasks = match store.list_tasks(None, true).await {
                        Ok(tasks) => tasks,
                        Err(error) => {
                            eprintln!("Failed to list tasks for lawful continuation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let explicit_binding =
                        match store.latest_explicit_run_graph_continuation_binding().await {
                            Ok(binding) => binding,
                            Err(error) => {
                                eprintln!("Failed to read explicit continuation binding: {error}");
                                return ExitCode::from(1);
                            }
                        };
                    let latest_run_graph_status = match store.latest_run_graph_status().await {
                        Ok(status) => status,
                        Err(error) => {
                            eprintln!("Failed to read latest run graph status: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let current_binding = match latest_run_graph_status.as_ref() {
                        Some(status) => {
                            match store.run_graph_continuation_binding(&status.run_id).await {
                                Ok(binding) => binding,
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read current latest-run continuation binding: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        None => None,
                    };
                    let runtime_binding = match select_task_next_lawful_binding(
                        &tasks,
                        explicit_binding.as_ref(),
                        current_binding.as_ref(),
                    ) {
                        Ok(binding) => binding,
                        Err(receipt) => {
                            if command.json {
                                crate::print_json_pretty(&serde_json::to_value(&receipt).expect(
                                    "task next-lawful source drift receipt should serialize",
                                ));
                            } else {
                                print_surface_line(command.render, "next lawful", &receipt.status);
                                print_surface_line(
                                    command.render,
                                    "blockers",
                                    &receipt.blocker_codes.join(", "),
                                );
                            }
                            return ExitCode::from(1);
                        }
                    };
                    let runtime_binding_task_missing_in_explicit_scope = command.scope.is_some()
                        && runtime_binding
                            .map(|binding| !task_exists_for_binding(&tasks, binding))
                            .unwrap_or(false);
                    let scoped_runtime_binding = if runtime_binding_task_missing_in_explicit_scope {
                        None
                    } else {
                        runtime_binding
                    };
                    let projection = match store
                        .scheduling_projection_scoped(
                            command.scope.as_deref(),
                            scoped_runtime_binding.map(|binding| binding.task_id.as_str()),
                        )
                        .await
                    {
                        Ok(projection) => projection,
                        Err(error) => {
                            eprintln!("Failed to compute lawful continuation candidates: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let mut ready_task_candidates = projection
                        .ready
                        .iter()
                        .map(|candidate| {
                            task_continuation_candidate(
                                &candidate.task,
                                candidate.ready_parallel_safe,
                            )
                        })
                        .collect::<Vec<_>>();
                    if runtime_binding_task_missing_in_explicit_scope {
                        if let Some(current_task_id) = projection.current_task_id.as_deref() {
                            ready_task_candidates
                                .retain(|candidate| candidate.task_id == current_task_id);
                        }
                    }
                    let receipt = task_next_lawful_receipt(
                        &tasks,
                        ready_task_candidates,
                        scoped_runtime_binding,
                    );
                    if command.json {
                        crate::print_json_pretty(
                            &serde_json::to_value(&receipt)
                                .expect("task next-lawful receipt should serialize"),
                        );
                    } else {
                        print_surface_line(command.render, "next lawful", &receipt.status);
                        print_surface_line(
                            command.render,
                            "posture",
                            &receipt.sequential_vs_parallel_posture,
                        );
                        if !receipt.blocker_codes.is_empty() {
                            print_surface_line(
                                command.render,
                                "blockers",
                                &receipt.blocker_codes.join(", "),
                            );
                        }
                        if let Some(task_id) = receipt
                            .active_bounded_unit
                            .get("task_id")
                            .and_then(serde_json::Value::as_str)
                        {
                            print_surface_line(command.render, "active bounded unit", task_id);
                        }
                    }
                    if receipt.status == task_json_success_status() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::NextDisplayId(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match open_read_only_task_store(state_dir).await {
                Ok(store) => match store.list_tasks(None, true).await {
                    Ok(tasks) => match task_rows_as_values(&tasks) {
                        Ok(rows) => {
                            let payload = crate::taskflow_task_bridge::next_display_id_payload(
                                &rows,
                                &command.parent_display_id,
                            );
                            let valid = payload
                                .get("valid")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false);
                            print_task_next_display_id(command.render, &payload, command.json);
                            if valid {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to compute next display id: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to list tasks for next display id: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Create(command) => run_task_create_like(command, false).await,
        TaskCommand::Ensure(command) => run_task_create_like(command, true).await,
        TaskCommand::Update(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let notes = match resolve_optional_text_arg(
                "notes",
                command.notes.as_deref(),
                command.notes_file.as_deref(),
            ) {
                Ok(notes) => notes,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let add_labels = parse_label_values(&command.add_labels);
            let remove_labels = parse_label_values(&command.remove_labels);
            let set_labels = parse_optional_label_value(command.set_labels.as_deref());
            let execution_mode = match task_update_semantics_arg(
                command.execution_mode.as_deref(),
                command.clear_execution_mode,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let order_bucket = match task_update_semantics_arg(
                command.order_bucket.as_deref(),
                command.clear_order_bucket,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parallel_group = match task_update_semantics_arg(
                command.parallel_group.as_deref(),
                command.clear_parallel_group,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let conflict_domain = match task_update_semantics_arg(
                command.conflict_domain.as_deref(),
                command.clear_conflict_domain,
            ) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let parent_id =
                match task_update_parent_arg(command.parent_id.as_deref(), command.clear_parent_id)
                {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .update_task(state_store::UpdateTaskRequest {
                        task_id: &command.task_id,
                        status: command.status.as_deref(),
                        notes: notes.as_deref(),
                        description: command.description.as_deref(),
                        parent_id,
                        add_labels: &add_labels,
                        remove_labels: &remove_labels,
                        set_labels: set_labels.as_deref(),
                        execution_mode,
                        order_bucket,
                        parallel_group,
                        conflict_domain,
                        planner_metadata: None,
                    })
                    .await
                {
                    Ok(task) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task update").await
                        {
                            return code;
                        }
                        print_task_mutation(
                            command.render,
                            "vida task update",
                            &task,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to update task: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Split(command) => run_task_split_like(command, "vida task split").await,
        TaskCommand::SpawnBlocker(command) => {
            run_task_spawn_blocker_like(command, "vida task spawn-blocker").await
        }
        TaskCommand::AdaptivePreview(command) => run_task_adaptive_preview(command).await,
        TaskCommand::Close(command) => {
            let explicit_state_dir = command.state_dir.is_some();
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let project_root = project_root_for_task_state(&state_dir);
            let feedback_source = command.source.as_deref().unwrap_or("vida task close");
            match StateStore::open_existing(state_dir.clone()).await {
                Ok(store) => match store.close_task(&command.task_id, &command.reason).await {
                    Ok(task) => {
                        if let Err(code) =
                            refresh_task_snapshot_after_mutation(&store, "vida task close").await
                        {
                            return code;
                        }
                        if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_implementer_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                        let task_value = serde_json::to_value(&task)
                            .expect("task close payload should serialize");
                        let telemetry = task_close_host_agent_telemetry(
                            &state_dir,
                            explicit_state_dir,
                            project_root.as_deref(),
                            &task_value,
                            &command.reason,
                            feedback_source,
                        );
                        let automation = if task_close_automation_requested(&command) {
                            Some(task_close_automation_receipt(
                                &command,
                                project_root.as_deref(),
                                Some(&task),
                            ))
                        } else {
                            None
                        };
                        let automation_blocked = automation
                            .as_ref()
                            .map(|receipt| receipt.status != "pass")
                            .unwrap_or(false);
                        if command.json {
                            crate::print_json_pretty(&serde_json::json!({
                                "status": if automation_blocked { "blocked" } else { "pass" },
                                "blocker_codes": automation
                                    .as_ref()
                                    .map(|receipt| receipt.blocker_codes.clone())
                                    .unwrap_or_default(),
                                "next_actions": automation
                                    .as_ref()
                                    .map(|receipt| receipt.next_actions.clone())
                                    .unwrap_or_default(),
                                "task": task,
                                "host_agent_telemetry": telemetry,
                                "automation": automation,
                            }));
                        } else {
                            print_task_mutation(command.render, "vida task close", &task, false);
                            let telemetry_status = telemetry
                                .get("status")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown");
                            let telemetry_reason = telemetry
                                .get("reason")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("");
                            let telemetry_summary = if telemetry_reason.is_empty() {
                                telemetry_status.to_string()
                            } else {
                                format!("{telemetry_status}: {telemetry_reason}")
                            };
                            print_surface_line(
                                command.render,
                                "host agent telemetry",
                                &telemetry_summary,
                            );
                            if let Some(automation) = &automation {
                                print_surface_line(
                                    command.render,
                                    "automation",
                                    &automation.status,
                                );
                                if !automation.blocker_codes.is_empty() {
                                    print_surface_line(
                                        command.render,
                                        "automation blockers",
                                        &automation.blocker_codes.join(", "),
                                    );
                                }
                            }
                        }
                        if automation_blocked {
                            ExitCode::from(1)
                        } else {
                            ExitCode::SUCCESS
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to close task: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Deps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.task_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read task dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read task dependencies from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::task_dependencies_from_rows(&rows, &command.task_id) {
                        Ok(dependencies) => {
                            print_task_dependencies(
                                command.render,
                                "vida task deps",
                                &command.task_id,
                                &dependencies,
                                command.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to read task dependencies from snapshot: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReverseDeps(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.reverse_dependencies(&command.task_id).await {
                    Ok(dependencies) => {
                        print_task_dependencies(
                            command.render,
                            "vida task reverse-deps",
                            &command.task_id,
                            &dependencies,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to read reverse dependencies: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!("Failed to read reverse dependencies from snapshot: {snapshot_error}");
                            return ExitCode::from(1);
                        }
                    };
                    let dependencies =
                        StateStore::reverse_dependencies_from_rows(&rows, &command.task_id);
                    print_task_dependencies(
                        command.render,
                        "vida task reverse-deps",
                        &command.task_id,
                        &dependencies,
                        command.json,
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Blocked(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.blocked_tasks().await {
                    Ok(tasks) => {
                        print_blocked_tasks(command.render, &tasks, command.summary, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute blocked tasks: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read blocked tasks from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    let tasks = StateStore::blocked_tasks_from_rows(&rows);
                    print_blocked_tasks(command.render, &tasks, command.summary, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Children(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_direct_children(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to read task direct children: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir, &command.task_id).await {
                Ok(tree) => {
                    print_task_dependency_tree(command.render, &tree, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to read task dependency tree: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReparentChildren(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .reparent_children(
                        &command.from_parent_id,
                        &command.to_parent_id,
                        &command.child_ids,
                        command.dry_run,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Err(code) = refresh_task_snapshot_after_mutation(
                            &store,
                            "vida task reparent-children",
                        )
                        .await
                        {
                            return code;
                        }
                        print_task_bulk_reparent_result(command.render, &result, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to bulk-reparent children: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ValidateGraph(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.validate_task_graph().await {
                    Ok(issues) => {
                        print_task_graph_issues(command.render, &issues, command.json);
                        if issues.is_empty() {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to validate task graph: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!("Failed to read task graph snapshot: {snapshot_error}");
                            return ExitCode::from(1);
                        }
                    };
                    let issues = StateStore::validate_task_graph_rows(&rows);
                    print_task_graph_issues(command.render, &issues, command.json);
                    if issues.is_empty() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Dep(command) => match command.command {
            TaskDependencyCommand::Add(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependency(
                            &add.task_id,
                            &add.depends_on_id,
                            &add.edge_type,
                            &add.created_by,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep add")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                add.render,
                                "vida task dep add",
                                &dependency,
                                add.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Remove(remove) => {
                let state_dir = remove
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .remove_task_dependency(
                            &remove.task_id,
                            &remove.depends_on_id,
                            &remove.edge_type,
                        )
                        .await
                    {
                        Ok(dependency) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task dep remove")
                                    .await
                            {
                                return code;
                            }
                            print_task_dependency_mutation(
                                remove.render,
                                "vida task dep remove",
                                &dependency,
                                remove.json,
                            );
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to remove task dependency: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::CriticalPath(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => match store.critical_path().await {
                    Ok(path) => {
                        print_task_critical_path(command.render, &path, command.json);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to compute critical path: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!(
                                "Failed to read critical path from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::critical_path_from_rows(&rows) {
                        Ok(path) => {
                            print_task_critical_path(command.render, &path, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to compute critical path from snapshot: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_adaptive_replan_finding_preview, build_spawn_blocker_preview,
        build_split_mutation_preview, canonical_json_string_array_entries,
        load_adaptive_preview_finding_json, normalize_task_json_contract_arrays,
        parse_adaptive_replan_finding_input, parse_label_values, parse_optional_label_value,
        parse_split_child_specs, persist_task_handoff_accept_receipt,
        select_task_next_lawful_binding, task_close_automation_receipt,
        task_close_commit_file_strings, task_close_host_agent_telemetry,
        task_close_uses_isolated_state_dir, task_create_title, task_handoff_accept_receipt,
        task_handoff_project_receipt_root, task_handoff_receipt_path, task_handoff_receipt_root,
        task_json_success_status, task_next_lawful_receipt, task_owned_status_receipt,
        validate_task_handoff_accept_receipt, ADAPTIVE_REPLAN_FINDING_KINDS,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::cli;
    use crate::test_cli_support::guard_current_dir;
    use crate::test_cli_support::EnvVarGuard;
    use std::fs;
    use std::process::ExitCode;

    async fn create_task_for_test(
        store: &crate::StateStore,
        task_id: &str,
        title: &str,
        issue_type: &str,
        status: &str,
        priority: u32,
        parent_id: Option<&str>,
    ) {
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id,
                title,
                display_id: None,
                description: "",
                issue_type,
                status,
                priority,
                parent_id,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: ".",
            })
            .await
            .expect("task should create");
    }

    fn minimal_task_create_args(
        positional_title: Option<&str>,
        title: Option<&str>,
    ) -> crate::TaskCreateArgs {
        crate::TaskCreateArgs {
            task_id: "task-title-test".to_string(),
            positional_title: positional_title.map(str::to_string),
            title: title.map(str::to_string),
            issue_type: "task".to_string(),
            status: "open".to_string(),
            priority: 2,
            display_id: None,
            parent_id: None,
            parent_display_id: None,
            auto_display_from: None,
            description: String::new(),
            labels: Vec::new(),
            execution_mode: None,
            order_bucket: None,
            parallel_group: None,
            conflict_domain: None,
            state_dir: None,
            render: crate::RenderMode::Plain,
            json: false,
        }
    }

    fn owned_task_record(task_id: &str, owned_paths: Vec<&str>) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: "Owned task".to_string(),
            description: String::new(),
            status: "in_progress".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata {
                owned_paths: owned_paths.into_iter().map(str::to_string).collect(),
                acceptance_targets: Vec::new(),
                proof_targets: Vec::new(),
                risk: None,
                estimate: None,
                lane_hint: None,
            },
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn task_owned_status_splits_dirty_files_by_owned_paths() {
        let receipt = task_owned_status_receipt(
            "task-owned",
            vec!["crates/vida/src".to_string()],
            Vec::new(),
            vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "README.md".to_string(),
            ],
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "planner_metadata.owned_paths");
        assert_eq!(receipt.owned_files, vec!["crates/vida/src/task_surface.rs"]);
        assert_eq!(
            receipt.stageable_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(receipt.unowned_files, vec!["README.md"]);
        assert_eq!(receipt.blocker_codes, vec!["dirty_ownership_ambiguous"]);
    }

    #[test]
    fn task_owned_status_fails_closed_without_ownership_source() {
        let receipt = task_owned_status_receipt(
            "task-owned",
            Vec::new(),
            Vec::new(),
            vec!["crates/vida/src/task_surface.rs".to_string()],
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "missing");
        assert_eq!(receipt.blocker_codes, vec!["missing_owned_paths"]);
        assert!(receipt.stageable_files.is_empty());
    }

    #[test]
    fn task_close_stage_owned_uses_task_planner_owned_paths_with_commit_files() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let files = task_close_commit_file_strings(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: "done".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "all".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                stage_owned: true,
                commit_files: vec![std::path::PathBuf::from("README.md")],
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            Some(&task),
        );

        assert_eq!(files, vec!["README.md", "crates/vida/src"]);
    }

    #[test]
    fn task_close_stage_owned_without_commit_fails_closed() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: "done".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "all".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: false,
                stage_owned: true,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            Some(&task),
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["stage_owned_requires_commit"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["stage_owned_requires_commit"]);
        assert_eq!(git.explicit_files, vec!["crates/vida/src"]);
    }

    #[test]
    fn task_handoff_accept_receipt_records_queryable_contents() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task/handoff", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task/handoff".to_string(),
                agent: Some("worker-1".to_string()),
                files: vec![
                    std::path::PathBuf::from("crates/vida/src/task_surface.rs"),
                    std::path::PathBuf::from("crates/vida/src/task_surface.rs"),
                ],
                proofs: vec![
                    " cargo test -p vida --bin vida task_handoff ".to_string(),
                    "cargo check -p vida --bin vida".to_string(),
                ],
                status: crate::TaskHandoffStatusArg::Pass,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.task_id, "task/handoff");
        assert_eq!(receipt.agent_id, "worker-1");
        assert_eq!(
            receipt.changed_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(
            receipt.proof_commands,
            vec![
                "cargo test -p vida --bin vida task_handoff",
                "cargo check -p vida --bin vida"
            ]
        );
        assert!(receipt
            .receipt_path
            .ends_with(".vida/receipts/task-handoffs/task-handoff-123.json"));
        assert_eq!(receipt.receipt_root, receipt_root.display().to_string());
        assert_eq!(receipt.isolation, "project_state_dir");
        validate_task_handoff_accept_receipt(&receipt)
            .expect("pass handoff with agent should validate");
        persist_task_handoff_accept_receipt(&receipt, &receipt_path)
            .expect("receipt should persist");
        let persisted = fs::read_to_string(&receipt_path).expect("receipt should be readable");
        let value: serde_json::Value =
            serde_json::from_str(&persisted).expect("receipt json should parse");
        assert_eq!(value["status"], "pass");
        assert_eq!(value["task_id"], "task/handoff");
        assert_eq!(value["agent_id"], "worker-1");
        assert_eq!(
            value["changed_files"],
            serde_json::json!(["crates/vida/src/task_surface.rs"])
        );
        let overwrite_error = persist_task_handoff_accept_receipt(&receipt, &receipt_path)
            .expect_err("receipt writer should not overwrite existing receipts");
        assert!(overwrite_error.contains("without overwrite"));
    }

    #[test]
    fn blocked_task_handoff_without_detail_fails_validation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task-a", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task-a".to_string(),
                agent: Some("worker-1".to_string()),
                files: Vec::new(),
                proofs: Vec::new(),
                status: crate::TaskHandoffStatusArg::Blocked,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        let error = validate_task_handoff_accept_receipt(&receipt)
            .expect_err("blocked handoff without blocker or proof should fail closed");
        assert_eq!(error.0, "blocked_handoff_requires_detail");
    }

    #[test]
    fn task_handoff_accept_without_agent_fails_validation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let receipt_root = task_handoff_project_receipt_root(harness.path());
        let receipt_path = task_handoff_receipt_path(&receipt_root, "task-a", "123");
        let receipt = task_handoff_accept_receipt(
            &crate::TaskHandoffAcceptArgs {
                task_id: "task-a".to_string(),
                agent: None,
                files: vec![std::path::PathBuf::from("crates/vida/src/task_surface.rs")],
                proofs: vec!["cargo check -p vida --bin vida".to_string()],
                status: crate::TaskHandoffStatusArg::Pass,
                blockers: Vec::new(),
                next_actions: Vec::new(),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            &receipt_path,
            &receipt_root,
            "project_state_dir",
            "2026-04-24T00:00:00Z".to_string(),
        );

        let error =
            validate_task_handoff_accept_receipt(&receipt).expect_err("missing agent should block");
        assert_eq!(error.0, "missing_agent_id");
    }

    #[test]
    fn task_handoff_accept_isolated_state_dir_writes_receipt_under_state_dir() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path().join("project");
        fs::create_dir_all(project_root.join(".vida/receipts"))
            .expect("project receipt directory should initialize");
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker directory should initialize");
        let isolated_state_dir = harness.path().join("isolated-state");
        runtime.block_on(async {
            let store = crate::StateStore::open(isolated_state_dir.clone())
                .await
                .expect("isolated state store should open");
            create_task_for_test(
                &store,
                "task-handoff",
                "Task handoff",
                "task",
                "open",
                2,
                None,
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let (receipt_root, isolation) = task_handoff_receipt_root(&isolated_state_dir, true);
        assert_eq!(isolation, "isolated_state_dir");
        assert_eq!(receipt_root, isolated_state_dir.join("receipts"));

        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let _cwd = guard_current_dir(&project_root);
        let code = runtime.block_on(crate::run(cli(&[
            "task",
            "handoff",
            "accept",
            "task-handoff",
            "--agent",
            "worker-1",
            "--file",
            "crates/vida/src/task_surface.rs",
            "--proof",
            "cargo check -p vida --bin vida",
            "--state-dir",
            isolated_state_dir
                .to_str()
                .expect("state dir should be utf8"),
            "--json",
        ])));
        drop(_cwd);

        assert_eq!(code, ExitCode::SUCCESS);
        let project_handoff_receipts = project_root.join(".vida/receipts/task-handoffs");
        assert!(
            !project_handoff_receipts.exists(),
            "isolated handoff must not write project receipts at {}",
            project_handoff_receipts.display()
        );
        let isolated_handoff_receipts = isolated_state_dir.join("receipts/task-handoffs");
        let receipts = fs::read_dir(&isolated_handoff_receipts)
            .expect("isolated receipt directory should exist")
            .collect::<Result<Vec<_>, _>>()
            .expect("isolated receipts should list");
        assert_eq!(receipts.len(), 1);
        let receipt_text =
            fs::read_to_string(receipts[0].path()).expect("isolated receipt should read");
        let receipt: serde_json::Value =
            serde_json::from_str(&receipt_text).expect("isolated receipt should parse");
        assert_eq!(receipt["status"], "pass");
        assert_eq!(receipt["task_id"], "task-handoff");
        assert_eq!(receipt["isolation"], "isolated_state_dir");
        assert_eq!(
            receipt["receipt_root"],
            isolated_state_dir.join("receipts").display().to_string()
        );
        assert!(receipt["receipt_path"]
            .as_str()
            .expect("receipt path should be string")
            .starts_with(
                isolated_handoff_receipts
                    .to_str()
                    .expect("receipt dir should be utf8")
            ));
    }

    #[test]
    fn task_next_lawful_selects_single_ready_candidate() {
        let mut task = owned_task_record("task-ready", vec![]);
        task.status = "open".to_string();
        task.title = "Ready task".to_string();
        let ready = vec![super::task_continuation_candidate(&task, false)];

        let receipt = task_next_lawful_receipt(&[task], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-ready");
        assert_eq!(
            receipt.why_this_unit,
            "single ready TaskFlow candidate after close/release automation"
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_single_candidate"
        );
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida task next-lawful"));
    }

    #[test]
    fn task_next_lawful_blocks_multiple_ready_candidates() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            super::task_continuation_candidate(&first, false),
            super::task_continuation_candidate(&second, false),
        ];

        let receipt = task_next_lawful_receipt(&[first, second], ready, None);

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["ambiguous_ready_task_candidates"]
        );
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(receipt.ready_task_candidates.len(), 2);
    }

    #[test]
    fn task_next_lawful_blocks_runtime_taskflow_active_conflict() {
        let mut runtime_task = owned_task_record("runtime-task", vec![]);
        runtime_task.status = "open".to_string();
        let active_task = owned_task_record("active-task", vec![]);
        let ready = vec![super::task_continuation_candidate(&runtime_task, false)];
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-1".to_string(),
            task_id: "runtime-task".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "task_id": "runtime-task",
                "kind": "run_graph_task"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "runtime binding".to_string(),
            primary_path: "vida taskflow consume continue --run-id run-1 --json".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        };

        let receipt = task_next_lawful_receipt(&[runtime_task, active_task], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["runtime_taskflow_active_conflict"]
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "runtime-task");
    }

    fn test_continuation_binding(
        run_id: &str,
        task_id: &str,
        binding_source: &str,
        active_kind: &str,
    ) -> crate::state_store::RunGraphContinuationBinding {
        crate::state_store::RunGraphContinuationBinding {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": active_kind,
                "task_id": task_id,
                "run_id": run_id,
            }),
            binding_source: binding_source.to_string(),
            why_this_unit: format!("{binding_source} selects {task_id}"),
            primary_path: "vida taskflow consume continue --json".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("continue".to_string()),
            recorded_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn task_next_lawful_prefers_current_binding_over_stale_closed_explicit_binding() {
        let mut stale_task = owned_task_record("stale-task", vec![]);
        stale_task.status = "closed".to_string();
        let current_task = owned_task_record("current-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "stale-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[stale_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("stale closed explicit binding should yield to current binding")
        .expect("current binding should select");

        assert_eq!(selected.task_id, "current-task");
        assert_eq!(
            selected.binding_source,
            "consume_continue_after_downstream_chain"
        );
    }

    #[test]
    fn task_next_lawful_prefers_current_binding_over_historical_task_close_reconcile() {
        let mut closed_task = owned_task_record("closed-task", vec![]);
        closed_task.status = "closed".to_string();
        let current_task = owned_task_record("current-task", vec![]);
        let mut explicit = test_continuation_binding(
            "old-run",
            "closed-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );
        explicit.active_bounded_unit = serde_json::json!({
            "kind": "downstream_dispatch_target",
            "task_id": "closed-task",
            "run_id": "old-run",
            "dispatch_target": "closure",
        });
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[closed_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("historical task-close reconcile should yield to current latest-run binding")
        .expect("current binding should select");

        assert_eq!(selected.task_id, "current-task");
        assert_eq!(
            selected.binding_source,
            "consume_continue_after_downstream_chain"
        );
    }

    #[test]
    fn task_next_lawful_blocks_open_explicit_and_current_source_drift() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let current_task = owned_task_record("current-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect_err("open explicit/current disagreement should fail closed");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["continuation_source_drift"]);
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("consume_continue_after_downstream_chain")));
    }

    #[test]
    fn task_next_lawful_allows_downstream_dispatch_target_from_current_binding() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "downstream_dispatch_target",
        );

        let receipt = task_next_lawful_receipt(&[closed_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "closed-feature-task"
        );
        assert_eq!(
            receipt.why_this_unit,
            "consume_continue_after_downstream_chain selects closed-feature-task"
        );
    }

    #[test]
    fn task_next_lawful_command_runs_with_single_ready_task() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "task-ready", "Ready task", "task", "open", 2, None).await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let code = runtime.block_on(crate::run(cli(&[
            "task",
            "next-lawful",
            "--state-dir",
            harness.path().to_str().expect("state path should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn task_create_title_resolves_positional_or_title_option() {
        assert_eq!(
            task_create_title(&minimal_task_create_args(Some("Positional title"), None))
                .expect("positional title should resolve"),
            "Positional title"
        );
        assert_eq!(
            task_create_title(&minimal_task_create_args(None, Some("Flag title")))
                .expect("--title should resolve"),
            "Flag title"
        );
    }

    #[test]
    fn task_create_title_rejects_missing_or_duplicate_sources() {
        let missing = task_create_title(&minimal_task_create_args(None, None))
            .expect_err("missing title should fail");
        assert!(missing.contains("Missing task title"));

        let duplicate = task_create_title(&minimal_task_create_args(Some("A"), Some("B")))
            .expect_err("duplicate title sources should fail");
        assert!(duplicate.contains("only one task title source"));
    }

    #[test]
    fn task_close_feedback_skips_isolated_explicit_state_dir() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path().join("project");
        fs::create_dir_all(project_root.join(".vida/state"))
            .expect("project state directory should initialize");
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        let isolated_state_dir = harness.path().join("isolated-state");
        let task_value = serde_json::json!({
            "id": "audit-p1-task-close-state-dir-feedback-isolation",
            "status": "closed",
        });

        assert!(task_close_uses_isolated_state_dir(
            &isolated_state_dir,
            true
        ));
        let telemetry = task_close_host_agent_telemetry(
            &isolated_state_dir,
            true,
            Some(&project_root),
            &task_value,
            "closed with isolated temp state",
            "vida task close",
        );

        assert_eq!(telemetry["status"], "skipped");
        assert_eq!(telemetry["reason"], "isolated_state_dir");
        assert_eq!(
            telemetry["state_dir"],
            isolated_state_dir.display().to_string()
        );
        assert_eq!(telemetry["feedback_store"], "not_recorded");
        assert!(!project_root
            .join(crate::HOST_AGENT_OBSERVABILITY_STATE)
            .exists());
        assert!(!project_root.join(crate::WORKER_STRATEGY_STATE).exists());
    }

    #[test]
    fn task_close_feedback_keeps_project_state_dir_admissible() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path();
        fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker directory should initialize");
        fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker directory should initialize");
        let project_state_dir = project_root.join(crate::state_store::default_state_dir());

        assert!(!task_close_uses_isolated_state_dir(
            &project_state_dir,
            true
        ));
    }

    #[test]
    fn task_close_commit_automation_requires_explicit_owned_files() {
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "audit-p1-task-close-release-options".to_string(),
                reason: "close bounded task".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "all".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                stage_owned: false,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["dirty_ownership_ambiguous"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["dirty_ownership_ambiguous"]);
    }

    #[test]
    fn task_close_push_automation_requires_explicit_commit() {
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "audit-p1-task-close-release-options".to_string(),
                reason: "close bounded task".to_string(),
                source: None,
                release: false,
                install: false,
                install_target: "all".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: true,
                stage_owned: false,
                commit_files: Vec::new(),
                commit_message: None,
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["push_requires_commit"]);
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["push_requires_commit"]);
    }

    #[test]
    fn task_json_success_status_defaults_to_release_contract_vocabulary() {
        assert_eq!(task_json_success_status(), "pass");
    }

    #[test]
    fn normalize_task_json_contract_arrays_fail_closed_for_whitespace_only_entries() {
        let mut summary_json = serde_json::json!({
            "status": task_json_success_status(),
            "blocker_codes": ["   "],
            "next_actions": ["Run `vida task import-jsonl --json`"],
        });

        assert!(normalize_task_json_contract_arrays(&mut summary_json).is_err());
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["pending"])),
            Some(vec!["pending".to_string()])
        );
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }

    #[test]
    fn parse_label_values_accepts_repeated_and_comma_separated_forms() {
        let labels = parse_label_values(&[
            "alpha,beta".to_string(),
            " gamma ".to_string(),
            "delta, ,epsilon".to_string(),
        ]);
        assert_eq!(labels, vec!["alpha", "beta", "gamma", "delta", "epsilon"]);
    }

    #[test]
    fn parse_optional_label_value_returns_none_for_absent_input() {
        assert_eq!(parse_optional_label_value(None), None);
        assert_eq!(
            parse_optional_label_value(Some("alpha, beta")),
            Some(vec!["alpha".to_string(), "beta".to_string()])
        );
    }

    #[test]
    fn adaptive_replan_finding_input_accepts_supported_finding_kinds() {
        for finding_kind in ADAPTIVE_REPLAN_FINDING_KINDS {
            let parsed = parse_adaptive_replan_finding_input(&serde_json::json!({
                "finding_kind": finding_kind,
                "source_task_id": "task-a",
                "summary": "bounded finding summary",
                "evidence_refs": ["receipt-b", " receipt-a ", "receipt-a"]
            }))
            .expect("supported finding kind should parse");

            assert_eq!(parsed.schema_version, "1");
            assert_eq!(parsed.input_kind, "adaptive_replan_finding_input");
            assert_eq!(parsed.finding_kind, *finding_kind);
            assert_eq!(parsed.source_task_id, "task-a");
            assert_eq!(
                parsed.evidence_refs,
                vec!["receipt-a".to_string(), "receipt-b".to_string()]
            );
            assert_eq!(parsed.operator_truth["parsing_and_validation_only"], true);
            assert_eq!(
                parsed.operator_truth["adaptive_mutation_execution_loop_implemented"],
                false
            );
            assert_eq!(
                parsed.operator_truth["adaptive_mutation_execution_loop_truth"],
                "not_implemented_in_this_slice"
            );
            assert_eq!(
                parsed.operator_truth["valid_input_does_not_mutate_task_graph"],
                true
            );
        }
    }

    #[test]
    fn adaptive_replan_finding_input_rejects_unsupported_kind() {
        let error = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "general_comment",
            "source_task_id": "task-a",
            "summary": "not actionable"
        }))
        .expect_err("unsupported finding kind should fail closed");

        assert_eq!(error.status, "blocked");
        assert_eq!(
            error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert_eq!(error.field.as_deref(), Some("finding_kind"));
        assert!(error
            .supported_finding_kinds
            .iter()
            .any(|kind| kind == "verification_finding"));
        assert_eq!(error.operator_truth["parsing_and_validation_only"], true);
    }

    #[test]
    fn adaptive_replan_finding_input_rejects_invalid_required_fields() {
        let missing_summary = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "proof_gap",
            "source_task_id": "task-a",
            "summary": "   "
        }))
        .expect_err("blank summary should fail closed");
        assert_eq!(missing_summary.field.as_deref(), Some("summary"));
        assert!(missing_summary.reason.contains("non-empty string"));

        let invalid_evidence = parse_adaptive_replan_finding_input(&serde_json::json!({
            "finding_kind": "oversized_task",
            "source_task_id": "task-a",
            "summary": "task is too broad",
            "evidence_refs": ["ok", ""]
        }))
        .expect_err("blank evidence ref should fail closed");
        assert_eq!(invalid_evidence.field.as_deref(), Some("evidence_refs"));
        assert!(invalid_evidence.reason.contains("entries"));
    }

    #[test]
    fn adaptive_replan_finding_preview_maps_supported_kinds_without_mutation() {
        let cases = [
            (
                "verification_finding",
                "blocker_resolution",
                "spawn_blocker_task",
            ),
            ("proof_gap", "blocker_resolution", "spawn_blocker_task"),
            ("scope_drift", "scope_replan", "replan_scope_review"),
            ("oversized_task", "task_decomposition", "split_task"),
        ];

        for (finding_kind, expected_category, expected_kind) in cases {
            let preview = build_adaptive_replan_finding_preview(
                &serde_json::json!({
                    "finding_kind": finding_kind,
                    "source_task_id": "task-a",
                    "summary": "bounded adaptive replanner input",
                    "evidence_refs": ["receipt-a", "receipt-a", "receipt-b"]
                }),
                "vida task adaptive-preview",
            )
            .expect("supported finding kind should preview");

            assert_eq!(preview.status, task_json_success_status());
            assert_eq!(preview.planned_mutation_category, expected_category);
            assert_eq!(preview.planned_mutation_kind, expected_kind);
            assert_eq!(preview.source_task_id, "task-a");
            assert!(preview.dry_run);
            assert!(!preview.applied);
            assert_eq!(
                preview.finding.evidence_refs,
                vec!["receipt-a", "receipt-b"]
            );
            assert_eq!(preview.operator_truth["graph_state_opened"], false);
            assert_eq!(preview.operator_truth["graph_state_mutated"], false);
            assert_eq!(
                preview.operator_truth["adaptive_mutation_execution_loop_implemented"],
                false
            );
            assert_eq!(
                preview.preview_receipt.receipt_kind,
                "adaptive_replan_finding_preview_receipt"
            );
            assert_eq!(preview.preview_receipt.schema_version, "1");
            assert_eq!(
                preview.preview_receipt.receipt_id,
                format!(
                    "adaptive-replan-preview:task-a:{finding_kind}:{expected_category}:{expected_kind}:evidence=receipt-a+receipt-b"
                )
            );
            assert_eq!(preview.preview_receipt.source_task_id, "task-a");
            assert_eq!(preview.preview_receipt.finding_kind, finding_kind);
            assert_eq!(
                preview.preview_receipt.planned_mutation_category,
                expected_category
            );
            assert_eq!(preview.preview_receipt.planned_mutation_kind, expected_kind);
            assert!(preview.preview_receipt.dry_run);
            assert!(!preview.preview_receipt.applied);
            assert!(!preview.preview_receipt.graph_state_opened);
            assert!(!preview.preview_receipt.graph_state_mutated);
            assert_eq!(
                preview.preview_receipt.operator_truth["preview_receipt_emitted"],
                true
            );
        }
    }

    #[test]
    fn adaptive_replan_finding_preview_receipt_is_stable_without_evidence() {
        let preview = build_adaptive_replan_finding_preview(
            &serde_json::json!({
                "finding_kind": "oversized_task",
                "source_task_id": "task-b",
                "summary": "task is too broad"
            }),
            "vida task adaptive-preview",
        )
        .expect("valid finding should preview");

        assert_eq!(
            preview.preview_receipt.receipt_id,
            "adaptive-replan-preview:task-b:oversized_task:task_decomposition:split_task:evidence=none"
        );
        assert_eq!(
            preview.preview_receipt.surface,
            "vida task adaptive-preview"
        );
        assert_eq!(preview.preview_receipt.schema_version, "1");
        assert_eq!(preview.preview_receipt.planned_mutation_kind, "split_task");
        assert_eq!(
            preview.preview_receipt.planned_mutation_category,
            "task_decomposition"
        );
        assert!(!preview.preview_receipt.graph_state_mutated);
    }

    #[test]
    fn adaptive_replan_finding_preview_rejects_invalid_input() {
        let error = build_adaptive_replan_finding_preview(
            &serde_json::json!({
                "finding_kind": "general_comment",
                "source_task_id": "task-a",
                "summary": "not actionable"
            }),
            "vida task adaptive-preview",
        )
        .expect_err("unsupported finding kind should fail closed");

        assert_eq!(error.status, "blocked");
        assert_eq!(error.field.as_deref(), Some("finding_kind"));
        assert_eq!(
            error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
    }

    #[test]
    fn task_adaptive_preview_command_accepts_inline_json_without_state_store() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: Some(
                        serde_json::json!({
                            "finding_kind": "oversized_task",
                            "source_task_id": "task-a",
                            "summary": "task is too broad"
                        })
                        .to_string(),
                    ),
                    finding_file: None,
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn task_adaptive_preview_command_accepts_finding_file_without_state_store() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let finding_path = harness.path().join("adaptive-finding.json");
        fs::write(
            &finding_path,
            serde_json::json!({
                "finding_kind": "proof_gap",
                "source_task_id": "task-a",
                "summary": "proof artifact missing",
                "evidence_refs": ["receipt-b", "receipt-a"]
            })
            .to_string(),
        )
        .expect("finding file should write");

        let loaded = load_adaptive_preview_finding_json(None, Some(finding_path.as_path()))
            .expect("finding file input should load");
        let preview = build_adaptive_replan_finding_preview(&loaded, "vida task adaptive-preview")
            .expect("finding file input should preview");
        assert_eq!(preview.planned_mutation_category, "blocker_resolution");
        assert_eq!(preview.planned_mutation_kind, "spawn_blocker_task");
        assert_eq!(
            preview.preview_receipt.receipt_id,
            "adaptive-replan-preview:task-a:proof_gap:blocker_resolution:spawn_blocker_task:evidence=receipt-a+receipt-b"
        );
        assert_eq!(preview.operator_truth["preview_receipt_emitted"], true);
        assert_eq!(preview.operator_truth["graph_state_mutated"], false);

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: None,
                    finding_file: Some(finding_path),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn adaptive_preview_finding_file_input_fails_closed_for_missing_or_invalid_file() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let missing_path = harness.path().join("missing-finding.json");
        let missing_error = load_adaptive_preview_finding_json(None, Some(missing_path.as_path()))
            .expect_err("missing finding file should fail closed");
        assert_eq!(missing_error.status, "blocked");
        assert_eq!(missing_error.field.as_deref(), Some("finding_file"));
        assert_eq!(
            missing_error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert_eq!(
            missing_error.operator_truth["valid_input_does_not_mutate_task_graph"],
            true
        );

        let invalid_path = harness.path().join("invalid-finding.json");
        fs::write(&invalid_path, "{not-json").expect("invalid finding file should write");
        let invalid_error = load_adaptive_preview_finding_json(None, Some(invalid_path.as_path()))
            .expect_err("invalid finding file should fail closed");
        assert_eq!(invalid_error.status, "blocked");
        assert_eq!(invalid_error.field.as_deref(), Some("finding_file"));
        assert_eq!(
            invalid_error.blocker_codes,
            vec!["invalid_adaptive_replan_finding_input".to_string()]
        );
        assert!(invalid_error.reason.contains("valid JSON"));

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::AdaptivePreview(crate::TaskAdaptivePreviewArgs {
                    finding_json: None,
                    finding_file: Some(missing_path),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(2)
        );
    }

    #[test]
    #[ignore = "covered by binary integration smoke; in-process sequential SurrealKv opens keep the lock longer than this unit test assumes"]
    fn task_command_round_trip_succeeds() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let jsonl_path = harness.path().join("issues.jsonl");
        fs::write(
            &jsonl_path,
            concat!(
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write sample task jsonl");

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "import-jsonl",
                    jsonl_path.to_str().expect("jsonl path should render"),
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "list",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );

        assert_eq!(
            tokio::runtime::Runtime::new()
                .expect("tokio runtime should initialize")
                .block_on(crate::run(cli(&[
                    "task",
                    "ready",
                    "--state-dir",
                    harness.path().to_str().expect("state path should render"),
                    "--json"
                ]))),
            std::process::ExitCode::SUCCESS
        );
    }

    #[test]
    fn task_split_command_creates_children_and_blocks_source_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "dep-task", "Dependency", "task", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                None,
            )
            .await;
            store
                .add_task_dependency("source-task", "dep-task", "depends-on", "test")
                .await
                .expect("dependency should create");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Split(crate::TaskSplitArgs {
                    task_id: "source-task".to_string(),
                    children: vec![
                        "source-task-a:First slice".to_string(),
                        "source-task-b:Second slice".to_string(),
                    ],
                    reason: "oversized task".to_string(),
                    dry_run: false,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            assert!(source.dependencies.iter().any(|dependency| {
                dependency.issue_id == "source-task"
                    && dependency.depends_on_id == "source-task-b"
                    && dependency.edge_type == "depends-on"
            }));

            let first_child = store
                .show_task("source-task-a")
                .await
                .expect("first split child should load");
            assert_eq!(
                first_child.description,
                "Split from `source-task`: oversized task"
            );
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task" && dependency.edge_type == "parent-child"
            }));
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "dep-task" && dependency.edge_type == "depends-on"
            }));

            let second_child = store
                .show_task("source-task-b")
                .await
                .expect("second split child should load");
            assert!(second_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task-a" && dependency.edge_type == "depends-on"
            }));
        });
    }

    #[test]
    fn task_spawn_blocker_command_creates_blocker_and_links_source() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "epic-root", "Epic", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "in_progress",
                2,
                Some("epic-root"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::SpawnBlocker(crate::TaskSpawnBlockerArgs {
                    task_id: "source-task".to_string(),
                    blocker_task_id: "blocker-task".to_string(),
                    title: "Blocker title".to_string(),
                    reason: "new dependency discovered".to_string(),
                    description: None,
                    issue_type: "task".to_string(),
                    status: "open".to_string(),
                    priority: None,
                    labels: Vec::new(),
                    dry_run: false,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            assert!(source.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "blocker-task" && dependency.edge_type == "blocks"
            }));

            let blocker = store
                .show_task("blocker-task")
                .await
                .expect("blocker task should load");
            assert_eq!(blocker.priority, 2);
            assert_eq!(
                blocker.description,
                "Blocker for `source-task`: new dependency discovered"
            );
            assert!(blocker.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "epic-root" && dependency.edge_type == "parent-child"
            }));
        });
    }

    #[test]
    fn split_preview_includes_first_class_graph_mutation_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "dep-task", "Dependency", "task", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                None,
            )
            .await;
            store
                .add_task_dependency("source-task", "dep-task", "depends-on", "test")
                .await
                .expect("dependency should create");
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            let rows = store.all_tasks().await.expect("task rows should load");
            let child_specs = parse_split_child_specs(&[
                "source-task-a:First slice".to_string(),
                "source-task-b:Second slice".to_string(),
            ])
            .expect("child specs should parse");

            let (result, _simulated_rows) = build_split_mutation_preview(
                &rows,
                &source,
                &child_specs,
                "oversized task",
                "vida task split",
                false,
            )
            .expect("split preview should build");

            let receipt = &result.graph_mutation_receipt;
            assert_eq!(receipt.receipt_kind, "task_graph_mutation_receipt");
            assert_eq!(receipt.schema_version, "1");
            assert_eq!(receipt.mutation_kind, "split_task");
            assert_eq!(receipt.source_task_id, "source-task");
            assert_eq!(receipt.dry_run, false);
            assert_eq!(receipt.applied, true);
            assert_eq!(receipt.before_validation.status, "pass");
            assert_eq!(receipt.after_validation.status, "pass");
            assert_eq!(receipt.before_task_count, rows.len());
            assert_eq!(receipt.after_task_count, rows.len() + 2);
            assert_eq!(
                receipt.planned_task_ids,
                vec!["source-task-a".to_string(), "source-task-b".to_string()]
            );
            assert_eq!(
                receipt.operator_truth["adaptive_replanner_loop_implemented"],
                false
            );
            assert_eq!(
                receipt.operator_truth["adaptive_replanner_loop_truth"],
                "not_implemented_in_this_slice"
            );
        });
    }

    #[test]
    fn spawn_blocker_preview_receipt_records_dry_run_truth() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                None,
            )
            .await;
            let source = store
                .show_task("source-task")
                .await
                .expect("source task should load");
            let rows = store.all_tasks().await.expect("task rows should load");
            let command = crate::TaskSpawnBlockerArgs {
                task_id: "source-task".to_string(),
                blocker_task_id: "blocker-task".to_string(),
                title: "Blocker title".to_string(),
                reason: "new dependency discovered".to_string(),
                description: None,
                issue_type: "task".to_string(),
                status: "open".to_string(),
                priority: None,
                labels: Vec::new(),
                dry_run: true,
                state_dir: Some(harness.path().to_path_buf()),
                render: crate::RenderMode::Plain,
                json: true,
            };

            let (result, _simulated_rows) =
                build_spawn_blocker_preview(&rows, &source, &command, "vida task spawn-blocker")
                    .expect("spawn blocker preview should build");

            let receipt = &result.graph_mutation_receipt;
            assert_eq!(receipt.receipt_kind, "task_graph_mutation_receipt");
            assert_eq!(receipt.mutation_kind, "spawn_blocker_task");
            assert_eq!(receipt.dry_run, true);
            assert_eq!(receipt.applied, false);
            assert_eq!(receipt.before_validation.status, "pass");
            assert_eq!(receipt.after_validation.status, "pass");
            assert_eq!(receipt.before_task_count, rows.len());
            assert_eq!(receipt.after_task_count, rows.len() + 1);
            assert_eq!(receipt.planned_task_ids, vec!["blocker-task".to_string()]);
            assert_eq!(
                receipt.planned_dependency_edges[0].reason,
                "spawn_blocker_dependency"
            );
            assert_eq!(
                receipt.operator_truth["records_before_after_validation"],
                true
            );
        });
    }

    #[test]
    fn taskflow_replan_split_defaults_to_dry_run() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                None,
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(crate::taskflow_proxy::run_taskflow_proxy(
                crate::ProxyArgs {
                    args: vec![
                        "replan".to_string(),
                        "split".to_string(),
                        "source-task".to_string(),
                        "--child".to_string(),
                        "source-task-a:First slice".to_string(),
                        "--child".to_string(),
                        "source-task-b:Second slice".to_string(),
                        "--reason".to_string(),
                        "oversized task".to_string(),
                        "--state-dir".to_string(),
                        harness.path().display().to_string(),
                        "--json".to_string(),
                    ],
                }
            )),
            ExitCode::SUCCESS
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            assert!(matches!(
                store.show_task("source-task-a").await,
                Err(crate::state_store::StateStoreError::MissingTask { .. })
            ));
            assert!(matches!(
                store.show_task("source-task-b").await,
                Err(crate::state_store::StateStoreError::MissingTask { .. })
            ));
        });
    }
}
