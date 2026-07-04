use super::*;
use crate::contract_profile_adapter::render_operator_contract_envelope;
use crate::task_cli_render::{
    print_task_bulk_reparent_result, print_task_closeout, print_task_defect_batch_rehome_result,
    print_task_dependency_bulk_add_result, print_task_dependency_bulk_add_result_for_surface,
    print_task_direct_children, print_task_show_missing, print_task_update_graph_blocked,
    task_closeout_payload, task_read_metadata_value, task_ready_payload, task_show_payload,
};
use crate::taskflow_proxy::paths_intersect;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use taskflow_core::task::block::{append_task_block_note, normalize_task_block_list};
use taskflow_core::task::dependencies::{
    parse_task_dependency_bulk_edges, task_dependency_bulk_edge_lines, TaskDependencyBulkEdge,
};
use taskflow_core::task::import_export::{
    task_import_jsonl_success_fields, task_replace_jsonl_success_fields, TaskImportJsonlSummary,
    TaskReplaceJsonlSummary,
};
use taskflow_core::task::progress::{
    parse_task_progress_basis,
    task_progress_summary_from_rows as core_task_progress_summary_from_rows, TaskProgressRow,
    TaskProgressSummary as CoreTaskProgressSummary,
};
use taskflow_core::task::verify::{
    all_structured_task_proof_targets_satisfied, append_task_browser_proof_note,
    append_task_proof_evidence_note, append_task_verify_note, canonical_task_proof_result,
    normalized_task_verify_evidence, structured_task_proof_evidence_match,
    task_browser_proof_target, task_reports_runtime_proof_blocker, task_verify_labels,
    TaskBrowserProofArtifact,
};

#[derive(Debug, Clone, serde::Serialize)]
struct TaskReplaceJsonlContinuationSummary {
    status: String,
    run_id: Option<String>,
    task_id: Option<String>,
    binding_source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
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

    fn fresh_snapshot(path: &std::path::Path) -> Self {
        Self {
            mode: "fresh_snapshot",
            degraded: false,
            snapshot_path: Some(path.display().to_string()),
            detail: "served from canonical task snapshot evidence with freshness metadata",
        }
    }

    fn fresh_snapshot_live_divergence(path: &std::path::Path) -> Self {
        Self {
            mode: "fresh_snapshot_live_divergence",
            degraded: true,
            snapshot_path: Some(path.display().to_string()),
            detail: "served from canonical task snapshot evidence because authoritative live task store is missing snapshot rows",
        }
    }
}

const TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT: usize = 25;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskCloseEpicProgressSummary {
    closed_task_id: String,
    epic_count: usize,
    reported_epic_count: usize,
    omitted_epic_count: usize,
    scope: String,
    epics: Vec<TaskCloseEpicProgressRow>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskCloseEpicProgressRow {
    epic_id: String,
    epic_title: String,
    epic_status: String,
    epic_priority: u32,
    closed_count: usize,
    total_count: usize,
    percent_closed: f64,
    child_task_count: usize,
    reported_child_task_count: usize,
    child_task_report_limit: usize,
    truncated_child_tasks: bool,
    tasks: Vec<TaskCloseEpicProgressTaskRow>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskCloseEpicProgressTaskRow {
    task_id: String,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
    blocker_state: String,
    blockers: Vec<TaskCloseEpicProgressBlocker>,
    next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskCloseEpicProgressBlocker {
    task_id: String,
    status: String,
    title: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct TaskCloseoutTempScan {
    pub(crate) enabled: bool,
    pub(crate) status: String,
    pub(crate) tracked_match_count: usize,
    pub(crate) tracked_matches: Vec<String>,
    pub(crate) command: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub(crate) struct TaskCloseoutSummary {
    pub(crate) task_id: String,
    pub(crate) status: String,
    pub(crate) basis: String,
    pub(crate) proof: serde_json::Value,
    pub(crate) closure: serde_json::Value,
    pub(crate) graph: serde_json::Value,
    pub(crate) progress: serde_json::Value,
    pub(crate) temp_scan: TaskCloseoutTempScan,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskEpicReconcileReceipt {
    surface: &'static str,
    status: String,
    scope: String,
    progress_basis: String,
    dry_run: bool,
    close_if_complete: bool,
    inspected_epic_count: usize,
    closed_epics: Vec<TaskEpicReconcileClosedRow>,
    blocked_epics: Vec<TaskEpicReconcileBlockedRow>,
    missing_children: Vec<String>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskEpicReconcileClosedRow {
    epic_id: String,
    child_count: usize,
    descendant_count: usize,
    progress_basis: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskEpicReconcileBlockedRow {
    epic_id: String,
    child_count: usize,
    descendant_count: usize,
    open_child_count: usize,
    in_progress_child_count: usize,
    open_descendant_count: usize,
    in_progress_descendant_count: usize,
    progress_basis: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskEpicProgressSummary {
    epic_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    total_descendant_count: usize,
    total_open_descendant_count: usize,
    total_in_progress_descendant_count: usize,
    total_closed_descendant_count: usize,
    percent_closed: f64,
    include_closed_epics: bool,
    progress_basis: String,
    epic_filter: Option<String>,
    epics: Vec<TaskEpicProgressRow>,
    read_metadata: TaskReadMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskEpicProgressRow {
    epic_id: String,
    epic_title: String,
    epic_status: String,
    epic_priority: u32,
    total_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    percent_complete: f64,
    direct_child_count: usize,
    nested_epic_count: usize,
    closure_candidate: bool,
    closure_candidate_state: String,
    recommended_next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskResetReceipt {
    surface: &'static str,
    status: String,
    task_id: String,
    dry_run: bool,
    include_steps: bool,
    inspected_count: usize,
    reset_count: usize,
    skipped_count: usize,
    reset_tasks: Vec<TaskResetTaskRow>,
    skipped_tasks: Vec<TaskResetTaskRow>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskResetTaskRow {
    task_id: String,
    title: String,
    issue_type: String,
    previous_status: String,
    next_status: String,
}

const EPIC_RECONCILE_PROGRESS_BASIS: &str = "descendants_excluding_root";

async fn reconcile_epics_from_descendant_progress(
    store: &StateStore,
    close_if_complete: bool,
    dry_run: bool,
) -> Result<TaskEpicReconcileReceipt, StateStoreError> {
    let tasks = store.all_tasks().await?;
    let progress_precompute = TaskProgressPrecompute::new(&tasks);
    let mut closed_epics = Vec::new();
    let mut blocked_epics = Vec::new();
    let mut inspected_epic_count = 0usize;
    let mut mutated_epics = BTreeSet::<String>::new();
    let mut epics = tasks
        .iter()
        .filter(|task| crate::state_store::work_item_is_program_container(&task.issue_type))
        .filter(|task| !StateStore::task_status_is_closed_like(&task.status))
        .cloned()
        .collect::<Vec<_>>();
    epics.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then(left.id.cmp(&right.id))
    });

    for epic in epics {
        inspected_epic_count += 1;
        if mutated_epics.contains(&epic.id) {
            continue;
        }

        let progress = task_progress_summary_for_basis_with_precompute(
            &tasks,
            &progress_precompute,
            &epic.id,
            EPIC_RECONCILE_PROGRESS_BASIS,
        )?;

        let child_indexes = progress_precompute
            .child_indexes_by_parent
            .get(epic.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let child_count = child_indexes.len();
        if child_count == 0 {
            blocked_epics.push(TaskEpicReconcileBlockedRow {
                epic_id: epic.id,
                child_count,
                descendant_count: progress.descendant_count,
                open_child_count: 0,
                in_progress_child_count: 0,
                open_descendant_count: progress.open_count,
                in_progress_descendant_count: progress.in_progress_count,
                progress_basis: progress.progress_basis,
                reason: "no_direct_children".to_string(),
            });
            continue;
        }

        let open_child_count = child_indexes
            .iter()
            .filter(|&&child_index| tasks[child_index].status.as_str() == "open")
            .count();
        let in_progress_child_count = child_indexes
            .iter()
            .filter(|&&child_index| tasks[child_index].status.as_str() == "in_progress")
            .count();
        let all_children_closed = child_indexes
            .iter()
            .all(|&child_index| StateStore::task_status_is_closed_like(&tasks[child_index].status));
        if !progress.closure_candidate {
            let reason = if !all_children_closed {
                "active_descendants_remaining".to_string()
            } else {
                progress.closure_candidate_state.clone()
            };
            blocked_epics.push(TaskEpicReconcileBlockedRow {
                epic_id: epic.id,
                child_count,
                descendant_count: progress.descendant_count,
                open_child_count,
                in_progress_child_count,
                open_descendant_count: progress.open_count,
                in_progress_descendant_count: progress.in_progress_count,
                progress_basis: progress.progress_basis,
                reason,
            });
            continue;
        }

        if close_if_complete && !dry_run {
            match store
                .close_task(
                    &epic.id,
                    "all descendant tasks closed by epic auto-reconcile",
                )
                .await
            {
                Ok(closed) => {
                    mutated_epics.insert(closed.id.clone());
                    closed_epics.push(TaskEpicReconcileClosedRow {
                        epic_id: closed.id,
                        child_count,
                        descendant_count: progress.descendant_count,
                        progress_basis: progress.progress_basis,
                        reason: "all_descendants_closed".to_string(),
                    });
                }
                Err(error) => blocked_epics.push(TaskEpicReconcileBlockedRow {
                    epic_id: epic.id,
                    child_count,
                    descendant_count: progress.descendant_count,
                    open_child_count,
                    in_progress_child_count,
                    open_descendant_count: progress.open_count,
                    in_progress_descendant_count: progress.in_progress_count,
                    progress_basis: progress.progress_basis,
                    reason: format!("close_failed:{error}"),
                }),
            }
        } else {
            closed_epics.push(TaskEpicReconcileClosedRow {
                epic_id: epic.id,
                child_count,
                descendant_count: progress.descendant_count,
                progress_basis: progress.progress_basis,
                reason: "eligible_all_descendants_closed".to_string(),
            });
        }
    }

    let next_actions = if close_if_complete || closed_epics.is_empty() {
        Vec::new()
    } else {
        vec![
            "Run vida task reconcile --epics --close-if-complete to close eligible epics."
                .to_string(),
        ]
    };

    Ok(TaskEpicReconcileReceipt {
        surface: "vida task reconcile",
        status: "pass".to_string(),
        scope: "epics".to_string(),
        progress_basis: EPIC_RECONCILE_PROGRESS_BASIS.to_string(),
        dry_run,
        close_if_complete,
        inspected_epic_count,
        closed_epics,
        blocked_epics,
        missing_children: Vec::new(),
        blocker_codes: Vec::new(),
        next_actions,
    })
}

const TASK_PRUNE_CLOSED_EPICS_SURFACE: &str = "vida task prune-closed-epics";

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskPruneClosedEpicsReceipt {
    surface: &'static str,
    status: String,
    dry_run: bool,
    state_dir: String,
    archive_path: Option<String>,
    inspected_task_count: usize,
    candidate_count: usize,
    archived_count: usize,
    pruned_count: usize,
    protected_count: usize,
    candidates: Vec<TaskPruneClosedEpicsRow>,
    protected: Vec<TaskPruneProtectedRow>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    graph_issues: Vec<state_store::TaskGraphIssue>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskPruneClosedEpicsRow {
    task_id: String,
    title: String,
    status: String,
    issue_type: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskPruneProtectedRow {
    task_id: String,
    title: String,
    status: String,
    issue_type: String,
    reason: String,
    blocking_task_ids: Vec<String>,
    runtime_refs: Vec<String>,
}

struct TaskPruneClosedEpicsPlan {
    receipt: TaskPruneClosedEpicsReceipt,
    candidate_ids: BTreeSet<String>,
    archive_tasks: Vec<state_store::TaskRecord>,
}

fn task_prune_status_is_live(status: &str) -> bool {
    matches!(status, "open" | "in_progress")
}

fn task_prune_parent_children(rows: &[state_store::TaskRecord]) -> BTreeMap<String, Vec<String>> {
    let mut children = BTreeMap::<String, Vec<String>>::new();
    for task in rows {
        if let Some(parent_id) = StateStore::parent_id_for_task(task) {
            children.entry(parent_id).or_default().push(task.id.clone());
        }
    }
    children
}

fn collect_task_prune_descendants(
    task_id: &str,
    children_by_parent: &BTreeMap<String, Vec<String>>,
    descendants: &mut BTreeSet<String>,
) {
    let Some(children) = children_by_parent.get(task_id) else {
        return;
    };
    for child_id in children {
        if descendants.insert(child_id.clone()) {
            collect_task_prune_descendants(child_id, children_by_parent, descendants);
        }
    }
}

fn retained_task_refs_to_prune_subtree(
    rows: &[state_store::TaskRecord],
    subtree_ids: &BTreeSet<String>,
) -> Vec<String> {
    let mut referrers = BTreeSet::<String>::new();
    for task in rows {
        if subtree_ids.contains(&task.id) {
            continue;
        }
        if task
            .dependencies
            .iter()
            .any(|dependency| subtree_ids.contains(&dependency.depends_on_id))
        {
            referrers.insert(task.id.clone());
        }
    }
    referrers.into_iter().collect()
}

fn runtime_refs_to_prune_subtree(
    runtime_refs_by_task: &BTreeMap<String, Vec<String>>,
    subtree_ids: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let mut blocking_task_ids = Vec::new();
    let mut runtime_refs = Vec::new();
    for task_id in subtree_ids {
        let Some(refs) = runtime_refs_by_task.get(task_id) else {
            continue;
        };
        if refs.is_empty() {
            continue;
        }
        blocking_task_ids.push(task_id.clone());
        for runtime_ref in refs {
            runtime_refs.push(format!("{task_id}:{runtime_ref}"));
        }
    }
    runtime_refs.sort();
    runtime_refs.dedup();
    (blocking_task_ids, runtime_refs)
}

fn task_prune_row(
    task: &state_store::TaskRecord,
    reason: impl Into<String>,
) -> TaskPruneClosedEpicsRow {
    TaskPruneClosedEpicsRow {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        reason: reason.into(),
    }
}

fn task_prune_protected_row(
    task: &state_store::TaskRecord,
    reason: impl Into<String>,
    blocking_task_ids: Vec<String>,
    runtime_refs: Vec<String>,
) -> TaskPruneProtectedRow {
    TaskPruneProtectedRow {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        reason: reason.into(),
        blocking_task_ids,
        runtime_refs,
    }
}

fn build_task_prune_closed_epics_plan(
    rows: &[state_store::TaskRecord],
    runtime_refs_by_task: &BTreeMap<String, Vec<String>>,
    dry_run: bool,
    state_dir: &std::path::Path,
    archive_path: Option<&std::path::Path>,
) -> TaskPruneClosedEpicsPlan {
    let children_by_parent = task_prune_parent_children(rows);
    let by_id = rows
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    let mut candidate_ids = BTreeSet::<String>::new();
    let mut candidate_reasons = BTreeMap::<String, String>::new();
    let mut protected = Vec::<TaskPruneProtectedRow>::new();

    for task in rows
        .iter()
        .filter(|task| crate::state_store::work_item_is_program_container(&task.issue_type))
    {
        if task_prune_status_is_live(&task.status) {
            protected.push(task_prune_protected_row(
                task,
                "open_or_in_progress_container",
                Vec::new(),
                Vec::new(),
            ));
            continue;
        }
        if !StateStore::task_status_is_closed_like(&task.status) {
            continue;
        }

        let mut subtree_ids = BTreeSet::<String>::new();
        subtree_ids.insert(task.id.clone());
        collect_task_prune_descendants(&task.id, &children_by_parent, &mut subtree_ids);

        let non_closed_descendants = subtree_ids
            .iter()
            .filter(|task_id| task_id.as_str() != task.id.as_str())
            .filter_map(|task_id| by_id.get(task_id.as_str()).copied())
            .filter(|descendant| !StateStore::task_status_is_closed_like(&descendant.status))
            .map(|descendant| descendant.id.clone())
            .collect::<Vec<_>>();
        if !non_closed_descendants.is_empty() {
            protected.push(task_prune_protected_row(
                task,
                "non_closed_descendant",
                non_closed_descendants,
                Vec::new(),
            ));
            continue;
        }

        let retained_refs = retained_task_refs_to_prune_subtree(rows, &subtree_ids);
        if !retained_refs.is_empty() {
            protected.push(task_prune_protected_row(
                task,
                "referenced_by_retained_task",
                retained_refs,
                Vec::new(),
            ));
            continue;
        }

        let (runtime_linked_task_ids, runtime_refs) =
            runtime_refs_to_prune_subtree(runtime_refs_by_task, &subtree_ids);
        if !runtime_linked_task_ids.is_empty() {
            protected.push(task_prune_protected_row(
                task,
                "runtime_linked_task",
                runtime_linked_task_ids,
                runtime_refs,
            ));
            continue;
        }

        let root_reason = if children_by_parent
            .get(&task.id)
            .map_or(true, |children| children.is_empty())
        {
            "closed_empty_container"
        } else {
            "closed_epic_subtree"
        };
        for task_id in subtree_ids {
            let reason = if task_id == task.id {
                root_reason
            } else {
                "closed_descendant_of_pruned_epic"
            };
            candidate_ids.insert(task_id.clone());
            candidate_reasons
                .entry(task_id)
                .or_insert(reason.to_string());
        }
    }

    let mut archive_tasks = rows
        .iter()
        .filter(|task| candidate_ids.contains(&task.id))
        .cloned()
        .collect::<Vec<_>>();
    archive_tasks.sort_by(|left, right| left.id.cmp(&right.id));
    let candidates = archive_tasks
        .iter()
        .map(|task| {
            task_prune_row(
                task,
                candidate_reasons
                    .get(&task.id)
                    .cloned()
                    .unwrap_or_else(|| "closed_epic_task".to_string()),
            )
        })
        .collect::<Vec<_>>();
    let remaining_rows = rows
        .iter()
        .filter(|task| !candidate_ids.contains(&task.id))
        .cloned()
        .collect::<Vec<_>>();
    let graph_issues =
        StateStore::validate_task_graph_rows_for_mutation(rows, &remaining_rows, &candidate_ids);
    let blocked = !graph_issues.is_empty();
    let blocker_codes = if blocked {
        vec!["task_prune_graph_integrity_blocked".to_string()]
    } else {
        Vec::new()
    };
    let next_actions = if blocked {
        vec![
            "Run vida task validate-graph and resolve graph issues before pruning task rows."
                .to_string(),
        ]
    } else if dry_run && !candidate_ids.is_empty() {
        vec![
            "Run vida task prune-closed-epics --apply to archive and prune eligible task rows."
                .to_string(),
        ]
    } else {
        Vec::new()
    };

    let receipt = TaskPruneClosedEpicsReceipt {
        surface: TASK_PRUNE_CLOSED_EPICS_SURFACE,
        status: if blocked { "blocked" } else { "pass" }.to_string(),
        dry_run,
        state_dir: state_dir.display().to_string(),
        archive_path: archive_path.map(|path| path.display().to_string()),
        inspected_task_count: rows.len(),
        candidate_count: candidates.len(),
        archived_count: 0,
        pruned_count: 0,
        protected_count: protected.len(),
        candidates,
        protected,
        blocker_codes,
        next_actions,
        graph_issues,
    };

    TaskPruneClosedEpicsPlan {
        receipt,
        candidate_ids,
        archive_tasks,
    }
}

fn task_prune_archive_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn task_prune_archive_path(
    state_dir: &std::path::Path,
    archive_dir: Option<&std::path::Path>,
) -> std::path::PathBuf {
    let base = archive_dir
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| state_dir.join("task-archives"));
    base.join(format!(
        "prune-closed-epics-{}.jsonl",
        task_prune_archive_timestamp()
    ))
}

fn write_task_prune_archive(
    archive_path: &std::path::Path,
    tasks: &[state_store::TaskRecord],
) -> Result<(), String> {
    use std::io::Write as _;

    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create archive directory `{}`: {error}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(archive_path)
        .map_err(|error| format!("create archive file `{}`: {error}", archive_path.display()))?;
    for task in tasks {
        let line = serde_json::to_string(task)
            .map_err(|error| format!("serialize task `{}` for archive: {error}", task.id))?;
        writeln!(file, "{line}")
            .map_err(|error| format!("write archive file `{}`: {error}", archive_path.display()))?;
    }
    Ok(())
}

fn print_task_prune_closed_epics_receipt(
    render: RenderMode,
    receipt: &TaskPruneClosedEpicsReceipt,
    as_json: bool,
) {
    let payload =
        serde_json::to_value(receipt).expect("task prune closed epics receipt should serialize");
    if as_json {
        crate::print_json_pretty(&payload);
    } else if matches!(render, crate::RenderMode::Plain) {
        println!(
            "{}",
            taskflow_format_toon::render_value_section(TASK_PRUNE_CLOSED_EPICS_SURFACE, &payload)
        );
    } else {
        print_surface_header(render, TASK_PRUNE_CLOSED_EPICS_SURFACE);
        print_surface_line(render, "status", &receipt.status);
        print_surface_line(
            render,
            "mode",
            if receipt.dry_run { "dry-run" } else { "apply" },
        );
        print_surface_line(render, "candidates", &receipt.candidate_count.to_string());
        print_surface_line(render, "pruned", &receipt.pruned_count.to_string());
        print_surface_line(render, "protected", &receipt.protected_count.to_string());
        if let Some(path) = receipt.archive_path.as_deref() {
            print_surface_line(render, "archive", path);
        }
        if let Some(action) = receipt.next_actions.first() {
            print_surface_line(render, "next", action);
        }
    }
}

async fn run_task_prune_closed_epics(command: TaskPruneClosedEpicsArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open_existing(state_dir.clone()).await {
        Ok(store) => {
            let rows = match store.all_tasks().await {
                Ok(rows) => rows,
                Err(error) => {
                    eprintln!("Failed to read task rows for prune: {error}");
                    return ExitCode::from(1);
                }
            };
            let archive_path = command
                .apply
                .then(|| task_prune_archive_path(&state_dir, command.archive_dir.as_deref()));
            let task_ids = rows
                .iter()
                .map(|task| task.id.clone())
                .collect::<BTreeSet<_>>();
            let runtime_refs_by_task = match store
                .task_runtime_reference_labels_for_tasks(&task_ids)
                .await
            {
                Ok(refs) => refs,
                Err(error) => {
                    eprintln!("Failed to read task runtime references for prune: {error}");
                    return ExitCode::from(1);
                }
            };
            let mut plan = build_task_prune_closed_epics_plan(
                &rows,
                &runtime_refs_by_task,
                !command.apply,
                &state_dir,
                archive_path.as_deref(),
            );
            if plan.receipt.status == "blocked" {
                print_task_prune_closed_epics_receipt(command.render, &plan.receipt, command.json);
                return ExitCode::from(1);
            }

            if command.apply && !plan.candidate_ids.is_empty() {
                let archive_path = archive_path.expect("apply with candidates should have archive");
                if let Err(error) = write_task_prune_archive(&archive_path, &plan.archive_tasks) {
                    plan.receipt.status = "blocked".to_string();
                    plan.receipt.blocker_codes =
                        vec!["task_prune_archive_write_failed".to_string()];
                    plan.receipt.next_actions = vec![
                        "Resolve archive directory permissions and rerun vida task prune-closed-epics --apply."
                            .to_string(),
                    ];
                    plan.receipt.archive_path = Some(archive_path.display().to_string());
                    plan.receipt.graph_issues = Vec::new();
                    if command.json {
                        let mut payload = serde_json::to_value(&plan.receipt)
                            .expect("blocked task prune receipt should serialize");
                        payload["archive_error"] = serde_json::json!(error);
                        crate::print_json_pretty(&payload);
                    } else {
                        eprintln!("Failed to archive pruned task rows: {error}");
                    }
                    return ExitCode::from(1);
                }

                let mut pruned_count = 0usize;
                for task_id in &plan.candidate_ids {
                    if let Err(error) = store.delete_task_record(task_id).await {
                        plan.receipt.status = "blocked".to_string();
                        plan.receipt.blocker_codes =
                            vec!["task_prune_delete_failed_after_archive".to_string()];
                        plan.receipt.next_actions = vec![format!(
                            "Inspect archive `{}` before retrying; task prune stopped on `{}`.",
                            archive_path.display(),
                            task_id
                        )];
                        plan.receipt.archive_path = Some(archive_path.display().to_string());
                        plan.receipt.archived_count = plan.archive_tasks.len();
                        plan.receipt.pruned_count = pruned_count;
                        if command.json {
                            let mut payload = serde_json::to_value(&plan.receipt)
                                .expect("blocked task prune receipt should serialize");
                            payload["delete_error"] = serde_json::json!(error.to_string());
                            crate::print_json_pretty(&payload);
                        } else {
                            eprintln!("Failed to delete pruned task row `{task_id}`: {error}");
                        }
                        return ExitCode::from(1);
                    }
                    pruned_count += 1;
                }
                if let Err(error) = store.refresh_task_snapshot().await {
                    eprintln!("Failed to refresh task snapshot after prune: {error}");
                    return ExitCode::from(1);
                }
                plan.receipt.archive_path = Some(archive_path.display().to_string());
                plan.receipt.archived_count = plan.archive_tasks.len();
                plan.receipt.pruned_count = pruned_count;
            }

            print_task_prune_closed_epics_receipt(command.render, &plan.receipt, command.json);
            ExitCode::SUCCESS
        }
        Err(error) => emit_task_state_store_open_error(
            "vida task prune-closed-epics",
            &state_dir,
            command.render,
            command.json,
            &error,
        ),
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskProofTargetStatus {
    target: String,
    status: String,
    evidence_source: String,
    evidence_detail: String,
    artifact_status: String,
    legacy_close_reason_match: bool,
    next_action: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskProofAttachBrowserReceipt {
    surface: &'static str,
    status: &'static str,
    task_id: String,
    route: String,
    result: String,
    expect: Option<String>,
    screenshot: Option<String>,
    evidence: Vec<String>,
    proof_target: String,
    artifact: TaskBrowserProofArtifact,
    notes_appended: bool,
    task: state_store::TaskRecord,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskProofAttachEvidenceReceipt {
    surface: &'static str,
    status: &'static str,
    task_id: String,
    proof_target: String,
    proof_targets: Vec<String>,
    command: String,
    result: String,
    artifact_ref: Option<String>,
    artifact_refs: Vec<String>,
    evidence: Vec<String>,
    notes_appended: bool,
    task: state_store::TaskRecord,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskPackFinalizeTaskResult {
    task_id: String,
    title: String,
    status_before: String,
    status_after: String,
    proof_targets: Vec<String>,
    proof_attached: bool,
    closed: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskPackFinalizeReceipt {
    surface: &'static str,
    status: String,
    selector_kind: String,
    selector_value: String,
    matched_count: usize,
    finalized_count: usize,
    blocked_count: usize,
    tasks: Vec<TaskPackFinalizeTaskResult>,
    reconcile_summary: Option<serde_json::Value>,
    orchestrator_init: serde_json::Value,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskTakeoverStatusReceipt {
    surface: &'static str,
    status: String,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    shared_fields: serde_json::Value,
    operator_contracts: serde_json::Value,
    task_id: String,
    allowed: bool,
    local_exception_takeover_state: String,
    root_local_write_allowed: bool,
    paths: Vec<String>,
    packet: serde_json::Value,
    lane: serde_json::Value,
    root_write_guard: serde_json::Value,
    active_takeover_state: String,
    takeover_ready_state: String,
    recommended_surface: Option<String>,
    reason: String,
    recommended_command: Option<String>,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
}

fn task_takeover_json_field(value: &serde_json::Value, key: &str) -> serde_json::Value {
    value.get(key).cloned().unwrap_or(serde_json::Value::Null)
}

fn task_takeover_status_artifact_refs(receipt: &TaskTakeoverStatusReceipt) -> serde_json::Value {
    serde_json::json!({
        "surface": receipt.surface,
        "task_id": receipt.task_id.clone(),
        "run_id": task_takeover_json_field(&receipt.lane, "run_id"),
        "lane_task_id": task_takeover_json_field(&receipt.lane, "task_id"),
        "lane_source": task_takeover_json_field(&receipt.lane, "source"),
        "dispatch_packet_path": task_takeover_json_field(&receipt.packet, "dispatch_packet_path"),
        "dispatch_result_path": task_takeover_json_field(&receipt.packet, "dispatch_result_path"),
        "downstream_dispatch_packet_path": task_takeover_json_field(&receipt.packet, "downstream_dispatch_packet_path"),
        "downstream_dispatch_result_path": task_takeover_json_field(&receipt.packet, "downstream_dispatch_result_path"),
        "root_local_write_allowed": receipt.root_local_write_allowed,
        "local_exception_takeover_state": receipt.local_exception_takeover_state.clone(),
        "active_takeover_state": receipt.active_takeover_state.clone(),
        "takeover_ready_state": receipt.takeover_ready_state.clone(),
        "recommended_surface": receipt.recommended_surface.clone(),
        "recommended_command": receipt.recommended_command.clone(),
    })
}

fn finalize_task_takeover_status_receipt(
    mut receipt: TaskTakeoverStatusReceipt,
) -> TaskTakeoverStatusReceipt {
    let operator_contracts = render_operator_contract_envelope(
        &receipt.status,
        receipt.blocker_codes.clone(),
        receipt.next_actions.clone(),
        task_takeover_status_artifact_refs(&receipt),
    );
    let trace_id = operator_contracts["trace_id"]
        .as_str()
        .map(ToOwned::to_owned);
    let workflow_class = operator_contracts["workflow_class"]
        .as_str()
        .map(ToOwned::to_owned);
    let risk_tier = operator_contracts["risk_tier"]
        .as_str()
        .map(ToOwned::to_owned);
    let artifact_refs = operator_contracts["artifact_refs"].clone();
    let status = operator_contracts["status"]
        .as_str()
        .unwrap_or(&receipt.status)
        .to_string();

    receipt.status = status.clone();
    receipt.trace_id = trace_id.clone();
    receipt.workflow_class = workflow_class.clone();
    receipt.risk_tier = risk_tier.clone();
    receipt.artifact_refs = artifact_refs.clone();
    receipt.shared_fields = serde_json::json!({
        "trace_id": trace_id,
        "workflow_class": workflow_class,
        "risk_tier": risk_tier,
        "status": status,
        "blocker_codes": receipt.blocker_codes.clone(),
        "next_actions": receipt.next_actions.clone(),
        "artifact_refs": artifact_refs,
    });
    receipt.operator_contracts = operator_contracts;
    receipt
}

fn task_exception_takeover_metadata_filename(run_id: &str) -> Result<String, String> {
    crate::exception_takeover_metadata::metadata_filename(run_id)
}

fn task_exception_takeover_metadata_path(
    state_root: &std::path::Path,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    crate::exception_takeover_metadata::metadata_path(state_root, run_id)
}

fn read_task_exception_takeover_metadata(
    state_root: &std::path::Path,
    run_id: &str,
) -> Result<Option<crate::exception_takeover_metadata::ExceptionTakeoverMetadata>, String> {
    crate::exception_takeover_metadata::read_exception_takeover_metadata(state_root, run_id)
}

fn task_exception_takeover_owned_write_scope(
    state_root: &std::path::Path,
    summary: &state_store::RunGraphDispatchReceiptSummary,
) -> Vec<String> {
    crate::exception_takeover_metadata::owned_write_scope_for_summary(state_root, summary)
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskBlockReceipt {
    surface: &'static str,
    status: &'static str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task_id: String,
    blocked: bool,
    closed: bool,
    previous_status: String,
    reason: String,
    evidence: Vec<String>,
    notes_appended: bool,
    task: state_store::TaskRecord,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
struct TaskVerifyReceipt {
    surface: &'static str,
    status: &'static str,
    task_id: String,
    partial: bool,
    closed: bool,
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
    proof_blocked_by_runtime: bool,
    proof_blocker: Option<String>,
    evidence: Vec<String>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    task: state_store::TaskRecord,
}

fn task_json_success_status() -> &'static str {
    crate::contract_profile_adapter::release_contract_status(true)
}

fn task_reconcile_closed_runs_error_run_id(error_text: &str) -> Option<String> {
    let marker = "current session does not own run `";
    let (_, rest) = error_text.split_once(marker)?;
    let run_id = rest.split('`').next()?.trim();
    (!run_id.is_empty()).then(|| run_id.to_string())
}

fn task_reconcile_closed_runs_error_payload(
    error: &state_store::StateStoreError,
) -> serde_json::Value {
    let error_text = error.to_string();
    let rejected_run_id = task_reconcile_closed_runs_error_run_id(&error_text);
    let inspect_command = rejected_run_id.as_ref().map(|run_id| {
        format!(
            "vida taskflow run-graph status {} --json",
            crate::shell_quote(run_id)
        )
    });
    let (blocker_codes, next_actions) = if let Some(command) = inspect_command.as_ref() {
        (
            vec!["closed_task_active_run_projection_mismatch".to_string()],
            vec![format!(
                "Inspect the rejected closed-task run with `{command}` before retrying reconcile-closed-runs."
            )],
        )
    } else {
        (
            vec!["tool_execution_failed".to_string()],
            vec![
                "Inspect the state-store error before retrying reconcile-closed-runs.".to_string(),
            ],
        )
    };
    crate::release1_operator_output::Release1OperatorOutputBuilder::new(
        "vida task reconcile-closed-runs",
    )
    .blocker_codes(blocker_codes)
    .next_actions(next_actions)
    .artifact_refs(serde_json::json!({
        "surface": "vida task reconcile-closed-runs",
        "run_id": rejected_run_id,
        "inspect_command": inspect_command,
    }))
    .extra_fields(serde_json::json!({
        "error": error_text,
        "repair_target": inspect_command,
    }))
    .build()
    .expect("task reconcile closed runs error payload should satisfy release-1 operator contract")
}

fn proof_target_has_close_reason_evidence(task: &state_store::TaskRecord, target: &str) -> bool {
    let Some(reason) = task.close_reason.as_deref() else {
        return false;
    };
    let target = target.trim();
    !target.is_empty()
        && reason
            .to_ascii_lowercase()
            .contains(&target.to_ascii_lowercase())
}

fn normalize_browser_proof_result(result: &str) -> Result<String, String> {
    canonical_task_proof_result(result)
        .map(str::to_string)
        .ok_or_else(|| "--result must be one of: pass, fail, blocked".to_string())
}

fn task_proof_target_status(task: &state_store::TaskRecord, target: &str) -> TaskProofTargetStatus {
    task_proof_target_status_with_inheritance(task, target, None)
}

fn task_direct_children_for_proof_inheritance<'a>(
    tasks: &'a [state_store::TaskRecord],
    parent_id: &str,
) -> Vec<&'a state_store::TaskRecord> {
    tasks
        .iter()
        .filter(|task| {
            task.dependencies.iter().any(|dependency| {
                dependency.edge_type == "parent-child" && dependency.depends_on_id == parent_id
            })
        })
        .collect()
}

fn inherited_child_proof_evidence_status(
    task: &state_store::TaskRecord,
    target: &str,
    tasks: &[state_store::TaskRecord],
) -> Option<TaskProofTargetStatus> {
    let children = task_direct_children_for_proof_inheritance(tasks, &task.id);
    if children.is_empty() {
        return None;
    }
    let open_children = children
        .iter()
        .filter(|child| !state_store::StateStore::task_status_is_closed_like(&child.status))
        .count();
    if open_children > 0 || children.len() != 1 {
        return None;
    }
    let child = children[0];
    let proof_match = structured_task_proof_evidence_match(child.notes.as_deref(), target)?;
    Some(TaskProofTargetStatus {
        target: target.trim().to_string(),
        status: "satisfied".to_string(),
        evidence_source: "inherited_child_task_proof_evidence".to_string(),
        evidence_detail: format!(
            "single closed child `{}` has matching structured proof evidence: {}",
            child.id, proof_match.evidence_detail
        ),
        artifact_status: proof_match.artifact_status,
        legacy_close_reason_match: proof_target_has_close_reason_evidence(task, target),
        next_action: "No action for this proof target.".to_string(),
    })
}

fn task_proof_target_status_with_inheritance(
    task: &state_store::TaskRecord,
    target: &str,
    inheritance_rows: Option<&[state_store::TaskRecord]>,
) -> TaskProofTargetStatus {
    let target = target.trim().to_string();
    let runtime_blocked =
        task_reports_runtime_proof_blocker(&task.labels, task.close_reason.as_deref());
    let legacy_close_reason_match = proof_target_has_close_reason_evidence(task, &target);
    if let Some(proof_match) = structured_task_proof_evidence_match(task.notes.as_deref(), &target)
    {
        return TaskProofTargetStatus {
            target,
            status: "satisfied".to_string(),
            evidence_source: proof_match.evidence_source,
            evidence_detail: proof_match.evidence_detail,
            artifact_status: proof_match.artifact_status,
            legacy_close_reason_match,
            next_action: "No action for this proof target.".to_string(),
        };
    }
    if let Some(rows) = inheritance_rows {
        if let Some(inherited) = inherited_child_proof_evidence_status(task, &target, rows) {
            return inherited;
        }
    }
    if runtime_blocked {
        return TaskProofTargetStatus {
            target: target.clone(),
            status: "blocked_by_runtime".to_string(),
            evidence_source: "close_reason".to_string(),
            evidence_detail: "task close_reason reports runtime proof blocker context".to_string(),
            artifact_status: "not_recorded".to_string(),
            legacy_close_reason_match,
            next_action: format!(
                "Resolve runtime proof blocker, then record evidence for proof target `{}`.",
                target
            ),
        };
    }
    let task_is_closed = state_store::StateStore::task_status_is_closed_like(&task.status);
    let status = if task_is_closed {
        "missing_evidence"
    } else {
        "pending"
    };
    TaskProofTargetStatus {
        target: target.clone(),
        status: status.to_string(),
        evidence_source: "planner_metadata.proof_targets".to_string(),
        evidence_detail: if legacy_close_reason_match {
            "legacy close_reason text matches target, but structured proof evidence is required"
                .to_string()
        } else {
            "no matching structured proof evidence found".to_string()
        },
        artifact_status: "not_recorded".to_string(),
        legacy_close_reason_match,
        next_action: if task_is_closed {
            format!(
                "Structured proof evidence is missing on already closed task `{}`; inspect task history or use an explicit repair/reopen flow before mutating proof evidence.",
                task.id
            )
        } else {
            format!(
                "Run or attach evidence for proof target `{}`, then close or update `{}`.",
                target, task.id
            )
        },
    }
}

fn task_proof_status_payload(
    task: &state_store::TaskRecord,
    read_metadata: Option<&TaskReadMetadata>,
) -> serde_json::Value {
    task_proof_status_payload_with_inheritance(task, read_metadata, None)
}

fn task_proof_status_payload_with_inheritance(
    task: &state_store::TaskRecord,
    read_metadata: Option<&TaskReadMetadata>,
    inheritance_rows: Option<&[state_store::TaskRecord]>,
) -> serde_json::Value {
    let targets = task
        .planner_metadata
        .proof_targets
        .iter()
        .map(|target| task_proof_target_status_with_inheritance(task, target, inheritance_rows))
        .collect::<Vec<_>>();
    let configured_count = targets.len();
    let satisfied_count = targets
        .iter()
        .filter(|target| target.status == "satisfied")
        .count();
    let runtime_blocked_count = targets
        .iter()
        .filter(|target| target.status == "blocked_by_runtime")
        .count();
    let missing_count = targets
        .iter()
        .filter(|target| target.status != "satisfied")
        .count();
    let missing_targets = targets
        .iter()
        .filter(|target| target.status != "satisfied")
        .map(|target| target.target.clone())
        .collect::<Vec<_>>();
    let quoted_task_id = crate::shell_quote(&task.id);
    let next_required_command = if configured_count == 0 {
        format!(
            "Add proof targets with `{}`.",
            operator_output::command_text::human_command(&format!(
                "vida task update {} --proof-target <command-or-artifact> --json",
                quoted_task_id
            ))
        )
    } else if missing_count == 0 {
        "No proof action required; all configured proof targets have structured proof evidence."
            .to_string()
    } else if state_store::StateStore::task_status_is_closed_like(&task.status) {
        format!(
            "Structured proof evidence is missing on already closed task `{}`; inspect task history or use an explicit repair/reopen flow before mutating proof evidence.",
            task.id
        )
    } else {
        format!(
            "Run or attach missing proof evidence, then inspect again with `{}`.",
            operator_output::command_text::human_command(&format!(
                "vida task proof status {}",
                quoted_task_id
            ))
        )
    };
    crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task proof status")
        .artifact_refs(serde_json::json!({
            "surface": "vida task proof status",
            "task_id": task.id,
        }))
        .extra_fields(serde_json::json!({
            "task_id": task.id,
            "task_status": task.status,
            "configured_proof_target_count": configured_count,
            "satisfied_count": satisfied_count,
            "missing_count": missing_count,
            "runtime_blocked_count": runtime_blocked_count,
            "missing_proof": configured_count > 0 && missing_count > 0,
            "proof_blocked_by_runtime": runtime_blocked_count > 0,
            "proof_targets": targets,
            "missing_targets": missing_targets,
            "next_required_command": next_required_command,
            "evidence_model": {
                "configured_targets_source": "task.planner_metadata.proof_targets",
                "satisfaction_source": "task_proof_evidence structured registry entries or schema-backed browser proof artifacts",
                "artifact_registry": "task_notes.task_proof_evidence|task_notes.task_browser_proof",
                "browser_proof_artifact_schema": taskflow_core::task::verify::TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION,
                "browser_proof_note_schema": taskflow_core::task::verify::TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION,
                "legacy_close_reason_text": "migration_context_not_authority"
            },
            "state_access": task_read_metadata_value(read_metadata),
        }))
        .build()
        .expect("task proof status payload should satisfy release-1 operator contract")
}

fn task_close_structured_proof_gate_payload(
    task: &state_store::TaskRecord,
    inheritance_rows: Option<&[state_store::TaskRecord]>,
) -> Option<serde_json::Value> {
    let proof_status = task_proof_status_payload_with_inheritance(task, None, inheritance_rows);
    let configured_count = proof_status["configured_proof_target_count"]
        .as_u64()
        .unwrap_or(0);
    let missing_count = proof_status["missing_count"].as_u64().unwrap_or(0);
    if configured_count == 0 || missing_count == 0 {
        return None;
    }

    let proof_blocked_by_runtime = proof_status["proof_blocked_by_runtime"]
        .as_bool()
        .unwrap_or(false);
    let blocker_code = if proof_blocked_by_runtime {
        "proof_blocked_by_runtime"
    } else {
        "missing_structured_proof_evidence"
    };
    let quoted_task_id = crate::shell_quote(&task.id);
    let missing_targets = proof_status["missing_targets"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let next_actions = missing_targets
        .iter()
        .map(|target| {
            let quoted_target = crate::shell_quote(target);
            format!(
                "Attach structured proof evidence with `{}`.",
                operator_output::command_text::human_command(&format!(
                    "vida task proof attach-evidence {} --proof-target {} --result pass --artifact-ref <artifact-ref> --evidence '<evidence summary>'",
                    quoted_task_id, quoted_target
                ))
            )
        })
        .collect::<Vec<_>>();
    Some(
        crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task close")
            .blocker_codes(vec![blocker_code.to_string()])
            .next_actions(next_actions)
            .artifact_refs(serde_json::json!({
                "surface": "vida task close",
                "task_id": task.id,
                "proof_status_surface": "vida task proof status",
            }))
            .extra_fields(serde_json::json!({
                "closed": false,
                "continuation_blocked": true,
                "automation_blocked": false,
                "feedback_blocked": false,
                "task_id": task.id,
                "reason": "task has configured proof targets without matching structured task_proof_evidence pass receipt",
                "missing_targets": missing_targets,
                "proof_status": proof_status,
                "task": task,
            }))
            .build()
            .expect("task close structured proof gate should satisfy release-1 operator contract"),
    )
}

fn print_task_close_structured_proof_gate_block(
    render: RenderMode,
    payload: &serde_json::Value,
    as_json: bool,
) {
    if as_json {
        crate::print_json_pretty(payload);
        return;
    }
    print_surface_header(render, "vida task close");
    print_surface_line(render, "status", "blocked");
    print_surface_line(
        render,
        "task",
        payload["task_id"].as_str().unwrap_or("unknown"),
    );
    let blockers = payload["blocker_codes"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    print_surface_line(render, "blockers", &blockers);
    let missing_targets = payload["missing_targets"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_default();
    print_surface_line(render, "missing targets", &missing_targets);
    if let Some(next_action) = payload["next_actions"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(serde_json::Value::as_str)
    {
        print_surface_line(render, "next", next_action);
    }
}

fn task_closeout_graph_payload(issues: &[state_store::TaskGraphIssue]) -> serde_json::Value {
    crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task validate-graph")
        .extra_fields(serde_json::json!({
            "valid": issues.is_empty(),
            "issue_count": issues.len(),
            "issues": issues,
        }))
        .build()
        .expect("task closeout graph payload should satisfy release-1 operator contract")
}

fn task_closeout_repo_root(state_dir: &std::path::Path) -> std::path::PathBuf {
    let parts = state_dir
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if parts.len() >= 3
        && parts[parts.len() - 3] == ".vida"
        && parts[parts.len() - 2] == "data"
        && parts[parts.len() - 1] == "state"
    {
        return state_dir
            .ancestors()
            .nth(3)
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| state_dir.to_path_buf());
    }
    std::env::current_dir().unwrap_or_else(|_| state_dir.to_path_buf())
}

fn task_closeout_git_executable(repo_root: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let path_var = std::env::var_os("PATH").ok_or_else(|| "PATH is not set".to_string())?;
    let current_dir = std::env::current_dir().ok();
    #[cfg(windows)]
    let executable_names = ["git.exe", "git.cmd", "git.bat", "git"];
    #[cfg(not(windows))]
    let executable_names = ["git"];

    for path_dir in std::env::split_paths(&path_var) {
        if path_dir.as_os_str().is_empty() || path_dir.is_relative() {
            continue;
        }
        if path_dir == repo_root || current_dir.as_ref().is_some_and(|cwd| &path_dir == cwd) {
            continue;
        }
        for executable_name in executable_names {
            let candidate = path_dir.join(executable_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err("no trusted absolute git executable found on PATH".to_string())
}

fn task_closeout_temp_scan(
    include_temp_scan: bool,
    state_dir: &std::path::Path,
) -> TaskCloseoutTempScan {
    if !include_temp_scan {
        return TaskCloseoutTempScan {
            enabled: false,
            status: "skipped".to_string(),
            tracked_match_count: 0,
            tracked_matches: Vec::new(),
            command: "git ls-files tmp* false true null undefined nul".to_string(),
            repo_root: None,
            error: None,
        };
    }
    let repo_root = task_closeout_repo_root(state_dir);
    let git_executable = match task_closeout_git_executable(&repo_root) {
        Ok(git_executable) => git_executable,
        Err(error) => {
            return TaskCloseoutTempScan {
                enabled: true,
                status: "blocked".to_string(),
                tracked_match_count: 0,
                tracked_matches: Vec::new(),
                command: "git ls-files tmp* false true null undefined nul".to_string(),
                repo_root: Some(repo_root.display().to_string()),
                error: Some(format!(
                    "failed to resolve trusted git temp scan executable: {error}"
                )),
            };
        }
    };
    let command_text = format!(
        "{} -C {} -c core.fsmonitor=false ls-files tmp* false true null undefined nul",
        crate::shell_quote(&git_executable.display().to_string()),
        crate::shell_quote(&repo_root.display().to_string())
    );
    let output = std::process::Command::new(&git_executable)
        .arg("-C")
        .arg(&repo_root)
        .arg("-c")
        .arg("core.fsmonitor=false")
        .args([
            "ls-files",
            "tmp*",
            "false",
            "true",
            "null",
            "undefined",
            "nul",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_GLOBAL")
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return TaskCloseoutTempScan {
                enabled: true,
                status: "blocked".to_string(),
                tracked_match_count: 0,
                tracked_matches: Vec::new(),
                command: command_text,
                repo_root: Some(repo_root.display().to_string()),
                error: Some(format!("failed to run git temp scan: {error}")),
            };
        }
    };
    if !output.status.success() {
        return TaskCloseoutTempScan {
            enabled: true,
            status: "blocked".to_string(),
            tracked_match_count: 0,
            tracked_matches: Vec::new(),
            command: command_text,
            repo_root: Some(repo_root.display().to_string()),
            error: Some(format!(
                "git temp scan exited with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )),
        };
    }
    let tracked_matches = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    TaskCloseoutTempScan {
        enabled: true,
        status: if tracked_matches.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        tracked_match_count: tracked_matches.len(),
        tracked_matches,
        command: command_text.to_string(),
        repo_root: Some(repo_root.display().to_string()),
        error: None,
    }
}

fn task_closeout_summary(
    task_id: &str,
    basis: &str,
    task: &state_store::TaskRecord,
    read_metadata: Option<&TaskReadMetadata>,
    rows: &[state_store::TaskRecord],
    graph_issues: &[state_store::TaskGraphIssue],
    include_temp_scan: bool,
    state_dir: &std::path::Path,
) -> Result<TaskCloseoutSummary, String> {
    let progress = task_progress_summary_for_basis(rows, task_id, basis)
        .map_err(|error| format!("Failed to compute closeout progress: {error}"))?;
    let proof = task_proof_status_payload_with_inheritance(task, read_metadata, Some(rows));
    let closure = crate::task_cli_render::build_pass_operator_surface_payload(
        "vida task closure-ready",
        serde_json::json!({
            "task_id": task_id,
            "state_access": task_read_metadata_value(read_metadata),
            "basis": basis,
            "ready_for_close": progress.ready_for_close,
            "closure_candidate": progress.closure_candidate,
            "closure_candidate_state": progress.closure_candidate_state,
            "closure_candidate_reason": progress.closure_candidate_reason,
            "next_required_command": progress.next_required_command,
            "recommended_next_action": progress.recommended_next_action,
            "progress": crate::task_cli_render::task_progress_value(&progress),
        }),
    );
    let graph = task_closeout_graph_payload(graph_issues);
    let progress_payload = crate::task_cli_render::task_progress_payload(&progress);
    let temp_scan = task_closeout_temp_scan(include_temp_scan, state_dir);
    let blocker_codes =
        task_closeout_blocker_codes(&proof, &closure, graph_issues.len(), &temp_scan);
    let mut next_actions = Vec::new();
    if let Some(command) = proof["next_required_command"].as_str() {
        next_actions.push(command.to_string());
    }
    if let Some(command) = closure["next_required_command"].as_str() {
        next_actions.push(command.to_string());
    }
    if !graph_issues.is_empty() {
        next_actions.push("Run vida task validate-graph and resolve graph issues.".to_string());
    }
    if temp_scan.tracked_match_count > 0 {
        next_actions.push(
            "Remove or explicitly justify tracked temporary artifacts before commit.".to_string(),
        );
    }
    next_actions.sort();
    next_actions.dedup();
    if blocker_codes.is_empty() {
        next_actions.clear();
    }
    Ok(TaskCloseoutSummary {
        task_id: task_id.to_string(),
        status: if blocker_codes.is_empty() {
            "pass".to_string()
        } else {
            "blocked".to_string()
        },
        basis: basis.to_string(),
        proof,
        closure,
        graph,
        progress: progress_payload,
        temp_scan,
        blocker_codes,
        next_actions,
    })
}

fn task_closeout_blocker_codes(
    proof: &serde_json::Value,
    closure: &serde_json::Value,
    graph_issue_count: usize,
    temp_scan: &TaskCloseoutTempScan,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if proof["configured_proof_target_count"].as_u64().unwrap_or(0) == 0 {
        blocker_codes.push("closeout_proof_targets_missing".to_string());
    }
    if proof["missing_count"].as_u64().unwrap_or(0) > 0 {
        blocker_codes.push("closeout_proof_evidence_missing".to_string());
    }
    let closure_state = closure["closure_candidate_state"].as_str().unwrap_or("");
    if !closure["ready_for_close"].as_bool().unwrap_or(false) && closure_state != "already_closed" {
        blocker_codes.push("closeout_closure_not_ready".to_string());
    }
    if graph_issue_count > 0 {
        blocker_codes.push("closeout_task_graph_invalid".to_string());
    }
    if temp_scan.tracked_match_count > 0 {
        blocker_codes.push("closeout_tracked_temp_artifacts".to_string());
    }
    if temp_scan.error.is_some() {
        blocker_codes.push("closeout_temp_scan_failed".to_string());
    }
    blocker_codes
}

fn print_task_proof_status(
    render: RenderMode,
    task: &state_store::TaskRecord,
    payload: &serde_json::Value,
) {
    if matches!(render, crate::RenderMode::Plain) {
        let rows = payload["proof_targets"]
            .as_array()
            .map(|targets| {
                targets
                    .iter()
                    .map(|target| {
                        serde_json::json!({
                            "target": target["target"].as_str().unwrap_or(""),
                            "status": target["status"].as_str().unwrap_or("unknown"),
                            "evidence_source": target["evidence_source"].as_str().unwrap_or("unknown"),
                            "artifact_status": target["artifact_status"].as_str().unwrap_or("unknown"),
                            "next_action": target["next_action"].as_str().unwrap_or(""),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let value = serde_json::json!({
            "task": task.id,
            "task_status": task.status,
            "configured_proof_target_count": payload["configured_proof_target_count"],
            "satisfied_count": payload["satisfied_count"],
            "missing_count": payload["missing_count"],
            "runtime_blocked_count": payload["runtime_blocked_count"],
            "proof_blocked_by_runtime": payload["proof_blocked_by_runtime"],
            "proof_targets": rows,
            "next_required_command": payload["next_required_command"].as_str().unwrap_or(""),
        });
        println!(
            "{}",
            taskflow_format_toon::render_value_section("vida task proof status", &value)
        );
        return;
    }
    print_surface_header(render, "vida task proof status");
    print_surface_line(render, "task", &task.id);
    print_surface_line(render, "task status", &task.status);
    print_surface_line(
        render,
        "proof targets",
        &payload["configured_proof_target_count"].to_string(),
    );
    print_surface_line(render, "satisfied", &payload["satisfied_count"].to_string());
    print_surface_line(render, "missing", &payload["missing_count"].to_string());
    print_surface_line(
        render,
        "runtime blocked",
        &payload["proof_blocked_by_runtime"].to_string(),
    );
    print_surface_line(
        render,
        "next",
        payload["next_required_command"].as_str().unwrap_or(""),
    );
}

async fn task_takeover_status_receipt(
    store: &StateStore,
    task: &state_store::TaskRecord,
    status_override: Option<state_store::RunGraphStatus>,
    lane_source_override: Option<&str>,
    allow_latest_fallback: bool,
) -> TaskTakeoverStatusReceipt {
    let (lane_source, status) = if let Some(status) = status_override {
        (lane_source_override.unwrap_or("run_id"), Some(status))
    } else if !allow_latest_fallback {
        (
            lane_source_override.unwrap_or("task_id"),
            store
                .latest_run_graph_status_for_task(&task.id)
                .await
                .ok()
                .flatten(),
        )
    } else {
        let current_status = store
            .latest_run_graph_status_for_current_session()
            .await
            .ok()
            .flatten();
        match current_status {
            Some(status) => ("current_session", Some(status)),
            None => (
                "latest",
                store.latest_run_graph_status().await.ok().flatten(),
            ),
        }
    };
    let Some(status) = status else {
        return finalize_task_takeover_status_receipt(TaskTakeoverStatusReceipt {
            surface: "vida task takeover status",
            status: "blocked".to_string(),
            trace_id: None,
            workflow_class: None,
            risk_tier: None,
            artifact_refs: serde_json::Value::Null,
            shared_fields: serde_json::Value::Null,
            operator_contracts: serde_json::Value::Null,
            task_id: task.id.clone(),
            allowed: false,
            local_exception_takeover_state: "not_recorded".to_string(),
            root_local_write_allowed: false,
            paths: Vec::new(),
            packet: serde_json::json!({
                "dispatch_packet_path": serde_json::Value::Null,
                "dispatch_result_path": serde_json::Value::Null,
            }),
            lane: serde_json::json!({
                "source": lane_source,
                "run_id": serde_json::Value::Null,
                "task_id": serde_json::Value::Null,
            }),
            root_write_guard: serde_json::json!({
                "status": "blocked_by_default",
                "root_local_write_allowed": false,
                "root_local_write_allowed_for_only_these_paths": [],
                "local_exception_takeover_state": "not_recorded",
                "latest_lane_status": serde_json::Value::Null,
                "local_exception_takeover_gate": serde_json::Value::Null,
                "latest_run_graph_task_stale": false,
                "reason": "no run-graph lane evidence is available",
            }),
            active_takeover_state: "not_recorded".to_string(),
            takeover_ready_state: "not_ready".to_string(),
            recommended_surface: Some("vida lane show".to_string()),
            reason: format!(
                "no run-graph lane evidence is available for takeover status of task `{}`",
                task.id
            ),
            recommended_command: Some(operator_output::command_text::human_command(
                "vida lane show --latest --json",
            )),
            next_actions: vec![format!(
                "Run `{}` to inspect latest lane evidence before attempting exception takeover for task `{}`.",
                operator_output::command_text::human_command("vida lane show --latest --json"),
                task.id
            )],
            blocker_codes: vec![if allow_latest_fallback {
                "missing_latest_lane_receipt".to_string()
            } else {
                "missing_lane_receipt".to_string()
            }],
        });
    };
    let summary = store
        .run_graph_dispatch_receipt_summary_for_status(&status)
        .await
        .ok()
        .flatten();
    let recovery = store.run_graph_recovery_summary(&status.run_id).await.ok();
    let recovery_gate = recovery.as_ref().map(|recovery| {
        recovery
            .delegation_gate
            .local_exception_takeover_gate
            .as_str()
    });
    let (summary, takeover_state) = match summary {
        Some(summary) => {
            let receipt = taskflow_authority::exception_takeover::ExceptionTakeoverReceipt {
                lane_status: summary.lane_status.as_str(),
                exception_path_receipt_id: summary.exception_path_receipt_id.as_deref(),
                supersedes_receipt_id: summary.supersedes_receipt_id.as_deref(),
            };
            let recovery = recovery_gate.map(|local_exception_takeover_gate| {
                taskflow_authority::exception_takeover::ExceptionTakeoverRecovery {
                    local_exception_takeover_gate,
                }
            });
            let state = taskflow_authority::exception_takeover::exception_takeover_state_label(
                Some(&receipt),
                recovery.as_ref(),
            );
            (Some(summary), state)
        }
        None => (None, None),
    };
    let task_matches_lane = status.task_id.trim() == task.id.trim();
    let receipt_recorded = takeover_state
        == Some(
            taskflow_authority::exception_takeover::ExceptionTakeoverStateLabel::ReceiptRecorded,
        );
    let takeover_active = takeover_state
        == Some(taskflow_authority::exception_takeover::ExceptionTakeoverStateLabel::Active);
    let state_label = takeover_state
        .map(taskflow_authority::exception_takeover::ExceptionTakeoverStateLabel::as_str)
        .unwrap_or("not_recorded")
        .to_string();
    let metadata_paths = summary
        .as_ref()
        .filter(|_| takeover_active)
        .map(|summary| task_exception_takeover_owned_write_scope(store.root(), summary))
        .unwrap_or_default();
    let paths = metadata_paths;
    let root_local_write_allowed = task_matches_lane && takeover_active && !paths.is_empty();
    let allowed = root_local_write_allowed;
    let (reason, blocker_codes, next_actions, recommended_command, recommended_surface) =
        if !task_matches_lane {
            (
                format!(
                    "latest lane task `{}` does not match requested task `{}`",
                    status.task_id, task.id
                ),
                vec!["latest_lane_task_mismatch".to_string()],
                vec![format!(
                    "Bind or inspect the correct bounded unit before local writes: `{}` and `{}`.",
                    operator_output::command_text::human_command(&format!(
                        "vida task show {}",
                        crate::shell_quote(&task.id)
                    )),
                    operator_output::command_text::human_command("vida lane show --latest --json")
                )],
                Some(operator_output::command_text::human_command(
                    "vida lane show --latest --json",
                )),
                Some("vida lane show".to_string()),
            )
        } else if allowed {
            (
            "exception takeover is active for this task; local writes are lawful only inside listed paths"
                .to_string(),
            Vec::new(),
            Vec::new(),
            None,
            None,
        )
        } else if receipt_recorded {
            let command = summary.as_ref().and_then(|summary| {
                summary
                    .exception_path_receipt_id
                    .as_deref()
                    .map(|receipt_id| {
                        format!(
                            "vida lane supersede {} --receipt-id {} --json",
                            crate::shell_quote(&summary.run_id),
                            crate::shell_quote(receipt_id)
                        )
                    })
            });
            (
            "exception receipt is recorded but supersession is required before local write is active"
                .to_string(),
            vec!["supersession_required".to_string()],
            command
                .iter()
                .map(|command| operator_output::command_text::human_command(command))
                .chain([
                    format!(
                        "Run `{}` if the receipt id is missing or stale.",
                        operator_output::command_text::human_command("vida lane show --latest --json")
                    ),
                ])
                .collect(),
            command.map(|command| operator_output::command_text::human_command(&command)),
            Some("vida lane supersede".to_string()),
        )
        } else if takeover_active && paths.is_empty() {
            (
                "exception takeover is active but receipt-bound owned_write_scope could not be read"
                    .to_string(),
                vec!["exception_takeover_scope_missing".to_string()],
                vec![format!(
                    "Inspect the lane receipt and exception metadata: `{}`.",
                    operator_output::command_text::human_command(&format!(
                "vida lane show {}",
                        crate::shell_quote(&status.run_id)
                    ))
                )],
                Some(format!(
                    "{}",
                    operator_output::command_text::human_command(&format!(
                        "vida lane show {} --json",
                        crate::shell_quote(&status.run_id)
                    ))
                )),
                Some("vida lane show".to_string()),
            )
        } else if state_label == "not_recorded" {
            let command = format!(
                "vida lane takeover-ready {}",
                crate::shell_quote(&status.run_id)
            );
            (
                "exception takeover is not recorded for this task".to_string(),
                vec!["exception_takeover_not_recorded".to_string()],
                vec![command.clone()],
                Some(command),
                Some("vida lane takeover-ready".to_string()),
            )
        } else {
            let command = format!(
                "vida lane takeover-ready {}",
                crate::shell_quote(&status.run_id)
            );
            (
                "exception takeover is not active for this task".to_string(),
                vec!["exception_takeover_not_active".to_string()],
                vec![command.clone()],
                Some(command),
                Some("vida lane takeover-ready".to_string()),
            )
        };
    let packet = serde_json::json!({
        "dispatch_packet_path": summary
            .as_ref()
            .and_then(|summary| summary.dispatch_packet_path.clone()),
        "dispatch_result_path": summary
            .as_ref()
            .and_then(|summary| summary.dispatch_result_path.clone()),
        "downstream_dispatch_packet_path": summary
            .as_ref()
            .and_then(|summary| summary.downstream_dispatch_packet_path.clone()),
        "downstream_dispatch_result_path": summary
            .as_ref()
            .and_then(|summary| summary.downstream_dispatch_result_path.clone()),
    });
    let lane = serde_json::json!({
        "source": lane_source,
        "run_id": status.run_id,
        "task_id": status.task_id,
        "dispatch_target": summary.as_ref().map(|summary| summary.dispatch_target.clone()),
        "lane_status": summary.as_ref().map(|summary| summary.lane_status.clone()),
        "dispatch_status": summary.as_ref().map(|summary| summary.dispatch_status.clone()),
        "selected_backend": summary.as_ref().and_then(|summary| summary.selected_backend.clone()),
        "exception_path_receipt_id": summary.as_ref().and_then(|summary| summary.exception_path_receipt_id.clone()),
        "supersedes_receipt_id": summary.as_ref().and_then(|summary| summary.supersedes_receipt_id.clone()),
        "exception_path_metadata_path": summary
            .as_ref()
            .and_then(|summary| task_exception_takeover_metadata_path(store.root(), &summary.run_id).ok())
            .map(|path| path.display().to_string()),
        "recovery_gate": recovery_gate,
    });
    let takeover_ready_state = taskflow_core::task::takeover::takeover_ready_state(
        allowed,
        receipt_recorded,
        task_matches_lane,
    )
    .to_string();
    let root_write_guard = serde_json::json!({
        "status": taskflow_core::task::takeover::root_write_guard_status(root_local_write_allowed),
        "root_local_write_allowed": root_local_write_allowed,
        "root_local_write_allowed_for_only_these_paths": if root_local_write_allowed { paths.clone() } else { Vec::<String>::new() },
        "local_exception_takeover_state": state_label.clone(),
        "latest_lane_status": summary.as_ref().map(|summary| summary.lane_status.clone()),
        "local_exception_takeover_gate": recovery_gate,
        "latest_run_graph_task_stale": !task_matches_lane,
        "reason": if root_local_write_allowed { serde_json::Value::Null } else { serde_json::json!(reason.clone()) },
    });

    finalize_task_takeover_status_receipt(TaskTakeoverStatusReceipt {
        surface: "vida task takeover status",
        status: if allowed {
            task_json_success_status().to_string()
        } else {
            "blocked".to_string()
        },
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: serde_json::Value::Null,
        shared_fields: serde_json::Value::Null,
        operator_contracts: serde_json::Value::Null,
        task_id: task.id.clone(),
        allowed,
        local_exception_takeover_state: state_label.clone(),
        root_local_write_allowed,
        paths,
        packet,
        lane,
        root_write_guard,
        active_takeover_state: if task_matches_lane {
            state_label.clone()
        } else {
            "stale_task_blocked".to_string()
        },
        takeover_ready_state,
        recommended_surface,
        reason,
        recommended_command,
        next_actions,
        blocker_codes,
    })
}

fn print_task_takeover_status(render: RenderMode, receipt: &TaskTakeoverStatusReceipt) {
    print_surface_header(render, "vida task takeover status");
    for (label, value) in task_takeover_status_default_lines(receipt) {
        print_surface_line(render, label, &value);
    }
}

fn task_takeover_status_default_lines(
    receipt: &TaskTakeoverStatusReceipt,
) -> Vec<(&'static str, String)> {
    let mut lines = vec![
        ("status", receipt.status.clone()),
        ("task", receipt.task_id.clone()),
        ("allowed", receipt.allowed.to_string()),
        (
            "takeover state",
            receipt.local_exception_takeover_state.clone(),
        ),
        (
            "root local write",
            receipt.root_local_write_allowed.to_string(),
        ),
    ];
    if !receipt.blocker_codes.is_empty() {
        lines.push(("blocker_codes", receipt.blocker_codes.join(", ")));
    }
    if let Some(command) = receipt.recommended_command.as_deref() {
        lines.push(("recommended command", command.to_string()));
    }
    for action in &receipt.next_actions {
        lines.push(("next action", action.clone()));
    }
    lines
}

fn print_task_block_receipt(render: RenderMode, receipt: &TaskBlockReceipt, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(receipt).expect("task block receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "reason", &receipt.reason);
    if !receipt.blocker_codes.is_empty() {
        print_surface_line(render, "blocker codes", &receipt.blocker_codes.join(", "));
    }
    if !receipt.next_actions.is_empty() {
        print_surface_line(render, "next actions", &receipt.next_actions.join(" | "));
    }
}

fn task_verify_planner_metadata(
    existing: &state_store::TaskPlannerMetadata,
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
) -> Option<state_store::TaskPlannerMetadata> {
    if !proof_blocked || !existing.proof_targets.is_empty() {
        return None;
    }
    let mut metadata = existing.clone();
    if evidence.is_empty() {
        if let Some(proof_blocker) = proof_blocker
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            metadata.proof_targets.push(proof_blocker.to_string());
        }
    } else {
        metadata.proof_targets.extend(evidence.iter().cloned());
    }
    if metadata.proof_targets.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

fn task_browser_proof_planner_metadata(
    existing: &state_store::TaskPlannerMetadata,
    proof_target: &str,
) -> state_store::TaskPlannerMetadata {
    let mut metadata = existing.clone();
    if !metadata
        .proof_targets
        .iter()
        .any(|target| target.trim() == proof_target.trim())
    {
        metadata.proof_targets.push(proof_target.trim().to_string());
    }
    metadata
}

fn task_evidence_proof_planner_metadata(
    existing: &state_store::TaskPlannerMetadata,
    proof_target: &str,
) -> state_store::TaskPlannerMetadata {
    if existing.proof_targets.is_empty() {
        task_browser_proof_planner_metadata(existing, proof_target)
    } else {
        existing.clone()
    }
}

fn task_pack_finalize_selector(
    order_bucket: Option<&str>,
    parallel_group: Option<&str>,
) -> Result<(&'static str, String), String> {
    let order_bucket = order_bucket
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let parallel_group = parallel_group
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (order_bucket, parallel_group) {
        (Some(_), Some(_)) => {
            Err("Provide exactly one of --order-bucket or --parallel-group.".to_string())
        }
        (Some(value), None) => Ok(("order_bucket", value.to_string())),
        (None, Some(value)) => Ok(("parallel_group", value.to_string())),
        (None, None) => Err("Provide --order-bucket or --parallel-group.".to_string()),
    }
}

fn task_matches_pack_selector(
    task: &state_store::TaskRecord,
    selector_kind: &str,
    selector_value: &str,
) -> bool {
    match selector_kind {
        "order_bucket" => task
            .execution_semantics
            .order_bucket
            .as_deref()
            .is_some_and(|value| value.trim() == selector_value),
        "parallel_group" => task
            .execution_semantics
            .parallel_group
            .as_deref()
            .is_some_and(|value| value.trim() == selector_value),
        _ => false,
    }
}

fn normalized_pack_finalize_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn task_pack_finalize_targets(task: &state_store::TaskRecord, overrides: &[String]) -> Vec<String> {
    let targets = if overrides.is_empty() {
        &task.planner_metadata.proof_targets
    } else {
        overrides
    };
    normalized_pack_finalize_values(targets)
}

fn task_pack_finalize_close_reason(
    reason_prefix: Option<&str>,
    selector_kind: &str,
    selector_value: &str,
    proof_targets: &[String],
) -> String {
    let proof_summary = if proof_targets.is_empty() {
        "no configured proof targets".to_string()
    } else {
        format!(
            "structured proof targets passed: {}",
            proof_targets.join(" | ")
        )
    };
    match reason_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(prefix) => format!(
            "{prefix}; pack-finalize selector {selector_kind}={selector_value}; {proof_summary}"
        ),
        None => {
            format!("pack-finalize selector {selector_kind}={selector_value}; {proof_summary}")
        }
    }
}

fn task_pack_finalize_orchestrator_init(
    state_dir: &std::path::Path,
    explicit_state_dir: bool,
) -> serde_json::Value {
    let mut command = match std::env::current_exe() {
        Ok(path) => std::process::Command::new(path),
        Err(error) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["orchestrator_init_unavailable"],
                "error": format!("Failed to resolve current vida executable: {error}"),
            });
        }
    };
    command.arg("orchestrator-init").arg("--json");
    if explicit_state_dir {
        command.arg("--state-dir").arg(state_dir);
    }
    match command.output() {
        Ok(output) if output.status.success() => {
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(payload) => serde_json::json!({
                    "status": payload["status"].as_str().unwrap_or("unknown"),
                    "active_bounded_unit": payload["active_bounded_unit"].clone(),
                    "why_this_unit": payload["why_this_unit"].clone(),
                    "sequential_vs_parallel_posture": payload["sequential_vs_parallel_posture"].clone(),
                    "blocker_codes": payload["blocker_codes"].clone(),
                    "next_actions": payload["next_actions"].clone(),
                }),
                Err(error) => serde_json::json!({
                    "status": "blocked",
                    "blocker_codes": ["orchestrator_init_json_invalid"],
                    "error": format!("Failed to parse orchestrator-init JSON: {error}"),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                }),
            }
        }
        Ok(output) => serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["orchestrator_init_failed"],
            "exit_code": output.status.code(),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }),
        Err(error) => serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["orchestrator_init_failed"],
            "error": error.to_string(),
        }),
    }
}

fn print_task_pack_finalize_receipt(
    render: RenderMode,
    receipt: &TaskPackFinalizeReceipt,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(receipt)
            .expect("task pack-finalize receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, receipt.surface);
    print_surface_line(render, "status", &receipt.status);
    print_surface_line(
        render,
        "selector",
        &format!("{}={}", receipt.selector_kind, receipt.selector_value),
    );
    print_surface_line(render, "matched", &receipt.matched_count.to_string());
    print_surface_line(render, "finalized", &receipt.finalized_count.to_string());
    print_surface_line(render, "blocked", &receipt.blocked_count.to_string());
    if !receipt.blocker_codes.is_empty() {
        print_surface_line(render, "blockers", &receipt.blocker_codes.join(", "));
    }
    if let Some(action) = receipt.next_actions.first() {
        print_surface_line(render, "next", action);
    }
}

async fn run_task_pack_finalize(command: TaskPackFinalizeArgs) -> ExitCode {
    let explicit_state_dir = command.state_dir.is_some();
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let (selector_kind, selector_value) = match task_pack_finalize_selector(
        command.order_bucket.as_deref(),
        command.parallel_group.as_deref(),
    ) {
        Ok(selector) => selector,
        Err(error) => {
            if command.json {
                crate::print_json_pretty(&serde_json::json!({
                    "surface": "vida task pack-finalize",
                    "status": "blocked",
                    "blocker_codes": ["pack_finalize_selector_required"],
                    "next_actions": [
                        "Rerun with exactly one selector: `vida task pack-finalize --order-bucket <bucket>` or `vida task pack-finalize --parallel-group <group>`."
                    ],
                    "artifact_refs": {"surface": "vida task pack-finalize"},
                    "error": error,
                }));
            } else {
                eprintln!("{error}");
            }
            return ExitCode::from(2);
        }
    };
    let proof_target_overrides = normalized_pack_finalize_values(&command.proof_targets);
    let evidence = normalized_task_verify_evidence(&command.evidence);
    let artifact_refs = normalized_pack_finalize_values(&command.artifact_refs);

    let (task_results, reconcile_summary) = match StateStore::open_existing(state_dir.clone()).await
    {
        Ok(store) => {
            let tasks = match store.list_tasks(None, true).await {
                Ok(tasks) => tasks,
                Err(error) => {
                    eprintln!("Failed to list tasks for pack-finalize: {error}");
                    return ExitCode::from(1);
                }
            };
            let mut candidates = tasks
                .into_iter()
                .filter(|task| {
                    !state_store::StateStore::task_status_is_closed_like(&task.status)
                        && task_matches_pack_selector(task, selector_kind, &selector_value)
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.id.cmp(&right.id));

            let mut results = Vec::new();
            for task in candidates {
                let status_before = task.status.clone();
                let proof_targets = task_pack_finalize_targets(&task, &proof_target_overrides);
                let mut current_task = task.clone();
                let mut blocker_codes = Vec::new();
                let mut next_actions = Vec::new();
                let mut error = None;
                let proof_attached = false;

                if !proof_targets.is_empty() && (!artifact_refs.is_empty() || !evidence.is_empty())
                {
                    blocker_codes
                        .push("pack_finalize_proof_self_certification_forbidden".to_string());
                    next_actions.push(format!(
                        "Attach verified proof to task `{}` with `vida task proof add` or `vida task verify` before retrying pack-finalize.",
                        current_task.id
                    ));
                    error = Some(
                        "pack-finalize cannot create passing proof evidence; proof must already exist before closure"
                            .to_string(),
                    );
                }

                if error.is_none() {
                    let inheritance_rows = store.list_tasks(None, true).await.ok();
                    if task_close_structured_proof_gate_payload(
                        &current_task,
                        inheritance_rows.as_deref(),
                    )
                    .is_some()
                    {
                        blocker_codes.push("missing_structured_proof_evidence".to_string());
                        next_actions.push(format!(
                            "Run `vida task proof status {}` and attach missing structured proof evidence.",
                            crate::shell_quote(&current_task.id)
                        ));
                    } else {
                        let close_reason = task_pack_finalize_close_reason(
                            command.reason.as_deref(),
                            selector_kind,
                            &selector_value,
                            &proof_targets,
                        );
                        match store.close_task(&current_task.id, &close_reason).await {
                            Ok(_) => {
                                if let Err(bridge_error) = crate::runtime_dispatch_state::maybe_bridge_closed_specification_task_into_latest_receipt(&store, &current_task.id).await {
                                    blocker_codes.push("post_close_receipt_bridge_failed".to_string());
                                    next_actions.push(format!(
                                        "Inspect latest dispatch receipt before treating `{}` as fully finalized.",
                                        current_task.id
                                    ));
                                    error = Some(bridge_error.to_string());
                                }
                                if error.is_none() {
                                    if let Err(bridge_error) = crate::runtime_dispatch_state::maybe_bridge_closed_implementer_task_into_latest_receipt(&store, &current_task.id).await {
                                        blocker_codes.push(
                                            "post_close_receipt_bridge_failed".to_string(),
                                        );
                                        next_actions.push(format!(
                                            "Inspect latest dispatch receipt before treating `{}` as fully finalized.",
                                            current_task.id
                                        ));
                                        error = Some(bridge_error.to_string());
                                    }
                                }
                                match store.show_task(&current_task.id).await {
                                    Ok(updated) => current_task = updated,
                                    Err(read_error) => {
                                        blocker_codes
                                            .push("post_close_task_read_failed".to_string());
                                        next_actions.push(format!(
                                            "Re-read task `{}` before relying on pack-finalize closure state.",
                                            current_task.id
                                        ));
                                        error = Some(read_error.to_string());
                                    }
                                }
                            }
                            Err(close_error) => {
                                blocker_codes.push("task_close_failed".to_string());
                                next_actions.push(format!(
                                    "Inspect `vida task closure-ready {}` before retrying pack-finalize.",
                                    crate::shell_quote(&current_task.id)
                                ));
                                error = Some(close_error.to_string());
                            }
                        }
                    }
                }

                let closed =
                    state_store::StateStore::task_status_is_closed_like(&current_task.status);
                results.push(TaskPackFinalizeTaskResult {
                    task_id: current_task.id.clone(),
                    title: current_task.title.clone(),
                    status_before,
                    status_after: current_task.status.clone(),
                    proof_targets,
                    proof_attached,
                    closed,
                    blocker_codes,
                    next_actions,
                    error,
                });
            }

            if let Err(code) =
                refresh_task_snapshot_after_mutation(&store, "vida task pack-finalize").await
            {
                return code;
            }

            let reconcile_summary = match store
                .reconcile_historical_closed_task_active_runs(command.limit)
                .await
            {
                Ok(summary) => Some(
                    serde_json::to_value(summary)
                        .expect("closed-run reconcile summary should serialize"),
                ),
                Err(error) => Some(task_reconcile_closed_runs_error_payload(&error)),
            };
            store.close().await;
            (results, reconcile_summary)
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let orchestrator_init = task_pack_finalize_orchestrator_init(&state_dir, explicit_state_dir);
    let mut blocker_codes = task_results
        .iter()
        .flat_map(|task| task.blocker_codes.iter().cloned())
        .collect::<BTreeSet<_>>();
    if task_results.is_empty() {
        blocker_codes.insert("pack_finalize_no_matching_tasks".to_string());
    }
    if reconcile_summary
        .as_ref()
        .is_some_and(|value| value["status"].as_str() == Some("blocked"))
    {
        blocker_codes.insert("closed_run_reconcile_blocked".to_string());
    }
    if orchestrator_init["status"].as_str() == Some("blocked") {
        blocker_codes.insert("orchestrator_init_blocked".to_string());
    }
    let blocker_codes = blocker_codes.into_iter().collect::<Vec<_>>();
    let finalized_count = task_results.iter().filter(|task| task.closed).count();
    let blocked_count = task_results.len().saturating_sub(finalized_count);
    let mut next_actions = task_results
        .iter()
        .flat_map(|task| task.next_actions.iter().cloned())
        .collect::<Vec<_>>();
    if task_results.is_empty() {
        next_actions.push(format!(
            "Update TaskFlow execution semantics or rerun with a selector that has open tasks: {selector_kind}={selector_value}."
        ));
    }
    if !blocker_codes.is_empty() && next_actions.is_empty() {
        next_actions.push(
            "Inspect the per-task pack-finalize results and resolve residual blockers before selecting unrelated work."
                .to_string(),
        );
    }
    let status = if blocker_codes.is_empty() {
        task_json_success_status().to_string()
    } else {
        "blocked".to_string()
    };
    let receipt = TaskPackFinalizeReceipt {
        surface: "vida task pack-finalize",
        status: status.clone(),
        selector_kind: selector_kind.to_string(),
        selector_value: selector_value.clone(),
        matched_count: task_results.len(),
        finalized_count,
        blocked_count,
        tasks: task_results,
        reconcile_summary,
        orchestrator_init,
        blocker_codes,
        next_actions,
        artifact_refs: serde_json::json!({
            "surface": "vida task pack-finalize",
            "selector_kind": selector_kind,
            "selector_value": selector_value,
            "artifact_refs": artifact_refs,
        }),
    };
    print_task_pack_finalize_receipt(command.render, &receipt, command.json);
    if status == task_json_success_status() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_task_verify_receipt(render: RenderMode, receipt: &TaskVerifyReceipt, as_json: bool) {
    if as_json {
        let payload =
            serde_json::to_value(receipt).expect("task verify receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "partial", &receipt.partial.to_string());
    print_surface_line(render, "source fixed", &receipt.source_fixed.to_string());
    print_surface_line(render, "tests green", &receipt.tests_green.to_string());
    print_surface_line(render, "proof blocked", &receipt.proof_blocked.to_string());
}

fn print_task_browser_proof_receipt(
    render: RenderMode,
    receipt: &TaskProofAttachBrowserReceipt,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(receipt)
            .expect("task browser proof receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "route", &receipt.route);
    print_surface_line(render, "result", &receipt.result);
    print_surface_line(render, "proof target", &receipt.proof_target);
    if let Some(screenshot) = receipt.screenshot.as_deref() {
        print_surface_line(render, "screenshot", screenshot);
    }
}

fn print_task_evidence_proof_receipt(
    render: RenderMode,
    receipt: &TaskProofAttachEvidenceReceipt,
    as_json: bool,
) {
    if as_json {
        let payload = serde_json::to_value(receipt)
            .expect("task proof evidence receipt should serialize to JSON");
        crate::print_json_pretty(&payload);
        return;
    }
    print_task_mutation(render, receipt.surface, &receipt.task, false);
    print_surface_line(render, "result", &receipt.result);
    if receipt.proof_targets.len() > 1 {
        print_surface_line(
            render,
            "proof targets",
            &format!("{} ({})", receipt.proof_targets.len(), receipt.proof_target),
        );
    } else {
        print_surface_line(render, "proof target", &receipt.proof_target);
    }
    print_surface_line(render, "command", &receipt.command);
    if receipt.artifact_refs.len() > 1 {
        print_surface_line(
            render,
            "artifacts",
            &format!(
                "{} ({})",
                receipt.artifact_refs.len(),
                receipt.artifact_refs.join(" | ")
            ),
        );
    } else if let Some(artifact_ref) = receipt.artifact_ref.as_deref() {
        print_surface_line(render, "artifact", artifact_ref);
    }
}

async fn run_task_import_jsonl(command: TaskImportJsonlArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open(state_dir).await {
        Ok(store) => match store.import_tasks_from_jsonl(&command.path).await {
            Ok(summary) => {
                if let Err(code) =
                    refresh_task_snapshot_after_mutation(&store, "vida task import-jsonl").await
                {
                    return code;
                }
                if command.json {
                    let mut summary_json = task_import_jsonl_success_fields(
                        task_json_success_status(),
                        &TaskImportJsonlSummary {
                            source_path: summary.source_path,
                            imported_count: summary.imported_count,
                            unchanged_count: summary.unchanged_count,
                            updated_count: summary.updated_count,
                        },
                    );
                    if let Err(error) = normalize_task_json_contract_arrays(&mut summary_json) {
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
                if command.json {
                    let mut payload = task_import_jsonl_error_payload(
                        &command.path.display().to_string(),
                        &error.to_string(),
                    );
                    if let Err(render_error) = normalize_task_json_contract_arrays(&mut payload) {
                        eprintln!("Failed to render task import-jsonl json: {render_error}");
                        return ExitCode::from(1);
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&payload)
                            .expect("json import error should render")
                    );
                } else {
                    eprintln!("Failed to import tasks from JSONL: {error}");
                }
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_task_replace_jsonl(command: TaskReplaceJsonlArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open(state_dir).await {
        Ok(store) => match store
            .replace_with_task_jsonl_snapshot_file(&command.path)
            .await
        {
            Ok(()) => {
                if let Err(code) =
                    refresh_task_snapshot_after_mutation(&store, "vida task replace-jsonl").await
                {
                    return code;
                }
                let continuation_summary =
                    match sync_replace_jsonl_continuation_binding(&store).await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                let source_path = command.path.display().to_string();
                if command.json {
                    let mut payload = task_replace_jsonl_success_fields(
                        task_json_success_status(),
                        &TaskReplaceJsonlSummary {
                            source_path: source_path.clone(),
                        },
                    );
                    payload["continuation_binding"] = serde_json::to_value(&continuation_summary)
                        .expect("replace-jsonl continuation summary should serialize");
                    crate::print_json_pretty(&payload);
                } else {
                    print_surface_header(command.render, "vida task replace-jsonl");
                    print_surface_line(command.render, "status", "pass");
                    print_surface_line(command.render, "operation", "replace_snapshot");
                    print_surface_line(command.render, "source path", &source_path);
                    print_surface_line(
                        command.render,
                        "continuation binding",
                        &continuation_summary.status,
                    );
                    if let Some(run_id) = continuation_summary.run_id.as_deref() {
                        print_surface_line(command.render, "run id", run_id);
                    }
                    if let Some(task_id) = continuation_summary.task_id.as_deref() {
                        print_surface_line(command.render, "task id", task_id);
                    }
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

async fn run_task_export_jsonl(command: TaskExportJsonlArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    match open_read_only_task_store(state_dir).await {
        Ok(store) => match store.export_tasks_to_jsonl(&command.path).await {
            Ok(exported_count) => {
                print_task_export_summary(
                    command.render,
                    u64::try_from(exported_count).expect("task export count should fit u64"),
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

fn task_import_jsonl_error_payload(path: &str, error: &str) -> serde_json::Value {
    let blocker_codes = vec![crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::DependencyGraphIssues,
    )
    .unwrap_or_else(|| "dependency_graph_issues".to_string())];
    let retry_command =
        operator_output::command_text::human_command("vida task import-jsonl <path> --json");
    let next_actions = vec![format!(
        "Repair the JSONL dependency graph issues, then rerun `{retry_command}`."
    )];
    let artifact_refs = serde_json::json!({
        "surface": "vida task import-jsonl",
        "source_path": path,
    });
    crate::release1_operator_output::build_release1_operator_output_payload(
        "vida task import-jsonl",
        blocker_codes,
        next_actions,
        artifact_refs,
        serde_json::json!({
            "source_path": path,
            "error": error,
        }),
    )
    .expect("task import-jsonl error payload should preserve release-1 operator contract")
}

async fn sync_replace_jsonl_continuation_binding(
    store: &StateStore,
) -> Result<TaskReplaceJsonlContinuationSummary, String> {
    let Some(status) = store.latest_run_graph_status().await.map_err(|error| {
        format!("Failed to read latest run-graph status after replace-jsonl: {error}")
    })?
    else {
        return Ok(TaskReplaceJsonlContinuationSummary {
            status: "no_active_run_graph_status".to_string(),
            run_id: None,
            task_id: None,
            binding_source: None,
        });
    };
    let run_id = status.run_id.clone();
    let task_id = Some(status.task_id.clone()).filter(|value| !value.trim().is_empty());
    let binding = crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &status,
        "task_replace_jsonl_snapshot_restore",
    )
    .await?;
    Ok(TaskReplaceJsonlContinuationSummary {
        status: if binding.is_some() {
            "bound".to_string()
        } else {
            "cleared".to_string()
        },
        run_id: Some(run_id),
        task_id,
        binding_source: binding.map(|binding| binding.binding_source),
    })
}

fn task_next_lawful_projection_name() -> &'static str {
    "task-next-lawful-latest"
}

fn task_show_projection_name(task_id: &str) -> String {
    format!(
        "task-show-{}-latest",
        crate::operator_projection_cache::sanitize_projection_component(task_id, "unknown", 160)
    )
}

fn task_ready_projection_name(scope_task_id: Option<&str>) -> String {
    format!(
        "task-ready-scope-{}-latest",
        crate::operator_projection_cache::sanitize_projection_component(
            scope_task_id.unwrap_or("default"),
            "unknown",
            160,
        )
    )
}

const TASK_READ_RECENT_PROJECTION_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(300);

fn task_graph_issue_from_invalid_record_reason(
    prefix: &str,
    reason: &str,
) -> Option<state_store::TaskGraphIssue> {
    let rest = reason.strip_prefix(prefix)?;
    let (issue_type, issue_id) = rest.split_once(" on ")?;
    let (issue_id, detail) = issue_id
        .split_once(": ")
        .map(|(id, detail)| (id, detail))
        .unwrap_or((issue_id, ""));
    if !matches!(
        issue_type,
        "open_parent_has_no_open_child" | "invalid_parent_child_kind"
    ) || issue_id.trim().is_empty()
    {
        return None;
    }
    Some(state_store::TaskGraphIssue {
        issue_type: issue_type.to_string(),
        issue_id: issue_id.trim().to_string(),
        depends_on_id: None,
        edge_type: Some("parent-child".to_string()),
        detail: if detail.trim().is_empty() {
            match issue_type {
                "invalid_parent_child_kind" => {
                    "parent-child edge violates work item parent kind policy".to_string()
                }
                _ => "open or in-progress parent has no direct non-closed child".to_string(),
            }
        } else {
            detail.trim().to_string()
        },
    })
}

fn task_update_graph_issue_from_invalid_record_reason(
    reason: &str,
) -> Option<state_store::TaskGraphIssue> {
    task_graph_issue_from_invalid_record_reason("task update would create invalid graph: ", reason)
}

fn task_update_close_authority_payload(task_id: &str) -> serde_json::Value {
    let quoted_task_id = crate::shell_quote(task_id);
    let close_command = operator_output::command_text::human_command(&format!(
        "vida task close {} --reason <closure-evidence>",
        quoted_task_id
    ));
    let blocker_code = crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::TaskUpdateCloseAuthorityRequired,
    )
    .unwrap_or_else(|| {
        state_store::StateStore::TASK_UPDATE_CLOSE_AUTHORITY_BLOCKER_CODE.to_string()
    });
    crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task update")
        .blocker_codes(vec![blocker_code])
        .next_actions(vec![format!(
            "Use `{close_command}` after structured proof evidence is attached."
        )])
        .artifact_refs(serde_json::json!({
            "surface": "vida task update",
            "task_id": task_id,
            "close_surface": "vida task close",
            "proof_status_surface": "vida task proof status",
        }))
        .extra_fields(serde_json::json!({
            "closed": false,
            "task_id": task_id,
            "reason": "generic task update cannot close tasks with configured proof targets",
            "required_surface": "vida task close",
        }))
        .build()
        .expect("task update close authority payload should satisfy release-1 contract")
}

fn print_task_update_close_authority_blocked(render: RenderMode, task_id: &str, as_json: bool) {
    if as_json {
        crate::print_json_pretty(&task_update_close_authority_payload(task_id));
        return;
    }
    let quoted_task_id = crate::shell_quote(task_id);
    print_surface_header(render, "vida task update");
    print_surface_line(render, "status", "blocked");
    print_surface_line(
        render,
        "blocker_codes",
        state_store::StateStore::TASK_UPDATE_CLOSE_AUTHORITY_BLOCKER_CODE,
    );
    print_surface_line(
        render,
        "reason",
        "generic task update cannot close tasks with configured proof targets",
    );
    print_surface_line(
        render,
        "next action",
        &format!(
            "vida task close {} --reason <closure-evidence>",
            quoted_task_id
        ),
    );
}

fn task_update_closed_task_mutation_payload(task_id: &str) -> serde_json::Value {
    let quoted_task_id = crate::shell_quote(task_id);
    let reopen_command = operator_output::command_text::human_command(&format!(
        "vida task update {} --status in_progress",
        quoted_task_id
    ));
    let blocker_code = crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::TaskUpdateClosedTaskMutationRequiresReopen,
    )
    .unwrap_or_else(|| {
        state_store::StateStore::TASK_UPDATE_CLOSED_TASK_MUTATION_BLOCKER_CODE.to_string()
    });
    crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task update")
        .blocker_codes(vec![blocker_code])
        .next_actions(vec![format!(
            "Reopen the task with `{reopen_command}` before mutating metadata or proof targets."
        )])
        .artifact_refs(serde_json::json!({
            "surface": "vida task update",
            "task_id": task_id,
            "reopen_surface": "vida task update",
            "repair_surface": "vida task proof attach-evidence",
        }))
        .extra_fields(serde_json::json!({
            "closed": true,
            "task_id": task_id,
            "reason": "closed tasks require explicit reopen before metadata or proof-target mutation",
            "required_surface": "vida task update --status in_progress",
        }))
        .build()
        .expect("closed task update payload should satisfy release-1 contract")
}

fn print_task_update_closed_task_mutation_blocked(
    render: RenderMode,
    task_id: &str,
    as_json: bool,
) {
    if as_json {
        crate::print_json_pretty(&task_update_closed_task_mutation_payload(task_id));
        return;
    }
    let quoted_task_id = crate::shell_quote(task_id);
    print_surface_header(render, "vida task update");
    print_surface_line(render, "status", "blocked");
    print_surface_line(
        render,
        "blocker_codes",
        state_store::StateStore::TASK_UPDATE_CLOSED_TASK_MUTATION_BLOCKER_CODE,
    );
    print_surface_line(
        render,
        "reason",
        "closed tasks require explicit reopen before metadata or proof-target mutation",
    );
    print_surface_line(
        render,
        "next action",
        &format!("vida task update {} --status in_progress", quoted_task_id),
    );
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

fn emit_task_state_store_open_error(
    surface: &str,
    state_dir: &std::path::Path,
    render: RenderMode,
    as_json: bool,
    error: &state_store::StateStoreError,
) -> ExitCode {
    let Some(diagnostic) = error.open_diagnostic(state_dir) else {
        eprintln!("Failed to open authoritative state store: {error}");
        return ExitCode::from(1);
    };
    let payload = crate::release1_operator_output::build_release1_operator_output_payload(
        surface,
        vec![diagnostic.blocker_code.clone()],
        vec![diagnostic.recovery_guidance.clone()],
        serde_json::json!({
            "state_dir": diagnostic.state_dir,
            "suspected_wal_or_sst_hint": diagnostic.suspected_wal_or_sst_hint,
        }),
        serde_json::json!({
            "state_access": {
                "mode": "blocked_storage_corruption",
                "state_dir": diagnostic.state_dir,
                "corruption_state": diagnostic.corruption_state,
                "suspected_wal_or_sst_hint": diagnostic.suspected_wal_or_sst_hint,
                "recovery_guidance": diagnostic.recovery_guidance,
                "silent_delete_allowed": diagnostic.silent_delete_allowed,
                "error": error.to_string(),
            },
        }),
    )
    .expect("task state-store diagnostic payload should preserve release-1 operator contract");
    if as_json {
        crate::print_json_pretty(&payload);
    } else if matches!(render, RenderMode::Plain) {
        operator_output::toon_report::print(
            surface,
            vec![
                operator_output::toon_report::OperatorToonField::text("status", "blocked"),
                operator_output::toon_report::OperatorToonField::value(
                    "blocker_codes",
                    payload["blocker_codes"].clone(),
                ),
                operator_output::toon_report::OperatorToonField::value(
                    "state_access",
                    payload["state_access"].clone(),
                ),
                operator_output::toon_report::OperatorToonField::value(
                    "next_actions",
                    payload["next_actions"].clone(),
                ),
            ],
        );
    } else {
        eprintln!("Failed to open authoritative state store: {error}");
    }
    ExitCode::from(1)
}

pub(crate) async fn open_read_only_task_store(
    state_dir: std::path::PathBuf,
) -> Result<StateStore, state_store::StateStoreError> {
    StateStore::open_existing_read_only(state_dir).await
}

fn is_authoritative_state_lock_error(error: &state_store::StateStoreError) -> bool {
    StateStore::error_is_lock_contention(error)
}

fn load_task_snapshot_rows(
    state_dir: &std::path::Path,
) -> Result<Vec<state_store::TaskRecord>, state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path)
}

pub(crate) async fn load_task_snapshot_rows_with_retry(
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

async fn load_task_snapshot_rows_fallback_with_metadata(
    state_dir: &std::path::Path,
    snapshot_path: &std::path::Path,
    detail: &'static str,
    authoritative_error: state_store::StateStoreError,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    match load_task_snapshot_rows_with_retry(state_dir).await {
        Ok(rows) => Ok((rows, TaskReadMetadata::snapshot(snapshot_path, detail))),
        Err(state_store::StateStoreError::Io(_)) => Err(authoritative_error),
        Err(snapshot_error) => Err(snapshot_error),
    }
}

async fn load_task_snapshot_rows_authoritative_first(
    state_dir: &std::path::Path,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(state_dir);
    match open_read_only_task_store(state_dir.to_path_buf()).await {
        Ok(store) => match store.list_tasks(None, true).await {
            Ok(rows) => match StateStore::read_fresh_tasks_from_jsonl_snapshot(state_dir) {
                Ok(snapshot_rows) if task_snapshot_is_richer_than_live(&snapshot_rows, &rows) => Ok((
                    snapshot_rows,
                    TaskReadMetadata::fresh_snapshot_live_divergence(&snapshot_path),
                )),
                _ => Ok((rows, TaskReadMetadata::authoritative_live())),
            },
            Err(error) if is_authoritative_state_lock_error(&error) => {
                load_task_snapshot_rows_fallback_with_metadata(
                    state_dir,
                    &snapshot_path,
                    "served from canonical task snapshot evidence after authoritative state lock contention",
                    error,
                )
                .await
            }
            Err(error) => Err(error),
        },
        Err(error @ state_store::StateStoreError::MissingStateDir(_)) => {
            load_task_snapshot_rows_fallback_with_metadata(
                state_dir,
                &snapshot_path,
                "served from canonical task snapshot evidence because authoritative state store is missing",
                error,
            )
            .await
        }
        Err(error) if is_authoritative_state_lock_error(&error) => {
            load_task_snapshot_rows_fallback_with_metadata(
                state_dir,
                &snapshot_path,
                "served from canonical task snapshot evidence after authoritative state lock contention",
                error,
            )
            .await
        }
        Err(error) => Err(error),
    }
}

fn task_snapshot_is_richer_than_live(
    snapshot_rows: &[state_store::TaskRecord],
    live_rows: &[state_store::TaskRecord],
) -> bool {
    if snapshot_rows.len() <= live_rows.len() {
        return false;
    }
    let live_ids = live_rows
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    snapshot_rows
        .iter()
        .any(|task| !live_ids.contains(task.id.as_str()))
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
    crate::operator_projection_cache::touch_state_mutation_marker(store.root());
    StateStore::touch_task_snapshot_state_marker(store.root());
    store
        .refresh_task_snapshot()
        .await
        .map(|_| ())
        .map_err(|error| {
            eprintln!("Failed to refresh canonical task snapshot after {surface}: {error}");
            ExitCode::from(1)
        })
}

async fn refresh_task_snapshot_for_task_after_mutation(
    store: &StateStore,
    task: &state_store::TaskRecord,
    surface: &str,
) -> Result<(), ExitCode> {
    crate::operator_projection_cache::touch_state_mutation_marker(store.root());
    StateStore::touch_task_snapshot_state_marker(store.root());
    store
        .refresh_task_snapshot_for_task(task)
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

async fn task_progress_summary_read_only(
    state_dir: &std::path::Path,
    task_id: &str,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    let (rows, _metadata) = load_task_snapshot_rows_authoritative_first(state_dir).await?;
    StateStore::task_progress_summary_from_rows(&rows, task_id)
}

fn task_dependency_bulk_edge_input(
    edge: TaskDependencyBulkEdge,
) -> state_store::TaskDependencyBulkAddInput {
    state_store::TaskDependencyBulkAddInput {
        issue_id: edge.issue_id,
        depends_on_id: edge.depends_on_id,
        edge_type: edge.edge_type,
    }
}

fn task_dependency_bulk_edge_inputs(
    inline_edges: &[String],
    edge_file: Option<&std::path::Path>,
) -> Result<Vec<state_store::TaskDependencyBulkAddInput>, String> {
    let mut raw_edges = inline_edges.to_vec();
    if let Some(path) = edge_file {
        let content = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read dependency edge file `{}`: {error}",
                path.display()
            )
        })?;
        raw_edges.extend(task_dependency_bulk_edge_lines(content.lines()));
    }
    if raw_edges.is_empty() {
        return Err("at least one --edge or --edge-file entry is required".to_string());
    }
    parse_task_dependency_bulk_edges(raw_edges.iter().map(String::as_str)).map(|edges| {
        edges
            .into_iter()
            .map(task_dependency_bulk_edge_input)
            .collect()
    })
}

async fn task_list_authoritative_first(
    state_dir: std::path::PathBuf,
    status: Option<&str>,
    include_all: bool,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let include_closed = task_list_include_closed_rows(status, include_all);
    let snapshot_path = StateStore::canonical_task_snapshot_path_for_state_root(&state_dir);
    for attempt in 0..80 {
        match open_read_only_task_store(state_dir.clone()).await {
            Ok(store) => match store.list_tasks(status, include_closed).await {
                Ok(rows) => return Ok((rows, TaskReadMetadata::authoritative_live())),
                Err(error) if is_authoritative_state_lock_error(&error) && attempt < 79 => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    return load_task_snapshot_rows_fallback_with_metadata(
                        &state_dir,
                        &snapshot_path,
                        "served from canonical task snapshot evidence after authoritative state lock contention",
                        error,
                    )
                    .await
                    .map(|(rows, metadata)| {
                        (
                            filter_task_rows_for_operator(rows, status, include_all, None, None, None, None),
                            metadata,
                        )
                    });
                }
                Err(error) => return Err(error),
            },
            Err(error @ state_store::StateStoreError::MissingStateDir(_)) => {
                return load_task_snapshot_rows_fallback_with_metadata(
                    &state_dir,
                    &snapshot_path,
                    "served from canonical task snapshot evidence because authoritative state store is missing",
                    error,
                )
                .await
                .map(|(rows, metadata)| {
                    (
                        filter_task_rows_for_operator(rows, status, include_all, None, None, None, None),
                        metadata,
                    )
                });
            }
            Err(error) if is_authoritative_state_lock_error(&error) && attempt < 79 => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            Err(error) if is_authoritative_state_lock_error(&error) => {
                return load_task_snapshot_rows_fallback_with_metadata(
                    &state_dir,
                    &snapshot_path,
                    "served from canonical task snapshot evidence after authoritative state lock contention",
                    error,
                )
                .await
                .map(|(rows, metadata)| {
                    (
                        filter_task_rows_for_operator(rows, status, include_all, None, None, None, None),
                        metadata,
                    )
                });
            }
            Err(error) => return Err(error),
        }
    }
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let filtered = rows
        .into_iter()
        .filter(|task| task_row_visible_by_closed_policy(task, status, include_all))
        .filter(|task| task_status_filter_matches(task, status))
        .collect();
    Ok((filtered, metadata))
}

fn task_list_include_closed_rows(status: Option<&str>, include_all: bool) -> bool {
    include_all
        || status.is_some_and(|wanted| StateStore::task_status_matches_filter("closed", wanted))
}

fn task_status_filter_matches(task: &state_store::TaskRecord, status: Option<&str>) -> bool {
    status
        .map(|wanted| StateStore::task_status_matches_filter(&task.status, wanted))
        .unwrap_or(true)
}

fn task_row_visible_by_closed_policy(
    task: &state_store::TaskRecord,
    status: Option<&str>,
    include_all: bool,
) -> bool {
    include_all
        || !StateStore::task_status_is_closed_like(&task.status)
        || task_list_include_closed_rows(status, include_all)
}

fn task_parent_filter_matches(task: &state_store::TaskRecord, parent_id: Option<&str>) -> bool {
    parent_id
        .map(|parent_id| {
            task.dependencies.iter().any(|dependency| {
                dependency.edge_type == "parent-child" && dependency.depends_on_id == parent_id
            })
        })
        .unwrap_or(true)
}

fn task_query_filter_matches(task: &state_store::TaskRecord, query: Option<&str>) -> bool {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return true;
    };
    let query = query.to_ascii_lowercase();
    [
        task.id.as_str(),
        task.title.as_str(),
        task.description.as_str(),
        task.notes.as_deref().unwrap_or(""),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(&query))
}

fn task_issue_type_filter_matches(
    task: &state_store::TaskRecord,
    issue_type: Option<&str>,
) -> bool {
    issue_type
        .map(|wanted| {
            state_store::canonical_work_item_issue_type(&task.issue_type)
                == state_store::canonical_work_item_issue_type(wanted)
        })
        .unwrap_or(true)
}

fn filter_task_rows_for_operator(
    rows: Vec<state_store::TaskRecord>,
    status: Option<&str>,
    include_all: bool,
    query: Option<&str>,
    issue_type: Option<&str>,
    parent_id: Option<&str>,
    limit: Option<usize>,
) -> Vec<state_store::TaskRecord> {
    rows.into_iter()
        .filter(|task| task_row_visible_by_closed_policy(task, status, include_all))
        .filter(|task| task_status_filter_matches(task, status))
        .filter(|task| task_issue_type_filter_matches(task, issue_type))
        .filter(|task| task_parent_filter_matches(task, parent_id))
        .filter(|task| task_query_filter_matches(task, query))
        .take(limit.unwrap_or(usize::MAX))
        .collect()
}

struct TaskListLikeInput<'a> {
    surface: &'static str,
    state_dir: std::path::PathBuf,
    render: crate::RenderMode,
    status: Option<&'a str>,
    all: bool,
    query: Option<&'a str>,
    issue_type: Option<&'a str>,
    parent_id: Option<&'a str>,
    limit: Option<usize>,
    fields: Option<&'a str>,
    view: &'a str,
    summary: bool,
    json: bool,
}

async fn run_task_list_like(command: TaskListLikeInput<'_>) -> ExitCode {
    match task_list_authoritative_first(command.state_dir, command.status, command.all).await {
        Ok((tasks, metadata)) => {
            let tasks = filter_task_rows_for_operator(
                tasks,
                command.status,
                command.all,
                command.query,
                command.issue_type,
                command.parent_id,
                command.limit,
            );
            let view = if command.view == "full" {
                "full"
            } else if command.view == "compact" {
                "compact"
            } else {
                "summary"
            };
            let view = if command.summary { "summary" } else { view };
            print_task_list(
                command.surface,
                command.render,
                &tasks,
                view,
                view == "full",
                command.fields,
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

async fn run_task_list(command: TaskListArgs) -> ExitCode {
    run_task_list_like(TaskListLikeInput {
        surface: "vida task list",
        state_dir: command
            .state_dir
            .unwrap_or_else(state_store::default_state_dir),
        render: command.render,
        status: command.status.as_deref(),
        all: command.all,
        query: command.query.as_deref(),
        issue_type: command.issue_type.as_deref(),
        parent_id: command.parent_id.as_deref(),
        limit: command.limit,
        fields: command.fields.as_deref(),
        view: &command.view,
        summary: command.summary,
        json: command.json,
    })
    .await
}

async fn run_task_search(command: TaskSearchArgs) -> ExitCode {
    run_task_list_like(TaskListLikeInput {
        surface: "vida task search",
        state_dir: command
            .state_dir
            .unwrap_or_else(state_store::default_state_dir),
        render: command.render,
        status: command.status.as_deref(),
        all: command.all,
        query: Some(&command.query),
        issue_type: command.issue_type.as_deref(),
        parent_id: command.parent_id.as_deref(),
        limit: Some(command.limit),
        fields: command.fields.as_deref(),
        view: &command.view,
        summary: false,
        json: command.json,
    })
    .await
}

async fn task_show_authoritative_first(
    state_dir: std::path::PathBuf,
    task_id: &str,
) -> Result<(state_store::TaskRecord, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let task = resolve_task_from_rows(&rows, task_id)?;
    Ok((task, metadata))
}

async fn task_ready_authoritative_first(
    state_dir: std::path::PathBuf,
    scope_task_id: Option<&str>,
) -> Result<(Vec<state_store::TaskRecord>, TaskReadMetadata), state_store::StateStoreError> {
    let (rows, metadata) = load_task_snapshot_rows_authoritative_first(&state_dir).await?;
    let tasks = StateStore::ready_tasks_scoped_from_rows(&rows, scope_task_id)?;
    Ok((tasks, metadata))
}

async fn task_critical_path_snapshot_first(
    state_dir: std::path::PathBuf,
) -> Result<state_store::TaskCriticalPath, state_store::StateStoreError> {
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => store.critical_path().await,
        Err(error @ state_store::StateStoreError::MissingStateDir(_)) => {
            match load_task_snapshot_rows_with_retry(&state_dir).await {
                Ok(rows) => StateStore::critical_path_from_rows(&rows),
                Err(state_store::StateStoreError::Io(_)) => Err(error),
                Err(snapshot_error) => Err(snapshot_error),
            }
        }
        Err(error) if is_authoritative_state_lock_error(&error) => {
            let rows = load_task_snapshot_rows_with_retry(&state_dir).await?;
            StateStore::critical_path_from_rows(&rows)
        }
        Err(error) => Err(error),
    }
}

fn task_rows_as_values(
    tasks: &[state_store::TaskRecord],
) -> Result<Vec<serde_json::Value>, String> {
    tasks
        .iter()
        .map(|task| serde_json::to_value(task).map_err(|error| error.to_string()))
        .collect()
}

fn task_close_epic_progress_summary(
    rows: &[state_store::TaskRecord],
    closed_task_id: &str,
    include_global_progress: bool,
) -> Result<TaskCloseEpicProgressSummary, state_store::StateStoreError> {
    let task_by_id = rows
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut children_by_parent =
        std::collections::BTreeMap::<String, Vec<&state_store::TaskRecord>>::new();
    for task in rows {
        if let Some(parent_id) = task_parent_id(task) {
            children_by_parent.entry(parent_id).or_default().push(task);
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.status.cmp(&right.status))
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let scoped_epic_ids = task_close_scoped_epic_ids(rows, closed_task_id);
    let total_epic_count = rows.iter().filter(|task| task.issue_type == "epic").count();
    let mut epics = rows
        .iter()
        .filter(|task| task.issue_type == "epic")
        .filter(|task| include_global_progress || scoped_epic_ids.contains(&task.id))
        .collect::<Vec<_>>();
    epics.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.status.cmp(&right.status))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut epic_rows = Vec::with_capacity(epics.len());
    for epic in epics {
        let progress = StateStore::task_progress_summary_from_rows(rows, &epic.id)?;
        let children = children_by_parent
            .get(&epic.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let tasks = children
            .iter()
            .filter(|task| state_store::work_item_contributes_to_task_stats(&task.issue_type))
            .take(TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT)
            .map(|task| task_close_epic_progress_task_row(task, &task_by_id))
            .collect::<Vec<_>>();
        let reportable_child_count = children
            .iter()
            .filter(|task| state_store::work_item_contributes_to_task_stats(&task.issue_type))
            .count();
        epic_rows.push(TaskCloseEpicProgressRow {
            epic_id: epic.id.clone(),
            epic_title: epic.title.clone(),
            epic_status: epic.status.clone(),
            epic_priority: epic.priority,
            closed_count: progress.closed_count,
            total_count: progress.descendant_count,
            percent_closed: progress.percent_closed,
            child_task_count: reportable_child_count,
            reported_child_task_count: tasks.len(),
            child_task_report_limit: TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT,
            truncated_child_tasks: reportable_child_count > TASK_CLOSE_EPIC_PROGRESS_CHILD_LIMIT,
            tasks,
        });
    }

    Ok(TaskCloseEpicProgressSummary {
        closed_task_id: closed_task_id.to_string(),
        epic_count: epic_rows.len(),
        reported_epic_count: epic_rows.len(),
        omitted_epic_count: total_epic_count.saturating_sub(epic_rows.len()),
        scope: if include_global_progress {
            "all_epics"
        } else {
            "closed_task_ancestor_epics"
        }
        .to_string(),
        epics: epic_rows,
    })
}

fn task_close_scoped_epic_ids(
    rows: &[state_store::TaskRecord],
    closed_task_id: &str,
) -> std::collections::BTreeSet<String> {
    let by_id = rows
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut scoped = std::collections::BTreeSet::new();
    let mut current_id = Some(closed_task_id.to_string());
    let mut visited = std::collections::BTreeSet::new();
    while let Some(task_id) = current_id {
        if !visited.insert(task_id.clone()) {
            break;
        }
        let Some(task) = by_id.get(task_id.as_str()) else {
            break;
        };
        if task.issue_type == "epic" {
            scoped.insert(task.id.clone());
        }
        current_id = task_parent_id(task);
    }
    scoped
}

fn task_epic_progress_summary(
    rows: &[state_store::TaskRecord],
    metadata: TaskReadMetadata,
    include_closed_epics: bool,
    epic_filter: Option<&str>,
    basis: &str,
) -> Result<TaskEpicProgressSummary, state_store::StateStoreError> {
    if let Some(epic_id) = epic_filter {
        let Some(epic) = rows.iter().find(|task| task.id == epic_id) else {
            return Err(state_store::StateStoreError::MissingTask {
                task_id: epic_id.to_string(),
            });
        };
        if epic.issue_type != "epic" {
            return Err(state_store::StateStoreError::InvalidTaskRecord {
                reason: format!("task `{epic_id}` is not an epic"),
            });
        }
    }
    let mut epics = rows
        .iter()
        .filter(|task| task.issue_type == "epic")
        .filter(|task| {
            epic_filter
                .map(|epic_id| task.id == epic_id)
                .unwrap_or(true)
        })
        .filter(|task| {
            include_closed_epics || matches!(task.status.as_str(), "open" | "in_progress")
        })
        .collect::<Vec<_>>();
    epics.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.status.cmp(&right.status))
            .then_with(|| left.id.cmp(&right.id))
    });
    let progress_precompute = TaskProgressPrecompute::new(rows);

    let mut open_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut closed_count = 0usize;
    let mut total_descendant_count = 0usize;
    let mut total_open_descendant_count = 0usize;
    let mut total_in_progress_descendant_count = 0usize;
    let mut total_closed_descendant_count = 0usize;
    let mut epic_rows = Vec::with_capacity(epics.len());

    for epic in epics {
        match epic.status.as_str() {
            "open" => open_count += 1,
            "in_progress" => in_progress_count += 1,
            "closed" => closed_count += 1,
            _ => {}
        }

        let progress = task_progress_summary_for_basis_with_precompute(
            rows,
            &progress_precompute,
            &epic.id,
            basis,
        )?;
        total_descendant_count += progress.descendant_count;
        total_open_descendant_count += progress.open_count;
        total_in_progress_descendant_count += progress.in_progress_count;
        total_closed_descendant_count += progress.closed_count;

        epic_rows.push(TaskEpicProgressRow {
            epic_id: epic.id.clone(),
            epic_title: epic.title.clone(),
            epic_status: epic.status.clone(),
            epic_priority: epic.priority,
            total_count: progress.descendant_count,
            open_count: progress.open_count,
            in_progress_count: progress.in_progress_count,
            closed_count: progress.closed_count,
            percent_complete: progress.percent_closed,
            direct_child_count: progress.direct_child_count,
            nested_epic_count: progress.epic_count,
            closure_candidate: progress.closure_candidate,
            closure_candidate_state: progress.closure_candidate_state,
            recommended_next_action: progress.recommended_next_action,
        });
    }

    let percent_closed = if total_descendant_count == 0 {
        0.0
    } else {
        (total_closed_descendant_count as f64 / total_descendant_count as f64) * 100.0
    };

    Ok(TaskEpicProgressSummary {
        epic_count: epic_rows.len(),
        open_count,
        in_progress_count,
        closed_count,
        total_descendant_count,
        total_open_descendant_count,
        total_in_progress_descendant_count,
        total_closed_descendant_count,
        percent_closed,
        include_closed_epics,
        progress_basis: basis.to_string(),
        epic_filter: epic_filter.map(ToOwned::to_owned),
        epics: epic_rows,
        read_metadata: metadata,
    })
}

fn task_progress_basis_arg(value: &str) -> Result<&'static str, String> {
    Ok(parse_task_progress_basis(value)?.as_str())
}

struct TaskProgressPrecompute {
    core_rows: Vec<TaskProgressRow>,
    row_index_by_id: BTreeMap<String, usize>,
    child_indexes_by_parent: BTreeMap<String, Vec<usize>>,
}

impl TaskProgressPrecompute {
    fn new(rows: &[state_store::TaskRecord]) -> Self {
        let mut core_rows = Vec::with_capacity(rows.len());
        let mut row_index_by_id = BTreeMap::new();
        let mut child_indexes_by_parent = BTreeMap::<String, Vec<usize>>::new();

        for (index, task) in rows.iter().enumerate() {
            core_rows.push(task_progress_row_from_record(task));
            row_index_by_id.insert(task.id.clone(), index);
            if let Some(parent_id) = StateStore::parent_id_for_task(task) {
                child_indexes_by_parent
                    .entry(parent_id)
                    .or_default()
                    .push(index);
            }
        }

        Self {
            core_rows,
            row_index_by_id,
            child_indexes_by_parent,
        }
    }
}

fn task_progress_summary_for_basis(
    rows: &[state_store::TaskRecord],
    task_id: &str,
    basis: &str,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    let precompute = TaskProgressPrecompute::new(rows);
    task_progress_summary_for_basis_with_precompute(rows, &precompute, task_id, basis)
}

fn task_progress_summary_for_basis_with_precompute(
    rows: &[state_store::TaskRecord],
    precompute: &TaskProgressPrecompute,
    task_id: &str,
    basis: &str,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    let progress_basis = parse_task_progress_basis(basis)
        .map_err(|reason| state_store::StateStoreError::InvalidTaskRecord { reason })?;
    let core_summary = core_task_progress_summary_from_rows(
        &precompute.core_rows,
        task_id,
        progress_basis,
        crate::launcher_task_commands::shell_quote,
        operator_output::command_text::human_command,
    )
    .map_err(|_| state_store::StateStoreError::MissingTask {
        task_id: task_id.to_string(),
    })?;
    task_progress_summary_from_core(rows, precompute, core_summary)
}

async fn task_stage_ensemble_operator_summary(
    store: &StateStore,
    task_id: &str,
) -> Result<serde_json::Value, state_store::StateStoreError> {
    let task = store.show_task(task_id).await?;
    let attempts = store.task_attempts_for_task(task_id).await?;
    let stage_summaries = store.task_stage_summaries_for_task(task_id).await?;
    Ok(task_stage_ensemble_operator_summary_value(
        &task,
        &attempts,
        &stage_summaries,
    ))
}

async fn task_stage_ensemble_operator_summary_from_state_dir(
    state_dir: &std::path::Path,
    task_id: &str,
) -> Option<serde_json::Value> {
    let store = StateStore::open_existing_read_only(state_dir.to_path_buf())
        .await
        .ok()?;
    task_stage_ensemble_operator_summary(&store, task_id)
        .await
        .ok()
}

pub(crate) fn task_stage_ensemble_operator_summary_value(
    task: &state_store::TaskRecord,
    attempts: &[state_store::TaskAttemptRecord],
    stage_summaries: &[state_store::TaskStageSummary],
) -> serde_json::Value {
    let mut stage_ids = std::collections::BTreeSet::<String>::new();
    let mut status_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut latest_attempt: Option<&state_store::TaskAttemptRecord> = None;
    let mut attempt_consolidation_receipt_id: Option<String> = None;
    let mut stale_count = 0usize;

    for attempt in attempts {
        stage_ids.insert(attempt.stage_id.clone());
        *status_counts.entry(attempt.status.clone()).or_insert(0) += 1;
        if attempt.status == "stale" || attempt.freshness != task.updated_at {
            stale_count += 1;
        }
        if attempt_consolidation_receipt_id.is_none() {
            attempt_consolidation_receipt_id = attempt.consolidation_receipt_id.clone();
        } else if attempt.consolidation_receipt_id.is_some()
            && latest_attempt
                .map(|latest| attempt.updated_at >= latest.updated_at)
                .unwrap_or(true)
        {
            attempt_consolidation_receipt_id = attempt.consolidation_receipt_id.clone();
        }
        if latest_attempt
            .map(|latest| {
                attempt
                    .updated_at
                    .cmp(&latest.updated_at)
                    .then_with(|| attempt.attempt_id.cmp(&latest.attempt_id))
                    .is_gt()
            })
            .unwrap_or(true)
        {
            latest_attempt = Some(attempt);
        }
    }

    let active_stage = latest_attempt.map(|attempt| attempt.stage_id.clone());
    let active_stage_summary = active_stage.as_deref().and_then(|stage_id| {
        stage_summaries
            .iter()
            .find(|summary| summary.stage_id == stage_id)
    });
    let latest_attempt_status = active_stage_summary
        .and_then(|summary| summary.latest_attempt_status.clone())
        .or_else(|| latest_attempt.map(|attempt| attempt.status.clone()));
    let latest_consolidation_receipt_id = active_stage_summary
        .and_then(|summary| summary.latest_consolidation_receipt_id.clone())
        .or(attempt_consolidation_receipt_id);
    let next_command = task_stage_ensemble_next_command(
        task,
        active_stage.as_deref(),
        latest_attempt_status.as_deref(),
        latest_consolidation_receipt_id.as_deref(),
        attempts.is_empty(),
    );

    serde_json::json!({
        "task_id": task.id,
        "active_stage": active_stage,
        "configured_stage_count": stage_ids.len(),
        "configured_attempt_count": attempts.len(),
        "status_counts": status_counts,
        "running_count": status_counts.get("running").copied().unwrap_or(0),
        "produced_count": status_counts.get("produced").copied().unwrap_or(0),
        "accepted_count": status_counts.get("accepted").copied().unwrap_or(0),
        "rejected_count": status_counts.get("rejected").copied().unwrap_or(0),
        "stale_count": stale_count,
        "latest_attempt_id": latest_attempt.map(|attempt| attempt.attempt_id.clone()),
        "latest_attempt_status": latest_attempt_status,
        "latest_consolidation_receipt_id": latest_consolidation_receipt_id,
        "next_command": next_command,
    })
}

fn task_stage_ensemble_next_command(
    task: &state_store::TaskRecord,
    active_stage: Option<&str>,
    latest_attempt_status: Option<&str>,
    latest_consolidation_receipt_id: Option<&str>,
    no_attempts: bool,
) -> String {
    if state_store::work_item_is_program_container(&task.issue_type) {
        let quoted_task_id = crate::launcher_task_commands::shell_quote(&task.id);
        return format!("vida task ready --scope {quoted_task_id} --limit 10");
    }
    let task_id = &task.id;
    let quoted_task_id = crate::launcher_task_commands::shell_quote(task_id);
    let stage = active_stage.unwrap_or("implementation");
    let quoted_stage = crate::launcher_task_commands::shell_quote(stage);
    match taskflow_core::task::attempts::attempt_stage_next_action(
        latest_attempt_status,
        latest_consolidation_receipt_id.is_some(),
        no_attempts,
    ) {
        taskflow_core::task::attempts::AttemptStageNextAction::Dispatch => {
            format!("vida task attempt dispatch {quoted_task_id} --stage {quoted_stage}")
        }
        taskflow_core::task::attempts::AttemptStageNextAction::Collect => {
            format!("vida task attempt collect {quoted_task_id} --stage {quoted_stage}")
        }
        taskflow_core::task::attempts::AttemptStageNextAction::Consolidate => {
            format!("vida task attempt consolidate {quoted_task_id} --stage {quoted_stage}")
        }
        taskflow_core::task::attempts::AttemptStageNextAction::Status => {
            format!("vida task stage status {quoted_task_id} --stage {quoted_stage}")
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct TaskStepRow {
    id: String,
    status: String,
    parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_title: Option<String>,
    created: String,
    closed: Option<String>,
    close_reason: Option<String>,
    owned_paths: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct TaskStepsReceipt {
    surface: &'static str,
    status: &'static str,
    since: String,
    count: usize,
    parent_id: Option<String>,
    status_filter: Option<String>,
    steps: Vec<TaskStepRow>,
}

fn print_task_steps_help() {
    println!(
        "vida task steps\n  Execution steps are non-bounded child records under a task, subtask, or defect.\n  Default output is compact TOON/plain; use --json for machine-readable rows.\n  Examples:\n    vida task steps --since 3h --with-parent\n    vida task steps --parent-id <task-id> --status in_progress --json\n  Fields: id, status, parent_id, parent_title, created, closed, close_reason, owned_paths.\n  Inspect attribution with: vida orchestrator-init --fields status,active_bounded_unit,active_step,active_parent_task,active_epic\n  Related: vida doctor active-task-attribution --help"
    );
}

fn parse_task_steps_since(value: &str) -> Result<std::time::Duration, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("--since must not be empty".to_string());
    }
    let (number, multiplier) = match value.chars().last() {
        Some('s') => (&value[..value.len() - 1], 1),
        Some('m') => (&value[..value.len() - 1], 60),
        Some('h') => (&value[..value.len() - 1], 60 * 60),
        Some('d') => (&value[..value.len() - 1], 24 * 60 * 60),
        Some(ch) if ch.is_ascii_digit() => (value, 1),
        Some(_) | None => return Err(format!("unsupported --since value: {value}")),
    };
    let amount = number
        .parse::<u64>()
        .map_err(|_| format!("unsupported --since value: {value}"))?;
    Ok(std::time::Duration::from_secs(
        amount.saturating_mul(multiplier),
    ))
}

fn task_step_timestamp(value: &str) -> Option<i64> {
    if let Ok(raw) = value.trim().parse::<i64>() {
        return Some(if raw > 10_000_000_000 {
            raw / 1_000_000_000
        } else {
            raw
        });
    }
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|value| value.unix_timestamp())
}

fn task_step_recent_enough(task: &state_store::TaskRecord, cutoff: i64) -> bool {
    let created = task_step_timestamp(&task.created_at);
    let closed = task.closed_at.as_deref().and_then(task_step_timestamp);
    created.into_iter().chain(closed).any(|ts| ts >= cutoff)
}

fn task_step_rows(
    rows: Vec<state_store::TaskRecord>,
    since: std::time::Duration,
    parent_id: Option<&str>,
    status: Option<&str>,
    with_parent: bool,
) -> Vec<TaskStepRow> {
    let cutoff = time::OffsetDateTime::now_utc().unix_timestamp() - since.as_secs() as i64;
    let task_by_id: BTreeMap<&str, &state_store::TaskRecord> =
        rows.iter().map(|task| (task.id.as_str(), task)).collect();
    let mut steps = rows
        .iter()
        .filter(|task| {
            taskflow_core::issue_type_is_execution_step(&taskflow_core::normalize_issue_type(
                &task.issue_type,
            ))
        })
        .filter(|task| task_step_recent_enough(task, cutoff))
        .filter(|task| {
            parent_id
                .map(|expected| StateStore::parent_id_for_task(task).as_deref() == Some(expected))
                .unwrap_or(true)
        })
        .filter(|task| {
            status
                .map(|expected| StateStore::task_status_matches_filter(&task.status, expected))
                .unwrap_or(true)
        })
        .map(|task| {
            let parent_id = StateStore::parent_id_for_task(task);
            let parent_title = if with_parent {
                parent_id
                    .as_deref()
                    .and_then(|id| task_by_id.get(id).map(|parent| parent.title.clone()))
            } else {
                None
            };
            TaskStepRow {
                id: task.id.clone(),
                status: task.status.clone(),
                parent_id,
                parent_title,
                created: task.created_at.clone(),
                closed: task.closed_at.clone(),
                close_reason: task.close_reason.clone(),
                owned_paths: task.planner_metadata.owned_paths.clone(),
            }
        })
        .collect::<Vec<_>>();
    steps.sort_by(|left, right| {
        right
            .created
            .cmp(&left.created)
            .then_with(|| left.id.cmp(&right.id))
    });
    steps
}

fn print_task_steps_receipt(render: RenderMode, receipt: &TaskStepsReceipt, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(receipt).expect("task steps receipt should serialize")
        );
        return;
    }
    let _ = render;
    println!(
        "task_steps[{}]{{id,status,parent_id,parent_title,created,closed,close_reason,owned_paths}}:",
        receipt.count
    );
    for row in &receipt.steps {
        println!(
            "  {}",
            serde_json::to_string(&serde_json::json!([
                row.id,
                row.status,
                row.parent_id,
                row.parent_title,
                row.created,
                row.closed,
                row.close_reason,
                row.owned_paths
            ]))
            .expect("task step row should serialize")
        );
    }
}

async fn run_task_steps(command: TaskStepsArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let since = match parse_task_steps_since(&command.since) {
        Ok(value) => value,
        Err(error) => {
            if command.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "surface": "vida task steps",
                        "status": "blocked",
                        "blocker_codes": ["invalid_since_filter"],
                        "error": error,
                    })
                );
            } else {
                eprintln!("{error}");
            }
            return ExitCode::from(2);
        }
    };
    let (rows, _metadata) = match load_task_snapshot_rows_authoritative_first(&state_dir).await {
        Ok(value) => value,
        Err(error) => {
            return emit_task_state_store_open_error(
                "vida task steps",
                &state_dir,
                command.render,
                command.json,
                &error,
            )
        }
    };
    let steps = task_step_rows(
        rows,
        since,
        command.parent_id.as_deref(),
        command.status.as_deref(),
        command.with_parent,
    );
    let receipt = TaskStepsReceipt {
        surface: "vida task steps",
        status: "success",
        since: command.since,
        count: steps.len(),
        parent_id: command.parent_id,
        status_filter: command.status,
        steps,
    };
    print_task_steps_receipt(command.render, &receipt, command.json);
    ExitCode::SUCCESS
}

fn task_progress_row_from_record(task: &state_store::TaskRecord) -> TaskProgressRow {
    TaskProgressRow {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        priority: task.priority,
        labels: task.labels.clone(),
        proof_targets: task.planner_metadata.proof_targets.clone(),
        proof_satisfied: all_structured_task_proof_targets_satisfied(
            task.notes.as_deref(),
            &task.planner_metadata.proof_targets,
        ),
        parent_id: task_parent_id(task),
    }
}

fn task_progress_summary_from_core(
    rows: &[state_store::TaskRecord],
    precompute: &TaskProgressPrecompute,
    core: CoreTaskProgressSummary,
) -> Result<state_store::TaskProgressSummary, state_store::StateStoreError> {
    let root_task = precompute
        .row_index_by_id
        .get(&core.root_task.id)
        .and_then(|index| rows.get(*index))
        .cloned()
        .ok_or_else(|| state_store::StateStoreError::MissingTask {
            task_id: core.root_task.id.clone(),
        })?;
    Ok(state_store::TaskProgressSummary {
        root_task,
        progress_basis: core.progress_basis,
        direct_child_count: core.direct_child_count,
        descendant_count: core.descendant_count,
        open_count: core.open_count,
        in_progress_count: core.in_progress_count,
        closed_count: core.closed_count,
        epic_count: core.epic_count,
        status_counts: core.status_counts,
        percent_closed: core.percent_closed,
        closure_candidate: core.closure_candidate,
        closure_candidate_state: core.closure_candidate_state,
        closure_candidate_reason: core.closure_candidate_reason,
        ready_for_close: core.ready_for_close,
        missing_proof: core.missing_proof,
        proof_blocked_by_runtime: core.proof_blocked_by_runtime,
        blocked_by_runtime: core.blocked_by_runtime,
        next_required_command: core.next_required_command,
        recommended_next_action: core.recommended_next_action,
        canonical_commands: core.canonical_commands,
    })
}

fn task_close_epic_progress_task_row(
    task: &state_store::TaskRecord,
    task_by_id: &std::collections::BTreeMap<&str, &state_store::TaskRecord>,
) -> TaskCloseEpicProgressTaskRow {
    let blockers = task_close_progress_blockers(task, task_by_id);
    let blocker_state = if blockers.is_empty() {
        "clear"
    } else {
        "blocked"
    };
    let next_action = task_close_progress_next_action(task, &blockers);
    TaskCloseEpicProgressTaskRow {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
        blocker_state: blocker_state.to_string(),
        blockers,
        next_action,
    }
}

fn task_close_progress_blockers(
    task: &state_store::TaskRecord,
    task_by_id: &std::collections::BTreeMap<&str, &state_store::TaskRecord>,
) -> Vec<TaskCloseEpicProgressBlocker> {
    task.dependencies
        .iter()
        .filter(|dependency| dependency.edge_type == "blocks")
        .filter_map(
            |dependency| match task_by_id.get(dependency.depends_on_id.as_str()) {
                Some(blocker) if !StateStore::task_status_is_closed_like(&blocker.status) => {
                    Some(TaskCloseEpicProgressBlocker {
                        task_id: blocker.id.clone(),
                        status: blocker.status.clone(),
                        title: Some(blocker.title.clone()),
                    })
                }
                Some(_) => None,
                None => Some(TaskCloseEpicProgressBlocker {
                    task_id: dependency.depends_on_id.clone(),
                    status: "missing".to_string(),
                    title: None,
                }),
            },
        )
        .collect()
}

fn task_close_progress_next_action(
    task: &state_store::TaskRecord,
    blockers: &[TaskCloseEpicProgressBlocker],
) -> String {
    if !blockers.is_empty() {
        let blocker_ids = blockers
            .iter()
            .map(|blocker| blocker.task_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "Resolve blocking tasks before closing `{}`: {blocker_ids}",
            task.id
        );
    }
    if StateStore::task_status_is_closed_like(&task.status) {
        return "No action; task is already closed.".to_string();
    }
    if task.issue_type == "epic" {
        return format!(
            "Inspect nested epic progress with `{}`.",
            operator_output::command_text::human_command(&format!(
                "vida task progress {} --json",
                task.id
            ))
        );
    }
    format!(
        "Continue `{}` or close it after proof is complete.",
        task.id
    )
}

fn task_close_automation_is_blocked(automation: Option<&TaskCloseAutomationReceipt>) -> bool {
    automation
        .map(|receipt| receipt.status != "pass")
        .unwrap_or(false)
}

fn task_close_result_payload(
    task: &state_store::TaskRecord,
    telemetry: &serde_json::Value,
    automation: Option<&TaskCloseAutomationReceipt>,
    telemetry_feedback_blocker: Option<&(Vec<String>, Vec<String>)>,
    epic_progress_summary: Option<&TaskCloseEpicProgressSummary>,
) -> serde_json::Value {
    let automation_blocked = task_close_automation_is_blocked(automation);
    let feedback_blocked = telemetry_feedback_blocker.is_some();
    let blocker_codes = if let Some((blocker_codes, _)) = telemetry_feedback_blocker {
        blocker_codes.clone()
    } else {
        automation
            .map(|receipt| receipt.blocker_codes.clone())
            .unwrap_or_default()
    };
    let next_actions = if let Some((_, next_actions)) = telemetry_feedback_blocker {
        next_actions.clone()
    } else {
        automation
            .map(|receipt| receipt.next_actions.clone())
            .unwrap_or_default()
    };
    let continuation_blocked = automation_blocked || feedback_blocked;
    let status = if automation_blocked {
        "blocked"
    } else {
        "pass"
    };
    serde_json::json!({
        "status": status,
        "closed": true,
        "continuation_blocked": continuation_blocked,
        "automation_blocked": automation_blocked,
        "feedback_blocked": feedback_blocked,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "task": task,
        "host_agent_telemetry": telemetry,
        "automation": automation,
        "epic_progress_summary": epic_progress_summary,
        "parent_epic_progress": epic_progress_summary,
    })
}

fn print_task_close_epic_progress_summary(
    render: RenderMode,
    summary: &TaskCloseEpicProgressSummary,
) {
    print_surface_line(
        render,
        "epic progress",
        &format!(
            "{} scoped epics after closing {} ({} omitted)",
            summary.reported_epic_count, summary.closed_task_id, summary.omitted_epic_count
        ),
    );
    for epic in &summary.epics {
        print_surface_line(
            render,
            &format!("epic {}", epic.epic_id),
            &format!(
                "{}/{} closed ({:.2}%)",
                epic.closed_count, epic.total_count, epic.percent_closed
            ),
        );
    }
}

fn print_task_epic_progress_summary(
    render: RenderMode,
    summary: &TaskEpicProgressSummary,
    as_json: bool,
    counts_only: bool,
) {
    let payload = if counts_only {
        crate::task_cli_render::build_pass_operator_surface_payload(
            "vida task progress --epics",
            serde_json::json!({
                "epic_progress_counts": {
                    "epic_count": summary.epic_count,
                    "open_count": summary.open_count,
                    "in_progress_count": summary.in_progress_count,
                    "closed_count": summary.closed_count,
                    "total_descendant_count": summary.total_descendant_count,
                    "total_open_descendant_count": summary.total_open_descendant_count,
                    "total_in_progress_descendant_count": summary.total_in_progress_descendant_count,
                    "total_closed_descendant_count": summary.total_closed_descendant_count,
                    "percent_closed": summary.percent_closed,
                    "include_closed_epics": summary.include_closed_epics,
                    "progress_basis": summary.progress_basis,
                    "epic_filter": summary.epic_filter,
                    "read_metadata": summary.read_metadata,
                },
            }),
        )
    } else {
        crate::task_cli_render::build_pass_operator_surface_payload(
            "vida task progress --epics",
            serde_json::json!({
                "epic_progress_summary": summary,
            }),
        )
    };
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task epic progress should render as json",
    ) {
        return;
    }

    print_surface_header(render, "vida task progress --epics");
    if counts_only {
        print_surface_line(render, "epics", &summary.epic_count.to_string());
        print_surface_line(render, "open epics", &summary.open_count.to_string());
        print_surface_line(
            render,
            "in progress epics",
            &summary.in_progress_count.to_string(),
        );
        print_surface_line(render, "closed epics", &summary.closed_count.to_string());
        print_surface_line(
            render,
            "descendants",
            &summary.total_descendant_count.to_string(),
        );
        print_surface_line(
            render,
            "open descendants",
            &summary.total_open_descendant_count.to_string(),
        );
        print_surface_line(
            render,
            "in progress descendants",
            &summary.total_in_progress_descendant_count.to_string(),
        );
        print_surface_line(
            render,
            "closed descendants",
            &summary.total_closed_descendant_count.to_string(),
        );
        print_surface_line(
            render,
            "percent complete",
            &format!("{:.2}%", summary.percent_closed),
        );
        return;
    }

    print_surface_line(render, "epics", &summary.epic_count.to_string());
    print_surface_line(render, "open epics", &summary.open_count.to_string());
    print_surface_line(
        render,
        "in progress epics",
        &summary.in_progress_count.to_string(),
    );
    print_surface_line(render, "closed epics", &summary.closed_count.to_string());
    print_surface_line(
        render,
        "descendants",
        &summary.total_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "open descendants",
        &summary.total_open_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "in progress descendants",
        &summary.total_in_progress_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "closed descendants",
        &summary.total_closed_descendant_count.to_string(),
    );
    print_surface_line(
        render,
        "percent complete",
        &format!("{:.2}%", summary.percent_closed),
    );
    for epic in &summary.epics {
        print_surface_line(
            render,
            &format!("epic {}", epic.epic_id),
            &format!(
                "{}: {}/{} closed ({:.2}%), open={}, in_progress={}",
                epic.epic_status,
                epic.closed_count,
                epic.total_count,
                epic.percent_complete,
                epic.open_count,
                epic.in_progress_count
            ),
        );
    }
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

    if let Some((canonical_status, canonical_gate)) =
        crate::agent_feedback_surface::canonical_close_status_from_reason(close_reason)
    {
        return serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "canonical_status": canonical_status,
            "canonical_gate": canonical_gate,
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

fn task_close_feedback_blocker_summary(
    telemetry: &serde_json::Value,
) -> Option<(Vec<String>, Vec<String>)> {
    let reason = telemetry
        .get("reason")
        .and_then(serde_json::Value::as_str)?;
    if reason != "feedback_deferred_for_canonical_close_status" {
        return None;
    }
    let canonical_status = telemetry
        .get("canonical_status")
        .and_then(serde_json::Value::as_str)?;
    let canonical_gate = telemetry
        .get("canonical_gate")
        .and_then(serde_json::Value::as_str)?;
    if canonical_status == "awaiting_approval" {
        return None;
    }
    let blocker_code = match canonical_status {
        "blocked" => "close_feedback_canonical_status_blocked",
        "awaiting_approval" => "close_feedback_canonical_status_awaiting_approval",
        _ => "close_feedback_canonical_status_deferred",
    };
    let next_action = match canonical_status {
        "blocked" => {
            "Resolve the blocked condition described in the close reason, then rerun `vida task close ...`."
        }
        "awaiting_approval" => {
            "Satisfy the approval requirement described in the close reason, then rerun `vida task close ...`."
        }
        _ => "Resolve the deferred canonical close condition, then rerun `vida task close ...`.",
    };
    Some((
        vec![
            blocker_code.to_string(),
            format!("canonical_gate_{canonical_gate}"),
        ],
        vec![next_action.to_string()],
    ))
}

fn resolve_optional_text_arg(
    label: &str,
    direct: Option<&str>,
    file_path: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    taskflow_core::task::note::resolve_optional_text_arg(label, direct, file_path)
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

fn task_execution_semantics_input_from_create_args(
    command: &TaskCreateArgs,
) -> taskflow_core::task::create::TaskExecutionSemanticsInput<'_> {
    taskflow_core::task::create::TaskExecutionSemanticsInput {
        execution_mode: command.execution_mode.as_deref(),
        order_bucket: command.order_bucket.as_deref(),
        parallel_group: command.parallel_group.as_deref(),
        conflict_domain: command.conflict_domain.as_deref(),
    }
}

fn task_execution_semantics_input_from_record(
    semantics: &state_store::TaskExecutionSemantics,
) -> taskflow_core::task::create::TaskExecutionSemanticsInput<'_> {
    taskflow_core::task::create::TaskExecutionSemanticsInput {
        execution_mode: semantics.execution_mode.as_deref(),
        order_bucket: semantics.order_bucket.as_deref(),
        parallel_group: semantics.parallel_group.as_deref(),
        conflict_domain: semantics.conflict_domain.as_deref(),
    }
}

fn task_create_semantics_requested(command: &TaskCreateArgs) -> bool {
    taskflow_core::task::create::task_create_semantics_requested(
        task_execution_semantics_input_from_create_args(command),
    )
}

fn task_create_semantics_mismatch(
    existing: &state_store::TaskExecutionSemantics,
    command: &TaskCreateArgs,
) -> bool {
    taskflow_core::task::create::task_create_semantics_mismatch(
        task_execution_semantics_input_from_record(existing),
        task_execution_semantics_input_from_create_args(command),
    )
}

fn task_update_semantics_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    taskflow_core::task::update::task_update_semantics_arg(value, clear)
}

fn task_update_parent_arg(
    value: Option<&str>,
    clear: bool,
) -> Result<Option<Option<&str>>, String> {
    taskflow_core::task::update::task_update_parent_arg(value, clear)
}

fn parse_label_values(values: &[String]) -> Vec<String> {
    taskflow_core::task::update::parse_label_values(values)
}

fn parse_proof_target_values(values: &[String]) -> Vec<String> {
    taskflow_core::task::update::parse_proof_target_values(values)
}

fn parse_literal_proof_target_values(values: &[String]) -> Vec<String> {
    taskflow_core::task::update::parse_literal_proof_target_values(values)
}

fn parse_literal_metadata_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_mixed_metadata_values(split_values: &[String], literal_values: &[String]) -> Vec<String> {
    let mut values = parse_label_values(split_values);
    values.extend(parse_literal_metadata_values(literal_values));
    values
}

fn parse_mixed_proof_target_values(
    split_values: &[String],
    literal_values: &[String],
) -> Vec<String> {
    normalize_proof_target_commands(parse_mixed_metadata_values(split_values, literal_values))
}

fn task_update_proof_targets_arg(
    values: &[String],
    clear: bool,
) -> Result<Option<Vec<String>>, String> {
    taskflow_core::task::update::task_update_proof_targets_arg(values, clear)
}

fn normalize_proof_target_commands(values: Vec<String>) -> Vec<String> {
    taskflow_core::task::update::normalize_proof_target_commands(values)
}

fn release_proof_template_targets() -> Vec<String> {
    vec![
        "cargo check -p vida --tests".to_string(),
        "vida task validate-graph --json".to_string(),
        "vida release install --json".to_string(),
        "vida --version".to_string(),
        "vida status --json".to_string(),
        "vida doctor --json".to_string(),
    ]
}

fn append_release_proof_template_targets(mut proof_targets: Vec<String>) -> Vec<String> {
    proof_targets.extend(release_proof_template_targets());
    proof_targets.sort();
    proof_targets.dedup();
    proof_targets
}

fn parse_optional_label_value(value: Option<&str>) -> Option<Vec<String>> {
    taskflow_core::task::update::parse_optional_label_value(value)
}

fn task_update_planner_metadata_requested(command: &crate::TaskUpdateArgs) -> bool {
    !command.owned_paths.is_empty()
        || !command.owned_path_literals.is_empty()
        || !command.acceptance_targets.is_empty()
        || !command.acceptance_target_literals.is_empty()
        || !command.proof_targets.is_empty()
        || !command.proof_target_literals.is_empty()
        || command.release_proof_template
        || command.clear_proof_targets
}

fn task_create_planner_metadata_arg(command: &TaskCreateArgs) -> state_store::TaskPlannerMetadata {
    state_store::TaskPlannerMetadata {
        owned_paths: parse_mixed_metadata_values(
            &command.owned_paths,
            &command.owned_path_literals,
        ),
        acceptance_targets: parse_mixed_metadata_values(
            &command.acceptance_targets,
            &command.acceptance_target_literals,
        ),
        proof_targets: if command.release_proof_template {
            append_release_proof_template_targets(parse_mixed_proof_target_values(
                &command.proof_targets,
                &command.proof_target_literals,
            ))
        } else {
            parse_mixed_proof_target_values(&command.proof_targets, &command.proof_target_literals)
        },
        ..state_store::TaskPlannerMetadata::default()
    }
}

fn task_create_invalid_parent_kind_payload(
    surface: &str,
    task_id: &str,
    issue_type: &str,
    parent_id: Option<&str>,
    reason: &str,
) -> serde_json::Value {
    let canonical_issue_type = state_store::canonical_work_item_issue_type(issue_type);
    let allowed_parent_kind = match canonical_issue_type.as_str() {
        "subtask" => "task",
        "step" => "task or subtask",
        _ => "documented TaskFlow parent kind",
    };
    let next_action = match canonical_issue_type.as_str() {
        "step" => {
            "Choose a valid parent and retry the command; steps require a task or subtask parent for new mutations."
        }
        "subtask" => "Create the subtask under a task parent.",
        _ => "Choose a valid parent kind for this work item and retry the task create command.",
    };
    crate::release1_operator_output::Release1OperatorOutputBuilder::new(surface)
        .blocker_codes(
            crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::DependencyGraphIssues,
            )
            .into_iter()
            .collect(),
        )
        .next_actions(vec![next_action.to_string()])
        .artifact_refs(serde_json::json!({
            "surface": surface,
            "task_id": task_id,
            "parent_id": parent_id,
            "graph_issue_type": "invalid_parent_child_kind",
        }))
        .extra_fields(serde_json::json!({
            "reason": reason,
            "task_id": task_id,
            "issue_type": issue_type,
            "canonical_issue_type": canonical_issue_type,
            "parent_id": parent_id,
            "allowed_parent_kind": allowed_parent_kind,
            "graph_issue": {
                "issue_type": "invalid_parent_child_kind",
                "issue_id": task_id,
                "depends_on_id": parent_id,
                "edge_type": "parent-child",
                "detail": reason,
            },
        }))
        .build()
        .expect(
            "task create invalid parent kind payload should satisfy release-1 operator contract",
        )
}

fn emit_task_create_invalid_parent_kind_error(
    surface: &str,
    render: RenderMode,
    as_json: bool,
    task_id: &str,
    issue_type: &str,
    parent_id: Option<&str>,
    reason: &str,
) {
    let payload =
        task_create_invalid_parent_kind_payload(surface, task_id, issue_type, parent_id, reason);
    if as_json {
        crate::print_json_pretty(&payload);
        return;
    }
    print_surface_header(render, surface);
    print_surface_line(render, "status", "blocked");
    print_surface_line(render, "blocker_codes", "dependency_graph_issues");
    print_surface_line(render, "task_id", task_id);
    if let Some(parent_id) = parent_id {
        print_surface_line(render, "parent_id", parent_id);
    }
    print_surface_line(render, "reason", reason);
    if let Some(allowed) = payload["allowed_parent_kind"].as_str() {
        print_surface_line(render, "allowed_parent_kind", allowed);
    }
    if let Some(next_action) = payload["next_actions"]
        .as_array()
        .and_then(|items| items.first().and_then(serde_json::Value::as_str))
    {
        print_surface_line(render, "next_action", next_action);
    }
}

fn maybe_emit_task_create_invalid_parent_kind_error(
    surface: &str,
    render: RenderMode,
    as_json: bool,
    task_id: &str,
    issue_type: &str,
    parent_id: Option<&str>,
    error: &state_store::StateStoreError,
) -> bool {
    let state_store::StateStoreError::InvalidTaskRecord { reason } = error else {
        return false;
    };
    if !reason
        .starts_with("task creation would create invalid graph: invalid_parent_child_kind on ")
    {
        return false;
    }
    emit_task_create_invalid_parent_kind_error(
        surface, render, as_json, task_id, issue_type, parent_id, reason,
    );
    true
}

fn task_update_planner_metadata_arg(
    existing: &state_store::TaskPlannerMetadata,
    command: &crate::TaskUpdateArgs,
) -> Result<Option<state_store::TaskPlannerMetadata>, String> {
    if !task_update_planner_metadata_requested(command) {
        return Ok(None);
    }
    let mut metadata = existing.clone();
    let owned_paths =
        parse_mixed_metadata_values(&command.owned_paths, &command.owned_path_literals);
    if !owned_paths.is_empty() {
        metadata.owned_paths = owned_paths;
    }
    let acceptance_targets = parse_mixed_metadata_values(
        &command.acceptance_targets,
        &command.acceptance_target_literals,
    );
    if !acceptance_targets.is_empty() {
        metadata.acceptance_targets = acceptance_targets;
    }
    if (!command.proof_targets.is_empty()
        || !command.proof_target_literals.is_empty()
        || command.release_proof_template)
        && command.clear_proof_targets
    {
        return Err(
            "Use either --proof-target/--proof-target-literal/--release-proof-template or --clear-proof-targets, not both."
                .to_string(),
        );
    }
    if command.clear_proof_targets {
        metadata.proof_targets = Vec::new();
    } else {
        let proof_targets =
            parse_mixed_proof_target_values(&command.proof_targets, &command.proof_target_literals);
        if command.release_proof_template {
            metadata.proof_targets =
                append_release_proof_template_targets(metadata.proof_targets.clone());
            metadata.proof_targets.extend(
                proof_targets
                    .into_iter()
                    .filter(|target| !target.trim().is_empty()),
            );
            metadata.proof_targets.sort();
            metadata.proof_targets.dedup();
        } else if !proof_targets.is_empty() {
            metadata.proof_targets = proof_targets;
        }
    }
    Ok(Some(metadata))
}

const TASK_BULK_IMPORT_SURFACE: &str = "vida task import";
const TASK_BULK_IMPORT_MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const TASK_BULK_IMPORT_MAX_TASKS: usize = 10_000;

#[derive(Debug, Clone)]
struct TaskBulkImportRawItem {
    index: usize,
    line: Option<usize>,
    value: serde_json::Value,
}

#[derive(Debug, Clone)]
struct TaskBulkImportParsedInput {
    input_format: String,
    requested_count: usize,
    tasks: Vec<TaskBulkImportPlannedTask>,
    validation_errors: Vec<TaskBulkImportValidationError>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct TaskBulkImportValidationError {
    index: usize,
    line: Option<usize>,
    task_id: Option<String>,
    field: Option<String>,
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskBulkImportPlannedTask {
    index: usize,
    line: Option<usize>,
    task_id: String,
    title: String,
    display_id: Option<String>,
    description: String,
    issue_type: String,
    status: String,
    priority: u32,
    parent_id: Option<String>,
    notes: Option<String>,
    labels: Vec<String>,
    execution_semantics: state_store::TaskExecutionSemantics,
    planner_metadata: state_store::TaskPlannerMetadata,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskBulkImportResult {
    source_path: String,
    input_format: String,
    dry_run: bool,
    applied: bool,
    requested_count: usize,
    planned_count: usize,
    created_count: usize,
    validation_error_count: usize,
    graph_issue_count: usize,
    planned_task_ids: Vec<String>,
    created_task_ids: Vec<String>,
    validation_errors: Vec<TaskBulkImportValidationError>,
    graph_issues: Vec<state_store::TaskGraphIssue>,
}

struct TaskBulkImportPlan {
    result: TaskBulkImportResult,
    tasks: Vec<TaskBulkImportPlannedTask>,
}

fn task_bulk_import_validation_error(
    index: usize,
    line: Option<usize>,
    task_id: Option<String>,
    field: Option<&str>,
    reason: impl Into<String>,
) -> TaskBulkImportValidationError {
    TaskBulkImportValidationError {
        index,
        line,
        task_id,
        field: field.map(ToOwned::to_owned),
        reason: reason.into(),
    }
}

fn task_bulk_import_format_label(format: crate::TaskImportFormatArg) -> &'static str {
    match format {
        crate::TaskImportFormatArg::Auto => "auto",
        crate::TaskImportFormatArg::Json => "json",
        crate::TaskImportFormatArg::Yaml => "yaml",
        crate::TaskImportFormatArg::Jsonl => "jsonl",
    }
}

fn resolve_task_bulk_import_format(
    path: &std::path::Path,
    requested: crate::TaskImportFormatArg,
) -> Result<&'static str, String> {
    match requested {
        crate::TaskImportFormatArg::Json => return Ok("json"),
        crate::TaskImportFormatArg::Yaml => return Ok("yaml"),
        crate::TaskImportFormatArg::Jsonl => return Ok("jsonl"),
        crate::TaskImportFormatArg::Auto => {}
    }
    match path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => Ok("json"),
        Some("yaml" | "yml") => Ok("yaml"),
        Some("jsonl" | "ndjson") => Ok("jsonl"),
        Some(extension) => Err(format!(
            "Cannot infer task import format from extension `{extension}`; pass --format json, --format yaml, or --format jsonl."
        )),
        None => Err(
            "Cannot infer task import format from a path without extension; pass --format json, --format yaml, or --format jsonl."
                .to_string(),
        ),
    }
}

fn read_task_bulk_import_file(path: &std::path::Path) -> Result<String, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect import file `{}`: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to read import file `{}`: symlinks are not allowed",
            path.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "Refusing to read import file `{}`: expected a regular file",
            path.display()
        ));
    }
    if metadata.len() > TASK_BULK_IMPORT_MAX_FILE_BYTES {
        return Err(format!(
            "Refusing to read import file `{}`: file is {} bytes, limit is {} bytes",
            path.display(),
            metadata.len(),
            TASK_BULK_IMPORT_MAX_FILE_BYTES
        ));
    }
    std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read import file `{}`: {error}", path.display()))
}

fn task_bulk_import_raw_items_from_value(
    value: serde_json::Value,
) -> Result<Vec<TaskBulkImportRawItem>, String> {
    let values = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(mut object) => match object.remove("tasks") {
            Some(serde_json::Value::Array(items)) => items,
            Some(_) => return Err("Task import object field `tasks` must be an array.".to_string()),
            None => {
                return Err(
                    "Task import file must be an array or an object containing a `tasks` array."
                        .to_string(),
                );
            }
        },
        _ => {
            return Err(
                "Task import file must be an array or an object containing a `tasks` array."
                    .to_string(),
            );
        }
    };
    if values.len() > TASK_BULK_IMPORT_MAX_TASKS {
        return Err(format!(
            "Task import contains {} tasks, limit is {} tasks.",
            values.len(),
            TASK_BULK_IMPORT_MAX_TASKS
        ));
    }
    Ok(values
        .into_iter()
        .enumerate()
        .map(|(index, value)| TaskBulkImportRawItem {
            index,
            line: None,
            value,
        })
        .collect())
}

fn parse_task_bulk_import_raw_items(
    path: &std::path::Path,
    requested_format: crate::TaskImportFormatArg,
) -> Result<(String, Vec<TaskBulkImportRawItem>), String> {
    let format = resolve_task_bulk_import_format(path, requested_format)?;
    let text = read_task_bulk_import_file(path)?;
    let items = match format {
        "json" => {
            let value = serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|error| format!("Failed to parse JSON task import file: {error}"))?;
            task_bulk_import_raw_items_from_value(value)?
        }
        "yaml" => {
            let value = serde_yaml::from_str::<serde_yaml::Value>(&text)
                .map_err(|error| format!("Failed to parse YAML task import file: {error}"))?;
            let value = serde_json::to_value(value)
                .map_err(|error| format!("Failed to normalize YAML task import file: {error}"))?;
            task_bulk_import_raw_items_from_value(value)?
        }
        "jsonl" => {
            let mut items = Vec::new();
            for (line_index, line) in text.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if items.len() >= TASK_BULK_IMPORT_MAX_TASKS {
                    return Err(format!(
                        "Task import contains more than {} tasks; first excess item is at line {}.",
                        TASK_BULK_IMPORT_MAX_TASKS,
                        line_index + 1
                    ));
                }
                let value =
                    serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                        format!(
                            "Failed to parse JSONL task import file at line {}: {error}",
                            line_index + 1
                        )
                    })?;
                items.push(TaskBulkImportRawItem {
                    index: items.len(),
                    line: Some(line_index + 1),
                    value,
                });
            }
            items
        }
        other => {
            return Err(format!("Unsupported task import format `{other}`."));
        }
    };
    Ok((format.to_string(), items))
}

fn task_bulk_json_field<'a>(
    value: &'a serde_json::Value,
    names: &[&str],
) -> Option<&'a serde_json::Value> {
    let object = value.as_object()?;
    names.iter().find_map(|name| object.get(*name))
}

fn task_bulk_json_object_field<'a>(
    value: &'a serde_json::Value,
    names: &[&str],
) -> Result<Option<&'a serde_json::Map<String, serde_json::Value>>, String> {
    match task_bulk_json_field(value, names) {
        Some(serde_json::Value::Object(object)) => Ok(Some(object)),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(_) => Err(format!("field `{}` must be an object", names[0])),
    }
}

fn task_bulk_json_string_field(
    value: &serde_json::Value,
    names: &[&str],
) -> Result<Option<String>, String> {
    match task_bulk_json_field(value, names) {
        Some(serde_json::Value::String(text)) => {
            Ok(Some(text.trim().to_string()).filter(|text| !text.is_empty()))
        }
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(_) => Err(format!("field `{}` must be a string", names[0])),
    }
}

fn task_bulk_json_u32_field(
    value: &serde_json::Value,
    names: &[&str],
) -> Result<Option<u32>, String> {
    match task_bulk_json_field(value, names) {
        Some(serde_json::Value::Number(number)) => number
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("field `{}` must be a non-negative u32", names[0])),
        Some(serde_json::Value::String(text)) => text
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("field `{}` must be a non-negative u32", names[0])),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(_) => Err(format!("field `{}` must be a non-negative u32", names[0])),
    }
}

fn task_bulk_string_list_from_value(
    field_name: &str,
    value: &serde_json::Value,
) -> Result<Vec<String>, String> {
    match value {
        serde_json::Value::String(text) => Ok(parse_label_values(&[text.to_string()])),
        serde_json::Value::Array(values) => {
            let mut result = Vec::new();
            for item in values {
                match item {
                    serde_json::Value::String(text) => {
                        result.extend(parse_literal_metadata_values(&[text.to_string()]));
                    }
                    _ => {
                        return Err(format!(
                            "field `{field_name}` array entries must be strings"
                        ));
                    }
                }
            }
            Ok(result)
        }
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(format!(
            "field `{field_name}` must be a string or an array of strings"
        )),
    }
}

fn task_bulk_json_string_list_field(
    value: &serde_json::Value,
    names: &[&str],
) -> Result<Vec<String>, String> {
    match task_bulk_json_field(value, names) {
        Some(value) => task_bulk_string_list_from_value(names[0], value),
        None => Ok(Vec::new()),
    }
}

fn task_bulk_nested_string_field(
    value: &serde_json::Value,
    nested: Option<&serde_json::Map<String, serde_json::Value>>,
    names: &[&str],
) -> Result<Option<String>, String> {
    if let Some(value) = task_bulk_json_string_field(value, names)? {
        return Ok(Some(value));
    }
    match nested.and_then(|object| names.iter().find_map(|name| object.get(*name))) {
        Some(serde_json::Value::String(text)) => {
            Ok(Some(text.trim().to_string()).filter(|text| !text.is_empty()))
        }
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(_) => Err(format!("field `{}` must be a string", names[0])),
    }
}

fn task_bulk_nested_string_list_field(
    value: &serde_json::Value,
    nested: Option<&serde_json::Map<String, serde_json::Value>>,
    names: &[&str],
) -> Result<Vec<String>, String> {
    let mut result = task_bulk_json_string_list_field(value, names)?;
    if let Some(nested_value) =
        nested.and_then(|object| names.iter().find_map(|name| object.get(*name)))
    {
        result.extend(task_bulk_string_list_from_value(names[0], nested_value)?);
    }
    Ok(result)
}

fn task_bulk_normalize_string_list(values: Vec<String>) -> Vec<String> {
    crate::runtime_assignment_policy::canonical_sorted_nonempty_strings(values)
}

fn task_bulk_merge_string_lists(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    task_bulk_normalize_string_list(left.into_iter().chain(right).collect())
}

fn task_bulk_normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn task_bulk_validate_execution_mode(
    task_id: &str,
    mode: Option<String>,
) -> Result<Option<String>, String> {
    let mode = task_bulk_normalize_optional_text(mode);
    let Some(mode) = mode else {
        return Ok(None);
    };
    match mode.as_str() {
        "sequential" | "parallel_safe" | "exclusive" | "container_only" => Ok(Some(mode)),
        _ => Err(format!(
            "task `{task_id}` execution_mode must be one of sequential, parallel_safe, exclusive, container_only"
        )),
    }
}

fn task_bulk_parse_item(
    raw: TaskBulkImportRawItem,
    command: &TaskBulkImportArgs,
) -> Result<TaskBulkImportPlannedTask, TaskBulkImportValidationError> {
    if !raw.value.is_object() {
        return Err(task_bulk_import_validation_error(
            raw.index,
            raw.line,
            None,
            None,
            "task import item must be an object",
        ));
    }
    let item = &raw.value;
    let task_id = task_bulk_json_string_field(item, &["id", "task_id"])
        .map_err(|reason| {
            task_bulk_import_validation_error(raw.index, raw.line, None, Some("id"), reason)
        })?
        .ok_or_else(|| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                None,
                Some("id"),
                "task import item requires `id` or `task_id`",
            )
        })?;
    let title = task_bulk_json_string_field(item, &["title"])
        .map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("title"),
                reason,
            )
        })?
        .ok_or_else(|| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("title"),
                "task import item requires `title`",
            )
        })?;
    let execution_object =
        task_bulk_json_object_field(item, &["execution_semantics"]).map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("execution_semantics"),
                reason,
            )
        })?;
    let planner_object =
        task_bulk_json_object_field(item, &["planner_metadata"]).map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("planner_metadata"),
                reason,
            )
        })?;

    let labels = task_bulk_merge_string_lists(
        parse_label_values(&command.labels),
        task_bulk_json_string_list_field(item, &["labels"]).map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("labels"),
                reason,
            )
        })?,
    );
    let owned_paths = task_bulk_merge_string_lists(
        parse_label_values(&command.owned_paths),
        task_bulk_nested_string_list_field(item, planner_object, &["owned_paths"]).map_err(
            |reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    Some(task_id.clone()),
                    Some("owned_paths"),
                    reason,
                )
            },
        )?,
    );
    let acceptance_targets = task_bulk_merge_string_lists(
        parse_label_values(&command.acceptance_targets),
        task_bulk_nested_string_list_field(
            item,
            planner_object,
            &["acceptance_targets", "acceptance"],
        )
        .map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("acceptance_targets"),
                reason,
            )
        })?,
    );
    let proof_targets = normalize_proof_target_commands(task_bulk_merge_string_lists(
        parse_label_values(&command.proof_targets),
        task_bulk_nested_string_list_field(item, planner_object, &["proof_targets", "proof"])
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    Some(task_id.clone()),
                    Some("proof_targets"),
                    reason,
                )
            })?,
    ));

    let execution_mode = task_bulk_nested_string_field(item, execution_object, &["execution_mode"])
        .map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("execution_mode"),
                reason,
            )
        })?
        .or_else(|| command.execution_mode.clone());
    let execution_mode =
        task_bulk_validate_execution_mode(&task_id, execution_mode).map_err(|reason| {
            task_bulk_import_validation_error(
                raw.index,
                raw.line,
                Some(task_id.clone()),
                Some("execution_mode"),
                reason,
            )
        })?;

    Ok(TaskBulkImportPlannedTask {
        index: raw.index,
        line: raw.line,
        task_id,
        title,
        display_id: task_bulk_json_string_field(item, &["display_id"]).map_err(|reason| {
            task_bulk_import_validation_error(raw.index, raw.line, None, Some("display_id"), reason)
        })?,
        description: task_bulk_json_string_field(item, &["description"])
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    None,
                    Some("description"),
                    reason,
                )
            })?
            .unwrap_or_default(),
        issue_type: task_bulk_json_string_field(item, &["issue_type", "type"])
            .map_err(|reason| {
                task_bulk_import_validation_error(raw.index, raw.line, None, Some("type"), reason)
            })?
            .unwrap_or_else(|| command.issue_type.clone()),
        status: task_bulk_json_string_field(item, &["status"])
            .map_err(|reason| {
                task_bulk_import_validation_error(raw.index, raw.line, None, Some("status"), reason)
            })?
            .unwrap_or_else(|| command.status.clone()),
        priority: task_bulk_json_u32_field(item, &["priority"])
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    None,
                    Some("priority"),
                    reason,
                )
            })?
            .unwrap_or(command.priority),
        parent_id: task_bulk_json_string_field(item, &["parent_id"])
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    None,
                    Some("parent_id"),
                    reason,
                )
            })?
            .or_else(|| command.parent_id.clone()),
        notes: task_bulk_json_string_field(item, &["notes"]).map_err(|reason| {
            task_bulk_import_validation_error(raw.index, raw.line, None, Some("notes"), reason)
        })?,
        labels,
        execution_semantics: state_store::TaskExecutionSemantics {
            execution_mode,
            order_bucket: task_bulk_nested_string_field(item, execution_object, &["order_bucket"])
                .map_err(|reason| {
                    task_bulk_import_validation_error(
                        raw.index,
                        raw.line,
                        None,
                        Some("order_bucket"),
                        reason,
                    )
                })?
                .or_else(|| command.order_bucket.clone()),
            parallel_group: task_bulk_nested_string_field(
                item,
                execution_object,
                &["parallel_group"],
            )
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    None,
                    Some("parallel_group"),
                    reason,
                )
            })?
            .or_else(|| command.parallel_group.clone()),
            conflict_domain: task_bulk_nested_string_field(
                item,
                execution_object,
                &["conflict_domain"],
            )
            .map_err(|reason| {
                task_bulk_import_validation_error(
                    raw.index,
                    raw.line,
                    None,
                    Some("conflict_domain"),
                    reason,
                )
            })?
            .or_else(|| command.conflict_domain.clone()),
        },
        planner_metadata: state_store::TaskPlannerMetadata {
            owned_paths,
            acceptance_targets,
            proof_targets,
            risk: task_bulk_nested_string_field(item, planner_object, &["risk"]).map_err(
                |reason| {
                    task_bulk_import_validation_error(
                        raw.index,
                        raw.line,
                        None,
                        Some("risk"),
                        reason,
                    )
                },
            )?,
            estimate: task_bulk_nested_string_field(item, planner_object, &["estimate"]).map_err(
                |reason| {
                    task_bulk_import_validation_error(
                        raw.index,
                        raw.line,
                        None,
                        Some("estimate"),
                        reason,
                    )
                },
            )?,
            lane_hint: task_bulk_nested_string_field(item, planner_object, &["lane_hint"])
                .map_err(|reason| {
                    task_bulk_import_validation_error(
                        raw.index,
                        raw.line,
                        None,
                        Some("lane_hint"),
                        reason,
                    )
                })?,
        },
    })
}

fn parse_task_bulk_import_input(
    command: &TaskBulkImportArgs,
) -> Result<TaskBulkImportParsedInput, String> {
    let (input_format, raw_items) =
        parse_task_bulk_import_raw_items(&command.file, command.format)?;
    let requested_count = raw_items.len();
    let mut tasks = Vec::new();
    let mut validation_errors = Vec::new();
    for raw in raw_items {
        match task_bulk_parse_item(raw, command) {
            Ok(task) => tasks.push(task),
            Err(error) => validation_errors.push(error),
        }
    }
    Ok(TaskBulkImportParsedInput {
        input_format,
        requested_count,
        tasks,
        validation_errors,
    })
}

fn task_bulk_import_task_record(
    task: &TaskBulkImportPlannedTask,
    created_by: &str,
    source_repo: &str,
) -> state_store::TaskRecord {
    let dependencies = task
        .parent_id
        .iter()
        .map(|parent_id| state_store::TaskDependencyRecord {
            issue_id: task.task_id.clone(),
            depends_on_id: parent_id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "planned".to_string(),
            created_by: created_by.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        })
        .collect::<Vec<_>>();
    state_store::TaskRecord {
        id: task.task_id.clone(),
        display_id: task_bulk_normalize_optional_text(task.display_id.clone()),
        title: task.title.trim().to_string(),
        description: task.description.clone(),
        status: task.status.trim().to_string(),
        priority: task.priority,
        issue_type: task.issue_type.trim().to_string(),
        created_at: "planned".to_string(),
        created_by: created_by.to_string(),
        updated_at: "planned".to_string(),
        closed_at: (task.status.trim() == "closed").then(|| "planned".to_string()),
        close_reason: None,
        source_repo: source_repo.to_string(),
        compaction_level: 0,
        original_size: 0,
        notes: task.notes.clone(),
        labels: task_bulk_normalize_string_list(task.labels.clone()),
        execution_semantics: task.execution_semantics.clone(),
        planner_metadata: state_store::TaskPlannerMetadata {
            owned_paths: task_bulk_normalize_string_list(task.planner_metadata.owned_paths.clone()),
            acceptance_targets: task_bulk_normalize_string_list(
                task.planner_metadata.acceptance_targets.clone(),
            ),
            proof_targets: task_bulk_normalize_string_list(
                task.planner_metadata.proof_targets.clone(),
            ),
            risk: task_bulk_normalize_optional_text(task.planner_metadata.risk.clone()),
            estimate: task_bulk_normalize_optional_text(task.planner_metadata.estimate.clone()),
            lane_hint: task_bulk_normalize_optional_text(task.planner_metadata.lane_hint.clone()),
        },
        provider_mapping: None,
        dependencies,
    }
}

fn task_bulk_import_validate_basic(
    existing_rows: &[state_store::TaskRecord],
    tasks: &[TaskBulkImportPlannedTask],
) -> Vec<TaskBulkImportValidationError> {
    let existing_ids = existing_rows
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    let existing_display_ids = existing_rows
        .iter()
        .filter_map(|task| {
            task.display_id
                .as_deref()
                .map(|display_id| (display_id, task.id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut batch_ids = BTreeMap::<&str, usize>::new();
    let mut batch_display_ids = BTreeMap::<&str, usize>::new();
    let mut errors = Vec::new();

    if tasks.is_empty() {
        errors.push(task_bulk_import_validation_error(
            0,
            None,
            None,
            Some("tasks"),
            "task import input contains no tasks",
        ));
        return errors;
    }

    for task in tasks {
        if let Some(previous_index) = batch_ids.get(task.task_id.as_str()).copied() {
            errors.push(task_bulk_import_validation_error(
                task.index,
                task.line,
                Some(task.task_id.clone()),
                Some("id"),
                format!("duplicate task id also appears at index {previous_index}"),
            ));
        } else {
            batch_ids.insert(task.task_id.as_str(), task.index);
        }
        if let Some(display_id) = task
            .display_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Some(previous_index) = batch_display_ids.get(display_id).copied() {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("display_id"),
                    format!("duplicate display_id also appears at index {previous_index}"),
                ));
            } else {
                batch_display_ids.insert(display_id, task.index);
            }
        }
    }

    for task in tasks {
        if existing_ids.contains(task.task_id.as_str()) {
            errors.push(task_bulk_import_validation_error(
                task.index,
                task.line,
                Some(task.task_id.clone()),
                Some("id"),
                format!("task already exists: {}", task.task_id),
            ));
        }
        if task.title.trim().is_empty() {
            errors.push(task_bulk_import_validation_error(
                task.index,
                task.line,
                Some(task.task_id.clone()),
                Some("title"),
                "task title is empty",
            ));
        }
        if state_store::work_item_requires_parent(&task.issue_type) && task.parent_id.is_none() {
            errors.push(task_bulk_import_validation_error(
                task.index,
                task.line,
                Some(task.task_id.clone()),
                Some("parent_id"),
                format!(
                    "task `{}` of type `{}` requires parent_id",
                    task.task_id, task.issue_type
                ),
            ));
        }
        if let Some(parent_id) = task.parent_id.as_deref() {
            if parent_id == task.task_id {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("parent_id"),
                    "task parent_id cannot point to itself",
                ));
            }
            if !existing_ids.contains(parent_id) && !batch_ids.contains_key(parent_id) {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("parent_id"),
                    format!(
                        "parent task does not exist in current state or import batch: {parent_id}"
                    ),
                ));
            }
        }
        if let Some(display_id) = task
            .display_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if existing_ids.contains(display_id) {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("display_id"),
                    format!("display_id `{display_id}` conflicts with an existing task id"),
                ));
            }
            if let Some(existing_task_id) = existing_display_ids.get(display_id) {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("display_id"),
                    format!(
                        "display_id `{display_id}` conflicts with existing task `{existing_task_id}`"
                    ),
                ));
            }
            if let Some(other_index) = batch_ids
                .get(display_id)
                .copied()
                .filter(|other_index| *other_index != task.index)
            {
                errors.push(task_bulk_import_validation_error(
                    task.index,
                    task.line,
                    Some(task.task_id.clone()),
                    Some("display_id"),
                    format!("display_id `{display_id}` conflicts with batch task id at index {other_index}"),
                ));
            }
        }
    }

    errors
}

fn task_bulk_import_build_plan(
    source_path: String,
    input_format: String,
    dry_run: bool,
    requested_count: usize,
    tasks: Vec<TaskBulkImportPlannedTask>,
    parse_errors: Vec<TaskBulkImportValidationError>,
    existing_rows: &[state_store::TaskRecord],
    created_by: &str,
    source_repo: &str,
) -> TaskBulkImportPlan {
    let mut validation_errors = parse_errors;
    validation_errors.extend(task_bulk_import_validate_basic(existing_rows, &tasks));

    let mut graph_issues = Vec::new();
    let planned_task_ids = tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    if validation_errors.is_empty() {
        let mut after_rows = existing_rows.to_vec();
        after_rows.extend(
            tasks
                .iter()
                .map(|task| task_bulk_import_task_record(task, created_by, source_repo)),
        );
        let touched_task_ids = tasks
            .iter()
            .flat_map(|task| {
                std::iter::once(task.task_id.clone()).chain(task.parent_id.iter().cloned())
            })
            .collect::<BTreeSet<_>>();
        graph_issues = state_store::StateStore::validate_task_graph_rows_for_mutation(
            existing_rows,
            &after_rows,
            &touched_task_ids,
        );
    }

    let result = TaskBulkImportResult {
        source_path,
        input_format,
        dry_run,
        applied: false,
        requested_count,
        planned_count: if validation_errors.is_empty() {
            tasks.len()
        } else {
            0
        },
        created_count: 0,
        validation_error_count: validation_errors.len(),
        graph_issue_count: graph_issues.len(),
        planned_task_ids,
        created_task_ids: Vec::new(),
        validation_errors,
        graph_issues,
    };
    TaskBulkImportPlan { result, tasks }
}

fn task_bulk_import_apply_order(
    existing_rows: &[state_store::TaskRecord],
    tasks: &[TaskBulkImportPlannedTask],
) -> Result<Vec<usize>, String> {
    let mut available = existing_rows
        .iter()
        .map(|task| task.id.clone())
        .collect::<BTreeSet<_>>();
    let batch_index_by_id = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.task_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut child_indexes_by_parent = BTreeMap::<&str, Vec<usize>>::new();
    let mut pending_parent_counts = vec![0usize; tasks.len()];
    let mut ready = VecDeque::new();

    for (task_index, task) in tasks.iter().enumerate() {
        match task.parent_id.as_deref() {
            Some(parent_id) if available.contains(parent_id) => ready.push_back(task_index),
            Some(parent_id) => {
                if let Some(parent_task_index) = batch_index_by_id.get(parent_id) {
                    pending_parent_counts[task_index] += 1;
                    child_indexes_by_parent
                        .entry(tasks[*parent_task_index].task_id.as_str())
                        .or_default()
                        .push(task_index);
                } else {
                    return Err(
                        "could not order imported tasks parent-first; inspect parent_id cycles or missing parents"
                            .to_string(),
                    );
                }
            }
            None => ready.push_back(task_index),
        }
    }

    let mut ordered = Vec::with_capacity(tasks.len());
    while let Some(task_index) = ready.pop_front() {
        if !available.insert(tasks[task_index].task_id.clone()) {
            continue;
        }
        ordered.push(task_index);
        if let Some(child_indexes) = child_indexes_by_parent.get(tasks[task_index].task_id.as_str())
        {
            for child_index in child_indexes {
                pending_parent_counts[*child_index] =
                    pending_parent_counts[*child_index].saturating_sub(1);
                if pending_parent_counts[*child_index] == 0 {
                    ready.push_back(*child_index);
                }
            }
        }
    }
    if ordered.len() != tasks.len() {
        return Err(
            "could not order imported tasks parent-first; inspect parent_id cycles or missing parents"
                .to_string(),
        );
    }
    Ok(ordered)
}

fn task_bulk_import_result_is_blocked(result: &TaskBulkImportResult) -> bool {
    result.validation_error_count > 0 || result.graph_issue_count > 0
}

fn task_bulk_import_result_payload(result: &TaskBulkImportResult) -> serde_json::Value {
    let mut blocker_codes = Vec::new();
    if result.validation_error_count > 0 {
        blocker_codes.extend(crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::SchemaContractMissing,
        ));
    }
    if result.graph_issue_count > 0 {
        blocker_codes.extend(crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::DependencyGraphIssues,
        ));
    }
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Repair the import file `{}` and rerun `vida task import --file {}` with --dry-run before applying.",
            result.source_path,
            crate::shell_quote(&result.source_path)
        )]
    };
    crate::release1_operator_output::build_release1_operator_output_payload(
        TASK_BULK_IMPORT_SURFACE,
        blocker_codes,
        next_actions,
        serde_json::json!({
            "surface": TASK_BULK_IMPORT_SURFACE,
            "source_path": result.source_path,
            "input_format": result.input_format,
        }),
        serde_json::json!({
            "result": result,
            "dry_run": result.dry_run,
            "applied": result.applied,
            "requested_count": result.requested_count,
            "planned_count": result.planned_count,
            "created_count": result.created_count,
            "validation_error_count": result.validation_error_count,
            "graph_issue_count": result.graph_issue_count,
        }),
    )
    .expect("task bulk import output should finalize release-1 operator output")
}

fn print_task_bulk_import_result(render: RenderMode, result: &TaskBulkImportResult, as_json: bool) {
    let payload = task_bulk_import_result_payload(result);
    if crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task bulk import result should render as json",
    ) {
        return;
    }
    print_surface_header(render, TASK_BULK_IMPORT_SURFACE);
    print_surface_line(
        render,
        "status",
        if task_bulk_import_result_is_blocked(result) {
            "blocked"
        } else {
            "pass"
        },
    );
    print_surface_line(render, "source", &result.source_path);
    print_surface_line(render, "format", &result.input_format);
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
    print_surface_line(render, "requested", &result.requested_count.to_string());
    print_surface_line(render, "planned", &result.planned_count.to_string());
    print_surface_line(render, "created", &result.created_count.to_string());
    print_surface_line(
        render,
        "validation_errors",
        &result.validation_error_count.to_string(),
    );
    print_surface_line(
        render,
        "graph_issues",
        &result.graph_issue_count.to_string(),
    );
    if let Some(error) = result.validation_errors.first() {
        print_surface_line(render, "first_error", &error.reason);
    }
}

fn task_bulk_import_blocked_result(
    command: &TaskBulkImportArgs,
    reason: impl Into<String>,
    field: &'static str,
) -> TaskBulkImportResult {
    let source_path = command.file.display().to_string();
    TaskBulkImportResult {
        source_path,
        input_format: task_bulk_import_format_label(command.format).to_string(),
        dry_run: command.dry_run,
        applied: false,
        requested_count: 0,
        planned_count: 0,
        created_count: 0,
        validation_error_count: 1,
        graph_issue_count: 0,
        planned_task_ids: Vec::new(),
        created_task_ids: Vec::new(),
        validation_errors: vec![task_bulk_import_validation_error(
            0,
            None,
            None,
            Some(field),
            reason,
        )],
        graph_issues: Vec::new(),
    }
}

async fn run_task_bulk_import(command: TaskBulkImportArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let source_path = command.file.display().to_string();
    let parsed = match parse_task_bulk_import_input(&command) {
        Ok(parsed) => parsed,
        Err(error) => {
            let result = task_bulk_import_blocked_result(&command, error, "file");
            print_task_bulk_import_result(command.render, &result, command.json);
            return ExitCode::from(2);
        }
    };
    let project_root = project_root_for_task_state(&state_dir).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let source_repo = project_root.display().to_string();
    let store = match open_task_store(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            if error.open_diagnostic(&state_dir).is_some() {
                return emit_task_state_store_open_error(
                    TASK_BULK_IMPORT_SURFACE,
                    &state_dir,
                    command.render,
                    command.json,
                    &error,
                );
            }
            let result = task_bulk_import_blocked_result(&command, error.to_string(), "state_dir");
            print_task_bulk_import_result(command.render, &result, command.json);
            return ExitCode::from(1);
        }
    };
    let existing_rows = match store.all_tasks().await {
        Ok(rows) => rows,
        Err(error) => {
            let result = task_bulk_import_blocked_result(&command, error.to_string(), "state_dir");
            print_task_bulk_import_result(command.render, &result, command.json);
            return ExitCode::from(1);
        }
    };
    let plan = task_bulk_import_build_plan(
        source_path,
        parsed.input_format,
        command.dry_run,
        parsed.requested_count,
        parsed.tasks,
        parsed.validation_errors,
        &existing_rows,
        &command.created_by,
        &source_repo,
    );
    if task_bulk_import_result_is_blocked(&plan.result) || command.dry_run {
        let exit_code = if task_bulk_import_result_is_blocked(&plan.result) {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
        print_task_bulk_import_result(command.render, &plan.result, command.json);
        return exit_code;
    }

    let order = match task_bulk_import_apply_order(&existing_rows, &plan.tasks) {
        Ok(order) => order,
        Err(error) => {
            let result = task_bulk_import_blocked_result(&command, error, "parent_id");
            print_task_bulk_import_result(command.render, &result, command.json);
            return ExitCode::from(2);
        }
    };
    let mut created_task_ids = Vec::new();
    for task_index in order {
        let task = &plan.tasks[task_index];
        let created = match store
            .create_task(state_store::CreateTaskRequest {
                task_id: &task.task_id,
                title: &task.title,
                display_id: task.display_id.as_deref(),
                description: &task.description,
                issue_type: &task.issue_type,
                status: &task.status,
                priority: task.priority,
                parent_id: task.parent_id.as_deref(),
                labels: &task.labels,
                execution_semantics: task.execution_semantics.clone(),
                planner_metadata: task.planner_metadata.clone(),
                created_by: &command.created_by,
                source_repo: &source_repo,
            })
            .await
        {
            Ok(created) => created,
            Err(error) => {
                let result = task_bulk_import_blocked_result(&command, error.to_string(), "task");
                print_task_bulk_import_result(command.render, &result, command.json);
                return ExitCode::from(1);
            }
        };
        let final_task = if let Some(notes) = task.notes.as_deref() {
            match store
                .update_task(state_store::UpdateTaskRequest {
                    task_id: &task.task_id,
                    title: None,
                    status: None,
                    priority: None,
                    notes: Some(notes),
                    description: None,
                    parent_id: None,
                    add_labels: &[],
                    remove_labels: &[],
                    set_labels: None,
                    execution_mode: None,
                    order_bucket: None,
                    parallel_group: None,
                    conflict_domain: None,
                    planner_metadata: None,
                })
                .await
            {
                Ok(updated) => updated,
                Err(error) => {
                    let result =
                        task_bulk_import_blocked_result(&command, error.to_string(), "notes");
                    print_task_bulk_import_result(command.render, &result, command.json);
                    return ExitCode::from(1);
                }
            }
        } else {
            created
        };
        created_task_ids.push(final_task.id);
    }
    if let Err(code) = refresh_task_snapshot_after_mutation(&store, TASK_BULK_IMPORT_SURFACE).await
    {
        return code;
    }
    let mut result = plan.result;
    result.applied = true;
    result.created_count = created_task_ids.len();
    result.created_task_ids = created_task_ids;
    print_task_bulk_import_result(command.render, &result, command.json);
    ExitCode::SUCCESS
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

fn build_split_mutation_preview(
    rows: &[state_store::TaskRecord],
    source: &state_store::TaskRecord,
    child_specs: &[taskflow_core::task::split::ParsedSplitChildSpec],
    reason: &str,
    surface: &str,
    dry_run: bool,
) -> Result<(TaskMutationResult, Vec<state_store::TaskRecord>), String> {
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
    if source.status == "closed" {
        simulated_rows[source_index].status = "in_progress".to_string();
        simulated_rows[source_index].closed_at = None;
        simulated_rows[source_index].close_reason = None;
    }

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
            provider_mapping: None,
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

    let blocker_labels = taskflow_core::task::spawn_blocker::merged_blocker_labels(
        &source.labels,
        &parse_label_values(&command.labels),
    );
    let blocker_priority =
        taskflow_core::task::spawn_blocker::blocker_priority(source.priority, command.priority);
    let blocker_description = taskflow_core::task::spawn_blocker::blocker_description(
        &source.id,
        &command.reason,
        command.description.as_deref(),
    );
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
        provider_mapping: None,
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
    if command.children.is_empty() {
        if command.json {
            crate::print_json_pretty(&serde_json::json!({
                "surface": surface,
                "status": "blocked",
                "blocker_codes": ["task_split_child_required"],
                "next_actions": [
                    format!("{surface} {} --child <child-id>:\"<title>\" --reason {}", crate::shell_quote(&command.task_id), crate::shell_quote(&command.reason)),
                    "Use `vida task split --help` for the child-spec format."
                ],
                "artifact_refs": {"surface": surface}
            }));
        } else {
            print_surface_header(command.render, surface);
            print_surface_line(command.render, "status", "blocked");
            print_surface_line(command.render, "blocker", "task_split_child_required");
            print_surface_line(
                command.render,
                "next",
                &format!(
                    "{surface} {} --child <child-id>:\"<title>\" --reason {}",
                    crate::shell_quote(&command.task_id),
                    crate::shell_quote(&command.reason)
                ),
            );
        }
        return ExitCode::from(2);
    }
    let child_specs = match taskflow_core::task::split::parse_split_child_specs(&command.children) {
        Ok(specs) => specs,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let store = match open_task_store(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            return emit_task_state_store_open_error(
                surface,
                &state_dir,
                command.render,
                command.json,
                &error,
            );
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
    let store = match open_task_store(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            return emit_task_state_store_open_error(
                surface,
                &state_dir,
                command.render,
                command.json,
                &error,
            );
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
            let surface = if ensure_existing {
                "vida task ensure"
            } else {
                "vida task create"
            };
            let usage =
                "vida task create <task-id> <title> OR vida task create <task-id> --title <title>";
            let next_action = format!("Provide a non-empty title with `{usage}`.");
            if command.json {
                let payload =
                    crate::release1_operator_output::Release1OperatorOutputBuilder::new(surface)
                        .blocker_codes(vec!["invalid_task_title_input".to_string()])
                        .next_actions(vec![next_action])
                        .artifact_refs(serde_json::json!({
                            "surface": surface,
                            "task_id": command.task_id.clone(),
                        }))
                        .extra_fields(serde_json::json!({
                            "reason": error,
                            "usage": usage,
                        }))
                        .build()
                        .expect(
                            "task create title error should satisfy release-1 operator contract",
                        );
                crate::print_json_pretty(&payload);
            } else {
                eprintln!("{error}");
                eprintln!("Usage: {usage}");
            }
            return ExitCode::from(2);
        }
    };
    if let Some(path) = command.notes_file.as_deref() {
        let action = format!(
            "Use `{}` for trusted inline create-time notes, or create the task first and then run `{}` when recording operator-owned progress.",
            operator_output::command_text::human_command(&format!(
                "vida task {} <task-id> <title> --notes <text> --json",
                if ensure_existing { "ensure" } else { "create" }
            )),
            operator_output::command_text::human_command(&format!(
                "vida task update <task-id> --notes-file {} --json",
                crate::shell_quote(&path.display().to_string())
            )),
        );
        if command.json {
            let surface = if ensure_existing {
                "vida task ensure"
            } else {
                "vida task create"
            };
            let payload = crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                surface,
            )
            .blocker_codes(vec!["untrusted_create_notes_file".to_string()])
            .next_actions(vec![action.clone()])
            .artifact_refs(serde_json::json!({
                "surface": surface,
                "task_id": command.task_id.clone(),
                "rejected_option": "--notes-file",
            }))
            .extra_fields(serde_json::json!({
                "rejected_option": "--notes-file",
                "rejected_path": path,
                "next_action": action,
            }))
            .build()
            .expect("task create notes-file rejection should satisfy release-1 operator contract");
            crate::print_json_pretty(&payload);
        } else {
            eprintln!(
                "Refusing --notes-file for `vida task {}`: path `{}` is outside the trusted inline intake boundary.",
                if ensure_existing { "ensure" } else { "create" },
                path.display()
            );
            eprintln!("{action}");
        }
        return ExitCode::from(2);
    }
    let notes = match resolve_optional_text_arg("notes", command.notes.as_deref(), None) {
        Ok(notes) => notes,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let planner_metadata = task_create_planner_metadata_arg(&command);
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let project_root = project_root_for_task_state(&state_dir).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    match open_task_store(state_dir.clone()).await {
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
                    let labels = parse_label_values(&command.labels);
                    if let Some(reason) = ensure_existing_task_mismatch_reason(
                        &task,
                        &title,
                        (!display_id.is_empty()).then_some(display_id.as_str()),
                        &command.issue_type,
                        &command.status,
                        parent_id.as_deref(),
                        &labels,
                    ) {
                        eprintln!("Failed to ensure task: {reason}");
                        return ExitCode::from(1);
                    }
                    if task_create_semantics_requested(&command)
                        && task_create_semantics_mismatch(&task.execution_semantics, &command)
                    {
                        eprintln!(
                            "Failed to ensure task: execution semantics mismatch for existing task; use `vida task update` to modify semantics explicitly."
                        );
                        return ExitCode::from(1);
                    }
                    print_task_mutation(command.render, "vida task ensure", &task, command.json);
                    return ExitCode::SUCCESS;
                }
            }
            let labels = parse_label_values(&command.labels);
            let source_project_root =
                crate::taskflow_task_bridge::infer_project_root_from_native_state_root_shape(
                    &state_dir,
                )
                .or_else(|| {
                    crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_dir)
                })
                .unwrap_or_else(|| project_root.clone());
            let source_repo = source_project_root.display().to_string();

            // Multi-session admission check (rule #3)
            // Check if another session holds an active exclusive claim on the same work scope
            let owner_evidence = crate::orchestrator_session_surface::build_runtime_owner_evidence(
                &state_dir, false,
            );
            let current_session_id = match &owner_evidence {
                Ok(evidence) => evidence["current_session"]["session_id"]
                    .as_str()
                    .unwrap_or("unknown"),
                Err(_) => "unknown",
            };
            let active_foreign_claims = store.active_foreign_claims(current_session_id).await;

            // Build a temporary task record for conflict checking
            let temp_execution_semantics = task_execution_semantics_from_create_args(&command);
            let temp_planner_metadata: state_store::TaskPlannerMetadata = planner_metadata.clone();
            let temp_task_id = command.task_id.trim().to_string();

            // Check for foreign claim conflicts
            if let Ok(foreign_claims) = &active_foreign_claims {
                // Use the same conflict checking logic as taskflow_proxy
                // We need to check if any foreign claim conflicts with our task
                let has_conflict = foreign_claims.iter().any(|claim| {
                    let claim_status = claim.status.trim().to_ascii_lowercase();
                    let claim_is_blocking_status = claim_status == "blocked";
                    let claim_is_exclusive = claim.lease_mode == "exclusive";
                    if !claim_is_blocking_status && !claim_is_exclusive {
                        return false;
                    }
                    // Check same task_id
                    if claim.task_id.as_deref() == Some(temp_task_id.as_str()) {
                        return true;
                    }
                    // Check same conflict_domain
                    if let Some(claim_domain) = claim.conflict_domain.as_deref() {
                        if temp_execution_semantics.conflict_domain.as_deref() == Some(claim_domain)
                        {
                            return true;
                        }
                    }
                    // Check intersecting owned_paths (from planner_metadata, not execution_semantics)
                    if !claim.owned_paths.is_empty()
                        && !temp_planner_metadata.owned_paths.is_empty()
                    {
                        for claim_path in &claim.owned_paths {
                            for task_path in &temp_planner_metadata.owned_paths {
                                if paths_intersect(claim_path, task_path) {
                                    return true;
                                }
                            }
                        }
                    }
                    // Exclusive claims also block writes intersecting their read-only paths.
                    if claim_is_exclusive {
                        if !claim.read_only_paths.is_empty()
                            && !temp_planner_metadata.owned_paths.is_empty()
                        {
                            for claim_path in &claim.read_only_paths {
                                for task_path in &temp_planner_metadata.owned_paths {
                                    if paths_intersect(claim_path, task_path) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    false
                });

                if has_conflict {
                    if command.json {
                        let surface = if ensure_existing {
                            "vida task ensure"
                        } else {
                            "vida task create"
                        };
                        let next_action = operator_output::command_text::human_command(
                            "vida orchestrator-session show --json",
                        );
                        let payload =
                            crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                                surface,
                            )
                            .blocker_codes(vec!["foreign_claim_conflict_blocked".to_string()])
                            .next_actions(vec![next_action.clone()])
                            .artifact_refs(serde_json::json!({
                                "surface": surface,
                                "current_session_id": current_session_id,
                                "blocking_surface": "vida orchestrator-session show",
                            }))
                            .extra_fields(serde_json::json!({
                                "reason": "Another orchestrator session holds an active exclusive claim on the same task, run, conflict domain, or intersecting paths. Wait for that session to complete or explicitly reclaim/supersede the claim before continuing.",
                                "next_action": next_action,
                                "blocking_surface": "vida orchestrator-session show",
                                "current_session_id": current_session_id,
                            }))
                            .build()
                            .expect("foreign claim conflict payload should satisfy release-1 operator contract");
                        crate::print_json_pretty(&payload);
                    } else {
                        eprintln!(
                            "Another orchestrator session holds an active exclusive claim on this work scope."
                        );
                        eprintln!(
                            "Inspect active sessions and claims with `vida orchestrator-session show`"
                        );
                    }
                    return ExitCode::from(1);
                }
            }

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
                    planner_metadata: planner_metadata.clone(),
                    created_by: "vida task",
                    source_repo: &source_repo,
                })
                .await
            {
                Ok(task) => {
                    let task = if let Some(notes) = notes.as_deref() {
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: None,
                            })
                            .await
                        {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!("Failed to apply task notes after create: {error}");
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        task
                    };
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
                    let surface = if ensure_existing {
                        "vida task ensure"
                    } else {
                        "vida task create"
                    };
                    if maybe_emit_task_create_invalid_parent_kind_error(
                        surface,
                        command.render,
                        command.json,
                        &command.task_id,
                        &command.issue_type,
                        parent_id.as_deref(),
                        &error,
                    ) {
                        return ExitCode::from(1);
                    }
                    eprintln!(
                        "Failed to {} task: {error}",
                        if ensure_existing { "ensure" } else { "create" }
                    );
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => emit_task_state_store_open_error(
            if ensure_existing {
                "vida task ensure"
            } else {
                "vida task create"
            },
            &state_dir,
            command.render,
            command.json,
            &error,
        ),
    }
}

fn ensure_existing_task_mismatch_reason(
    task: &state_store::TaskRecord,
    expected_title: &str,
    expected_display_id: Option<&str>,
    expected_issue_type: &str,
    expected_status: &str,
    expected_parent_id: Option<&str>,
    expected_labels: &[String],
) -> Option<String> {
    let existing_parent_id = task_parent_id(task);
    taskflow_core::task::create::ensure_existing_task_mismatch_reason(
        taskflow_core::task::create::ExistingTaskActual {
            task_id: &task.id,
            title: &task.title,
            display_id: task.display_id.as_deref(),
            issue_type: &task.issue_type,
            status: &task.status,
            parent_id: existing_parent_id.as_deref(),
            labels: &task.labels,
        },
        taskflow_core::task::create::ExistingTaskExpectation {
            title: expected_title,
            display_id: expected_display_id,
            issue_type: expected_issue_type,
            status: expected_status,
            parent_id: expected_parent_id,
            labels: expected_labels,
        },
    )
}

fn task_create_title(command: &TaskCreateArgs) -> Result<String, String> {
    taskflow_core::task::create::task_create_title(
        taskflow_core::task::create::TaskCreateTitleInput {
            positional_title: command.positional_title.as_deref(),
            title_option: command.title.as_deref(),
        },
    )
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
    stage_error_detail: Option<String>,
    commit_message: Option<String>,
    commit_exit_code: Option<i32>,
    push_exit_code: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct TaskOwnedStatusReceipt {
    pub(crate) status: String,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) task_id: String,
    pub(crate) repo_root: String,
    pub(crate) active_step: Option<TaskOwnedStatusUnit>,
    pub(crate) active_parent_task: Option<TaskOwnedStatusUnit>,
    pub(crate) active_epic: Option<TaskOwnedStatusUnit>,
    pub(crate) ownership_source: String,
    pub(crate) owned_paths: Vec<String>,
    pub(crate) dirty_files: Vec<String>,
    pub(crate) owned_files: Vec<String>,
    pub(crate) unowned_files: Vec<String>,
    pub(crate) unowned_paths: Vec<String>,
    pub(crate) matched_files: Vec<String>,
    pub(crate) unmatched_files: Vec<String>,
    pub(crate) stageable_files: Vec<String>,
    pub(crate) confidence: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct TaskOwnedStatusUnit {
    pub(crate) task_id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) issue_type: String,
    pub(crate) owned_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskDirtyClassifyReceipt {
    status: String,
    repo_root: String,
    dirty_files: Vec<String>,
    groups: Vec<TaskDirtyClassifyGroup>,
    unclassified: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskDirtyClassifyGroup {
    task_id: String,
    epic_id: Option<String>,
    files: Vec<String>,
    confidence: String,
    reasons: Vec<String>,
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
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    shared_fields: serde_json::Value,
    operator_contracts: serde_json::Value,
    active_bounded_unit: serde_json::Value,
    binding_source: Option<String>,
    why_this_unit: String,
    sequential_vs_parallel_posture: String,
    recommended_primary: Option<TaskContinuationCandidate>,
    recommended_parallel_batch: Vec<TaskContinuationCandidate>,
    why_not_auto_bound: Option<String>,
    bind_command: Option<String>,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_codes: Vec<String>,
    next_action: Option<String>,
    next_actions: Vec<String>,
    source_surfaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ambiguity_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_explanation: Option<serde_json::Value>,
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
            status: false,
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
            next_actions.push(format!(
                "Fix release build failures, then rerun `{}`.",
                operator_output::command_text::human_command("vida task close --release --json")
            ));
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
        command.commit.then(|| {
            let reason = command
                .reason
                .as_deref()
                .or_else(|| task.and_then(|task| task.close_reason.as_deref()))
                .unwrap_or("reason file evidence");
            format!("Close {}: {}", command.task_id, reason)
        })
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

    let ignored_dirty_files = if command.commit {
        match dirty_paths_for_repo(&repo_root) {
            Ok(dirty_paths) => {
                task_close_ignored_dirty_files_for_explicit_commit(dirty_paths, &explicit_files)
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
    } else {
        Vec::new()
    };

    if command.commit {
        let stage_files: Vec<std::path::PathBuf> = explicit_files
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let add_status = std::process::Command::new("git")
            .arg("add")
            .arg("--")
            .args(&stage_files)
            .current_dir(&repo_root)
            .output();
        match add_status {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let failure = classify_task_close_git_stage_failure(
                    String::from_utf8_lossy(&output.stderr).trim(),
                    None,
                );
                return blocked_task_close_git_receipt_with_stage_detail(
                    explicit_files,
                    commit_message,
                    failure.blocker_code,
                    failure.next_action,
                    Some(failure.detail),
                );
            }
            Err(error) => {
                let failure = classify_task_close_git_stage_failure("", Some(&error));
                return blocked_task_close_git_receipt_with_stage_detail(
                    explicit_files,
                    commit_message,
                    failure.blocker_code,
                    failure.next_action,
                    Some(failure.detail),
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
                            next_actions: task_close_commit_allowlist_next_actions(
                                &ignored_dirty_files,
                            ),
                            explicit_files,
                            stage_error_detail: None,
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
                            stage_error_detail: None,
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
                            stage_error_detail: None,
                            commit_message,
                            commit_exit_code: status.code(),
                            push_exit_code: None,
                        },
                    }
                } else {
                    TaskCloseGitAutomationReceipt {
                        status: "pass".to_string(),
                        blocker_codes: Vec::new(),
                        next_actions: task_close_commit_allowlist_next_actions(
                            &ignored_dirty_files,
                        ),
                        explicit_files,
                        stage_error_detail: None,
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
                stage_error_detail: None,
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
                stage_error_detail: None,
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
            stage_error_detail: None,
            commit_message,
            commit_exit_code: None,
            push_exit_code: None,
        }
    }
}

struct TaskCloseGitStageFailure<'a> {
    blocker_code: &'a str,
    next_action: &'a str,
    detail: String,
}

fn classify_task_close_git_stage_failure(
    stderr: &str,
    error: Option<&std::io::Error>,
) -> TaskCloseGitStageFailure<'static> {
    let normalized_stderr = stderr.trim();
    let normalized_lower = normalized_stderr.to_ascii_lowercase();
    if normalized_lower.contains("read-only")
        || normalized_lower.contains("permission denied")
        || normalized_lower.contains("operation not permitted")
        || normalized_lower.contains("sandbox")
    {
        let detail = if normalized_stderr.is_empty() {
            "git add failed because the worktree appears read-only or sandbox-blocked".to_string()
        } else {
            normalized_stderr.to_string()
        };
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_read_only_or_sandbox_blocked",
            next_action: "Make the worktree writable or rerun outside the blocking sandbox, then retry the task-close command.",
            detail,
        };
    }
    if normalized_lower.contains("index.lock") {
        let detail = if normalized_stderr.is_empty() {
            "git add failed because `.git/index.lock` is present".to_string()
        } else {
            normalized_stderr.to_string()
        };
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_index_lock_blocked",
            next_action: "Clear the `.git/index.lock` blocker or stop the concurrent git writer, then retry the task-close command.",
            detail,
        };
    }
    if let Some(error) = error {
        let detail = format!("git add failed to start: {error}");
        let lower = detail.to_ascii_lowercase();
        if lower.contains("read-only")
            || lower.contains("permission denied")
            || lower.contains("operation not permitted")
            || lower.contains("sandbox")
        {
            return TaskCloseGitStageFailure {
                blocker_code: "git_stage_read_only_or_sandbox_blocked",
                next_action: "Make the worktree writable or rerun outside the blocking sandbox, then retry the task-close command.",
                detail,
            };
        }
        if lower.contains("index.lock") {
            return TaskCloseGitStageFailure {
                blocker_code: "git_stage_index_lock_blocked",
                next_action: "Clear the `.git/index.lock` blocker or stop the concurrent git writer, then retry the task-close command.",
                detail,
            };
        }
        return TaskCloseGitStageFailure {
            blocker_code: "git_stage_failed",
            next_action: "Verify the explicit commit files exist and can be staged.",
            detail,
        };
    }

    TaskCloseGitStageFailure {
        blocker_code: "git_stage_failed",
        next_action: "Verify the explicit commit files exist and can be staged.",
        detail: if normalized_stderr.is_empty() {
            "git add failed without stderr output".to_string()
        } else {
            normalized_stderr.to_string()
        },
    }
}

fn task_close_commit_allowlist_next_actions(ignored_dirty_files: &[String]) -> Vec<String> {
    if ignored_dirty_files.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Ignored {} unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied.",
            ignored_dirty_files.len()
        )]
    }
}

fn task_close_ignored_dirty_files_for_explicit_commit(
    dirty_paths: Vec<String>,
    explicit_files: &[String],
) -> Vec<String> {
    dirty_paths
        .into_iter()
        .filter(|path| !path_is_explicitly_owned(path, explicit_files))
        .collect()
}

fn blocked_task_close_git_receipt(
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    blocker_code: &str,
    next_action: &str,
) -> TaskCloseGitAutomationReceipt {
    blocked_task_close_git_receipt_with_stage_detail(
        explicit_files,
        commit_message,
        blocker_code,
        next_action,
        None,
    )
}

fn blocked_task_close_git_receipt_with_stage_detail(
    explicit_files: Vec<String>,
    commit_message: Option<String>,
    blocker_code: &str,
    next_action: &str,
    stage_error_detail: Option<String>,
) -> TaskCloseGitAutomationReceipt {
    TaskCloseGitAutomationReceipt {
        status: "blocked".to_string(),
        blocker_codes: vec![blocker_code.to_string()],
        next_actions: vec![next_action.to_string()],
        explicit_files,
        stage_error_detail,
        commit_message,
        commit_exit_code: None,
        push_exit_code: None,
    }
}

fn task_close_commit_file_strings(
    command: &TaskCloseArgs,
    task: Option<&state_store::TaskRecord>,
) -> Vec<String> {
    taskflow_core::task::close::task_close_commit_file_strings(
        command
            .commit_files
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        command.stage_owned,
        task.map(|task| task.planner_metadata.owned_paths.clone()),
    )
}

pub(crate) fn task_owned_status_receipt(
    task_id: &str,
    metadata_owned_paths: Vec<String>,
    override_files: Vec<String>,
    dirty_files: Vec<String>,
    repo_root: String,
    active_step: Option<TaskOwnedStatusUnit>,
    active_parent_task: Option<TaskOwnedStatusUnit>,
    active_epic: Option<TaskOwnedStatusUnit>,
) -> TaskOwnedStatusReceipt {
    let override_files = taskflow_core::task::close::canonical_owned_paths(override_files);
    let metadata_owned_paths =
        taskflow_core::task::close::canonical_owned_paths(metadata_owned_paths);
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
            repo_root,
            active_step,
            active_parent_task,
            active_epic,
            ownership_source,
            owned_paths,
            dirty_files,
            owned_files: Vec::new(),
            unowned_files: Vec::new(),
            unowned_paths: Vec::new(),
            matched_files: Vec::new(),
            unmatched_files: Vec::new(),
            stageable_files: Vec::new(),
            confidence: "none".to_string(),
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
    let confidence = if blocked {
        "mixed"
    } else if dirty_files.is_empty() {
        "clean"
    } else {
        "high"
    };

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
        repo_root,
        active_step,
        active_parent_task,
        active_epic,
        ownership_source,
        owned_paths,
        dirty_files,
        owned_files: owned_files.clone(),
        unowned_files: unowned_files.clone(),
        unowned_paths: unowned_files.clone(),
        matched_files: owned_files.clone(),
        unmatched_files: unowned_files,
        stageable_files,
        confidence: confidence.to_string(),
    }
}

pub(crate) fn task_owned_status_unit(task: &state_store::TaskRecord) -> TaskOwnedStatusUnit {
    TaskOwnedStatusUnit {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        owned_paths: task.planner_metadata.owned_paths.clone(),
    }
}

fn task_dirty_classify_receipt(
    rows: &[state_store::TaskRecord],
    dirty_files: Vec<String>,
    repo_root: String,
) -> TaskDirtyClassifyReceipt {
    let mut classified = BTreeSet::new();
    let mut groups = Vec::new();
    let mut candidates: Vec<&state_store::TaskRecord> = rows
        .iter()
        .filter(|task| task.closed_at.is_none())
        .filter(|task| !task.planner_metadata.owned_paths.is_empty())
        .collect();
    candidates.sort_by(|left, right| {
        task_dirty_candidate_rank(right)
            .cmp(&task_dirty_candidate_rank(left))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.id.cmp(&right.id))
    });

    for task in candidates {
        let owned_paths = taskflow_core::task::close::canonical_owned_paths(
            task.planner_metadata.owned_paths.clone(),
        );
        let files: Vec<String> = dirty_files
            .iter()
            .filter(|path| !classified.contains(*path))
            .filter(|path| path_is_explicitly_owned(path, &owned_paths))
            .cloned()
            .collect();
        if files.is_empty() {
            continue;
        }
        for file in &files {
            classified.insert(file.clone());
        }
        let epic_id = task_ancestor_with_issue_type(rows, task, "epic").map(|epic| epic.id.clone());
        let mut reasons = vec!["dirty file matched planner_metadata.owned_paths".to_string()];
        if task.status == "in_progress" {
            reasons.push("task is in_progress".to_string());
        } else {
            reasons.push(format!("task status is {}", task.status));
        }
        if !task.planner_metadata.proof_targets.is_empty() {
            reasons.push("task has known proof targets".to_string());
        }
        if epic_id.is_some() {
            reasons.push("task resolves to parent epic".to_string());
        }
        let confidence = if task.status == "in_progress" {
            "high"
        } else if task.issue_type == "step" || !task.planner_metadata.proof_targets.is_empty() {
            "medium"
        } else {
            "low"
        };
        groups.push(TaskDirtyClassifyGroup {
            task_id: task.id.clone(),
            epic_id,
            files,
            confidence: confidence.to_string(),
            reasons,
        });
    }

    let unclassified: Vec<String> = dirty_files
        .iter()
        .filter(|path| !classified.contains(*path))
        .cloned()
        .collect();
    let next_actions = if dirty_files.is_empty() {
        vec!["No dirty files detected in the current git worktree.".to_string()]
    } else if unclassified.is_empty() {
        vec!["Review groups, then stage only files for the selected bounded task.".to_string()]
    } else {
        vec![
            "Review unclassified files before staging or create/update TaskFlow ownership metadata."
                .to_string(),
        ]
    };
    let status = if unclassified.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    TaskDirtyClassifyReceipt {
        status: status.to_string(),
        repo_root,
        dirty_files,
        groups,
        unclassified,
        next_actions,
    }
}

fn task_dirty_candidate_rank(task: &state_store::TaskRecord) -> u8 {
    if task.status == "in_progress" && task.issue_type == "step" {
        5
    } else if task.status == "in_progress" {
        4
    } else if task.issue_type == "step" {
        3
    } else if !task.planner_metadata.proof_targets.is_empty() {
        2
    } else {
        1
    }
}

fn print_task_dirty_classify_receipt(
    render: RenderMode,
    receipt: &TaskDirtyClassifyReceipt,
    as_json: bool,
) {
    if as_json {
        crate::print_json_pretty(&serde_json::to_value(receipt).unwrap_or_else(|_| {
            serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["classify_dirty_serialization_failed"],
            })
        }));
        return;
    }
    let _ = render;
    println!("status: {}", receipt.status);
    println!("repo_root: {}", receipt.repo_root);
    println!("dirty_files: {}", receipt.dirty_files.len());
    println!("groups: {}", receipt.groups.len());
    println!("unclassified: {}", receipt.unclassified.len());
    for action in &receipt.next_actions {
        println!("- {action}");
    }
}

pub(crate) fn task_record_by_id<'a>(
    rows: &'a [state_store::TaskRecord],
    task_id: &str,
) -> Option<&'a state_store::TaskRecord> {
    rows.iter().find(|task| task.id == task_id)
}

fn task_ancestor_with_issue_type<'a>(
    rows: &'a [state_store::TaskRecord],
    task: &'a state_store::TaskRecord,
    issue_type: &str,
) -> Option<&'a state_store::TaskRecord> {
    let mut current = task;
    while let Some(parent_id) = task_parent_id(current) {
        let Some(parent) = task_record_by_id(rows, &parent_id) else {
            return None;
        };
        if parent.issue_type == issue_type {
            return Some(parent);
        }
        current = parent;
    }
    None
}

fn task_is_descendant_of(
    rows: &[state_store::TaskRecord],
    task: &state_store::TaskRecord,
    ancestor_id: &str,
) -> bool {
    let mut current = task;
    while let Some(parent_id) = task_parent_id(current) {
        if parent_id == ancestor_id {
            return true;
        }
        let Some(parent) = task_record_by_id(rows, &parent_id) else {
            return false;
        };
        current = parent;
    }
    false
}

pub(crate) fn active_step_for_owned_status<'a>(
    rows: &'a [state_store::TaskRecord],
    selected_task_id: Option<&str>,
) -> Option<&'a state_store::TaskRecord> {
    rows.iter().find(|task| {
        task.issue_type == "step"
            && task.status == "in_progress"
            && selected_task_id
                .map(|selected| task_is_descendant_of(rows, task, selected))
                .unwrap_or(true)
    })
}

pub(crate) fn task_owned_status_context(
    rows: &[state_store::TaskRecord],
    selected_task: &state_store::TaskRecord,
    include_active_step: bool,
) -> (
    Option<TaskOwnedStatusUnit>,
    Option<TaskOwnedStatusUnit>,
    Option<TaskOwnedStatusUnit>,
) {
    if !include_active_step {
        return (None, None, None);
    }
    let active_step = active_step_for_owned_status(rows, Some(&selected_task.id));
    let parent_task = active_step
        .and_then(|step| task_parent_id(step))
        .and_then(|parent_id| task_record_by_id(rows, &parent_id))
        .unwrap_or(selected_task);
    let active_epic = task_ancestor_with_issue_type(rows, parent_task, "epic");
    (
        active_step.map(task_owned_status_unit),
        Some(task_owned_status_unit(parent_task)),
        active_epic.map(task_owned_status_unit),
    )
}

fn select_task_for_owned_status(
    rows: &[state_store::TaskRecord],
    requested_task_id: Option<&str>,
    with_active_step: bool,
) -> Result<state_store::TaskRecord, state_store::StateStoreError> {
    if let Some(task_id) = requested_task_id {
        return resolve_task_from_rows(rows, task_id);
    }
    if with_active_step {
        if let Some(step) = active_step_for_owned_status(rows, None) {
            if let Some(parent_id) = task_parent_id(step) {
                if let Some(parent) = task_record_by_id(rows, &parent_id) {
                    return Ok(parent.clone());
                }
            }
            return Ok(step.clone());
        }
    }
    rows.iter()
        .find(|task| task.status == "in_progress" && task.issue_type != "step")
        .cloned()
        .ok_or_else(|| state_store::StateStoreError::MissingTask {
            task_id: "active owned-status task".to_string(),
        })
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

fn task_handoff_receipt_path(
    receipt_root: &std::path::Path,
    task_id: &str,
    filename_timestamp: &str,
) -> std::path::PathBuf {
    task_handoff_receipt_dir(receipt_root).join(format!(
        "{}-{}.json",
        taskflow_core::task::handoff::sanitize_task_handoff_receipt_component(task_id),
        filename_timestamp
    ))
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
    let changed_files = taskflow_core::task::close::canonical_owned_paths(
        command
            .files
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    );
    let proof_commands =
        taskflow_core::task::handoff::canonical_nonempty_strings(command.proofs.clone());
    let blocker_codes =
        taskflow_core::task::handoff::canonical_nonempty_strings(command.blockers.clone());
    let next_actions =
        taskflow_core::task::handoff::canonical_nonempty_strings(command.next_actions.clone());
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

fn task_next_lawful_bind_command(candidate: &TaskContinuationCandidate) -> String {
    format!(
        "vida taskflow run-graph dispatch-init {} --json",
        crate::shell_quote(&candidate.task_id)
    )
}

fn task_next_lawful_recommended_primary(
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Option<TaskContinuationCandidate> {
    ready_task_candidates.first().cloned()
}

fn task_next_lawful_recommended_parallel_batch(
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Vec<TaskContinuationCandidate> {
    ready_task_candidates
        .iter()
        .filter(|candidate| candidate.ready_parallel_safe)
        .cloned()
        .collect()
}

fn task_next_lawful_unique_top_priority_candidate(
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Option<TaskContinuationCandidate> {
    let top_priority = ready_task_candidates
        .iter()
        .map(|candidate| candidate.priority)
        .min()?;
    let mut top_candidates = ready_task_candidates
        .iter()
        .filter(|candidate| candidate.priority == top_priority);
    let candidate = top_candidates.next()?.clone();
    if top_candidates.next().is_none() {
        Some(candidate)
    } else {
        None
    }
}

fn task_epic_ancestor_id(tasks: &[state_store::TaskRecord], task_id: &str) -> Option<String> {
    let by_id = tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut current_id = task_id;
    let mut visited = std::collections::BTreeSet::<String>::new();
    loop {
        if !visited.insert(current_id.to_string()) {
            return None;
        }
        let task = by_id.get(current_id)?;
        if state_store::work_item_is_program_container(&task.issue_type) {
            return Some(task.id.clone());
        }
        let Some(parent_id) = task_parent_id(task) else {
            return None;
        };
        current_id = by_id.get(parent_id.as_str())?.id.as_str();
    }
}

fn task_next_lawful_apply_strategy(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    strategy: Option<&str>,
) -> Vec<TaskContinuationCandidate> {
    match strategy.unwrap_or("default") {
        "epic-sequential" => {
            let Some(primary) = ready_task_candidates.first() else {
                return ready_task_candidates;
            };
            let primary_epic_id = task_epic_ancestor_id(tasks, &primary.task_id);
            ready_task_candidates
                .into_iter()
                .filter(|candidate| {
                    task_epic_ancestor_id(tasks, &candidate.task_id) == primary_epic_id
                })
                .collect()
        }
        _ => ready_task_candidates,
    }
}

fn task_next_lawful_why_not_auto_bound(
    blocker_code: Option<&str>,
    ready_task_candidates: &[TaskContinuationCandidate],
) -> Option<String> {
    match blocker_code {
        Some("ambiguous_ready_task_candidates") => Some(format!(
            "multiple ready candidates ({}) require an explicit bounded-unit binding; recommendations are ranked guidance only",
            ready_task_candidates.len()
        )),
        Some("multiple_active_tasks") => Some(
            "multiple active TaskFlow tasks require reconciliation before automatic binding".to_string(),
        ),
        Some("runtime_ready_candidate_conflict") => Some(
            "runtime binding conflicts with ready TaskFlow candidates, so operator confirmation is required".to_string(),
        ),
        Some("continuation_source_drift") => Some(
            "continuation sources disagree, so automatic binding would risk selecting the wrong bounded unit".to_string(),
        ),
        Some(_) => Some("blocking runtime evidence prevents automatic binding".to_string()),
        None => None,
    }
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
        "vida taskflow run-graph status --json projection_truth.continuation_binding".to_string(),
    ]
}

fn task_next_lawful_ambiguity_reason(receipt: &TaskNextLawfulReceipt) -> Option<String> {
    if receipt.status != "blocked" || !receipt.active_bounded_unit.is_null() {
        return None;
    }
    receipt
        .why_not_auto_bound
        .clone()
        .or_else(|| receipt.next_action.clone())
}

fn task_next_lawful_artifact_refs(receipt: &TaskNextLawfulReceipt) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida task next-lawful",
        "active_bounded_unit": receipt.active_bounded_unit.clone(),
        "binding_source": receipt.binding_source.clone(),
        "ambiguity_reason": receipt.ambiguity_reason.clone(),
        "recommended_primary_task_id": receipt
            .recommended_primary
            .as_ref()
            .map(|candidate| candidate.task_id.clone()),
        "bind_command": receipt.bind_command.clone(),
        "ready_task_candidate_count": receipt.ready_task_candidates.len(),
        "source_surfaces": receipt.source_surfaces.clone(),
    })
}

fn finalize_task_next_lawful_receipt(mut receipt: TaskNextLawfulReceipt) -> TaskNextLawfulReceipt {
    receipt.ambiguity_reason = task_next_lawful_ambiguity_reason(&receipt);
    let operator_contracts = render_operator_contract_envelope(
        &receipt.status,
        receipt.blocker_codes.clone(),
        receipt.next_actions.clone(),
        task_next_lawful_artifact_refs(&receipt),
    );
    let trace_id = operator_contracts["trace_id"]
        .as_str()
        .map(ToOwned::to_owned);
    let workflow_class = operator_contracts["workflow_class"]
        .as_str()
        .map(ToOwned::to_owned);
    let risk_tier = operator_contracts["risk_tier"]
        .as_str()
        .map(ToOwned::to_owned);
    let artifact_refs = operator_contracts["artifact_refs"].clone();
    let status = operator_contracts["status"]
        .as_str()
        .unwrap_or(&receipt.status)
        .to_string();

    receipt.status = status.clone();
    receipt.trace_id = trace_id.clone();
    receipt.workflow_class = workflow_class.clone();
    receipt.risk_tier = risk_tier.clone();
    receipt.artifact_refs = artifact_refs.clone();
    receipt.shared_fields = serde_json::json!({
        "trace_id": trace_id,
        "workflow_class": workflow_class,
        "risk_tier": risk_tier,
        "status": status,
        "blocker_codes": receipt.blocker_codes.clone(),
        "next_actions": receipt.next_actions.clone(),
        "artifact_refs": artifact_refs,
    });
    receipt.operator_contracts = operator_contracts;
    receipt
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

fn continuation_binding_has_live_unit(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    if !continuation_binding_requires_open_task(binding) {
        return true;
    }
    task_status_for_binding(tasks, binding).is_some_and(|status| status != "closed")
}

fn continuation_binding_is_closed_downstream_marker(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
) -> bool {
    !continuation_binding_requires_open_task(binding)
        && task_status_for_binding(tasks, binding).is_some_and(|status| status == "closed")
}

fn continuation_binding_is_retired_terminal_closure_marker(
    tasks: &[state_store::TaskRecord],
    binding: &state_store::RunGraphContinuationBinding,
    status: Option<&state_store::RunGraphStatus>,
) -> bool {
    continuation_binding_requires_open_task(binding)
        && binding.binding_source == "consume_continue_after_downstream_chain"
        && binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str)
            == Some("run_graph_task")
        && task_status_for_binding(tasks, binding).is_some_and(|status| status == "closed")
        && status.is_some_and(|status| {
            status.run_id == binding.run_id
                && status.task_id == binding.task_id
                && status.active_node == "closure"
                && status.next_node.is_none()
                && status.status == "completed"
                && status.lifecycle_stage == "closure_complete"
                && matches!(
                    status.policy_gate.as_str(),
                    "historical_closed_task_stale_run_retired"
                        | "closed_task_stale_run_retired"
                        | "not_required"
                )
                && status.handoff_state == "none"
                && status.context_state == "sealed"
                && status.resume_target == "none"
                && !status.recovery_ready
        })
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

fn continuation_binding_is_superseded_same_task_explicit(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && explicit.run_id != current.run_id
        && explicit.task_id == current.task_id
}

fn continuation_binding_is_unscoped_dispatch_init_projection(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "run_graph_dispatch_init"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
}

fn continuation_binding_is_unrelated_prelaunch_blocked_projection(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "dispatch_prelaunch_blocked"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
}

fn continuation_binding_is_newer_explicit_task_override(
    explicit: &state_store::RunGraphContinuationBinding,
    current: &state_store::RunGraphContinuationBinding,
) -> bool {
    explicit.binding_source == "explicit_continuation_bind_task"
        && current.binding_source == "explicit_continuation_bind"
        && explicit.run_id != current.run_id
        && explicit.task_id != current.task_id
        && explicit.recorded_at > current.recorded_at
}

fn select_task_next_lawful_binding<'a>(
    tasks: &[state_store::TaskRecord],
    explicit_binding: Option<&'a state_store::RunGraphContinuationBinding>,
    current_binding: Option<&'a state_store::RunGraphContinuationBinding>,
) -> Result<Option<&'a state_store::RunGraphContinuationBinding>, TaskNextLawfulReceipt> {
    let has_single_active_task =
        crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks).len() == 1;
    match (explicit_binding, current_binding) {
        (Some(explicit), Some(current)) if !continuation_bindings_same_unit(explicit, current) => {
            let explicit_live = continuation_binding_has_live_unit(tasks, explicit);
            let current_live = continuation_binding_has_live_unit(tasks, current);
            if continuation_binding_is_historical_task_close_reconcile(explicit, current)
                && current_live
            {
                return Ok(Some(current));
            }
            if continuation_binding_is_superseded_same_task_explicit(explicit, current)
                && current_live
            {
                return Ok(Some(current));
            }
            if continuation_binding_is_unscoped_dispatch_init_projection(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            if continuation_binding_is_unrelated_prelaunch_blocked_projection(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            if continuation_binding_is_newer_explicit_task_override(explicit, current)
                && explicit_live
            {
                return Ok(Some(explicit));
            }
            match (explicit_live, current_live) {
                (false, false) => return Ok(None),
                (false, true) => return Ok(Some(current)),
                (true, false) => return Ok(Some(explicit)),
                (true, true) => {}
            }
            let recovery_command = operator_output::command_text::human_command(&format!(
                "vida taskflow recovery status {}",
                crate::shell_quote(&current.run_id)
            ));
            let lane_show_command = operator_output::command_text::human_command(&format!(
                "vida lane show {}",
                crate::shell_quote(&current.run_id)
            ));
            let explicit_status_command = operator_output::command_text::human_command(&format!(
                "vida taskflow run-graph status {}",
                crate::shell_quote(&explicit.run_id)
            ));
            Err(blocked_task_next_lawful_receipt(
                explicit.active_bounded_unit.clone(),
                Vec::new(),
                "continuation_source_drift",
                &format!(
                    "Continuation sources disagree: explicit binding `{}`/`{}` points to `{}`, while current latest-run binding `{}`/`{}` from `{}` points to `{}`. Inspect current blocked-run recovery with `{recovery_command}`, lane evidence with `{lane_show_command}`, and explicit binding state with `{explicit_status_command}` before continuing.",
                    explicit.run_id,
                    explicit.binding_source,
                    explicit.task_id,
                    current.run_id,
                    current.binding_source,
                    current.binding_source,
                    current.task_id,
                ),
            ))
        }
        (Some(explicit), Some(_current)) => {
            if continuation_binding_has_live_unit(tasks, explicit) {
                Ok(Some(explicit))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(explicit))
            }
        }
        (Some(explicit), None) => {
            if continuation_binding_has_live_unit(tasks, explicit) {
                Ok(Some(explicit))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(explicit))
            }
        }
        (None, Some(current)) => {
            if continuation_binding_has_live_unit(tasks, current) {
                Ok(Some(current))
            } else if has_single_active_task {
                Ok(None)
            } else {
                Ok(Some(current))
            }
        }
        (None, None) => Ok(None),
    }
}

fn blocked_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    blocker_code: &str,
    next_action: &str,
) -> TaskNextLawfulReceipt {
    blocked_task_next_lawful_receipt_with_blockers(
        active_bounded_unit,
        ready_task_candidates,
        vec![blocker_code.to_string()],
        next_action,
        true,
    )
}

fn blocked_task_next_lawful_receipt_with_blockers(
    active_bounded_unit: serde_json::Value,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    mut blocker_codes: Vec<String>,
    next_action: &str,
    include_bind_command: bool,
) -> TaskNextLawfulReceipt {
    if blocker_codes.is_empty() {
        blocker_codes.push("runtime_recovery_blocked".to_string());
    }
    let next_actions = vec![next_action.to_string()];
    let recommended_primary = task_next_lawful_recommended_primary(&ready_task_candidates);
    let bind_command = include_bind_command
        .then(|| {
            recommended_primary
                .as_ref()
                .map(task_next_lawful_bind_command)
        })
        .flatten();
    let recommended_parallel_batch =
        task_next_lawful_recommended_parallel_batch(&ready_task_candidates);
    let primary_blocker = blocker_codes.first().map(String::as_str);
    let why_not_auto_bound =
        task_next_lawful_why_not_auto_bound(primary_blocker, &ready_task_candidates);
    finalize_task_next_lawful_receipt(TaskNextLawfulReceipt {
        status: "blocked".to_string(),
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: serde_json::Value::Null,
        shared_fields: serde_json::Value::Null,
        operator_contracts: serde_json::Value::Null,
        active_bounded_unit,
        binding_source: None,
        why_this_unit: "blocked_until_unique_lawful_continuation_is_evidenced".to_string(),
        sequential_vs_parallel_posture: "unknown_until_explicit_binding".to_string(),
        recommended_primary,
        recommended_parallel_batch,
        why_not_auto_bound,
        bind_command,
        ready_task_candidates,
        blocker_codes,
        next_action: next_actions.first().cloned(),
        next_actions,
        source_surfaces: task_continuation_source_surfaces(),
        ambiguity_reason: None,
        operator_explanation: None,
    })
}

fn pass_task_next_lawful_receipt(
    active_bounded_unit: serde_json::Value,
    binding_source: Option<String>,
    why_this_unit: &str,
    sequential_vs_parallel_posture: &str,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    next_action: String,
) -> TaskNextLawfulReceipt {
    let next_actions = vec![next_action];
    let recommended_primary = task_next_lawful_recommended_primary(&ready_task_candidates);
    let bind_command = recommended_primary
        .as_ref()
        .map(task_next_lawful_bind_command);
    let recommended_parallel_batch =
        task_next_lawful_recommended_parallel_batch(&ready_task_candidates);
    finalize_task_next_lawful_receipt(TaskNextLawfulReceipt {
        status: task_json_success_status().to_string(),
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: serde_json::Value::Null,
        shared_fields: serde_json::Value::Null,
        operator_contracts: serde_json::Value::Null,
        active_bounded_unit,
        binding_source,
        why_this_unit: why_this_unit.to_string(),
        sequential_vs_parallel_posture: sequential_vs_parallel_posture.to_string(),
        recommended_primary,
        recommended_parallel_batch,
        why_not_auto_bound: None,
        bind_command,
        ready_task_candidates,
        blocker_codes: Vec::new(),
        next_action: next_actions.first().cloned(),
        next_actions,
        source_surfaces: task_continuation_source_surfaces(),
        ambiguity_reason: None,
        operator_explanation: None,
    })
}

fn task_next_lawful_attach_explanation(
    mut receipt: TaskNextLawfulReceipt,
    explain: bool,
    strategy: Option<&str>,
    selected_task_id: Option<&str>,
) -> TaskNextLawfulReceipt {
    if explain {
        receipt.operator_explanation = Some(serde_json::json!({
            "strategy": strategy.unwrap_or("default"),
            "selected_task_id": selected_task_id,
            "status": receipt.status,
            "blocker_codes": receipt.blocker_codes,
            "why_this_unit": receipt.why_this_unit,
            "why_not_auto_bound": receipt.why_not_auto_bound,
            "bind_command": receipt.bind_command,
            "candidate_count": receipt.ready_task_candidates.len()
        }));
    }
    receipt
}

fn task_next_lawful_select_ready_candidate_receipt(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    selected_task_id: &str,
) -> TaskNextLawfulReceipt {
    match crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks).as_slice() {
        [active] => {
            return blocked_task_next_lawful_receipt(
                task_continuation_active_unit(active),
                ready_task_candidates,
                "select_conflicts_with_active_taskflow_task",
                &format!(
                    "TaskFlow task `{}` is already in_progress; continue or close it before selecting another continuation item.",
                    active.id
                ),
            );
        }
        [] => {}
        _ => {
            return blocked_task_next_lawful_receipt(
                serde_json::Value::Null,
                ready_task_candidates,
                "multiple_active_tasks",
                "Close or reconcile extra in_progress tasks before selecting a continuation item.",
            );
        }
    }

    let Some(selected_index) = ready_task_candidates
        .iter()
        .position(|candidate| candidate.task_id == selected_task_id)
    else {
        return blocked_task_next_lawful_receipt(
            serde_json::Value::Null,
            ready_task_candidates,
            "selected_task_not_ready",
            &format!(
                "Selected task `{}` is not a ready lawful candidate; choose one of the returned ready_task_candidates.",
                selected_task_id
            ),
        );
    };
    let mut ordered_candidates = ready_task_candidates;
    let selected = ordered_candidates.remove(selected_index);
    ordered_candidates.insert(0, selected.clone());
    pass_task_next_lawful_receipt(
        serde_json::json!({
            "task_id": selected.task_id,
            "title": selected.title,
            "status": selected.status,
            "issue_type": selected.issue_type,
        }),
        Some("operator_selected_ready_candidate".to_string()),
        "Operator selected a ready TaskFlow candidate with --select.",
        if selected.ready_parallel_safe {
            "parallel_safe_operator_selected_candidate"
        } else {
            "sequential_only_operator_selected_candidate"
        },
        ordered_candidates,
        format!(
            "Bind selected ready task `{}` with the returned bind_command.",
            selected_task_id
        ),
    )
}

fn runtime_binding_task_closed_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let run_id = binding.run_id.trim();
    if run_id.is_empty() {
        return crate::status_surface_signals::continuation_binding_ambiguous_next_action()
            .to_string();
    }
    let run_id = crate::shell_quote(run_id);
    let recovery_command = operator_output::command_text::human_command(&format!(
        "vida taskflow recovery status {run_id} --json"
    ));
    let continue_command =
        operator_output::command_text::human_command("vida taskflow consume continue --json");
    format!(
        "Runtime binding points to closed task `{}` for run `{run_id}`. Inspect the concrete recovery state with `{recovery_command}`; resolve or retire the blocked run, then refresh continuation evidence with `{continue_command}` before selecting the next bounded step.",
        binding.task_id
    )
}

fn runtime_binding_task_paused_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let task_id = crate::shell_quote(&binding.task_id);
    let run_id = crate::shell_quote(&binding.run_id);
    let resume_command = operator_output::command_text::human_command(&format!(
        "vida task update {task_id} --status in_progress --json"
    ));
    let bind_command = operator_output::command_text::human_command(&format!(
        "vida taskflow continuation bind {run_id} --task-id <task-id> --json"
    ));
    format!(
        "Runtime binding points to paused task `{}`. Resume it with `{resume_command}`, or bind a different lawful unit with `{bind_command}` if the pause is still intentional.",
        binding.task_id
    )
}

fn runtime_binding_task_missing_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let base = crate::status_surface_signals::runtime_binding_task_missing_next_action(
        Some(binding.run_id.as_str()),
        &binding.task_id,
    );
    let run_id = binding.run_id.trim();
    if run_id.is_empty() {
        return base;
    }
    let run_id = crate::shell_quote(run_id);
    let bind_command = operator_output::command_text::human_command(&format!(
        "vida taskflow continuation bind {run_id} --task-id <task-id> --json"
    ));
    format!(
        "{base} After recovery proves the run is safe to rebind, record the explicit replacement with `{bind_command}` for missing task `{}`.",
        binding.task_id
    )
}

fn runtime_binding_open_delegated_cycle_next_action(
    binding: &state_store::RunGraphContinuationBinding,
) -> String {
    let lane_show_command = operator_output::command_text::human_command(&format!(
        "vida lane show {} --json",
        crate::shell_quote(&binding.run_id)
    ));
    let recovery_command = operator_output::command_text::human_command(&format!(
        "vida taskflow recovery status {} --json",
        crate::shell_quote(&binding.run_id)
    ));
    format!(
        "Runtime binding for task `{}` is still inside an open delegated cycle for run `{}`. Inspect `{lane_show_command}` and `{recovery_command}`; wait for a receipt-backed delegated completion or record structured exception takeover before selecting another TaskFlow step.",
        binding.task_id, binding.run_id
    )
}

fn push_unique_next_lawful_blocker(blocker_codes: &mut Vec<String>, blocker_code: &str) {
    let Some(canonical) = crate::release1_contracts::canonical_blocker_code_str(blocker_code)
    else {
        return;
    };
    if !blocker_codes.iter().any(|code| code == canonical) {
        blocker_codes.push(canonical.to_string());
    }
}

fn runtime_recovery_task_next_lawful_blocker_codes(
    recovery: Option<&state_store::RunGraphRecoverySummary>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if let Some(recovery) = recovery {
        if recovery.delegation_gate.delegated_cycle_open
            || recovery.delegation_gate.local_exception_takeover_gate
                == "blocked_open_delegated_cycle"
        {
            push_unique_next_lawful_blocker(&mut blocker_codes, "open_delegated_cycle");
        }
        if let Some(blocker_code) = recovery.delegation_gate.blocker_code.as_deref() {
            push_unique_next_lawful_blocker(&mut blocker_codes, blocker_code);
        }
    }
    if let (Some(recovery), Some(dispatch)) = (recovery, dispatch) {
        if dispatch.run_id == recovery.run_id {
            if let Some(blocker_code) = dispatch.blocker_code.as_deref() {
                push_unique_next_lawful_blocker(&mut blocker_codes, blocker_code);
            }
        }
    }
    if blocker_codes.is_empty() {
        push_unique_next_lawful_blocker(&mut blocker_codes, "open_delegated_cycle");
    }
    blocker_codes
}

fn blocked_runtime_recovery_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    recovery: Option<&state_store::RunGraphRecoverySummary>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> TaskNextLawfulReceipt {
    blocked_task_next_lawful_receipt_with_blockers(
        binding.active_bounded_unit.clone(),
        ready_task_candidates,
        runtime_recovery_task_next_lawful_blocker_codes(recovery, dispatch),
        &runtime_binding_open_delegated_cycle_next_action(binding),
        false,
    )
}

fn runtime_dispatch_receipt_has_ready_downstream_handoff(
    expected_run_id: Option<&str>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    crate::runtime_dispatch_receipt_helpers::dispatch_summary_has_clean_ready_downstream_handoff(
        dispatch,
        expected_run_id,
    )
}

fn runtime_dispatch_receipt_has_completed_lane(
    expected_run_id: Option<&str>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    crate::runtime_dispatch_receipt_helpers::dispatch_summary_has_clean_completed_lane(
        dispatch,
        expected_run_id,
    )
}

fn downstream_dispatch_command_for_task_next_lawful(
    dispatch: &state_store::RunGraphDispatchReceiptSummary,
) -> Option<String> {
    crate::continuation_binding_summary::downstream_dispatch_command_for_summary(dispatch)
}

fn runtime_recovery_blocks_task_next_lawful(
    recovery: Option<&state_store::RunGraphRecoverySummary>,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    recovery.is_some_and(|recovery| {
        (recovery.delegation_gate.delegated_cycle_open
            || recovery.delegation_gate.local_exception_takeover_gate
                == "blocked_open_delegated_cycle"
            || recovery.resume_status == "running")
            && !runtime_dispatch_receipt_has_ready_downstream_handoff(
                Some(recovery.run_id.as_str()),
                dispatch,
            )
            && !runtime_dispatch_receipt_has_completed_lane(
                Some(recovery.run_id.as_str()),
                dispatch,
            )
    })
}

fn runtime_binding_has_active_exception_takeover(
    binding: &state_store::RunGraphContinuationBinding,
    dispatch: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> bool {
    let Some(dispatch) = dispatch else {
        return false;
    };
    let exception_takeover_state = crate::release1_contracts::exception_takeover_state(
        dispatch.exception_path_receipt_id.as_deref(),
        dispatch.supersedes_receipt_id.as_deref(),
        None,
    );
    dispatch.run_id == binding.run_id
        && (dispatch.lane_status == "lane_exception_takeover"
            || exception_takeover_state.is_active())
        && dispatch
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && dispatch
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn pass_exception_takeover_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
) -> TaskNextLawfulReceipt {
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_exception_takeover_dispatch".to_string()),
        &format!(
            "Latest runtime dispatch records exception-takeover evidence for task `{}`.",
            binding.task_id
        ),
        "sequential_only_exception_takeover",
        ready_task_candidates,
        format!(
            "Finish the active exception-takeover unit for `{}` before selecting another TaskFlow step.",
            binding.task_id
        ),
    )
}

fn pass_ready_downstream_handoff_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    terminal_consume_continue_run_id: Option<&str>,
    downstream_dispatch_command: Option<&str>,
) -> TaskNextLawfulReceipt {
    let next_action = if terminal_consume_continue_run_id == Some(binding.run_id.as_str()) {
        downstream_dispatch_command
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|command| {
                let command = operator_output::command_text::human_command(command);
                format!(
                    "Continue `{}` with downstream handoff command `{}`.",
                    binding.task_id, command
                )
            })
            .unwrap_or_else(|| {
                let lane_show_command = operator_output::command_text::human_command(&format!(
                    "vida lane show {} --json",
                    crate::shell_quote(&binding.run_id)
                ));
                format!("Inspect `{}` with `{lane_show_command}`.", binding.task_id)
            })
    } else {
        let continue_command = operator_output::command_text::human_command(&format!(
            "vida taskflow consume continue --run-id {} --json",
            crate::shell_quote(&binding.run_id)
        ));
        format!("Continue `{}` with `{continue_command}`.", binding.task_id)
    };
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_ready_downstream_handoff".to_string()),
        &format!(
            "Latest runtime dispatch records a ready downstream handoff for task `{}`.",
            binding.task_id
        ),
        "sequential_only_downstream_bound",
        ready_task_candidates,
        next_action,
    )
}

fn pass_completed_lane_task_next_lawful_receipt(
    binding: &state_store::RunGraphContinuationBinding,
    ready_task_candidates: Vec<TaskContinuationCandidate>,
) -> TaskNextLawfulReceipt {
    pass_task_next_lawful_receipt(
        binding.active_bounded_unit.clone(),
        Some("latest_run_graph_completed_dispatch_receipt".to_string()),
        &format!(
            "Latest dispatch receipt records completed delegated lane evidence for task `{}`.",
            binding.task_id
        ),
        "sequential_only_completed_lane_reconciled",
        ready_task_candidates,
        format!(
            "Continue `{}` after completed delegated lane reconciliation; inspect `vida taskflow run-graph status {}` if downstream binding is still expected.",
            binding.task_id,
            crate::shell_quote(&binding.run_id)
        ),
    )
}

fn task_next_lawful_receipt(
    tasks: &[state_store::TaskRecord],
    ready_task_candidates: Vec<TaskContinuationCandidate>,
    runtime_binding: Option<&state_store::RunGraphContinuationBinding>,
) -> TaskNextLawfulReceipt {
    let active_tasks = crate::continuation_binding_summary::taskflow_leaf_active_tasks(tasks);

    if let Some(binding) = runtime_binding {
        let binding_task = tasks.iter().find(|task| task.id == binding.task_id);
        let missing_runtime_binding_with_single_active_task =
            continuation_binding_requires_open_task(binding)
                && binding_task.is_none()
                && active_tasks.len() == 1;
        if !missing_runtime_binding_with_single_active_task
            && !continuation_binding_is_closed_downstream_marker(tasks, binding)
        {
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
                        &runtime_binding_task_missing_next_action(binding),
                    );
                };
                if task.status == "closed" {
                    return blocked_task_next_lawful_receipt(
                        binding.active_bounded_unit.clone(),
                        ready_task_candidates,
                        "runtime_binding_task_closed",
                        &runtime_binding_task_closed_next_action(binding),
                    );
                }
                if task.status == "paused" {
                    return blocked_task_next_lawful_receipt(
                        binding.active_bounded_unit.clone(),
                        ready_task_candidates,
                        "runtime_binding_task_paused",
                        &runtime_binding_task_paused_next_action(binding),
                    );
                }
            }
            let ready_conflict = ready_task_candidates
                .iter()
                .any(|candidate| candidate.task_id != binding.task_id);
            if ready_conflict
                && binding.binding_source != "explicit_continuation_bind_task"
                && !ready_task_candidates
                    .iter()
                    .any(|candidate| candidate.task_id == binding.task_id)
            {
                let next_action =
                    crate::status_surface_signals::continuation_binding_ambiguous_next_action();
                return blocked_task_next_lawful_receipt(
                    binding.active_bounded_unit.clone(),
                    ready_task_candidates,
                    "runtime_ready_candidate_conflict",
                    &next_action,
                );
            }
            return pass_task_next_lawful_receipt(
                binding.active_bounded_unit.clone(),
                Some(binding.binding_source.clone()),
                &binding.why_this_unit,
                &binding.sequential_vs_parallel_posture,
                ready_task_candidates,
                format!(
                    "Continue `{}` via the bound runtime path: {}.",
                    binding.task_id, binding.primary_path
                ),
            );
        }
    }

    match active_tasks.as_slice() {
        [active] => pass_task_next_lawful_receipt(
            task_continuation_active_unit(active),
            Some("taskflow_single_in_progress".to_string()),
            "Single TaskFlow in_progress task is the authoritative active bounded unit.",
            "sequential_only_taskflow_active",
            ready_task_candidates,
            format!("Continue active task `{}`.", active.id),
        ),
        [] => match ready_task_candidates.as_slice() {
            [candidate] => pass_task_next_lawful_receipt(
                serde_json::json!({
                    "task_id": candidate.task_id,
                    "title": candidate.title,
                    "status": candidate.status,
                    "issue_type": candidate.issue_type,
                }),
                None,
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
            _ => {
                if let Some(candidate) =
                    task_next_lawful_unique_top_priority_candidate(&ready_task_candidates)
                {
                    pass_task_next_lawful_receipt(
                        serde_json::json!({
                            "task_id": candidate.task_id.clone(),
                            "title": candidate.title.clone(),
                            "status": candidate.status.clone(),
                            "issue_type": candidate.issue_type.clone(),
                        }),
                        None,
                        "unique highest-priority ready TaskFlow candidate after close/release automation",
                        if candidate.ready_parallel_safe {
                            "parallel_safe_unique_top_priority_candidate"
                        } else {
                            "sequential_only_unique_top_priority_candidate"
                        },
                        ready_task_candidates.clone(),
                        format!(
                            "Continue unique highest-priority ready task `{}`.",
                            candidate.task_id
                        ),
                    )
                } else {
                    blocked_task_next_lawful_receipt(
                        serde_json::Value::Null,
                        ready_task_candidates,
                        "ambiguous_ready_task_candidates",
                        "Multiple ready tasks share continuation priority; choose and bind the intended bounded unit explicitly before implementation.",
                    )
                }
            }
        },
        _ => blocked_task_next_lawful_receipt(
            serde_json::Value::Null,
            ready_task_candidates,
            "multiple_active_tasks",
            "Close or reconcile extra in_progress tasks before selecting a continuation item.",
        ),
    }
}

pub(crate) fn dirty_paths_for_repo(repo_root: &std::path::Path) -> Result<Vec<String>, String> {
    let output = git_status_output_for_repo(repo_root)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(porcelain_status_path)
        .collect::<Vec<_>>())
}

pub(crate) fn dirty_repo_root_for_current_process() -> std::path::PathBuf {
    let cwd_root = std::env::current_dir()
        .ok()
        .and_then(|cwd| nearest_git_worktree_root(&cwd));
    let exe_root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().and_then(nearest_git_worktree_root));
    if let Some(root) = cwd_root.as_ref() {
        if dirty_paths_for_repo(root).is_ok_and(|paths| !paths.is_empty()) {
            return root.clone();
        }
    }
    if let Some(root) = exe_root.as_ref() {
        if dirty_paths_for_repo(root).is_ok_and(|paths| !paths.is_empty()) {
            return root.clone();
        }
    }
    cwd_root
        .or(exe_root)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn nearest_git_worktree_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    start
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(std::path::Path::to_path_buf)
}

fn git_status_output_for_repo(repo_root: &std::path::Path) -> Result<std::process::Output, String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output();
    match output {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && cfg!(windows) => {
            std::process::Command::new("C:\\Program Files\\Git\\cmd\\git.exe")
                .args(["status", "--porcelain"])
                .current_dir(repo_root)
                .output()
                .map_err(|fallback_error| fallback_error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn git_text_for_repo(repo_root: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn git_diff_text_for_paths(
    repo_root: &std::path::Path,
    diff_args: &[&str],
    paths: &[String],
) -> Result<String, String> {
    if paths.is_empty() {
        return Ok(String::new());
    }

    let output = std::process::Command::new("git")
        .args(diff_args)
        .arg("--")
        .args(paths)
        .current_dir(repo_root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn limit_lines(text: &str, max_lines: usize) -> (String, bool) {
    let mut lines = text.lines();
    let limited = lines.by_ref().take(max_lines).collect::<Vec<_>>();
    let truncated = lines.next().is_some();
    (limited.join("\n"), truncated)
}

fn limit_diff_hunks(text: &str, max_hunks: usize, max_lines: usize) -> (String, bool) {
    let mut hunk_count = 0usize;
    let mut line_count = 0usize;
    let mut truncated = false;
    let mut selected = Vec::new();

    for line in text.lines() {
        if line.starts_with("@@") {
            hunk_count += 1;
            if hunk_count > max_hunks {
                truncated = true;
                break;
            }
        }
        if line_count >= max_lines {
            truncated = true;
            break;
        }
        selected.push(line);
        line_count += 1;
    }

    (selected.join("\n"), truncated)
}

fn validator_blocker_history(task: &state_store::TaskRecord, max_lines: usize) -> Vec<String> {
    task.notes
        .as_deref()
        .unwrap_or("")
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("validator")
                || lower.contains("blocking_findings")
                || lower.contains("residual_risk")
                || lower.contains("blocked")
        })
        .take(max_lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn task_validator_packet_payload(
    task: &state_store::TaskRecord,
    read_metadata: &TaskReadMetadata,
    repo_root: &std::path::Path,
    command: &TaskValidatorPacketArgs,
) -> serde_json::Value {
    let dirty_paths = dirty_paths_for_repo(repo_root).unwrap_or_default();
    let owned_files = if task.planner_metadata.owned_paths.is_empty() {
        dirty_paths.clone()
    } else {
        task.planner_metadata.owned_paths.clone()
    };
    let proof_commands = if command.proofs.is_empty() {
        task.planner_metadata.proof_targets.clone()
    } else {
        command.proofs.clone()
    };
    let diffstat = git_diff_text_for_paths(repo_root, &["diff", "--stat", "HEAD"], &owned_files)
        .unwrap_or_else(|error| format!("git_diff_stat_failed: {error}"));
    let diff = git_diff_text_for_paths(repo_root, &["diff", "--unified=3", "HEAD"], &owned_files)
        .unwrap_or_else(|error| format!("git_diff_failed: {error}"));
    let (diff_hunks, diff_truncated) =
        limit_diff_hunks(&diff, command.max_hunks, command.max_lines);
    let (diffstat_limited, diffstat_truncated) = limit_lines(&diffstat, 40);
    let prior_validator_blockers = validator_blocker_history(task, 20);
    let requested_schema = serde_json::json!({
        "verdict": "PASS|BLOCKED",
        "blocking_findings": ["file:line - blocking issue, or empty"],
        "residual_risks": ["non-blocking risk, or empty"],
        "evidence_checked": ["commands/files reviewed"],
    });

    serde_json::json!({
        "surface": "vida task validator-packet",
        "active_bounded_unit": {
            "task_id": task.id,
            "title": task.title,
            "status": task.status,
            "issue_type": task.issue_type,
            "priority": task.priority,
        },
        "read_metadata": read_metadata,
        "repo_root": repo_root.display().to_string(),
        "owned_files": owned_files,
        "dirty_files": dirty_paths,
        "diffstat": diffstat_limited,
        "diffstat_truncated": diffstat_truncated,
        "key_hunks": diff_hunks,
        "key_hunks_truncated": diff_truncated,
        "proof_commands": proof_commands,
        "prior_validator_blockers": prior_validator_blockers,
        "requested_schema": requested_schema,
    })
}

fn render_validator_packet_text(payload: &serde_json::Value) -> String {
    let unit = &payload["active_bounded_unit"];
    let list = |field: &str| -> String {
        payload[field]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|item| format!("- {item}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "- none".to_string())
    };
    format!(
        "VALIDATOR PACKET\nactive_bounded_unit: {task_id}\ntitle: {title}\nstatus: {status}\nrepo_root: {repo_root}\n\nOWNED FILES\n{owned_files}\n\nDIRTY FILES\n{dirty_files}\n\nDIFFSTAT\n{diffstat}\n\nKEY HUNKS\n{key_hunks}\n\nPROOF COMMANDS\n{proof_commands}\n\nPRIOR VALIDATOR BLOCKERS\n{prior_validator_blockers}\n\nREQUESTED RESPONSE SCHEMA\nVERDICT: PASS|BLOCKED\nBLOCKING_FINDINGS:\nRESIDUAL_RISKS:\nEVIDENCE_CHECKED:\n",
        task_id = unit["task_id"].as_str().unwrap_or("unknown"),
        title = unit["title"].as_str().unwrap_or(""),
        status = unit["status"].as_str().unwrap_or("unknown"),
        repo_root = payload["repo_root"].as_str().unwrap_or(""),
        owned_files = list("owned_files"),
        dirty_files = list("dirty_files"),
        diffstat = payload["diffstat"].as_str().unwrap_or(""),
        key_hunks = payload["key_hunks"].as_str().unwrap_or(""),
        proof_commands = list("proof_commands"),
        prior_validator_blockers = list("prior_validator_blockers"),
    )
}

async fn run_task_validator_packet(command: TaskValidatorPacketArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let repo_root = project_root_for_task_state(&state_dir)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    match task_show_authoritative_first(state_dir, &command.task_id).await {
        Ok((task, metadata)) => {
            let payload = task_validator_packet_payload(&task, &metadata, &repo_root, &command);
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                let text = render_validator_packet_text(&payload);
                if matches!(command.render, crate::RenderMode::Plain) {
                    println!("{text}");
                } else {
                    print_surface_header(command.render, "vida task validator-packet");
                    print_surface_line(command.render, "packet", &text);
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if command.json {
                crate::print_json_pretty(&serde_json::json!({
                    "status": "blocked",
                    "surface": "vida task validator-packet",
                    "blocker_codes": ["task_validator_packet_failed"],
                    "next_actions": ["Run `vida task show <task-id>` to verify the task exists, then retry validator-packet."],
                    "error": error.to_string(),
                    "task_id": command.task_id,
                }));
            } else {
                eprintln!("task validator-packet failed: {error}");
            }
            ExitCode::from(1)
        }
    }
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

async fn run_task_attempt_dispatch(command: TaskAttemptDispatchArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open_existing(state_dir).await {
        Ok(store) => match task_attempt_dispatch_records(&store, &command).await {
            Ok((attempts, stage_policy)) => {
                let summary = store
                    .task_stage_summary(&command.task_id, &command.stage_id)
                    .await
                    .ok();
                print_task_attempt_payload(
                    "vida task attempt dispatch",
                    command.render,
                    command.json,
                    task_attempt_dispatch_payload(
                        "vida task attempt dispatch",
                        &command.task_id,
                        &command.stage_id,
                        attempts,
                        summary,
                        stage_policy,
                    ),
                )
            }
            Err(error) => print_task_attempt_payload(
                "vida task attempt dispatch",
                command.render,
                command.json,
                task_attempt_error_payload(
                    "vida task attempt dispatch",
                    Some(command.task_id.as_str()),
                    Some(command.stage_id.as_str()),
                    None,
                    &error,
                ),
            ),
        },
        Err(error) => print_task_attempt_payload(
            "vida task attempt dispatch",
            command.render,
            command.json,
            task_attempt_store_error_payload("vida task attempt dispatch", &error),
        ),
    }
}

async fn run_task_attempt_status(command: TaskAttemptStatusArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open_existing_read_only(state_dir).await {
        Ok(store) => match store
            .task_stage_summary(&command.task_id, &command.stage_id)
            .await
        {
            Ok(summary) => print_task_attempt_payload(
                "vida task attempt status",
                command.render,
                command.json,
                task_attempt_summary_payload("vida task attempt status", summary),
            ),
            Err(error) => print_task_attempt_payload(
                "vida task attempt status",
                command.render,
                command.json,
                task_attempt_error_payload(
                    "vida task attempt status",
                    Some(command.task_id.as_str()),
                    Some(command.stage_id.as_str()),
                    None,
                    &error,
                ),
            ),
        },
        Err(error) => print_task_attempt_payload(
            "vida task attempt status",
            command.render,
            command.json,
            task_attempt_store_error_payload("vida task attempt status", &error),
        ),
    }
}

async fn run_task_attempt_collect(command: TaskAttemptCollectArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let state_root = std::path::PathBuf::from(&state_dir);
    match StateStore::open_existing(state_dir).await {
        Ok(store) => {
            let attempt_id = match command.attempt_id.clone() {
                Some(attempt_id) => attempt_id,
                None => {
                    match latest_task_stage_attempt_id(&store, &command.task_id, &command.stage_id)
                        .await
                    {
                        Ok(attempt_id) => attempt_id,
                        Err(error) => {
                            return print_task_attempt_payload(
                                "vida task attempt collect",
                                command.render,
                                command.json,
                                task_attempt_error_payload(
                                    "vida task attempt collect",
                                    Some(command.task_id.as_str()),
                                    Some(command.stage_id.as_str()),
                                    None,
                                    &error,
                                ),
                            );
                        }
                    }
                }
            };
            let existing_attempt = match store.task_attempt(&attempt_id).await {
                Ok(attempt) => attempt,
                Err(error) => {
                    return print_task_attempt_payload(
                        "vida task attempt collect",
                        command.render,
                        command.json,
                        task_attempt_error_payload(
                            "vida task attempt collect",
                            Some(command.task_id.as_str()),
                            Some(command.stage_id.as_str()),
                            Some(attempt_id.as_str()),
                            &error,
                        ),
                    );
                }
            };
            let artifact_refs =
                if taskflow_core::task::attempts::normalize_artifact_refs(&command.artifact_refs)
                    .is_empty()
                {
                    existing_attempt.artifact_refs.clone()
                } else {
                    command.artifact_refs.clone()
                };
            let validated_artifacts = match validate_attempt_artifacts_for_task(
                &store,
                &existing_attempt,
                &artifact_refs,
                state_root.as_path(),
            )
            .await
            {
                Ok(values) => values,
                Err(reason) => {
                    let error = state_store::StateStoreError::InvalidTaskRecord {
                        reason: format!("attempt_artifact_validation_failed: {reason}"),
                    };
                    return print_task_attempt_payload(
                        "vida task attempt collect",
                        command.render,
                        command.json,
                        task_attempt_artifact_error_payload(
                            "vida task attempt collect",
                            Some(command.task_id.as_str()),
                            Some(command.stage_id.as_str()),
                            Some(attempt_id.as_str()),
                            &error,
                        ),
                    );
                }
            };
            let request = state_store::TransitionTaskAttemptRequest {
                attempt_id: attempt_id.clone(),
                task_id: command.task_id.clone(),
                stage_id: command.stage_id.clone(),
                status: command.status,
                artifact_refs,
                consolidation_receipt_id: command.consolidation_receipt_id,
            };
            match store.transition_task_attempt(request).await {
                Ok(attempt) => {
                    let summary = store
                        .task_stage_summary(&attempt.task_id, &attempt.stage_id)
                        .await
                        .ok();
                    print_task_attempt_payload(
                        "vida task attempt collect",
                        command.render,
                        command.json,
                        task_attempt_collect_payload(
                            "vida task attempt collect",
                            &attempt,
                            summary,
                            validated_artifacts,
                        ),
                    )
                }
                Err(error) => print_task_attempt_payload(
                    "vida task attempt collect",
                    command.render,
                    command.json,
                    task_attempt_error_payload(
                        "vida task attempt collect",
                        Some(command.task_id.as_str()),
                        Some(command.stage_id.as_str()),
                        Some(attempt_id.as_str()),
                        &error,
                    ),
                ),
            }
        }
        Err(error) => print_task_attempt_payload(
            "vida task attempt collect",
            command.render,
            command.json,
            task_attempt_store_error_payload("vida task attempt collect", &error),
        ),
    }
}

async fn run_task_attempt_consolidate(command: TaskAttemptConsolidateArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let state_root = std::path::PathBuf::from(&state_dir);
    match StateStore::open_existing(state_dir).await {
        Ok(store) => {
            match task_attempt_consolidation_for_command(&store, &command, state_root.as_path())
                .await
            {
                Ok((receipt, summary)) => print_task_attempt_payload(
                    "vida task attempt consolidate",
                    command.render,
                    command.json,
                    task_attempt_consolidation_payload(
                        "vida task attempt consolidate",
                        receipt,
                        summary,
                    ),
                ),
                Err(error) => print_task_attempt_payload(
                    "vida task attempt consolidate",
                    command.render,
                    command.json,
                    if task_attempt_error_is_artifact_validation(&error) {
                        task_attempt_artifact_error_payload(
                            "vida task attempt consolidate",
                            Some(command.task_id.as_str()),
                            Some(command.stage_id.as_str()),
                            None,
                            &error,
                        )
                    } else {
                        task_attempt_error_payload(
                            "vida task attempt consolidate",
                            Some(command.task_id.as_str()),
                            Some(command.stage_id.as_str()),
                            None,
                            &error,
                        )
                    },
                ),
            }
        }
        Err(error) => print_task_attempt_payload(
            "vida task attempt consolidate",
            command.render,
            command.json,
            task_attempt_store_error_payload("vida task attempt consolidate", &error),
        ),
    }
}

async fn run_task_stage(command: TaskStageArgs) -> ExitCode {
    match command.command {
        TaskStageCommand::Status(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match task_stage_status_payload_for_command(&store, &command).await {
                    Ok(payload) => print_task_attempt_payload(
                        "vida task stage status",
                        command.render,
                        command.json,
                        payload,
                    ),
                    Err(error) => print_task_attempt_payload(
                        "vida task stage status",
                        command.render,
                        command.json,
                        task_attempt_error_payload(
                            "vida task stage status",
                            Some(command.task_id.as_str()),
                            command.stage_id.as_deref(),
                            None,
                            &error,
                        ),
                    ),
                },
                Err(error) => print_task_attempt_payload(
                    "vida task stage status",
                    command.render,
                    command.json,
                    task_attempt_store_error_payload("vida task stage status", &error),
                ),
            }
        }
    }
}

async fn latest_task_stage_attempt_id(
    store: &StateStore,
    task_id: &str,
    stage_id: &str,
) -> Result<String, state_store::StateStoreError> {
    let summary = store.task_stage_summary(task_id, stage_id).await?;
    summary.latest_attempt_id.ok_or_else(|| {
        state_store::StateStoreError::InvalidTaskRecord {
            reason: format!(
                "task attempt collect requires an attempt id because task `{task_id}` stage `{stage_id}` has no attempts"
            ),
        }
    })
}

#[derive(Default)]
struct TaskAttemptArtifactConsolidation {
    artifact_refs: Vec<String>,
    facts: Vec<String>,
    hypotheses: Vec<String>,
    conflicts: Vec<String>,
    partial_attempt_ids: Vec<String>,
    timeout_attempt_ids: Vec<String>,
    cap_limited_attempt_ids: Vec<String>,
}

async fn task_attempt_consolidation_for_command(
    store: &StateStore,
    command: &TaskAttemptConsolidateArgs,
    state_root: &std::path::Path,
) -> Result<
    (
        state_store::TaskStageConsolidationReceipt,
        Option<state_store::TaskStageSummary>,
    ),
    state_store::StateStoreError,
> {
    let attempts = store
        .task_stage_attempts(&command.task_id, &command.stage_id)
        .await?;
    let task = store.show_task(&command.task_id).await?;
    let mut consolidated =
        consolidate_attempt_artifacts(&attempts, &task.planner_metadata.owned_paths, state_root)
            .map_err(|reason| state_store::StateStoreError::InvalidTaskRecord {
                reason: format!("attempt_artifact_validation_failed: {reason}"),
            })?;
    taskflow_core::task::attempts::merge_repeated_values(&mut consolidated.facts, &command.facts);
    taskflow_core::task::attempts::merge_repeated_values(
        &mut consolidated.hypotheses,
        &command.hypotheses,
    );
    taskflow_core::task::attempts::merge_repeated_values(
        &mut consolidated.conflicts,
        &command.conflicts,
    );
    taskflow_core::task::attempts::merge_repeated_values(
        &mut consolidated.partial_attempt_ids,
        &command.partial_attempt_ids,
    );
    taskflow_core::task::attempts::merge_repeated_values(
        &mut consolidated.timeout_attempt_ids,
        &command.timeout_attempt_ids,
    );
    taskflow_core::task::attempts::merge_repeated_values(
        &mut consolidated.cap_limited_attempt_ids,
        &command.cap_limited_attempt_ids,
    );
    let receipt = store
        .consolidate_task_stage_attempts(state_store::ConsolidateTaskStageAttemptsRequest {
            receipt_id: command.consolidation_receipt_id.clone(),
            task_id: command.task_id.clone(),
            stage_id: command.stage_id.clone(),
            consolidator_profile: command.consolidator_profile.clone(),
            merge_policy: command.merge_policy.clone(),
            artifact_refs: consolidated.artifact_refs,
            facts: consolidated.facts,
            hypotheses: consolidated.hypotheses,
            conflicts: consolidated.conflicts,
            partial_attempt_ids: consolidated.partial_attempt_ids,
            timeout_attempt_ids: consolidated.timeout_attempt_ids,
            cap_limited_attempt_ids: consolidated.cap_limited_attempt_ids,
        })
        .await?;
    let summary = store
        .task_stage_summary(&receipt.task_id, &receipt.stage_id)
        .await?;
    Ok((receipt, Some(summary)))
}

fn consolidate_attempt_artifacts(
    attempts: &[state_store::TaskAttemptRecord],
    owned_paths: &[String],
    state_root: &std::path::Path,
) -> Result<TaskAttemptArtifactConsolidation, String> {
    let mut consolidated = TaskAttemptArtifactConsolidation::default();
    for attempt in attempts {
        let artifacts =
            validate_attempt_artifacts(&attempt.artifact_refs, attempt, owned_paths, state_root)?;
        for (artifact_ref, json) in artifacts {
            if !consolidated.artifact_refs.contains(&artifact_ref) {
                consolidated.artifact_refs.push(artifact_ref.clone());
            }
            taskflow_core::task::attempts::append_json_string_array(
                &json,
                &["observed_facts", "facts"],
                &mut consolidated.facts,
            );
            taskflow_core::task::attempts::append_json_string_array(
                &json,
                &["hypotheses"],
                &mut consolidated.hypotheses,
            );
            taskflow_core::task::attempts::append_json_string_array(
                &json,
                &["conflicts"],
                &mut consolidated.conflicts,
            );
            let result_status = json["result_status"]
                .as_str()
                .or_else(|| json["status"].as_str())
                .unwrap_or("");
            if attempt.status == "partially_accepted"
                || matches!(result_status, "partial" | "partially_accepted")
            {
                taskflow_core::task::attempts::push_unique(
                    &mut consolidated.partial_attempt_ids,
                    &attempt.attempt_id,
                );
            }
            if json["timeout"].as_bool() == Some(true) || result_status == "timeout" {
                taskflow_core::task::attempts::push_unique(
                    &mut consolidated.timeout_attempt_ids,
                    &attempt.attempt_id,
                );
            }
            if json["cap_limited"].as_bool() == Some(true) || result_status == "cap_limited" {
                taskflow_core::task::attempts::push_unique(
                    &mut consolidated.cap_limited_attempt_ids,
                    &attempt.attempt_id,
                );
            }
        }
    }
    Ok(consolidated)
}

async fn validate_attempt_artifacts_for_task(
    store: &StateStore,
    attempt: &state_store::TaskAttemptRecord,
    artifact_refs: &[String],
    state_root: &std::path::Path,
) -> Result<Vec<String>, String> {
    let task = store
        .show_task(&attempt.task_id)
        .await
        .map_err(|error| format!("failed to read task owned_paths: {error}"))?;
    validate_attempt_artifacts(
        artifact_refs,
        attempt,
        &task.planner_metadata.owned_paths,
        state_root,
    )
    .map(|artifacts| {
        artifacts
            .into_iter()
            .map(|(artifact_ref, _)| artifact_ref)
            .collect()
    })
}

fn validate_attempt_artifacts(
    values: &[String],
    attempt: &state_store::TaskAttemptRecord,
    owned_paths: &[String],
    state_root: &std::path::Path,
) -> Result<Vec<(String, serde_json::Value)>, String> {
    let refs = taskflow_core::task::attempts::validate_attempt_artifact_refs(values)?;
    refs.into_iter()
        .map(|artifact_ref| {
            let path = crate::runtime_dispatch_packets::validate_attempt_artifact_ref(
                &artifact_ref,
                state_root,
            )?;
            let state_root = runtime_path_policy::StateRoot::open(state_root).map_err(|error| {
                format!("failed to open attempt artifact root for `{artifact_ref}`: {error}")
            })?;
            let file = runtime_path_policy::existing_regular_file_under_root(
                &state_root,
                &path,
                runtime_path_policy::ArtifactPathKind::TaskAttemptArtifact,
            )
            .map_err(|error| {
                format!("attempt artifact `{artifact_ref}` is not readable: {error}")
            })?;
            let json = runtime_path_policy::bounded_json::read_json_value_file(
                &file,
                runtime_path_policy::bounded_json::TASK_ATTEMPT_ARTIFACT_LIMIT,
            )
            .map_err(|error| {
                format!("attempt artifact `{artifact_ref}` is not valid JSON: {error}")
            })?;
            taskflow_core::task::attempts::validate_stage_attempt_artifact_identity(
                &json,
                &attempt.attempt_id,
                &attempt.task_id,
                &attempt.stage_id,
                &artifact_ref,
            )?;
            taskflow_core::task::attempts::validate_attempt_artifact_changed_files_scope(
                &json,
                &artifact_ref,
                owned_paths,
            )?;
            Ok((artifact_ref, json))
        })
        .collect()
}

async fn task_stage_status_payload_for_command(
    store: &StateStore,
    command: &TaskStageStatusArgs,
) -> Result<serde_json::Value, state_store::StateStoreError> {
    if let Some(stage_id) = command.stage_id.as_deref() {
        let summary = store.task_stage_summary(&command.task_id, stage_id).await?;
        return Ok(task_stage_status_payload(
            "vida task stage status",
            &command.task_id,
            Some(stage_id),
            vec![summary],
        ));
    }
    let attempts = store.task_attempts_for_task(&command.task_id).await?;
    let mut stage_ids = attempts
        .iter()
        .map(|attempt| attempt.stage_id.clone())
        .collect::<Vec<_>>();
    stage_ids.sort();
    stage_ids.dedup();
    let mut summaries = Vec::new();
    for stage_id in stage_ids {
        summaries.push(
            store
                .task_stage_summary(&command.task_id, &stage_id)
                .await?,
        );
    }
    let active_stage = summaries.first().map(|summary| summary.stage_id.clone());
    Ok(task_stage_status_payload(
        "vida task stage status",
        &command.task_id,
        active_stage.as_deref(),
        summaries,
    ))
}

async fn task_attempt_dispatch_records(
    store: &StateStore,
    command: &TaskAttemptDispatchArgs,
) -> Result<(Vec<state_store::TaskAttemptRecord>, serde_json::Value), state_store::StateStoreError>
{
    if command.backend.is_some() || command.model_profile.is_some() || command.isolation.is_some() {
        let attempt = store
            .record_task_attempt(state_store::RecordTaskAttemptRequest {
                attempt_id: command.attempt_id.clone(),
                task_id: command.task_id.clone(),
                stage_id: command.stage_id.clone(),
                backend: command
                    .backend
                    .clone()
                    .unwrap_or_else(|| "manual_override".to_string()),
                model_profile: command
                    .model_profile
                    .clone()
                    .unwrap_or_else(|| "manual_override".to_string()),
                isolation: command
                    .isolation
                    .clone()
                    .unwrap_or_else(|| "readonly".to_string()),
                freshness: None,
                status: "submitted".to_string(),
                artifact_refs: Vec::new(),
                consolidation_receipt_id: None,
                selected_model_profile_readiness_status: Some("manual_override".to_string()),
                budget_posture: Some("manual_override".to_string()),
                cap_posture: Some("manual_override".to_string()),
                write_scope_classification: command.isolation.clone(),
            })
            .await?;
        return Ok((
            vec![attempt],
            serde_json::json!({
                "status": "pass",
                "stage_id": command.stage_id,
                "source": "manual_override",
                "attempt_count": 1,
            }),
        ));
    }

    let snapshot = crate::read_or_sync_launcher_activation_snapshot(store)
        .await
        .map_err(|error| state_store::StateStoreError::InvalidTaskRecord {
            reason: format!("stage_attempt_policy_load_failed: {error}"),
        })?;
    let stage_policy = crate::runtime_assignment_builder::build_stage_attempt_policy_from_config(
        &snapshot.compiled_bundle,
        &command.stage_id,
    );
    if stage_policy["status"].as_str() != Some("pass") {
        return Err(state_store::StateStoreError::InvalidTaskRecord {
            reason: format!(
                "stage_attempt_policy_blocked: {}",
                stage_policy["blocker_codes"]
            ),
        });
    }

    let mut attempts = Vec::new();
    for assignment in stage_policy["attempts"].as_array().into_iter().flatten() {
        if assignment["enabled"].as_bool() != Some(true) {
            continue;
        }
        attempts.push(
            store
                .record_task_attempt(state_store::RecordTaskAttemptRequest {
                    attempt_id: Some(task_attempt_policy_attempt_id(
                        &command.task_id,
                        &command.stage_id,
                        assignment["attempt_id"].as_str().unwrap_or("attempt"),
                    )),
                    task_id: command.task_id.clone(),
                    stage_id: command.stage_id.clone(),
                    backend: json_trimmed_string_field_any(
                        assignment,
                        &[
                            "selected_dispatch_backend_id",
                            "selected_backend_id",
                            "selected_backend",
                            "selected_carrier_id",
                            "requested_carrier_id",
                        ],
                    )
                    .unwrap_or_else(|| "unknown_backend".to_string()),
                    model_profile: json_trimmed_string_field_any(
                        assignment,
                        &[
                            "selected_model_profile_id",
                            "selected_model_profile",
                            "requested_model_profile_id",
                        ],
                    )
                    .unwrap_or_else(|| "unknown_model_profile".to_string()),
                    isolation: json_trimmed_string_field_any(assignment, &["isolation"])
                        .unwrap_or_else(|| "readonly".to_string()),
                    freshness: None,
                    status: "submitted".to_string(),
                    artifact_refs: Vec::new(),
                    consolidation_receipt_id: None,
                    selected_model_profile_readiness_status: json_trimmed_string_field_any(
                        assignment,
                        &["selected_model_profile_readiness_status"],
                    ),
                    budget_posture: json_trimmed_string_field_any(
                        assignment,
                        &["budget_verdict", "budget_policy"],
                    ),
                    cap_posture: stage_policy["fanout"]["cap_posture"]
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| Some("configured".to_string())),
                    write_scope_classification: json_trimmed_string_field_any(
                        assignment,
                        &["selected_write_scope", "write_scope"],
                    ),
                })
                .await?,
        );
    }
    Ok((attempts, stage_policy))
}

fn task_attempt_policy_attempt_id(task_id: &str, stage_id: &str, attempt_id: &str) -> String {
    format!(
        "{}--{}--{}",
        task_attempt_record_component(task_id),
        task_attempt_record_component(stage_id),
        task_attempt_record_component(attempt_id)
    )
}

fn task_attempt_record_component(value: &str) -> String {
    taskflow_host_bridge::artifact_scope::normalized_record_component(value, "attempt")
}

async fn run_task_attempt(command: TaskAttemptArgs) -> ExitCode {
    match command.command {
        TaskAttemptCommand::Dispatch(command) => run_task_attempt_dispatch(command).await,
        TaskAttemptCommand::Status(command) => run_task_attempt_status(command).await,
        TaskAttemptCommand::Collect(command) => run_task_attempt_collect(command).await,
        TaskAttemptCommand::Consolidate(command) => run_task_attempt_consolidate(command).await,
        TaskAttemptCommand::Record(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let request = state_store::RecordTaskAttemptRequest {
                        attempt_id: command.attempt_id,
                        task_id: command.task_id.clone(),
                        stage_id: command.stage_id.clone(),
                        backend: command.backend,
                        model_profile: command.model_profile,
                        isolation: command.isolation,
                        freshness: command.freshness,
                        status: command.status,
                        artifact_refs: command.artifact_refs,
                        consolidation_receipt_id: command.consolidation_receipt_id,
                        selected_model_profile_readiness_status: None,
                        budget_posture: None,
                        cap_posture: None,
                        write_scope_classification: None,
                    };
                    match store.record_task_attempt(request).await {
                        Ok(attempt) => {
                            let summary = store
                                .task_stage_summary(&attempt.task_id, &attempt.stage_id)
                                .await
                                .ok();
                            print_task_attempt_payload(
                                "vida task attempt record",
                                command.render,
                                command.json,
                                task_attempt_success_payload(
                                    "vida task attempt record",
                                    &attempt,
                                    summary,
                                ),
                            )
                        }
                        Err(error) => print_task_attempt_payload(
                            "vida task attempt record",
                            command.render,
                            command.json,
                            task_attempt_error_payload(
                                "vida task attempt record",
                                Some(command.task_id.as_str()),
                                Some(command.stage_id.as_str()),
                                None,
                                &error,
                            ),
                        ),
                    }
                }
                Err(error) => print_task_attempt_payload(
                    "vida task attempt record",
                    command.render,
                    command.json,
                    task_attempt_store_error_payload("vida task attempt record", &error),
                ),
            }
        }
        TaskAttemptCommand::Transition(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let request = state_store::TransitionTaskAttemptRequest {
                        attempt_id: command.attempt_id.clone(),
                        task_id: command.task_id.clone(),
                        stage_id: command.stage_id.clone(),
                        status: command.status,
                        artifact_refs: command.artifact_refs,
                        consolidation_receipt_id: command.consolidation_receipt_id,
                    };
                    match store.transition_task_attempt(request).await {
                        Ok(attempt) => {
                            let summary = store
                                .task_stage_summary(&attempt.task_id, &attempt.stage_id)
                                .await
                                .ok();
                            print_task_attempt_payload(
                                "vida task attempt transition",
                                command.render,
                                command.json,
                                task_attempt_success_payload(
                                    "vida task attempt transition",
                                    &attempt,
                                    summary,
                                ),
                            )
                        }
                        Err(error) => print_task_attempt_payload(
                            "vida task attempt transition",
                            command.render,
                            command.json,
                            task_attempt_error_payload(
                                "vida task attempt transition",
                                Some(command.task_id.as_str()),
                                Some(command.stage_id.as_str()),
                                Some(command.attempt_id.as_str()),
                                &error,
                            ),
                        ),
                    }
                }
                Err(error) => print_task_attempt_payload(
                    "vida task attempt transition",
                    command.render,
                    command.json,
                    task_attempt_store_error_payload("vida task attempt transition", &error),
                ),
            }
        }
        TaskAttemptCommand::Summary(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing_read_only(state_dir).await {
                Ok(store) => match store
                    .task_stage_summary(&command.task_id, &command.stage_id)
                    .await
                {
                    Ok(summary) => print_task_attempt_payload(
                        "vida task attempt summary",
                        command.render,
                        command.json,
                        task_attempt_summary_payload("vida task attempt summary", summary),
                    ),
                    Err(error) => print_task_attempt_payload(
                        "vida task attempt summary",
                        command.render,
                        command.json,
                        task_attempt_error_payload(
                            "vida task attempt summary",
                            Some(command.task_id.as_str()),
                            Some(command.stage_id.as_str()),
                            None,
                            &error,
                        ),
                    ),
                },
                Err(error) => print_task_attempt_payload(
                    "vida task attempt summary",
                    command.render,
                    command.json,
                    task_attempt_store_error_payload("vida task attempt summary", &error),
                ),
            }
        }
    }
}

fn print_task_attempt_payload(
    surface: &str,
    render: RenderMode,
    as_json: bool,
    payload: serde_json::Value,
) -> ExitCode {
    if !crate::surface_render::print_surface_json(
        &payload,
        as_json,
        "task attempt payload should render as json",
    ) {
        if matches!(render, RenderMode::Plain) {
            operator_output::toon_report::print(
                surface,
                vec![
                    operator_output::toon_report::OperatorToonField::value(
                        "status",
                        payload["status"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "attempt",
                        payload["attempt"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "stage_summary",
                        payload["stage_summary"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "attempts",
                        payload["attempts"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "collected_artifacts",
                        payload["collected_artifacts"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "canonical_task_notes_mutated",
                        payload["canonical_task_notes_mutated"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "consolidation_receipt",
                        payload["consolidation_receipt"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "facts",
                        payload["facts"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "hypotheses",
                        payload["hypotheses"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "conflicts",
                        payload["conflicts"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "active_stage",
                        payload["active_stage"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "stages",
                        payload["stages"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "binding_error_kind",
                        payload["binding_error_kind"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "binding_error",
                        payload["binding_error"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "blocker_codes",
                        payload["blocker_codes"].clone(),
                    ),
                    operator_output::toon_report::OperatorToonField::value(
                        "next_actions",
                        payload["next_actions"].clone(),
                    ),
                ],
            );
        } else {
            print_surface_header(render, surface);
            print_surface_line(
                render,
                "status",
                payload["status"].as_str().unwrap_or("blocked"),
            );
        }
    }
    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn task_attempt_success_payload(
    surface: &str,
    attempt: &state_store::TaskAttemptRecord,
    summary: Option<state_store::TaskStageSummary>,
) -> serde_json::Value {
    task_attempt_operator_payload(
        surface,
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "surface": surface,
            "task_id": attempt.task_id,
            "stage_id": attempt.stage_id,
            "attempt_id": attempt.attempt_id,
        }),
        serde_json::json!({
            "attempt": attempt,
            "stage_summary": summary,
        }),
    )
}

fn task_attempt_summary_payload(
    surface: &str,
    summary: state_store::TaskStageSummary,
) -> serde_json::Value {
    task_attempt_operator_payload(
        surface,
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "surface": surface,
            "task_id": summary.task_id,
            "stage_id": summary.stage_id,
        }),
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "stage_summary": summary,
        }),
    )
}

fn task_attempt_dispatch_payload(
    surface: &str,
    task_id: &str,
    stage_id: &str,
    attempts: Vec<state_store::TaskAttemptRecord>,
    summary: Option<state_store::TaskStageSummary>,
    stage_policy: serde_json::Value,
) -> serde_json::Value {
    let artifact_refs = serde_json::json!({
        "surface": surface,
        "task_id": task_id,
        "stage_id": stage_id,
        "attempt_ids": attempts
            .iter()
            .map(|attempt| attempt.attempt_id.clone())
            .collect::<Vec<_>>(),
    });
    task_attempt_operator_payload(
        surface,
        Vec::new(),
        Vec::new(),
        artifact_refs,
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "attempts": attempts,
            "stage_summary": summary,
            "stage_policy": stage_policy,
            "canonical_task_notes_mutated": false,
        }),
    )
}

fn task_attempt_collect_payload(
    surface: &str,
    attempt: &state_store::TaskAttemptRecord,
    summary: Option<state_store::TaskStageSummary>,
    validated_artifacts: Vec<String>,
) -> serde_json::Value {
    let mut payload = task_attempt_success_payload(surface, attempt, summary);
    payload["collected_artifacts"] = serde_json::json!(validated_artifacts);
    payload["artifact_refs"] = serde_json::json!({
        "validated": payload["collected_artifacts"].clone(),
    });
    payload["canonical_task_notes_mutated"] = serde_json::json!(false);
    payload
}

fn task_attempt_consolidation_payload(
    surface: &str,
    receipt: state_store::TaskStageConsolidationReceipt,
    summary: Option<state_store::TaskStageSummary>,
) -> serde_json::Value {
    let mut summary_value = serde_json::to_value(&summary).unwrap_or(serde_json::Value::Null);
    if let Some(summary_object) = summary_value.as_object_mut() {
        let missing_latest = summary_object
            .get("latest_consolidation_receipt_id")
            .is_none_or(serde_json::Value::is_null);
        if missing_latest {
            summary_object.insert(
                "latest_consolidation_receipt_id".to_string(),
                serde_json::json!(receipt.receipt_id.clone()),
            );
        }
    }
    task_attempt_operator_payload(
        surface,
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "surface": surface,
            "task_id": receipt.task_id,
            "stage_id": receipt.stage_id,
            "consolidation_receipt_id": receipt.receipt_id,
            "artifact_refs": receipt.artifact_refs,
        }),
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "stage_summary": summary_value,
            "consolidation_receipt": receipt,
            "facts": receipt.facts,
            "hypotheses": receipt.hypotheses,
            "conflicts": receipt.conflicts,
            "canonical_task_notes_mutated": false,
        }),
    )
}

fn task_stage_status_payload(
    surface: &str,
    task_id: &str,
    active_stage: Option<&str>,
    summaries: Vec<state_store::TaskStageSummary>,
) -> serde_json::Value {
    let stages = summaries
        .iter()
        .map(|summary| (summary.stage_id.clone(), serde_json::json!(summary)))
        .collect::<serde_json::Map<_, _>>();
    task_attempt_operator_payload(
        surface,
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "surface": surface,
            "task_id": task_id,
            "active_stage": active_stage,
            "stage_count": stages.len(),
        }),
        serde_json::json!({
            "task_id": task_id,
            "active_stage": active_stage,
            "stages": stages,
            "attempt": serde_json::Value::Null,
            "stage_summary": summaries.first(),
        }),
    )
}

fn task_attempt_store_error_payload(
    surface: &str,
    error: &state_store::StateStoreError,
) -> serde_json::Value {
    task_attempt_operator_payload(
        surface,
        vec![crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::ProjectActivationUnknown,
        )
        .to_string()],
        vec![
            "Run `vida project-activator` or `vida boot` before recording task attempts."
                .to_string(),
        ],
        serde_json::json!({ "surface": surface }),
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "stage_summary": serde_json::Value::Null,
            "error": error.to_string(),
            "canonical_task_notes_mutated": false,
        }),
    )
}

fn task_attempt_error_payload(
    surface: &str,
    task_id: Option<&str>,
    stage_id: Option<&str>,
    attempt_id: Option<&str>,
    error: &state_store::StateStoreError,
) -> serde_json::Value {
    let error_text = error.to_string();
    let binding_error_kind = task_attempt_binding_error_kind(&error_text);
    let next_action = task_attempt_binding_next_action(binding_error_kind);
    let blocker = match error {
        state_store::StateStoreError::MissingTask { .. } => {
            crate::release1_contracts::BlockerCode::NextActionTargetMissing
        }
        state_store::StateStoreError::InvalidTaskRecord { .. } => {
            crate::release1_contracts::BlockerCode::DispatchPacketContractInvalid
        }
        _ => crate::release1_contracts::BlockerCode::Unsupported,
    };
    let mut artifact_refs = serde_json::json!({
        "surface": surface,
        "task_id": task_id,
        "stage_id": stage_id,
    });
    if let Some(attempt_id) = attempt_id {
        artifact_refs["attempt_id"] = serde_json::json!(attempt_id);
    }
    task_attempt_operator_payload(
        surface,
        vec![crate::release1_contracts::blocker_code_str(blocker).to_string()],
        vec![next_action.to_string()],
        artifact_refs,
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "stage_summary": serde_json::Value::Null,
            "error": error_text,
            "binding_error": error_text,
            "binding_error_kind": binding_error_kind,
            "canonical_task_notes_mutated": false,
        }),
    )
}

fn task_attempt_error_is_artifact_validation(error: &state_store::StateStoreError) -> bool {
    error
        .to_string()
        .contains("attempt_artifact_validation_failed")
}

fn task_attempt_artifact_error_payload(
    surface: &str,
    task_id: Option<&str>,
    stage_id: Option<&str>,
    attempt_id: Option<&str>,
    error: &state_store::StateStoreError,
) -> serde_json::Value {
    let error_text = error.to_string();
    let max_bytes = runtime_path_policy::size_limits::TASK_ATTEMPT_ARTIFACT_MAX_BYTES;
    let artifact_contract =
        taskflow_core::task::attempts::stage_attempt_artifact_contract(max_bytes);
    let next_action =
        taskflow_core::task::attempts::stage_attempt_artifact_contract_hint(max_bytes);
    let mut artifact_refs = serde_json::json!({
        "surface": surface,
        "task_id": task_id,
        "stage_id": stage_id,
    });
    if let Some(attempt_id) = attempt_id {
        artifact_refs["attempt_id"] = serde_json::json!(attempt_id);
    }
    task_attempt_operator_payload(
        surface,
        vec![crate::release1_contracts::blocker_code_str(
            crate::release1_contracts::BlockerCode::DispatchPacketContractInvalid,
        )
        .to_string()],
        vec![next_action],
        artifact_refs,
        serde_json::json!({
            "attempt": serde_json::Value::Null,
            "stage_summary": serde_json::Value::Null,
            "error": error_text,
            "artifact_contract": artifact_contract,
            "canonical_task_notes_mutated": false,
        }),
    )
}

fn task_attempt_binding_error_kind(error: &str) -> &'static str {
    if error.contains("is closed") {
        "closed_task_binding"
    } else if error.contains("stale_task_binding") || error.contains("freshness") {
        "stale_task_binding"
    } else if error.contains("requires a leaf task") {
        "container_task_binding"
    } else {
        "invalid_task_attempt_binding"
    }
}

fn task_attempt_binding_next_action(kind: &str) -> &'static str {
    match kind {
        "closed_task_binding" => {
            "The task is already closed; inspect `vida task progress <task-id>` for historical attempt state or retry with an open leaf task."
        }
        "stale_task_binding" => {
            "The attempt is stale for the current task update timestamp; inspect `vida task progress <task-id>` and dispatch a fresh attempt before transitioning."
        }
        "container_task_binding" => {
            "Select an open leaf task with `vida task ready --scope <container-task-id> --limit 10`, then run the attempt command on that leaf task."
        }
        _ => {
            "Inspect the task binding with `vida task show <task-id>` and retry with a live leaf task, matching stage id, and canonical attempt status."
        }
    }
}

fn task_attempt_operator_payload(
    surface: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    crate::release1_operator_output::build_release1_operator_output_payload(
        surface,
        blocker_codes,
        next_actions,
        artifact_refs,
        extra_fields,
    )
    .expect("task attempt payload should satisfy release-1 operator contract")
}

fn task_reset_candidate_ids(
    tasks: &[state_store::TaskRecord],
    root_task_id: &str,
) -> BTreeSet<String> {
    let mut selected = BTreeSet::new();
    let mut queue = VecDeque::from([root_task_id.to_string()]);
    while let Some(task_id) = queue.pop_front() {
        if !selected.insert(task_id.clone()) {
            continue;
        }
        for child in tasks.iter().filter(|task| {
            StateStore::parent_id_for_task(task).as_deref() == Some(task_id.as_str())
        }) {
            queue.push_back(child.id.clone());
        }
    }
    selected
}

fn task_reset_row(task: &state_store::TaskRecord, next_status: &str) -> TaskResetTaskRow {
    TaskResetTaskRow {
        task_id: task.id.clone(),
        title: task.title.clone(),
        issue_type: task.issue_type.clone(),
        previous_status: task.status.clone(),
        next_status: next_status.to_string(),
    }
}

fn task_reset_includes_issue_type(issue_type: &str, include_steps: bool) -> bool {
    if taskflow_core::issue_type_is_execution_step(issue_type) {
        return include_steps;
    }
    !crate::state_store::work_item_is_program_container(issue_type)
}

fn print_task_reset_receipt(receipt: &TaskResetReceipt, render: RenderMode, as_json: bool) {
    if as_json {
        let value = serde_json::to_value(receipt).expect("task reset receipt should serialize");
        crate::print_json_pretty(&value);
        return;
    }
    let _ = render;
    println!(
        "vida task reset: status={} task_id={} dry_run={} reset_count={} skipped_count={}",
        receipt.status,
        receipt.task_id,
        receipt.dry_run,
        receipt.reset_count,
        receipt.skipped_count
    );
    for task in &receipt.reset_tasks {
        println!(
            "- {} [{}] {} -> {}",
            task.task_id, task.issue_type, task.previous_status, task.next_status
        );
    }
    if !receipt.skipped_tasks.is_empty() {
        println!("skipped:");
        for task in &receipt.skipped_tasks {
            println!(
                "- {} [{}] {}",
                task.task_id, task.issue_type, task.previous_status
            );
        }
    }
}

fn task_subcommand_invoked_as_classify_dirty(args: impl IntoIterator<Item = String>) -> bool {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "task" {
            return args.next().as_deref() == Some("classify-dirty");
        }
    }
    false
}

async fn run_task_reset(command: crate::TaskResetArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let store = match StateStore::open_existing(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            return emit_task_state_store_open_error(
                "vida task reset",
                &state_dir,
                command.render,
                command.json,
                &error,
            );
        }
    };
    let tasks = match store.all_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to list tasks for reset: {error}");
            return ExitCode::from(1);
        }
    };
    if !tasks.iter().any(|task| task.id == command.task_id) {
        eprintln!("Task `{}` was not found.", command.task_id);
        return ExitCode::from(1);
    }
    let candidate_ids = task_reset_candidate_ids(&tasks, &command.task_id);
    let mut reset_tasks = Vec::new();
    let mut skipped_tasks = Vec::new();
    let mut candidates = tasks
        .iter()
        .filter(|task| candidate_ids.contains(&task.id))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));

    for task in &candidates {
        if !task_reset_includes_issue_type(&task.issue_type, command.include_steps) {
            skipped_tasks.push(task_reset_row(task, &task.status));
            continue;
        }
        if task.status == "open" && task.closed_at.is_none() && task.close_reason.is_none() {
            skipped_tasks.push(task_reset_row(task, "open"));
            continue;
        }
        reset_tasks.push(task_reset_row(task, "open"));
    }

    if !command.dry_run {
        for task in &reset_tasks {
            if let Err(error) = store
                .update_task(state_store::UpdateTaskRequest {
                    task_id: &task.task_id,
                    title: None,
                    status: Some("open"),
                    priority: None,
                    notes: None,
                    description: None,
                    parent_id: None,
                    add_labels: &[],
                    remove_labels: &[],
                    set_labels: None,
                    execution_mode: None,
                    order_bucket: None,
                    parallel_group: None,
                    conflict_domain: None,
                    planner_metadata: None,
                })
                .await
            {
                eprintln!("Failed to reset task `{}`: {error}", task.task_id);
                return ExitCode::from(1);
            }
        }
        if let Err(code) = refresh_task_snapshot_after_mutation(&store, "vida task reset").await {
            return code;
        }
    }

    let receipt = TaskResetReceipt {
        surface: "vida task reset",
        status: if command.dry_run {
            "dry_run".to_string()
        } else {
            task_json_success_status().to_string()
        },
        task_id: command.task_id,
        dry_run: command.dry_run,
        include_steps: command.include_steps,
        inspected_count: candidates.len(),
        reset_count: reset_tasks.len(),
        skipped_count: skipped_tasks.len(),
        reset_tasks,
        skipped_tasks,
    };
    print_task_reset_receipt(&receipt, command.render, command.json);
    ExitCode::SUCCESS
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
                "ready"
                | "deps"
                | "reverse-deps"
                | "blocked"
                | "children"
                | "reparent-children"
                | "move-children"
                | "defect-batch-rehome"
                | "defect-batch"
                | "tree"
                | "subtree"
                | "critical-path"
                | "next-display-id"
                | "create"
                | "ensure"
                | "update"
                | "reset"
                | "close"
                | "pack-finalize"
                | "prune-closed-epics"
                | "split"
                | "spawn-blocker"
                | "list"
                | "adaptive-preview"
                | "show"
                | "validator-packet"
                | "classify-dirty"
                | "import"
                | "create-bulk"
                | "bulk-create"
                | "import-jsonl"
                | "replace-jsonl"
                | "export-jsonl"
                | "validate-graph"
                | "dep"
                | "handoff"
                | "attempt"
                | "stage"
                | "next-lawful",
            ) => {
                print_taskflow_proxy_help(Some("task"));
                ExitCode::SUCCESS
            }
            Some(topic) => {
                eprintln!("Unsupported task help topic: {topic}");
                ExitCode::from(2)
            }
        },
        TaskCommand::Steps(command) => run_task_steps(command).await,
        TaskCommand::Import(command) => run_task_bulk_import(command).await,
        TaskCommand::ImportJsonl(command) => run_task_import_jsonl(command).await,
        TaskCommand::ReplaceJsonl(command) => run_task_replace_jsonl(command).await,
        TaskCommand::ExportJsonl(command) => run_task_export_jsonl(command).await,
        TaskCommand::List(command) => run_task_list(command).await,
        TaskCommand::Search(command) => run_task_search(command).await,
        TaskCommand::ValidatorPacket(command) => run_task_validator_packet(command).await,
        TaskCommand::Show(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let view = match command.view.trim() {
                "compact" => "compact",
                "summary" | "" => "summary",
                "full" => "full",
                other => {
                    eprintln!(
                        "Invalid task show view `{other}`. Supported views: compact, summary, full. Try `vida task show {} --view full`.",
                        command.task_id
                    );
                    return ExitCode::from(2);
                }
            };
            let cache_allowed = command.json && view == "summary";
            if cache_allowed {
                let projection_name = task_show_projection_name(&command.task_id);
                if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
                    &state_dir,
                    &projection_name,
                ) {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
                if let Some(cached) =
                    crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                        &state_dir,
                        &projection_name,
                        TASK_READ_RECENT_PROJECTION_MAX_AGE,
                    )
                {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
            }
            match task_show_authoritative_first(state_dir.clone(), &command.task_id).await {
                Ok((task, metadata)) => {
                    if command.json {
                        let payload = task_show_payload(&task, Some(&metadata), view);
                        crate::print_json_pretty(&payload);
                        if cache_allowed {
                            crate::operator_projection_cache::write_json_projection(
                                &state_dir,
                                &task_show_projection_name(&command.task_id),
                                &payload,
                            );
                        }
                    } else {
                        print_task_show(command.render, &task, false, Some(&metadata), view);
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    if let state_store::StateStoreError::MissingTask { task_id } = &error {
                        print_task_show_missing(command.render, task_id, command.json);
                        return ExitCode::from(1);
                    }
                    eprintln!("Failed to show task: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::OwnedStatus(command) => {
            let invoked_as_classify_dirty =
                task_subcommand_invoked_as_classify_dirty(std::env::args());
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            if invoked_as_classify_dirty {
                let repo_root = dirty_repo_root_for_current_process();
                return match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                    Ok((rows, _metadata)) => match dirty_paths_for_repo(&repo_root) {
                        Ok(dirty_files) => {
                            let receipt = task_dirty_classify_receipt(
                                &rows,
                                dirty_files,
                                repo_root.display().to_string(),
                            );
                            print_task_dirty_classify_receipt(
                                command.render,
                                &receipt,
                                command.json,
                            );
                            if receipt.status == "pass" {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to read dirty git status: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => emit_task_state_store_open_error(
                        "vida task classify-dirty",
                        &state_dir,
                        command.render,
                        command.json,
                        &error,
                    ),
                };
            }
            let repo_root = if command.from_dirty {
                dirty_repo_root_for_current_process()
            } else {
                project_root_for_task_state(&state_dir)
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
            };
            match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                Ok((rows, _metadata)) => {
                    let task = match select_task_for_owned_status(
                        &rows,
                        command.task_id.as_deref(),
                        command.with_active_step,
                    ) {
                        Ok(task) => task,
                        Err(error) => {
                            let receipt = serde_json::json!({
                                "status": "blocked",
                                "blocker_codes": ["missing_task_context"],
                                "next_actions": ["Pass `<task-id>` or rerun with `--with-active-step` while a TaskFlow step is in_progress."],
                                "task_id": command.task_id.clone().unwrap_or_else(|| "unresolved".to_string()),
                                "error": error.to_string(),
                            });
                            if command.json {
                                crate::print_json_pretty(&receipt);
                            } else {
                                eprintln!("Failed to inspect task owned status: {error}");
                            }
                            return ExitCode::from(1);
                        }
                    };
                    let (active_step, active_parent_task, active_epic) =
                        task_owned_status_context(&rows, &task, command.with_active_step);
                    let dirty_files = match dirty_paths_for_repo(&repo_root) {
                        Ok(paths) => paths,
                        Err(error) => {
                            let receipt = TaskOwnedStatusReceipt {
                                status: "blocked".to_string(),
                                blocker_codes: vec!["git_status_failed".to_string()],
                                next_actions: vec![
                                    "Run the command from a git worktree or resolve git status errors before staging.".to_string(),
                                ],
                                task_id: task.id.clone(),
                                repo_root: repo_root.display().to_string(),
                                active_step,
                                active_parent_task,
                                active_epic,
                                ownership_source: "unresolved".to_string(),
                                owned_paths: Vec::new(),
                                dirty_files: Vec::new(),
                                owned_files: Vec::new(),
                                unowned_files: Vec::new(),
                                unowned_paths: Vec::new(),
                                matched_files: Vec::new(),
                                unmatched_files: Vec::new(),
                                stageable_files: Vec::new(),
                                confidence: "none".to_string(),
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
                        repo_root.display().to_string(),
                        active_step,
                        active_parent_task,
                        active_epic,
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
                match task_show_authoritative_first(state_dir, &command.task_id).await {
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
        TaskCommand::Takeover(command) => match command.command {
            TaskTakeoverCommand::Status(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let requested_task_id = match (&command.task_id, &command.task_id_filter) {
                    (Some(positional), Some(flag)) if positional.trim() != flag.trim() => {
                        let receipt = finalize_task_takeover_status_receipt(
                            TaskTakeoverStatusReceipt {
                            surface: "vida task takeover status",
                            status: "blocked".to_string(),
                            trace_id: None,
                            workflow_class: None,
                            risk_tier: None,
                            artifact_refs: serde_json::Value::Null,
                            shared_fields: serde_json::Value::Null,
                            operator_contracts: serde_json::Value::Null,
                            task_id: positional.clone(),
                            allowed: false,
                            local_exception_takeover_state: "not_recorded".to_string(),
                            root_local_write_allowed: false,
                            paths: Vec::new(),
                            packet: serde_json::json!({}),
                            lane: serde_json::json!({}),
                            root_write_guard: serde_json::json!({
                                "status": "blocked_by_default",
                                "root_local_write_allowed": false,
                                "root_local_write_allowed_for_only_these_paths": [],
                                "local_exception_takeover_state": "not_recorded",
                                "reason": "conflicting task id filters",
                            }),
                            active_takeover_state: "not_recorded".to_string(),
                            takeover_ready_state: "not_ready".to_string(),
                            recommended_surface: None,
                            reason: "positional task id and --task-id disagree".to_string(),
                            recommended_command: None,
                            next_actions: vec![
                                "Rerun with one task id source or matching positional and --task-id values."
                                    .to_string(),
                            ],
                            blocker_codes: vec!["task_filter_conflict".to_string()],
                        });
                        if command.json {
                            crate::print_json_pretty(
                                &serde_json::to_value(&receipt)
                                    .expect("takeover status receipt should serialize"),
                            );
                        } else {
                            print_task_takeover_status(command.render, &receipt);
                        }
                        return ExitCode::from(1);
                    }
                    (Some(positional), _) => Some(positional.trim().to_string()),
                    (_, Some(flag)) => Some(flag.trim().to_string()),
                    (None, None) => None,
                };
                match StateStore::open_existing_read_only(state_dir).await {
                    Ok(store) => {
                        let task_id_was_requested = requested_task_id
                            .as_ref()
                            .is_some_and(|value| !value.trim().is_empty());
                        let (status_override, lane_source) =
                            if let Some(run_id) = command.run_id.as_deref() {
                                match store.run_graph_status(run_id).await {
                                    Ok(status) => (Some(status), Some("run_id")),
                                    Err(error) => {
                                        eprintln!("Failed to inspect run graph status: {error}");
                                        return ExitCode::from(1);
                                    }
                                }
                            } else if let Some(task_id) = requested_task_id
                                .as_deref()
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                            {
                                match store.latest_run_graph_status_for_task(task_id).await {
                                    Ok(status) => (status, Some("task_id")),
                                    Err(error) => {
                                        eprintln!(
                                        "Failed to inspect task-scoped run graph status: {error}"
                                    );
                                        return ExitCode::from(1);
                                    }
                                }
                            } else {
                                let current = store
                                    .latest_run_graph_status_for_current_session()
                                    .await
                                    .ok()
                                    .flatten();
                                match current {
                                    Some(status) => (Some(status), Some("current_session")),
                                    None => (
                                        store.latest_run_graph_status().await.ok().flatten(),
                                        Some("latest"),
                                    ),
                                }
                            };
                        let Some(task_id) = requested_task_id
                            .filter(|value| !value.trim().is_empty())
                            .or_else(|| {
                                status_override
                                    .as_ref()
                                    .map(|status| status.task_id.clone())
                            })
                        else {
                            let receipt = finalize_task_takeover_status_receipt(
                                TaskTakeoverStatusReceipt {
                                surface: "vida task takeover status",
                                status: "blocked".to_string(),
                                trace_id: None,
                                workflow_class: None,
                                risk_tier: None,
                                artifact_refs: serde_json::Value::Null,
                                shared_fields: serde_json::Value::Null,
                                operator_contracts: serde_json::Value::Null,
                                task_id: String::new(),
                                allowed: false,
                                local_exception_takeover_state: "not_recorded".to_string(),
                                root_local_write_allowed: false,
                                paths: Vec::new(),
                                packet: serde_json::json!({}),
                                lane: serde_json::json!({}),
                                root_write_guard: serde_json::json!({
                                    "status": "blocked_by_default",
                                    "root_local_write_allowed": false,
                                    "root_local_write_allowed_for_only_these_paths": [],
                                    "local_exception_takeover_state": "not_recorded",
                                    "reason": "missing task and lane evidence",
                                }),
                                active_takeover_state: "not_recorded".to_string(),
                                takeover_ready_state: "not_ready".to_string(),
                                recommended_surface: Some("vida lane show".to_string()),
                                reason: "no task id was supplied and no latest lane task id is available"
                                    .to_string(),
                                recommended_command: Some(
                                    operator_output::command_text::human_command(
                                        "vida lane show --latest --json",
                                    ),
                                ),
                                next_actions: vec![
                                    format!(
                                        "Supply --task-id or inspect lane evidence with `{}`.",
                                        operator_output::command_text::human_command(
                                            "vida lane show --latest --json"
                                        )
                                    ),
                                ],
                                blocker_codes: vec!["missing_task_and_lane_evidence".to_string()],
                            });
                            if command.json {
                                crate::print_json_pretty(
                                    &serde_json::to_value(&receipt)
                                        .expect("takeover status receipt should serialize"),
                                );
                            } else {
                                print_task_takeover_status(command.render, &receipt);
                            }
                            return ExitCode::from(1);
                        };
                        match store.show_task(&task_id).await {
                            Ok(task) => {
                                let receipt = task_takeover_status_receipt(
                                    &store,
                                    &task,
                                    status_override,
                                    lane_source,
                                    !task_id_was_requested,
                                )
                                .await;
                                if command.json {
                                    crate::print_json_pretty(
                                        &serde_json::to_value(&receipt)
                                            .expect("takeover status receipt should serialize"),
                                    );
                                } else {
                                    print_task_takeover_status(command.render, &receipt);
                                }
                                if receipt.allowed {
                                    ExitCode::SUCCESS
                                } else {
                                    ExitCode::from(1)
                                }
                            }
                            Err(error) => {
                                eprintln!("Failed to inspect task takeover status: {error}");
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
        },
        TaskCommand::Progress(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            if command.epics {
                let basis = match task_progress_basis_arg(&command.basis) {
                    Ok(basis) => basis,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                    Ok((rows, metadata)) => {
                        match task_epic_progress_summary(
                            &rows,
                            metadata,
                            command.all,
                            command.epic.as_deref(),
                            basis,
                        ) {
                            Ok(summary) => {
                                print_task_epic_progress_summary(
                                    command.render,
                                    &summary,
                                    command.json,
                                    command.counts_only,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to compute epic progress summary: {error}");
                                ExitCode::from(1)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to read task progress rows: {error}");
                        ExitCode::from(1)
                    }
                }
            } else {
                let Some(task_id) = command.task_id.as_deref() else {
                    crate::task_cli_render::print_task_progress_missing_selector(
                        command.render,
                        command.json,
                    );
                    return ExitCode::from(2);
                };
                let basis = match task_progress_basis_arg(&command.basis) {
                    Ok(basis) => basis,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                if basis == "direct_children" {
                    match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                        Ok((rows, _metadata)) => {
                            match task_progress_summary_for_basis(&rows, task_id, basis) {
                                Ok(summary) => {
                                    let stage_ensemble =
                                        task_stage_ensemble_operator_summary_from_state_dir(
                                            &state_dir, task_id,
                                        )
                                        .await;
                                    crate::task_cli_render::print_task_progress_with_stage_ensemble(
                                        command.render,
                                        &summary,
                                        stage_ensemble,
                                        command.json,
                                        command.counts_only,
                                    );
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!("Failed to compute task progress: {error}");
                                    ExitCode::from(1)
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to read task progress rows: {error}");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    match StateStore::open_existing_read_only(state_dir.clone()).await {
                        Ok(store) => match store.task_progress_summary(task_id).await {
                            Ok(summary) => {
                                let stage_ensemble =
                                    task_stage_ensemble_operator_summary(&store, task_id)
                                        .await
                                        .ok();
                                crate::task_cli_render::print_task_progress_with_stage_ensemble(
                                    command.render,
                                    &summary,
                                    stage_ensemble,
                                    command.json,
                                    command.counts_only,
                                );
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
                            match StateStore::task_progress_summary_from_rows(&rows, task_id) {
                                Ok(summary) => {
                                    let stage_ensemble =
                                        task_stage_ensemble_operator_summary_from_state_dir(
                                            &state_dir, task_id,
                                        )
                                        .await;
                                    crate::task_cli_render::print_task_progress_with_stage_ensemble(
                                        command.render,
                                        &summary,
                                        stage_ensemble,
                                        command.json,
                                        command.counts_only,
                                    );
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to compute task progress from snapshot: {error}"
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
            }
        }
        TaskCommand::ClosureReady(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let basis = match task_progress_basis_arg(&command.basis) {
                Ok(basis) => basis,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match load_task_snapshot_rows_authoritative_first(&state_dir).await {
                Ok((rows, metadata)) => {
                    match task_progress_summary_for_basis(&rows, &command.task_id, basis) {
                        Ok(summary) => {
                            let payload =
                                crate::task_cli_render::build_pass_operator_surface_payload(
                                    "vida task closure-ready",
                                    serde_json::json!({
                                       "task_id": command.task_id,
                                       "state_access": task_read_metadata_value(Some(&metadata)),
                                       "basis": basis,
                                       "ready_for_close": summary.ready_for_close,
                                       "closure_candidate": summary.closure_candidate,
                                       "closure_candidate_state": summary.closure_candidate_state,
                                        "closure_candidate_reason": summary.closure_candidate_reason,
                                        "next_required_command": summary.next_required_command,
                                        "recommended_next_action": summary.recommended_next_action,
                                        "progress": crate::task_cli_render::task_progress_value(&summary),
                                    }),
                                );
                            if command.json {
                                crate::print_json_pretty(&payload);
                            } else if matches!(command.render, crate::RenderMode::Plain) {
                                println!(
                                    "{}",
                                    crate::task_cli_render::task_progress_toon_text(
                                        "vida task closure-ready",
                                        &summary,
                                    )
                                );
                            } else {
                                print_surface_header(command.render, "vida task closure-ready");
                                print_surface_line(command.render, "task", &command.task_id);
                                print_surface_line(
                                    command.render,
                                    "ready",
                                    if payload["ready_for_close"].as_bool().unwrap_or(false) {
                                        "true"
                                    } else {
                                        "false"
                                    },
                                );
                                print_surface_line(
                                    command.render,
                                    "state",
                                    payload["closure_candidate_state"]
                                        .as_str()
                                        .unwrap_or("unknown"),
                                );
                                if let Some(command_text) =
                                    payload["next_required_command"].as_str()
                                {
                                    print_surface_line(
                                        command.render,
                                        "next command",
                                        command_text,
                                    );
                                }
                            }
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to compute closure readiness: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to read task progress rows: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Closeout(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let basis = match task_progress_basis_arg(&command.basis) {
                Ok(basis) => basis,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => {
                    let task = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to inspect closeout task: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read task closeout rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let graph_issues = match store.validate_task_graph().await {
                        Ok(issues) => issues,
                        Err(error) => {
                            eprintln!("Failed to validate task graph for closeout: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    match task_closeout_summary(
                        &command.task_id,
                        basis,
                        &task,
                        Some(&TaskReadMetadata::authoritative_live()),
                        &rows,
                        &graph_issues,
                        command.include_temp_scan,
                        &state_dir,
                    ) {
                        Ok(summary) => {
                            print_task_closeout(command.render, &summary, command.json);
                            if summary.status == "pass" {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("{error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) if is_authoritative_state_lock_error(&error) => {
                    let rows = match load_task_snapshot_rows_with_retry(&state_dir).await {
                        Ok(rows) => rows,
                        Err(snapshot_error) => {
                            eprintln!("Failed to read task closeout snapshot: {snapshot_error}");
                            return ExitCode::from(1);
                        }
                    };
                    let Some(task) = rows.iter().find(|row| row.id == command.task_id).cloned()
                    else {
                        eprintln!("Failed to inspect closeout task: task not found");
                        return ExitCode::from(1);
                    };
                    let graph_issues = StateStore::validate_task_graph_rows(&rows);
                    match task_closeout_summary(
                        &command.task_id,
                        basis,
                        &task,
                        Some(&TaskReadMetadata::snapshot(
                            &state_dir,
                            "served from task snapshot after authoritative lock contention",
                        )),
                        &rows,
                        &graph_issues,
                        command.include_temp_scan,
                        &state_dir,
                    ) {
                        Ok(summary) => {
                            print_task_closeout(command.render, &summary, command.json);
                            if summary.status == "pass" {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("{error}");
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
        TaskCommand::Proof(command) => match command.command {
            TaskProofCommand::Status(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match task_show_authoritative_first(state_dir.clone(), &command.task_id).await {
                    Ok((task, metadata)) => {
                        let inheritance_rows =
                            load_task_snapshot_rows_with_retry(&state_dir).await.ok();
                        let payload = task_proof_status_payload_with_inheritance(
                            &task,
                            Some(&metadata),
                            inheritance_rows.as_deref(),
                        );
                        if command.json {
                            crate::print_json_pretty(&payload);
                        } else {
                            print_task_proof_status(command.render, &task, &payload);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to inspect task proof status: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskProofCommand::AttachBrowser(command) => {
                let route = command.route.trim();
                if route.is_empty() {
                    eprintln!("--route cannot be empty");
                    return ExitCode::from(2);
                }
                let result = match normalize_browser_proof_result(&command.result) {
                    Ok(result) => result,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => {
                        let existing = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read task before browser proof attachment: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let evidence = normalized_task_verify_evidence(&command.evidence);
                        let browser_artifact = match TaskBrowserProofArtifact::new(
                            route,
                            &result,
                            command.expect.as_deref(),
                            command.screenshot.as_deref(),
                            &evidence,
                        ) {
                            Some(artifact) => artifact,
                            None => {
                                eprintln!("Failed to build browser proof artifact");
                                return ExitCode::from(2);
                            }
                        };
                        let proof_target = browser_artifact.proof_target.clone();
                        let browser_notes = append_task_browser_proof_note(
                            existing.notes.as_deref(),
                            &browser_artifact,
                        );
                        let notes = append_task_proof_evidence_note(
                            Some(&browser_notes),
                            &proof_target,
                            Some(&proof_target),
                            &result,
                            "browser",
                            command.screenshot.as_deref(),
                            &evidence,
                        );
                        let planner_metadata = task_browser_proof_planner_metadata(
                            &existing.planner_metadata,
                            &proof_target,
                        );
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(&notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: Some(planner_metadata),
                            })
                            .await
                        {
                            Ok(task) => {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task proof attach-browser",
                                )
                                .await
                                {
                                    return code;
                                }
                                let receipt = TaskProofAttachBrowserReceipt {
                                    surface: "vida task proof attach-browser",
                                    status: task_json_success_status(),
                                    task_id: task.id.clone(),
                                    route: route.to_string(),
                                    result,
                                    expect: command.expect,
                                    screenshot: command.screenshot,
                                    evidence,
                                    proof_target,
                                    artifact: browser_artifact,
                                    notes_appended: true,
                                    task,
                                };
                                print_task_browser_proof_receipt(
                                    command.render,
                                    &receipt,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to attach browser proof to task: {error}");
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
            TaskProofCommand::AttachEvidence(command) => {
                let proof_targets = parse_literal_proof_target_values(&command.proof_target);
                if proof_targets.is_empty() {
                    if command.json {
                        crate::print_json_pretty(&serde_json::json!({
                            "surface": "vida task proof attach-evidence",
                            "status": "blocked",
                            "blocker_codes": ["task_proof_target_required"],
                            "next_actions": [
                                format!(
                                    "vida task proof attach-evidence {} --proof-target \"<proof target>\" --result {}",
                                    crate::shell_quote(&command.task_id),
                                    crate::shell_quote(&command.result)
                                ),
                                "Use `vida task proof status <task-id>` to inspect configured proof targets."
                            ],
                            "artifact_refs": {"surface": "vida task proof attach-evidence"}
                        }));
                    } else {
                        print_surface_header(command.render, "vida task proof attach-evidence");
                        print_surface_line(command.render, "status", "blocked");
                        print_surface_line(command.render, "blocker", "task_proof_target_required");
                        print_surface_line(
                            command.render,
                            "next",
                            &format!(
                                "vida task proof attach-evidence {} --proof-target \"<proof target>\" --result {}",
                                crate::shell_quote(&command.task_id),
                                crate::shell_quote(&command.result)
                            ),
                        );
                    }
                    return ExitCode::from(2);
                }
                let result = match normalize_browser_proof_result(&command.result) {
                    Ok(result) => result,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                let evidence = normalized_task_verify_evidence(&command.evidence);
                let artifact_refs = command
                    .artifact_ref
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let artifact_ref = artifact_refs.first().cloned();
                let artifact_ref_note = if artifact_refs.len() > 1 {
                    Some(artifact_refs.join(" | "))
                } else {
                    artifact_ref.clone()
                };
                let command_text = command
                    .command
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| proof_targets.first().map(String::as_str).unwrap_or(""))
                    .to_string();
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => {
                        let existing = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read task before proof evidence attachment: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        if existing.status == "closed" {
                            if command.json {
                                crate::print_json_pretty(&serde_json::json!({
                                    "surface": "vida task proof attach-evidence",
                                    "status": "blocked",
                                    "task_id": command.task_id,
                                    "blocker_codes": ["task_proof_evidence_closed_task"],
                                    "next_actions": [
                                        "Reopen the task before attaching new proof evidence, or use a dedicated repair command when one is available.",
                                        "Inspect existing proof with `vida task proof status <task-id>`."
                                    ],
                                    "artifact_refs": {"surface": "vida task proof attach-evidence"}
                                }));
                            } else {
                                print_surface_header(
                                    command.render,
                                    "vida task proof attach-evidence",
                                );
                                print_surface_line(command.render, "status", "blocked");
                                print_surface_line(
                                    command.render,
                                    "blocker",
                                    "task_proof_evidence_closed_task",
                                );
                                print_surface_line(
                                    command.render,
                                    "next",
                                    "Reopen the task before attaching new proof evidence, or use a dedicated repair command when one is available.",
                                );
                            }
                            return ExitCode::from(2);
                        }
                        let mut notes = existing.notes.clone().unwrap_or_default();
                        let mut planner_metadata = existing.planner_metadata.clone();
                        for proof_target in &proof_targets {
                            notes = append_task_proof_evidence_note(
                                if notes.trim().is_empty() {
                                    None
                                } else {
                                    Some(notes.as_str())
                                },
                                proof_target,
                                Some(if command.command.is_some() {
                                    command_text.as_str()
                                } else {
                                    proof_target.as_str()
                                }),
                                &result,
                                "command",
                                artifact_ref_note.as_deref(),
                                &evidence,
                            );
                            planner_metadata = task_evidence_proof_planner_metadata(
                                &planner_metadata,
                                proof_target,
                            );
                        }
                        match store
                            .update_task(state_store::UpdateTaskRequest {
                                task_id: &command.task_id,
                                title: None,
                                status: None,
                                priority: None,
                                notes: Some(&notes),
                                description: None,
                                parent_id: None,
                                add_labels: &[],
                                remove_labels: &[],
                                set_labels: None,
                                execution_mode: None,
                                order_bucket: None,
                                parallel_group: None,
                                conflict_domain: None,
                                planner_metadata: Some(planner_metadata),
                            })
                            .await
                        {
                            Ok(task) => {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task proof attach-evidence",
                                )
                                .await
                                {
                                    return code;
                                }
                                let receipt = TaskProofAttachEvidenceReceipt {
                                    surface: "vida task proof attach-evidence",
                                    status: task_json_success_status(),
                                    task_id: task.id.clone(),
                                    proof_target: proof_targets.join(" | "),
                                    proof_targets: proof_targets.clone(),
                                    command: command_text,
                                    result,
                                    artifact_ref,
                                    artifact_refs,
                                    evidence,
                                    notes_appended: true,
                                    task,
                                };
                                print_task_evidence_proof_receipt(
                                    command.render,
                                    &receipt,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to attach proof evidence to task: {error}");
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
            TaskProofCommand::AttachReleaseBundle(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let proof_targets = match StateStore::open_existing(state_dir.clone()).await {
                    Ok(store) => {
                        let task = match store.show_task(&command.task_id).await {
                            Ok(task) => task,
                            Err(error) => {
                                eprintln!("Failed to read task before release proof bundle attachment: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        task.planner_metadata
                            .proof_targets
                            .iter()
                            .map(|target| target.trim())
                            .filter(|target| !target.is_empty())
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                    }
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        return ExitCode::from(1);
                    }
                };
                if proof_targets.is_empty() {
                    if command.json {
                        crate::print_json_pretty(&serde_json::json!({
                            "surface": "vida task proof attach-release-bundle",
                            "status": "blocked",
                            "task_id": command.task_id,
                            "blocker_codes": ["task_proof_targets_missing"],
                            "next_actions": ["Configure proof targets on the task, then rerun attach-release-bundle."],
                            "artifact_refs": {"surface": "vida task proof attach-release-bundle"}
                        }));
                    } else {
                        print_surface_header(command.render, "vida task proof attach-release-bundle");
                        print_surface_line(command.render, "status", "blocked");
                        print_surface_line(command.render, "blocker", "task_proof_targets_missing");
                    }
                    return ExitCode::from(2);
                }

                let current_exe = match std::env::current_exe() {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("Failed to resolve current vida executable: {error}");
                        return ExitCode::from(1);
                    }
                };
                let mut args = vec![
                    "task".to_string(),
                    "proof".to_string(),
                    "attach-evidence".to_string(),
                    command.task_id.clone(),
                ];
                for target in &proof_targets {
                    args.push("--proof-target".to_string());
                    args.push(target.clone());
                }
                args.push("--result".to_string());
                args.push(command.result.clone());
                let command_text = command.command.clone().unwrap_or_else(|| {
                    "vida task proof attach-release-bundle".to_string()
                });
                args.push("--command".to_string());
                args.push(command_text);
                for artifact_ref in &command.artifact_ref {
                    args.push("--artifact-ref".to_string());
                    args.push(artifact_ref.clone());
                }
                for evidence in &command.evidence {
                    args.push("--evidence".to_string());
                    args.push(evidence.clone());
                }
                args.push("--state-dir".to_string());
                args.push(state_dir.display().to_string());
                if command.json {
                    args.push("--json".to_string());
                }

                match std::process::Command::new(current_exe).args(&args).output() {
                    Ok(output) => {
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                        eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        ExitCode::from(output.status.code().unwrap_or(1) as u8)
                    }
                    Err(error) => {
                        eprintln!("Failed to run attach-evidence for release proof bundle: {error}");
                        ExitCode::from(1)
                    }
                }
            }
        },
        TaskCommand::Ready(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            let cache_allowed = command.json
                && command.fields.is_none()
                && command.limit.is_none()
                && command.view.trim() == "summary";
            if cache_allowed {
                let projection_name = task_ready_projection_name(command.scope.as_deref());
                if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
                    &state_dir,
                    &projection_name,
                ) {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
                if let Some(cached) =
                    crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                        &state_dir,
                        &projection_name,
                        TASK_READ_RECENT_PROJECTION_MAX_AGE,
                    )
                {
                    println!("{cached}");
                    return ExitCode::SUCCESS;
                }
            }
            match task_ready_authoritative_first(state_dir.clone(), command.scope.as_deref()).await
            {
                Ok((tasks, metadata)) => {
                    if command.json {
                        let payload = task_ready_payload(
                            command.scope.as_deref(),
                            &tasks,
                            Some(&metadata),
                            &command.view,
                            command.fields.as_deref(),
                            command.limit,
                        );
                        crate::print_json_pretty(&payload);
                        if cache_allowed {
                            crate::operator_projection_cache::write_json_projection(
                                &state_dir,
                                &task_ready_projection_name(command.scope.as_deref()),
                                &payload,
                            );
                        }
                    } else {
                        print_task_ready(
                            command.render,
                            command.scope.as_deref(),
                            &tasks,
                            false,
                            Some(&metadata),
                            &command.view,
                            command.fields.as_deref(),
                            command.limit,
                        );
                    }
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
            if command.refresh {
                proxy_args.push("--refresh".to_string());
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
                        Some(status) => match store
                            .run_graph_status_is_stale_for_task_continuation_binding(status)
                            .await
                        {
                            Ok(true) => None,
                            Ok(false) => {
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
                            Err(error) => {
                                eprintln!(
                                    "Failed to classify stale run-graph status for task continuation: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        },
                        None => None,
                    };
                    let stale_closed_runtime_binding =
                        latest_run_graph_status.as_ref().and_then(|status| {
                            tasks.iter().find_map(|task| {
                                (task.id == status.task_id
                                    && state_store::StateStore::task_status_is_closed_like(
                                        &task.status,
                                    ))
                                .then(|| {
                                    state_store::RunGraphContinuationBinding {
                                        run_id: status.run_id.clone(),
                                        task_id: status.task_id.clone(),
                                        status: "bound".to_string(),
                                        active_bounded_unit: serde_json::json!({
                                            "kind": "run_graph_task",
                                            "task_id": status.task_id,
                                            "run_id": status.run_id,
                                            "active_node": status.active_node,
                                            "task_status": task.status,
                                        }),
                                        binding_source: "latest_run_graph_status_closed_task"
                                            .to_string(),
                                        why_this_unit: format!(
                                            "Latest runtime state points at closed task `{}`.",
                                            status.task_id
                                        ),
                                        primary_path: "diagnosis_path".to_string(),
                                        sequential_vs_parallel_posture:
                                            "unknown_until_run_graph_blocker_resolved".to_string(),
                                        request_text: None,
                                        recorded_at: "synthetic-read-only".to_string(),
                                    }
                                })
                            })
                        });
                    let runtime_binding = match select_task_next_lawful_binding(
                        &tasks,
                        explicit_binding.as_ref(),
                        current_binding.as_ref(),
                    ) {
                        Ok(binding) => binding,
                        Err(receipt) => {
                            if command.json {
                                let receipt_json = serde_json::to_value(&receipt).expect(
                                    "task next-lawful source drift receipt should serialize",
                                );
                                if command.scope.is_none() {
                                    crate::operator_projection_cache::write_json_projection(
                                        &state_dir,
                                        task_next_lawful_projection_name(),
                                        &receipt_json,
                                    );
                                }
                                crate::print_json_pretty(&receipt_json);
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
                    let runtime_binding_status = match runtime_binding {
                        Some(binding) => store.run_graph_status(&binding.run_id).await.ok(),
                        None => None,
                    };
                    let runtime_binding_task_missing_in_explicit_scope = command.scope.is_some()
                        && runtime_binding
                            .map(|binding| !task_exists_for_binding(&tasks, binding))
                            .unwrap_or(false);
                    let runtime_binding_is_closed_downstream_marker = runtime_binding
                        .map(|binding| {
                            continuation_binding_is_closed_downstream_marker(&tasks, binding)
                        })
                        .unwrap_or(false);
                    let runtime_binding_is_retired_terminal_closure_marker = runtime_binding
                        .map(|binding| {
                            continuation_binding_is_retired_terminal_closure_marker(
                                &tasks,
                                binding,
                                runtime_binding_status.as_ref(),
                            )
                        })
                        .unwrap_or(false);
                    let scoped_runtime_binding = if runtime_binding_task_missing_in_explicit_scope
                        || runtime_binding_is_closed_downstream_marker
                        || runtime_binding_is_retired_terminal_closure_marker
                    {
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
                    let ready_task_candidates = projection
                        .ready
                        .iter()
                        .map(|candidate| {
                            task_continuation_candidate(
                                &candidate.task,
                                candidate.ready_parallel_safe,
                            )
                        })
                        .collect::<Vec<_>>();
                    let ready_task_candidates = task_next_lawful_apply_strategy(
                        &tasks,
                        ready_task_candidates,
                        command.strategy.as_deref(),
                    );
                    let scoped_runtime_binding = if scoped_runtime_binding.is_none()
                        && ready_task_candidates.is_empty()
                        && crate::continuation_binding_summary::taskflow_leaf_active_tasks(&tasks)
                            .is_empty()
                    {
                        stale_closed_runtime_binding.as_ref()
                    } else {
                        scoped_runtime_binding
                    };
                    let runtime_recovery = match scoped_runtime_binding {
                        Some(binding) => {
                            store.run_graph_recovery_summary(&binding.run_id).await.ok()
                        }
                        None => None,
                    };
                    let latest_dispatch_receipt = match store
                        .latest_run_graph_dispatch_receipt_summary()
                        .await
                    {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!("Failed to read latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let terminal_consume_continue_run_id =
                        crate::latest_terminal_consume_continue_snapshot_run_id(&state_dir)
                            .ok()
                            .flatten();
                    let mut receipt = if let Some(selected_task_id) = command.select.as_deref() {
                        if scoped_runtime_binding.is_some() {
                            blocked_task_next_lawful_receipt(
                                serde_json::Value::Null,
                                ready_task_candidates,
                                "select_conflicts_with_active_runtime_binding",
                                "Cannot apply --select while an active runtime binding is present; resolve or complete the current binding first.",
                            )
                        } else {
                            task_next_lawful_select_ready_candidate_receipt(
                                &tasks,
                                ready_task_candidates,
                                selected_task_id,
                            )
                        }
                    } else {
                        match scoped_runtime_binding {
                            Some(binding)
                                if latest_dispatch_receipt.as_ref().is_some_and(|dispatch| {
                                    continuation_binding_has_live_unit(&tasks, binding)
                                        && dispatch.run_id == binding.run_id
                                        && runtime_dispatch_receipt_has_ready_downstream_handoff(
                                            Some(binding.run_id.as_str()),
                                            Some(dispatch),
                                        )
                                }) =>
                            {
                                pass_ready_downstream_handoff_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                    terminal_consume_continue_run_id.as_deref(),
                                    latest_dispatch_receipt
                                        .as_ref()
                                        .and_then(downstream_dispatch_command_for_task_next_lawful)
                                        .as_deref(),
                                )
                            }
                            Some(binding)
                                if continuation_binding_has_live_unit(&tasks, binding)
                                    && runtime_recovery_blocks_task_next_lawful(
                                        runtime_recovery.as_ref(),
                                        latest_dispatch_receipt.as_ref(),
                                    )
                                    && runtime_binding_has_active_exception_takeover(
                                        binding,
                                        latest_dispatch_receipt.as_ref(),
                                    ) =>
                            {
                                pass_exception_takeover_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                )
                            }
                            Some(binding)
                                if continuation_binding_has_live_unit(&tasks, binding)
                                    && runtime_dispatch_receipt_has_completed_lane(
                                        Some(binding.run_id.as_str()),
                                        latest_dispatch_receipt.as_ref(),
                                    ) =>
                            {
                                pass_completed_lane_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                )
                            }
                            Some(binding)
                                if runtime_recovery_blocks_task_next_lawful(
                                    runtime_recovery.as_ref(),
                                    latest_dispatch_receipt.as_ref(),
                                ) =>
                            {
                                blocked_runtime_recovery_task_next_lawful_receipt(
                                    binding,
                                    ready_task_candidates,
                                    runtime_recovery.as_ref(),
                                    latest_dispatch_receipt.as_ref(),
                                )
                            }
                            _ => task_next_lawful_receipt(
                                &tasks,
                                ready_task_candidates,
                                scoped_runtime_binding,
                            ),
                        }
                    };
                    receipt = task_next_lawful_attach_explanation(
                        receipt,
                        command.explain,
                        command.strategy.as_deref(),
                        command.select.as_deref(),
                    );
                    if command.json {
                        let receipt_json = serde_json::to_value(&receipt)
                            .expect("task next-lawful receipt should serialize");
                        if command.scope.is_none() {
                            crate::operator_projection_cache::write_json_projection(
                                &state_dir,
                                task_next_lawful_projection_name(),
                                &receipt_json,
                            );
                        }
                        crate::print_json_pretty(&receipt_json);
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
        TaskCommand::Reset(command) => run_task_reset(command).await,
        TaskCommand::Update(command) => {
            let state_dir = command
                .state_dir
                .clone()
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
                Ok(store) => {
                    let planner_metadata = if task_update_planner_metadata_requested(&command) {
                        match store.show_task(&command.task_id).await {
                            Ok(existing) => match task_update_planner_metadata_arg(
                                &existing.planner_metadata,
                                &command,
                            ) {
                                Ok(planner_metadata) => planner_metadata,
                                Err(error) => {
                                    eprintln!("{error}");
                                    return ExitCode::from(2);
                                }
                            },
                            Err(error) => {
                                eprintln!(
                                    "Failed to read task before planner metadata update: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        None
                    };
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: command.title.as_deref(),
                            status: command.status.as_deref(),
                            priority: command.priority,
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
                            planner_metadata,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task update")
                                    .await
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
                            if let state_store::StateStoreError::InvalidTaskRecord { reason } =
                                &error
                            {
                                if let Some(task_id) =
                                    state_store::StateStore::task_update_close_authority_task_id_from_reason(reason)
                                {
                                    print_task_update_close_authority_blocked(
                                        command.render,
                                        task_id,
                                        command.json,
                                    );
                                    return ExitCode::from(1);
                                }
                                if let Some(task_id) = state_store::StateStore::task_update_closed_task_mutation_task_id_from_reason(reason)
                                {
                                    print_task_update_closed_task_mutation_blocked(
                                        command.render,
                                        task_id,
                                        command.json,
                                    );
                                    return ExitCode::from(1);
                                }
                                if command.json {
                                    if let Some(issue) =
                                        task_update_graph_issue_from_invalid_record_reason(reason)
                                    {
                                        print_task_update_graph_blocked(&issue, command.json);
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            eprintln!("Failed to update task: {error}");
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
        TaskCommand::Note(command) => match command.command {
            TaskNoteCommand::Append(command) => {
                let state_dir = command
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let message = match resolve_optional_text_arg(
                    "message",
                    command.message.as_deref(),
                    command.message_file.as_deref(),
                ) {
                    Ok(Some(message)) if !message.trim().is_empty() => message.trim().to_string(),
                    Ok(_) => {
                        eprintln!("A non-empty --message or --message-file value is required");
                        return ExitCode::from(2);
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => {
                        match store
                            .append_task_notes(&command.task_id, &command.separator, &message)
                            .await
                        {
                            Ok(task) => {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task note append",
                                )
                                .await
                                {
                                    return code;
                                }
                                print_task_mutation(
                                    command.render,
                                    "vida task note append",
                                    &task,
                                    command.json,
                                );
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("Failed to append task note: {error}");
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
        },
        TaskCommand::Block(command) => {
            let reason = command.reason.trim();
            if reason.is_empty() {
                eprintln!("--reason cannot be empty");
                return ExitCode::from(2);
            }
            let evidence = command
                .evidence
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            let blocker_codes = normalize_task_block_list(&command.blockers);
            let next_actions = command
                .next_actions
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let existing = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to read task before block mutation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let previous_status = existing.status.clone();
                    if state_store::StateStore::task_status_is_closed_like(&existing.status) {
                        let receipt = TaskBlockReceipt {
                            surface: "vida task block",
                            status: "blocked",
                            blocker_codes: vec!["task_already_closed".to_string()],
                            next_actions: vec![
                                "Inspect the closed task or reopen it before recording a runtime blocker."
                                    .to_string(),
                            ],
                            task_id: existing.id.clone(),
                            blocked: false,
                            closed: true,
                            previous_status,
                            reason: reason.to_string(),
                            evidence: evidence.clone(),
                            notes_appended: false,
                            task: existing,
                        };
                        print_task_block_receipt(command.render, &receipt, command.json);
                        return ExitCode::from(1);
                    }

                    let notes = append_task_block_note(
                        existing.notes.as_deref(),
                        reason,
                        &evidence,
                        &blocker_codes,
                        &next_actions,
                    );
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: None,
                            status: Some("blocked"),
                            priority: None,
                            notes: Some(&notes),
                            description: None,
                            parent_id: None,
                            add_labels: &[],
                            remove_labels: &[],
                            set_labels: None,
                            execution_mode: None,
                            order_bucket: None,
                            parallel_group: None,
                            conflict_domain: None,
                            planner_metadata: None,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task block")
                                    .await
                            {
                                return code;
                            }
                            let receipt = TaskBlockReceipt {
                                surface: "vida task block",
                                status: task_json_success_status(),
                                blocker_codes,
                                next_actions,
                                task_id: task.id.clone(),
                                blocked: task.status == "blocked",
                                closed: state_store::StateStore::task_status_is_closed_like(
                                    &task.status,
                                ),
                                previous_status,
                                reason: reason.to_string(),
                                evidence,
                                notes_appended: true,
                                task,
                            };
                            print_task_block_receipt(command.render, &receipt, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to block task: {error}");
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
        TaskCommand::Verify(command) => {
            let evidence = normalized_task_verify_evidence(&command.evidence);
            let proof_blocker = command
                .proof_blocker
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if command.proof_blocked && proof_blocker.is_none() && evidence.is_empty() {
                eprintln!("--proof-blocked requires --proof-blocker or --evidence");
                return ExitCode::from(2);
            }
            if !command.source_fixed && !command.tests_green && !command.proof_blocked {
                eprintln!(
                    "task verify requires at least one of --source-fixed, --tests-green, or --proof-blocked"
                );
                return ExitCode::from(2);
            }
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let existing = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to read task before verify mutation: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    if state_store::StateStore::task_status_is_closed_like(&existing.status) {
                        let receipt = TaskVerifyReceipt {
                            surface: "vida task verify",
                            status: "blocked",
                            task_id: existing.id.clone(),
                            partial: false,
                            closed: true,
                            source_fixed: command.source_fixed,
                            tests_green: command.tests_green,
                            proof_blocked: command.proof_blocked,
                            proof_blocked_by_runtime: false,
                            proof_blocker: proof_blocker.map(str::to_string),
                            evidence,
                            blocker_codes: vec!["task_already_closed".to_string()],
                            next_actions: vec![
                                "Inspect the closed task or reopen it before recording partial verification."
                                    .to_string(),
                            ],
                            task: existing,
                        };
                        print_task_verify_receipt(command.render, &receipt, command.json);
                        return ExitCode::from(1);
                    }
                    let notes = append_task_verify_note(
                        existing.notes.as_deref(),
                        command.source_fixed,
                        command.tests_green,
                        command.proof_blocked,
                        proof_blocker,
                        &evidence,
                    );
                    let add_labels = task_verify_labels(
                        command.source_fixed,
                        command.tests_green,
                        command.proof_blocked,
                    );
                    let planner_metadata = task_verify_planner_metadata(
                        &existing.planner_metadata,
                        command.proof_blocked,
                        proof_blocker,
                        &evidence,
                    );
                    match store
                        .update_task(state_store::UpdateTaskRequest {
                            task_id: &command.task_id,
                            title: None,
                            status: None,
                            priority: None,
                            notes: Some(&notes),
                            description: None,
                            parent_id: None,
                            add_labels: &add_labels,
                            remove_labels: &[],
                            set_labels: None,
                            execution_mode: None,
                            order_bucket: None,
                            parallel_group: None,
                            conflict_domain: None,
                            planner_metadata,
                        })
                        .await
                    {
                        Ok(task) => {
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task verify")
                                    .await
                            {
                                return code;
                            }
                            let proof_blocked_by_runtime = command.proof_blocked
                                && task_reports_runtime_proof_blocker(
                                    &task.labels,
                                    task.close_reason.as_deref(),
                                );
                            let receipt = TaskVerifyReceipt {
                                surface: "vida task verify",
                                status: task_json_success_status(),
                                task_id: task.id.clone(),
                                partial: true,
                                closed: state_store::StateStore::task_status_is_closed_like(
                                    &task.status,
                                ),
                                source_fixed: command.source_fixed,
                                tests_green: command.tests_green,
                                proof_blocked: command.proof_blocked,
                                proof_blocked_by_runtime,
                                proof_blocker: proof_blocker.map(str::to_string),
                                evidence,
                                blocker_codes: Vec::new(),
                                next_actions: if command.proof_blocked {
                                    vec![
                                        "Resolve or attach final proof evidence before closing this task."
                                            .to_string(),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                task,
                            };
                            print_task_verify_receipt(command.render, &receipt, command.json);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Failed to verify task: {error}");
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
        TaskCommand::Attempt(command) => run_task_attempt(command).await,
        TaskCommand::Stage(command) => run_task_stage(command).await,
        TaskCommand::Split(command) => run_task_split_like(command, "vida task split").await,
        TaskCommand::SpawnBlocker(command) => {
            run_task_spawn_blocker_like(command, "vida task spawn-blocker").await
        }
        TaskCommand::AdaptivePreview(command) => run_task_adaptive_preview(command).await,
        TaskCommand::PackFinalize(command) => run_task_pack_finalize(command).await,
        TaskCommand::Close(command) => {
            let explicit_state_dir = command.state_dir.is_some();
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            let project_root = project_root_for_task_state(&state_dir);
            let feedback_source = command.source.as_deref().unwrap_or("vida task close");
            let close_reason = match resolve_optional_text_arg(
                "reason",
                command.reason.as_deref(),
                command.reason_file.as_deref(),
            ) {
                Ok(Some(reason)) => reason,
                Ok(None) => {
                    eprintln!("task close requires --reason or --reason-file");
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            match StateStore::open_existing(state_dir.clone()).await {
                Ok(store) => {
                    let preclose_task = match store.show_task(&command.task_id).await {
                        Ok(task) => task,
                        Err(error) => {
                            eprintln!("Failed to close task: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let inheritance_rows = store.list_tasks(None, true).await.ok();
                    if let Some(payload) = task_close_structured_proof_gate_payload(
                        &preclose_task,
                        inheritance_rows.as_deref(),
                    ) {
                        print_task_close_structured_proof_gate_block(
                            command.render,
                            &payload,
                            command.json,
                        );
                        return ExitCode::from(1);
                    }
                    if crate::agent_feedback_surface::canonical_close_status_from_reason(
                        &close_reason,
                    )
                    .is_some()
                    {
                        let task_value = serde_json::to_value(&preclose_task)
                            .expect("task close payload should serialize");
                        let telemetry = task_close_host_agent_telemetry(
                            &state_dir,
                            explicit_state_dir,
                            project_root.as_deref(),
                            &task_value,
                            &close_reason,
                            feedback_source,
                        );
                        if let Some((blocker_codes, next_actions)) =
                            task_close_feedback_blocker_summary(&telemetry)
                        {
                            if command.json {
                                let payload =
                                    crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                                        "vida task close",
                                    )
                                    .blocker_codes(blocker_codes)
                                    .next_actions(next_actions)
                                    .artifact_refs(serde_json::json!({
                                        "surface": "vida task close",
                                        "task_id": preclose_task.id.clone(),
                                        "feedback_source": feedback_source,
                                    }))
                                    .extra_fields(serde_json::json!({
                                        "task": preclose_task,
                                        "host_agent_telemetry": telemetry,
                                        "automation": null,
                                    }))
                                    .build()
                                    .expect("task close feedback blocker should satisfy release-1 operator contract");
                                crate::print_json_pretty(&payload);
                            } else {
                                print_task_mutation(
                                    command.render,
                                    "vida task close",
                                    &preclose_task,
                                    false,
                                );
                                print_surface_line(
                                    command.render,
                                    "telemetry blockers",
                                    &blocker_codes.join(", "),
                                );
                                for action in next_actions {
                                    print_surface_line(command.render, "next", &action);
                                }
                            }
                            return ExitCode::from(1);
                        }
                    }
                    match store.close_task(&command.task_id, &close_reason).await {
                        Ok(_task) => {
                            if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_specification_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                            if let Err(error) = crate::runtime_dispatch_state::maybe_bridge_closed_implementer_task_into_latest_receipt(&store, &command.task_id).await {
                            eprintln!("Failed to bridge closed task into latest run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                            let task = match store.show_task(&command.task_id).await {
                                Ok(task) if task.status == "closed" => task,
                                Ok(task) => {
                                    eprintln!(
                                        "Task close drifted after post-close reconciliation: `{}` is `{}` instead of `closed`.",
                                        command.task_id, task.status
                                    );
                                    return ExitCode::from(1);
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to re-read closed task after post-close reconciliation: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                            if let Err(code) =
                                refresh_task_snapshot_after_mutation(&store, "vida task close")
                                    .await
                            {
                                return code;
                            }
                            let task_value = serde_json::to_value(&task)
                                .expect("task close payload should serialize");
                            let telemetry = task_close_host_agent_telemetry(
                                &state_dir,
                                explicit_state_dir,
                                project_root.as_deref(),
                                &task_value,
                                &close_reason,
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
                            let telemetry_feedback_blocker =
                                task_close_feedback_blocker_summary(&telemetry);
                            let epic_progress_summary = match store.all_tasks().await {
                                Ok(rows) => {
                                    match task_close_epic_progress_summary(
                                        &rows,
                                        &command.task_id,
                                        command.include_global_progress,
                                    ) {
                                        Ok(summary) => Some(summary),
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to compute task close epic progress summary: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read tasks for task close epic progress summary: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                            if command.json {
                                let payload = task_close_result_payload(
                                    &task,
                                    &telemetry,
                                    automation.as_ref(),
                                    telemetry_feedback_blocker.as_ref(),
                                    epic_progress_summary.as_ref(),
                                );
                                crate::print_json_pretty(&payload);
                            } else {
                                print_task_mutation(
                                    command.render,
                                    "vida task close",
                                    &task,
                                    false,
                                );
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
                                if let Some((blocker_codes, _)) = &telemetry_feedback_blocker {
                                    print_surface_line(
                                        command.render,
                                        "telemetry blockers",
                                        &blocker_codes.join(", "),
                                    );
                                }
                                if let Some((_, next_actions)) = &telemetry_feedback_blocker {
                                    for action in next_actions {
                                        print_surface_line(command.render, "next", action);
                                    }
                                }
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
                                if let Some(summary) = &epic_progress_summary {
                                    print_task_close_epic_progress_summary(command.render, summary);
                                }
                            }
                            if task_close_automation_is_blocked(automation.as_ref()) {
                                ExitCode::from(1)
                            } else {
                                ExitCode::SUCCESS
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to close task: {error}");
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
        TaskCommand::Reconcile(command) => {
            if !command.epics {
                let payload =
                    serde_json::to_value(taskflow_core::task::reconcile::scope_required_payload(
                        "vida task reconcile",
                        command.dry_run,
                        command.close_if_complete,
                    ))
                    .expect("task reconcile scope-required payload should serialize");
                if command.json {
                    crate::print_json_pretty(&payload);
                } else if matches!(command.render, crate::RenderMode::Plain) {
                    println!(
                        "{}",
                        taskflow_format_toon::render_value_section("vida task reconcile", &payload)
                    );
                } else {
                    print_surface_line(command.render, "status", "blocked");
                    print_surface_line(
                        command.render,
                        "next",
                        "Run vida task reconcile --epics to inspect open epics.",
                    );
                }
                return ExitCode::from(1);
            }

            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match reconcile_epics_from_descendant_progress(
                    &store,
                    command.close_if_complete,
                    command.dry_run,
                )
                .await
                {
                    Ok(receipt) => {
                        let payload = serde_json::to_value(&receipt)
                            .expect("task epic reconcile receipt should serialize");
                        if command.json {
                            crate::print_json_pretty(&payload);
                        } else if matches!(command.render, crate::RenderMode::Plain) {
                            println!(
                                "{}",
                                taskflow_format_toon::render_value_section(
                                    "vida task reconcile",
                                    &payload,
                                )
                            );
                        } else {
                            print_surface_line(command.render, "status", &receipt.status);
                            print_surface_line(
                                command.render,
                                "closed epics",
                                &receipt.closed_epics.len().to_string(),
                            );
                            print_surface_line(
                                command.render,
                                "blocked epics",
                                &receipt.blocked_epics.len().to_string(),
                            );
                            if let Some(action) = receipt.next_actions.first() {
                                print_surface_line(command.render, "next", action);
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to reconcile epics: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::ReconcileClosedRuns(command) => {
            let state_dir = command
                .state_dir
                .clone()
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .reconcile_historical_closed_task_active_runs(command.limit)
                    .await
                {
                    Ok(summary) => {
                        let next_actions = summary
                            .skipped_runs
                            .first()
                            .map(|skipped| {
                                vec![format!(
                                    "Inspect skipped closed-task run `{}` with `{}`; reason={}.",
                                    skipped.run_id, skipped.inspect_command, skipped.reason
                                )]
                            })
                            .unwrap_or_default();
                        if command.json {
                            let payload =
                                crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                                    "vida task reconcile-closed-runs",
                                )
                                .artifact_refs(serde_json::json!({
                                    "surface": "vida task reconcile-closed-runs",
                                }))
                                .extra_fields(serde_json::json!({
                                    "summary": summary,
                                    "recommended_next_actions": next_actions,
                                }))
                                .build()
                                .expect("task reconcile closed runs payload should satisfy release-1 operator contract");
                            crate::print_json_pretty(&payload);
                        } else {
                            print_surface_line(
                                command.render,
                                "reconciled closed-task runs",
                                &summary.reconciled_count.to_string(),
                            );
                            if let Some(action) = next_actions.first() {
                                print_surface_line(command.render, "next", action);
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        if command.json {
                            let payload = task_reconcile_closed_runs_error_payload(&error);
                            crate::print_json_pretty(&payload);
                        } else {
                            eprintln!("Failed to reconcile closed-task runs: {error}");
                        }
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::PruneClosedEpics(command) => run_task_prune_closed_epics(command).await,
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
                            eprintln!(
                                "Failed to read reverse dependencies from snapshot: {snapshot_error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    match StateStore::reverse_dependencies_from_rows(&rows, &command.task_id) {
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
                            eprintln!("Failed to read reverse dependencies from snapshot: {error}");
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
                    print_task_direct_children(command.render, &tree, command.full, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    if command.json {
                        let next_action = format!(
                            "Run `{}` to inspect graph cycles or reduce traversal scope with `{}`.",
                            operator_output::command_text::human_command(
                                "vida task validate-graph --json"
                            ),
                            operator_output::command_text::human_command(
                                "vida task children <task-id> --json"
                            )
                        );
                        let payload =
                            crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                                "vida task children",
                            )
                            .blocker_codes(vec!["task_tree_traversal_failed".to_string()])
                            .next_actions(vec![next_action.clone()])
                            .artifact_refs(serde_json::json!({
                                "surface": "vida task children",
                                "task_id": command.task_id.clone(),
                            }))
                            .extra_fields(serde_json::json!({
                                "task_id": command.task_id.clone(),
                                "reason": error.to_string(),
                                "next_action": next_action,
                            }))
                            .build()
                            .expect("task children traversal error should satisfy release-1 operator contract");
                        crate::print_json_pretty(&payload);
                    } else {
                        eprintln!("Failed to read task direct children: {error}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        TaskCommand::Tree(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match task_dependency_tree_read_only(state_dir.clone(), &command.task_id).await {
                Ok(tree) => {
                    let progress = if command.full {
                        match task_progress_summary_read_only(&state_dir, &command.task_id).await {
                            Ok(summary) => Some(summary),
                            Err(error) => {
                                eprintln!("Failed to compute task tree progress: {error}");
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        None
                    };
                    print_task_dependency_tree(
                        command.render,
                        &tree,
                        command.full,
                        progress.as_ref(),
                        command.json,
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    if command.json {
                        let next_action = format!(
                            "Run `{}` to inspect graph cycles or reduce traversal scope with `{}`.",
                            operator_output::command_text::human_command(
                                "vida task validate-graph --json"
                            ),
                            operator_output::command_text::human_command(
                                "vida task children <task-id> --json"
                            )
                        );
                        let payload =
                            crate::release1_operator_output::Release1OperatorOutputBuilder::new(
                                "vida task tree",
                            )
                            .blocker_codes(vec!["task_tree_traversal_failed".to_string()])
                            .next_actions(vec![next_action.clone()])
                            .artifact_refs(serde_json::json!({
                                "surface": "vida task tree",
                                "task_id": command.task_id.clone(),
                            }))
                            .extra_fields(serde_json::json!({
                                "task_id": command.task_id.clone(),
                                "reason": error.to_string(),
                                "next_action": next_action,
                            }))
                            .build()
                            .expect("task tree traversal error should satisfy release-1 operator contract");
                        crate::print_json_pretty(&payload);
                    } else {
                        eprintln!("Failed to read task dependency tree: {error}");
                    }
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
        TaskCommand::DefectBatchRehome(command) => {
            let state_dir = command
                .state_dir
                .unwrap_or_else(state_store::default_state_dir);
            match StateStore::open_existing(state_dir).await {
                Ok(store) => match store
                    .defect_batch_rehome(
                        &command.from_parent_id,
                        &command.to_parent_id,
                        &command.child_ids,
                        &command.pause_task_ids,
                        &command.start_task_ids,
                        command.dry_run,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Err(code) = refresh_task_snapshot_after_mutation(
                            &store,
                            "vida task defect-batch-rehome",
                        )
                        .await
                        {
                            return code;
                        }
                        print_task_defect_batch_rehome_result(
                            command.render,
                            &result,
                            command.json,
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("Failed to defect-batch rehome tasks: {error}");
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
            TaskDependencyCommand::AddBulk(add) => {
                let state_dir = add
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let edges =
                    match task_dependency_bulk_edge_inputs(&add.edges, add.edge_file.as_deref()) {
                        Ok(edges) => edges,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependencies_bulk(&edges, &add.created_by, add.dry_run)
                        .await
                    {
                        Ok(result) => {
                            if result.failed_count == 0 && !result.dry_run {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task dep add-bulk",
                                )
                                .await
                                {
                                    return code;
                                }
                            }
                            let exit_code = if result.failed_count == 0 {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            };
                            print_task_dependency_bulk_add_result(add.render, &result, add.json);
                            exit_code
                        }
                        Err(error) => {
                            eprintln!("Failed to add task dependencies in bulk: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to open authoritative state store: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            TaskDependencyCommand::Ensure(ensure) => {
                let state_dir = ensure
                    .state_dir
                    .clone()
                    .unwrap_or_else(state_store::default_state_dir);
                let edges = vec![state_store::TaskDependencyBulkAddInput {
                    issue_id: ensure.task_id.clone(),
                    depends_on_id: ensure.depends_on_id.clone(),
                    edge_type: ensure.edge_type.clone(),
                }];
                match StateStore::open_existing(state_dir).await {
                    Ok(store) => match store
                        .add_task_dependencies_bulk(&edges, &ensure.created_by, false)
                        .await
                    {
                        Ok(result) => {
                            if result.failed_count == 0 && result.created_count > 0 {
                                if let Err(code) = refresh_task_snapshot_after_mutation(
                                    &store,
                                    "vida task dep ensure",
                                )
                                .await
                                {
                                    return code;
                                }
                            }
                            let exit_code = if result.failed_count == 0 {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            };
                            print_task_dependency_bulk_add_result_for_surface(
                                ensure.render,
                                &result,
                                ensure.json,
                                "vida task dep ensure",
                                "task dependency ensure result should render as json",
                            );
                            exit_code
                        }
                        Err(error) => {
                            eprintln!("Failed to ensure task dependency: {error}");
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
            match task_critical_path_snapshot_first(state_dir).await {
                Ok(path) => {
                    print_task_critical_path(command.render, &path, command.json);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to compute critical path: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        blocked_runtime_recovery_task_next_lawful_receipt, blocked_task_next_lawful_receipt,
        build_adaptive_replan_finding_preview, build_spawn_blocker_preview,
        build_split_mutation_preview, canonical_json_string_array_entries,
        classify_task_close_git_stage_failure, ensure_existing_task_mismatch_reason,
        git_diff_text_for_paths, limit_diff_hunks, load_adaptive_preview_finding_json,
        normalize_task_json_contract_arrays, parse_adaptive_replan_finding_input,
        parse_label_values, parse_optional_label_value, parse_proof_target_values,
        pass_completed_lane_task_next_lawful_receipt,
        pass_exception_takeover_task_next_lawful_receipt,
        pass_ready_downstream_handoff_task_next_lawful_receipt,
        persist_task_handoff_accept_receipt, reconcile_epics_from_descendant_progress,
        render_validator_packet_text, runtime_binding_has_active_exception_takeover,
        runtime_binding_open_delegated_cycle_next_action, runtime_recovery_blocks_task_next_lawful,
        select_task_next_lawful_binding, task_attempt_policy_attempt_id,
        task_browser_proof_planner_metadata, task_close_automation_is_blocked,
        task_close_automation_receipt, task_close_commit_allowlist_next_actions,
        task_close_commit_file_strings, task_close_epic_progress_summary,
        task_close_feedback_blocker_summary, task_close_host_agent_telemetry,
        task_close_ignored_dirty_files_for_explicit_commit, task_close_result_payload,
        task_close_uses_isolated_state_dir, task_continuation_candidate,
        task_create_planner_metadata_arg, task_create_semantics_mismatch,
        task_create_semantics_requested, task_create_title, task_critical_path_snapshot_first,
        task_evidence_proof_planner_metadata, task_exception_takeover_metadata_path,
        task_exception_takeover_owned_write_scope, task_handoff_accept_receipt,
        task_handoff_project_receipt_root, task_handoff_receipt_path, task_handoff_receipt_root,
        task_json_success_status, task_next_lawful_apply_strategy, task_next_lawful_receipt,
        task_next_lawful_select_ready_candidate_receipt, task_owned_status_receipt, task_parent_id,
        task_progress_summary_for_basis, task_ready_authoritative_first,
        task_takeover_status_default_lines, task_takeover_status_receipt,
        task_update_planner_metadata_arg, validate_task_handoff_accept_receipt,
        TaskCloseAutomationReceipt, TaskContinuationCandidate, TaskProofAttachBrowserReceipt,
        TaskProofAttachEvidenceReceipt, ADAPTIVE_REPLAN_FINDING_KINDS,
    };
    use crate::state_store;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::cli;
    use crate::test_cli_support::guard_current_dir;
    use crate::test_cli_support::EnvVarGuard;
    use std::fs;
    use std::process::Command;
    use std::process::ExitCode;
    use taskflow_core::task::verify::{
        append_task_browser_proof_note, task_browser_proof_target, TaskBrowserProofArtifact,
        TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION, TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION,
    };

    #[test]
    fn task_attempt_policy_attempt_id_uses_shared_record_components() {
        assert_eq!(
            task_attempt_policy_attempt_id("task:1", "stage one", "!!!"),
            "task-1--stage-one--attempt"
        );
    }

    #[test]
    fn validator_packet_limits_diff_hunks_and_lines() {
        let diff =
            "diff --git a/a b/a\n@@ -1 +1 @@\n-old\n+new\n@@ -4 +4 @@\n-a\n+b\n@@ -8 +8 @@\n-c\n+d";

        let (limited, truncated) = limit_diff_hunks(diff, 2, 20);

        assert!(truncated);
        assert!(limited.contains("@@ -1 +1 @@"));
        assert!(limited.contains("@@ -4 +4 @@"));
        assert!(!limited.contains("@@ -8 +8 @@"));
    }

    #[test]
    fn validator_packet_diff_is_limited_to_owned_paths() {
        let repo = std::env::temp_dir().join(format!(
            "vida-validator-packet-owned-diff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&repo).expect("create temporary repo");

        let run_git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&repo)
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        };

        run_git(&["init"]);
        run_git(&["config", "user.email", "test@example.invalid"]);
        run_git(&["config", "user.name", "VIDA Test"]);
        fs::write(repo.join("owned.txt"), "owned=old\n").expect("write owned fixture");
        fs::write(repo.join("secret.env"), "API_TOKEN=old\n").expect("write secret fixture");
        run_git(&["add", "owned.txt", "secret.env"]);
        run_git(&["commit", "-m", "seed"]);

        fs::write(repo.join("owned.txt"), "owned=new\n").expect("modify owned fixture");
        fs::write(repo.join("secret.env"), "API_TOKEN=LEAK_ME\n").expect("modify secret fixture");

        let owned_paths = vec!["owned.txt".to_string()];
        let diff = git_diff_text_for_paths(&repo, &["diff", "--unified=3", "HEAD"], &owned_paths)
            .expect("owned path diff should succeed");
        let diffstat = git_diff_text_for_paths(&repo, &["diff", "--stat", "HEAD"], &owned_paths)
            .expect("owned path diffstat should succeed");

        assert!(diff.contains("owned=new"));
        assert!(diffstat.contains("owned.txt"));
        assert!(!diff.contains("secret.env"));
        assert!(!diff.contains("LEAK_ME"));
        assert!(!diffstat.contains("secret.env"));

        fs::remove_dir_all(repo).expect("remove temporary repo");
    }

    #[test]
    fn validator_packet_text_contains_required_schema_and_context() {
        let payload = serde_json::json!({
            "active_bounded_unit": {
                "task_id": "task-a",
                "title": "Task A",
                "status": "in_progress"
            },
            "repo_root": "C:/project/vida-stack",
            "owned_files": ["crates/vida/src/cli.rs"],
            "dirty_files": ["crates/vida/src/cli.rs"],
            "diffstat": "1 file changed",
            "key_hunks": "@@ -1 +1 @@",
            "proof_commands": ["cargo check -p vida --tests"],
            "prior_validator_blockers": ["validator blocking_findings: none"],
        });

        let text = render_validator_packet_text(&payload);

        assert!(text.contains("active_bounded_unit: task-a"));
        assert!(text.contains("OWNED FILES"));
        assert!(text.contains("KEY HUNKS"));
        assert!(text.contains("PROOF COMMANDS"));
        assert!(text.contains("VERDICT: PASS|BLOCKED"));
        assert!(text.contains("BLOCKING_FINDINGS:"));
    }

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

    #[test]
    fn task_reset_cli_accepts_dry_run_include_steps_and_json() {
        let parsed = cli(&[
            "task",
            "reset",
            "task-1",
            "--dry-run",
            "--include-steps",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Reset(command) = args.command else {
            panic!("reset command should parse");
        };

        assert_eq!(command.task_id, "task-1");
        assert!(command.dry_run);
        assert!(command.include_steps);
        assert!(command.json);
    }

    #[test]
    fn task_reset_command_reopens_task_subtree_without_steps_by_default() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "feature-task",
                "Feature",
                "task",
                "in_progress",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "feature-subtask",
                "Subtask",
                "subtask",
                "closed",
                3,
                Some("feature-task"),
            )
            .await;
            create_task_for_test(
                &store,
                "feature-step",
                "Step",
                "step",
                "closed",
                4,
                Some("feature-subtask"),
            )
            .await;
        }));

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Reset(crate::TaskResetArgs {
                    task_id: "feature-task".to_string(),
                    state_dir: Some(harness.path().to_path_buf()),
                    ..crate::TaskResetArgs::default()
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            assert_eq!(
                store
                    .show_task("feature-task")
                    .await
                    .expect("feature")
                    .status,
                "open"
            );
            assert_eq!(
                store
                    .show_task("feature-subtask")
                    .await
                    .expect("subtask")
                    .status,
                "open"
            );
            assert_eq!(
                store.show_task("feature-step").await.expect("step").status,
                "closed"
            );
        }));
    }

    #[test]
    fn task_reset_command_can_include_steps() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-1",
                "Task",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "step-1",
                "Step",
                "step",
                "closed",
                2,
                Some("task-1"),
            )
            .await;
        }));

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Reset(crate::TaskResetArgs {
                    task_id: "task-1".to_string(),
                    include_steps: true,
                    state_dir: Some(harness.path().to_path_buf()),
                    ..crate::TaskResetArgs::default()
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let step = store.show_task("step-1").await.expect("step");
            assert_eq!(step.status, "open");
            assert_eq!(step.closed_at, None);
            assert_eq!(step.close_reason, None);
        }));
    }

    #[test]
    fn task_read_uses_fresh_snapshot_when_live_store_lost_rows() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "live-epic", "Live epic", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "snapshot-only-task",
                "Snapshot-only task",
                "task",
                "open",
                2,
                Some("live-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
            store
                .delete_task_record("snapshot-only-task")
                .await
                .expect("live task deletion should simulate stale live store");
            fs::remove_file(
                crate::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                    harness.path(),
                ),
            )
            .expect("lost live rows should not carry a legitimate mutation marker");
            drop(store);

            let (rows, metadata) =
                super::load_task_snapshot_rows_authoritative_first(harness.path())
                    .await
                    .expect("task rows should load");
            assert_eq!(metadata.mode, "fresh_snapshot_live_divergence");
            assert!(metadata.degraded);
            assert!(rows.iter().any(|task| task.id == "live-epic"));
            assert!(rows.iter().any(|task| task.id == "snapshot-only-task"));
        });
    }

    #[test]
    fn task_operator_filter_matches_issue_type_aliases_canonically() {
        let rows = vec![
            task_record_for_progress("legacy-todo", "open", "todo", Some("task-parent")),
            task_record_for_progress("canonical-step", "open", "step", Some("task-parent")),
            task_record_for_progress("sub-task", "open", "sub_task", Some("task-parent")),
            task_record_for_progress("plain-task", "open", "task", None),
        ];

        let step_rows = super::filter_task_rows_for_operator(
            rows.clone(),
            None,
            true,
            None,
            Some("step"),
            None,
            None,
        );
        let step_ids = step_rows
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(step_ids, vec!["legacy-todo", "canonical-step"]);

        let todo_rows = super::filter_task_rows_for_operator(
            rows.clone(),
            None,
            true,
            None,
            Some("todo"),
            None,
            None,
        );
        let todo_ids = todo_rows
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(todo_ids, vec!["legacy-todo", "canonical-step"]);

        let subtask_rows = super::filter_task_rows_for_operator(
            rows,
            None,
            true,
            None,
            Some("subtask"),
            None,
            None,
        );
        assert_eq!(subtask_rows.len(), 1);
        assert_eq!(subtask_rows[0].id, "sub-task");
    }

    fn run_cli_on_runtime_stack_for_test(args: Vec<String>) -> ExitCode {
        std::thread::Builder::new()
            .name("vida-test-cli-runtime".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let runtime =
                    tokio::runtime::Runtime::new().expect("test CLI runtime should initialize");
                let argv = args.iter().map(String::as_str).collect::<Vec<_>>();
                runtime.block_on(crate::run(cli(&argv)))
            })
            .expect("test CLI runtime thread should spawn")
            .join()
            .expect("test CLI runtime thread should complete")
    }

    fn run_on_runtime_stack_for_test(work: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .name("vida-test-expanded-stack".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(work)
            .expect("expanded test stack thread should spawn")
            .join()
            .expect("expanded test stack thread should complete");
    }

    fn task_record_for_progress(
        task_id: &str,
        status: &str,
        issue_type: &str,
        parent_id: Option<&str>,
    ) -> state_store::TaskRecord {
        state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority: 1,
            issue_type: issue_type.to_string(),
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            updated_at: "0".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: Default::default(),
            planner_metadata: Default::default(),
            provider_mapping: None,
            dependencies: parent_id
                .map(|parent_id| {
                    vec![state_store::TaskDependencyRecord {
                        issue_id: task_id.to_string(),
                        depends_on_id: parent_id.to_string(),
                        edge_type: "parent-child".to_string(),
                        created_at: "0".to_string(),
                        created_by: "test".to_string(),
                        metadata: "{}".to_string(),
                        thread_id: String::new(),
                    }]
                })
                .unwrap_or_default(),
        }
    }

    #[test]
    fn close_epic_progress_excludes_execution_steps_from_child_report() {
        let rows = vec![
            task_record_for_progress("epic", "open", "epic", None),
            task_record_for_progress("delivery-task", "closed", "task", Some("epic")),
            task_record_for_progress("step-child", "open", "step", Some("epic")),
            task_record_for_progress("todo-child", "open", "todo", Some("epic")),
        ];

        let summary = task_close_epic_progress_summary(&rows, "delivery-task", true)
            .expect("epic progress should summarize");

        assert_eq!(summary.epics.len(), 1);
        let epic = &summary.epics[0];
        assert_eq!(epic.total_count, 1);
        assert_eq!(epic.closed_count, 1);
        assert_eq!(epic.child_task_count, 1);
        assert_eq!(epic.reported_child_task_count, 1);
        assert_eq!(epic.tasks[0].task_id, "delivery-task");
    }

    fn task_bulk_import_command_for_test(
        file: std::path::PathBuf,
        state_dir: std::path::PathBuf,
    ) -> crate::TaskBulkImportArgs {
        crate::TaskBulkImportArgs {
            file,
            format: crate::TaskImportFormatArg::Auto,
            parent_id: None,
            issue_type: "task".to_string(),
            status: "open".to_string(),
            priority: 2,
            labels: Vec::new(),
            execution_mode: None,
            order_bucket: None,
            parallel_group: None,
            conflict_domain: None,
            owned_paths: Vec::new(),
            acceptance_targets: Vec::new(),
            proof_targets: Vec::new(),
            dry_run: false,
            created_by: "vida task import".to_string(),
            state_dir: Some(state_dir),
            render: crate::RenderMode::Plain,
            json: true,
        }
    }

    #[test]
    fn task_bulk_import_accepts_jsonl_fixture_with_metadata_and_parent_first_ordering() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let source = harness.path().join("tasks.jsonl");
        let state_dir = harness.path().join("bulk-state");
        fs::write(
            &source,
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "id": "bulk-child",
                    "title": "Bulk child",
                    "type": "task",
                    "parent_id": "bulk-parent"
                }),
                serde_json::json!({
                    "id": "bulk-parent",
                    "title": "Bulk parent",
                    "type": "epic",
                    "labels": ["file-label"],
                    "notes": "created from fixture",
                    "execution_semantics": {
                        "execution_mode": "parallel_safe",
                        "order_bucket": "wave-a",
                        "parallel_group": "operator-cli",
                        "conflict_domain": "task-import"
                    },
                    "planner_metadata": {
                        "owned_paths": ["crates/vida/src/task_surface.rs"],
                        "acceptance_targets": ["bulk import works"],
                        "proof_targets": ["cargo test -p vida task_bulk_import"]
                    }
                })
            ),
        )
        .expect("jsonl fixture should write");

        let mut command = task_bulk_import_command_for_test(source.clone(), state_dir.clone());
        command.labels = vec!["global-label".to_string()];
        let parsed =
            super::parse_task_bulk_import_input(&command).expect("jsonl fixture should parse");
        let plan = super::task_bulk_import_build_plan(
            command.file.display().to_string(),
            parsed.input_format,
            false,
            parsed.requested_count,
            parsed.tasks,
            parsed.validation_errors,
            &[],
            &command.created_by,
            ".",
        );

        assert!(!super::task_bulk_import_result_is_blocked(&plan.result));
        assert_eq!(plan.result.planned_count, 2);
        assert_eq!(
            plan.result.planned_task_ids,
            vec!["bulk-child".to_string(), "bulk-parent".to_string()]
        );
        let order = super::task_bulk_import_apply_order(&[], &plan.tasks)
            .expect("batch should order parent first");
        let ordered_ids = order
            .into_iter()
            .map(|index| plan.tasks[index].task_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ordered_ids, vec!["bulk-parent", "bulk-child"]);
        let parent = plan
            .tasks
            .iter()
            .find(|task| task.task_id == "bulk-parent")
            .expect("parent task should be planned");
        assert_eq!(parent.notes.as_deref(), Some("created from fixture"));
        assert_eq!(
            parent.labels,
            vec!["file-label".to_string(), "global-label".to_string()]
        );
        assert_eq!(
            parent.execution_semantics.execution_mode.as_deref(),
            Some("parallel_safe")
        );
        assert_eq!(
            parent.planner_metadata.acceptance_targets,
            vec!["bulk import works".to_string()]
        );
    }

    #[test]
    fn task_bulk_import_rejects_jsonl_batches_above_task_count_limit() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let source = harness.path().join("too-many.jsonl");
        let mut text = String::new();
        for index in 0..=super::TASK_BULK_IMPORT_MAX_TASKS {
            text.push_str(
                &serde_json::json!({
                    "id": format!("bulk-{index}"),
                    "title": "Bulk task",
                    "type": "epic"
                })
                .to_string(),
            );
            text.push('\n');
        }
        fs::write(&source, text).expect("oversized jsonl fixture should write");

        let command = task_bulk_import_command_for_test(source, harness.path().to_path_buf());
        let error = super::parse_task_bulk_import_input(&command)
            .expect_err("jsonl batch above task count limit should fail before validation");

        assert!(error.contains("more than 10000 tasks"));
    }

    #[test]
    fn task_bulk_import_apply_order_handles_large_reversed_chains() {
        let mut tasks = Vec::new();
        for index in (0..512).rev() {
            tasks.push(super::TaskBulkImportPlannedTask {
                index,
                line: None,
                task_id: format!("chain-{index}"),
                title: format!("Chain task {index}"),
                display_id: None,
                description: String::new(),
                issue_type: "epic".to_string(),
                status: "open".to_string(),
                priority: 2,
                parent_id: (index > 0).then(|| format!("chain-{}", index - 1)),
                notes: None,
                labels: Vec::new(),
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            });
        }

        let order = super::task_bulk_import_apply_order(&[], &tasks)
            .expect("reversed chain should order parent-first");
        let ordered_ids = order
            .into_iter()
            .map(|task_index| tasks[task_index].task_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ordered_ids.first().copied(), Some("chain-0"));
        assert_eq!(ordered_ids.last().copied(), Some("chain-511"));
        assert_eq!(ordered_ids.len(), 512);
    }

    #[test]
    fn task_bulk_import_dry_run_validates_yaml_without_mutating_store() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let source = harness.path().join("tasks.yaml");
        let state_dir = harness.path().join("bulk-state");
        fs::write(
            &source,
            "tasks:\n  - id: dry-run-task\n    title: Dry run task\n    type: epic\n    labels: dry-run,operator-dx\n",
        )
        .expect("yaml fixture should write");

        let mut command = task_bulk_import_command_for_test(source.clone(), state_dir.clone());
        command.dry_run = true;
        let parsed =
            super::parse_task_bulk_import_input(&command).expect("yaml fixture should parse");
        let plan = super::task_bulk_import_build_plan(
            command.file.display().to_string(),
            parsed.input_format,
            command.dry_run,
            parsed.requested_count,
            parsed.tasks,
            parsed.validation_errors,
            &[],
            &command.created_by,
            ".",
        );

        assert!(!super::task_bulk_import_result_is_blocked(&plan.result));
        assert!(plan.result.dry_run);
        assert!(!plan.result.applied);
        assert_eq!(plan.result.created_count, 0);
        assert_eq!(plan.result.planned_count, 1);
        assert_eq!(
            plan.result.planned_task_ids,
            vec!["dry-run-task".to_string()]
        );
    }

    #[test]
    fn task_bulk_import_validation_reports_clear_errors_before_mutation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let source = harness.path().join("invalid.json");
        fs::write(
            &source,
            serde_json::json!({
                "tasks": [
                    {
                        "id": "missing-parent",
                        "title": "Missing parent task",
                        "type": "task"
                    }
                ]
            })
            .to_string(),
        )
        .expect("invalid fixture should write");
        let command = task_bulk_import_command_for_test(source, harness.path().to_path_buf());
        let parsed = super::parse_task_bulk_import_input(&command)
            .expect("invalid semantic input should parse structurally");
        let plan = super::task_bulk_import_build_plan(
            command.file.display().to_string(),
            parsed.input_format,
            true,
            parsed.requested_count,
            parsed.tasks,
            parsed.validation_errors,
            &[],
            &command.created_by,
            ".",
        );

        assert_eq!(plan.result.validation_error_count, 1);
        assert_eq!(
            plan.result.validation_errors[0].field.as_deref(),
            Some("parent_id")
        );
        assert!(plan.result.validation_errors[0]
            .reason
            .contains("requires parent_id"));
        assert_eq!(plan.result.planned_count, 0);
        let payload = super::task_bulk_import_result_payload(&plan.result);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["schema_contract_missing"])
        );
    }

    #[test]
    fn direct_child_progress_marks_non_epic_parent_ready_when_children_are_closed() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "root-epic",
                "Root epic",
                "epic",
                "in_progress",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "parent-task",
                "Parent task",
                "task",
                "in_progress",
                2,
                Some("root-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "child-a",
                "Child A",
                "subtask",
                "closed",
                2,
                Some("parent-task"),
            )
            .await;
            create_task_for_test(
                &store,
                "child-b",
                "Child B",
                "subtask",
                "closed",
                2,
                Some("parent-task"),
            )
            .await;

            let rows = store.all_tasks().await.expect("tasks should read");
            let summary = task_progress_summary_for_basis(&rows, "parent-task", "direct_children")
                .expect("direct child progress should compute");

            assert_eq!(summary.direct_child_count, 2);
            assert_eq!(summary.closed_count, 2);
            assert_eq!(summary.open_count, 0);
            assert_eq!(summary.in_progress_count, 0);
            assert_eq!(summary.closure_candidate_state, "ready_to_close");
            assert!(summary.ready_for_close);
            assert_eq!(
                summary.next_required_command.as_deref(),
                Some("vida task close parent-task --reason \"direct children closed\"")
            );
        });

        runtime.shutdown_timeout(std::time::Duration::from_millis(250));
    }

    #[test]
    fn reconcile_epics_blocks_legacy_closed_direct_child_with_open_grandchild() {
        run_on_runtime_stack_for_test(|| {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
            runtime.block_on(async {
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
                let store = crate::StateStore::open(harness.path().to_path_buf())
                    .await
                    .expect("state store should open");

                create_task_for_test(
                    &store,
                    "legacy-epic",
                    "Legacy Epic",
                    "epic",
                    "open",
                    1,
                    None,
                )
                .await;
                create_task_for_test(
                    &store,
                    "legacy-child",
                    "Legacy Child",
                    "task",
                    "open",
                    2,
                    Some("legacy-epic"),
                )
                .await;
                create_task_for_test(
                    &store,
                    "legacy-grandchild",
                    "Legacy Grandchild",
                    "task",
                    "open",
                    3,
                    Some("legacy-child"),
                )
                .await;

                let mut child = store
                    .show_task("legacy-child")
                    .await
                    .expect("legacy child should exist");
                child.status = "closed".to_string();
                child.closed_at = Some("2026-03-08T00:10:00Z".to_string());
                child.close_reason =
                    Some("legacy fixture closed before graph validation".to_string());
                store
                    .persist_task_record(child)
                    .await
                    .expect("legacy fixture should persist directly");

                let rows = store.all_tasks().await.expect("tasks should read");
                let direct_summary =
                    task_progress_summary_for_basis(&rows, "legacy-epic", "direct_children")
                        .expect("direct child progress should compute");
                assert!(direct_summary.ready_for_close);

                let dry_run = reconcile_epics_from_descendant_progress(&store, false, true)
                    .await
                    .expect("reconcile should inspect legacy state");
                assert_eq!(dry_run.progress_basis, "descendants_excluding_root");
                assert!(dry_run
                    .closed_epics
                    .iter()
                    .all(|row| row.epic_id != "legacy-epic"));
                let blocked = dry_run
                    .blocked_epics
                    .iter()
                    .find(|row| row.epic_id == "legacy-epic")
                    .expect("legacy epic should be blocked by open grandchild");
                assert_eq!(blocked.reason, "active_descendants_remaining");
                assert_eq!(blocked.progress_basis, "descendants_excluding_root");
                assert_eq!(blocked.child_count, 1);
                assert_eq!(blocked.open_child_count, 0);
                assert_eq!(blocked.descendant_count, 2);
                assert_eq!(blocked.open_descendant_count, 1);

                let close_if_complete =
                    reconcile_epics_from_descendant_progress(&store, true, false)
                        .await
                        .expect("close-if-complete should still block legacy state");
                assert!(close_if_complete
                    .closed_epics
                    .iter()
                    .all(|row| row.epic_id != "legacy-epic"));
                let epic = store
                    .show_task("legacy-epic")
                    .await
                    .expect("legacy epic should remain readable");
                assert_eq!(epic.status, "open");
            });

            runtime.shutdown_timeout(std::time::Duration::from_millis(250));
        });
    }

    #[test]
    fn task_takeover_status_cli_accepts_json_task_and_run_filters() {
        let parsed = cli(&[
            "task",
            "takeover",
            "status",
            "--task-id",
            "task-1",
            "--run-id",
            "run-1",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Takeover(takeover) = args.command else {
            panic!("takeover command should parse");
        };
        let crate::TaskTakeoverCommand::Status(status) = takeover.command;

        assert_eq!(status.task_id_filter.as_deref(), Some("task-1"));
        assert_eq!(status.run_id.as_deref(), Some("run-1"));
        assert!(status.json);
    }

    #[test]
    fn task_takeover_status_cli_accepts_positional_task_id() {
        let parsed = cli(&["task", "takeover", "status", "task-1", "--json"]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Takeover(takeover) = args.command else {
            panic!("takeover command should parse");
        };
        let crate::TaskTakeoverCommand::Status(status) = takeover.command;

        assert_eq!(status.task_id.as_deref(), Some("task-1"));
        assert!(status.json);
    }

    #[test]
    fn task_takeover_status_labels_release_takeover_states() {
        assert_eq!(
            taskflow_core::task::takeover::exception_takeover_state_label(false, false),
            "not_recorded"
        );
        assert_eq!(
            taskflow_core::task::takeover::exception_takeover_state_label(false, true),
            "receipt_recorded"
        );
        assert_eq!(
            taskflow_core::task::takeover::exception_takeover_state_label(true, true),
            "active"
        );
    }

    #[test]
    fn task_takeover_status_reads_receipt_bound_owned_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let run_id = "task-takeover-status-scope";
        let metadata_path =
            task_exception_takeover_metadata_path(harness.path(), run_id).expect("metadata path");
        fs::create_dir_all(metadata_path.parent().expect("metadata dir should exist"))
            .expect("metadata dir should create");
        fs::write(
            &metadata_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "source_exception_path_receipt_id": "takeover-receipt",
                "reason_class": "test_exception_takeover",
                "active_bounded_unit": "task-takeover-status-scope",
                "owned_write_scope": [
                    " crates/vida/src/task_surface.rs "
                ],
                "why_delegated_or_rerouted_path_is_not_currently_lawful": "test delegated path blocked",
                "why_local_write_is_the_smallest_safe_bounded_workaround": "test bounded write scope",
                "return_to_normal_posture_condition": "test verification completes",
                "verification_plan": ["test"],
                "recorded_at": "2026-05-13T00:00:00Z"
            })
            .to_string(),
        )
        .expect("metadata should write");
        let summary = state_store::RunGraphDispatchReceiptSummary {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("takeover-receipt".to_string()),
            exception_path_receipt_id: Some("takeover-receipt".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("host_tool_bridge".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("host_tool_capability_missing".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("implementer".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-06-04T00:00:00Z".to_string(),
        };

        assert_eq!(
            task_exception_takeover_owned_write_scope(harness.path(), &summary),
            vec!["crates/vida/src/task_surface.rs".to_string()]
        );
    }

    #[test]
    fn task_takeover_status_blocks_active_takeover_without_receipt_bound_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record(
                "task-takeover-wide-scope",
                vec!["attacker-controlled/wide-scope"],
            );
            let status = state_store::RunGraphStatus {
                run_id: "task-takeover-no-metadata".to_string(),
                task_id: task.id.clone(),
                task_class: "implementation".to_string(),
                active_node: "implementer".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: "implementer".to_string(),
                lifecycle_stage: "implementer_blocked".to_string(),
                policy_gate: "blocked_open_delegated_cycle".to_string(),
                handoff_state: "bridge_request_pending".to_string(),
                context_state: "ready".to_string(),
                checkpoint_kind: "runtime_dispatch".to_string(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
            store
                .record_run_graph_status(&status)
                .await
                .expect("run graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&state_store::RunGraphDispatchReceipt {
                    run_id: status.run_id.clone(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "blocked".to_string(),
                    lane_status: "lane_exception_takeover".to_string(),
                    supersedes_receipt_id: Some("takeover-receipt".to_string()),
                    exception_path_receipt_id: Some("takeover-receipt".to_string()),
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("junior".to_string()),
                    activation_runtime_role: Some("implementer".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-04T00:00:00Z".to_string(),
                })
                .await
                .expect("dispatch receipt should persist");

            let receipt =
                task_takeover_status_receipt(&store, &task, Some(status), Some("run_id"), true)
                    .await;

            assert!(!receipt.allowed);
            assert!(!receipt.root_local_write_allowed);
            assert!(receipt.paths.is_empty());
            assert_eq!(
                receipt.root_write_guard["root_local_write_allowed_for_only_these_paths"],
                serde_json::json!([])
            );
            assert_eq!(
                receipt.blocker_codes,
                vec!["exception_takeover_scope_missing".to_string()]
            );
        });
    }

    #[test]
    fn task_takeover_status_resolves_task_scoped_active_exception_takeover() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record(
                "task-takeover-active-scope",
                vec!["crates/vida/src/task_surface.rs"],
            );
            create_task_for_test(
                &store,
                "task-takeover-active-parent",
                "Task takeover active parent",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                &task.id,
                &task.title,
                &task.issue_type,
                &task.status,
                task.priority,
                Some("task-takeover-active-parent"),
            )
            .await;
            let run_id = "run-task-takeover-active-scope";
            let metadata_path = task_exception_takeover_metadata_path(harness.path(), run_id)
                .expect("metadata path");
            fs::create_dir_all(metadata_path.parent().expect("metadata dir should exist"))
                .expect("metadata dir should create");
            fs::write(
                &metadata_path,
                serde_json::json!({
                    "run_id": run_id,
                    "dispatch_target": "implementer",
                    "source_exception_path_receipt_id": "takeover-receipt",
                    "reason_class": "test_exception_takeover",
                    "active_bounded_unit": "task-takeover-active-scope",
                    "owned_write_scope": ["crates/vida/src/task_surface.rs"],
                    "why_delegated_or_rerouted_path_is_not_currently_lawful": "test delegated path blocked",
                    "why_local_write_is_the_smallest_safe_bounded_workaround": "test bounded write scope",
                    "return_to_normal_posture_condition": "test verification completes",
                    "verification_plan": ["test"],
                    "recorded_at": "2026-05-13T00:00:00Z"
                })
                .to_string(),
            )
            .expect("metadata should write");
            let status = state_store::RunGraphStatus {
                run_id: run_id.to_string(),
                task_id: task.id.clone(),
                task_class: "implementation".to_string(),
                active_node: "implementer".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: "implementer".to_string(),
                lifecycle_stage: "implementer_blocked".to_string(),
                policy_gate: "blocked_open_delegated_cycle".to_string(),
                handoff_state: "bridge_request_pending".to_string(),
                context_state: "ready".to_string(),
                checkpoint_kind: "runtime_dispatch".to_string(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
            store
                .record_run_graph_status(&status)
                .await
                .expect("run graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&state_store::RunGraphDispatchReceipt {
                    run_id: status.run_id.clone(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "blocked".to_string(),
                    lane_status: "lane_exception_takeover".to_string(),
                    supersedes_receipt_id: Some("takeover-receipt".to_string()),
                    exception_path_receipt_id: Some("takeover-receipt".to_string()),
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida lane exception-takeover".to_string()),
                    dispatch_command: Some("vida lane exception-takeover".to_string()),
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("junior".to_string()),
                    activation_runtime_role: Some("implementer".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-27T00:00:00Z".to_string(),
                })
                .await
                .expect("dispatch receipt should persist");

            let receipt =
                task_takeover_status_receipt(&store, &task, None, Some("task_id"), false).await;

            assert_eq!(receipt.status, task_json_success_status());
            assert!(receipt.allowed);
            assert!(receipt.root_local_write_allowed);
            assert_eq!(receipt.local_exception_takeover_state, "active");
            assert_eq!(receipt.lane["source"], "task_id");
            assert_eq!(receipt.lane["run_id"], run_id);
            assert_eq!(receipt.lane["task_id"], task.id);
            assert_eq!(
                receipt.root_write_guard["root_local_write_allowed_for_only_these_paths"],
                serde_json::json!(["crates/vida/src/task_surface.rs"])
            );
            assert!(receipt.recommended_command.is_none());
            assert!(receipt.blocker_codes.is_empty());
        });
    }

    #[test]
    fn task_takeover_status_blocks_completed_lane_exception_scope() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record(
                "task-takeover-completed-lane",
                vec!["crates/vida/src/task_surface.rs"],
            );
            let run_id = "task-takeover-completed-lane";
            let metadata_path = task_exception_takeover_metadata_path(harness.path(), run_id)
                .expect("metadata path");
            fs::create_dir_all(metadata_path.parent().expect("metadata dir should exist"))
                .expect("metadata dir should create");
            fs::write(
                &metadata_path,
                serde_json::json!({
                    "run_id": run_id,
                    "dispatch_target": "implementer",
                    "source_exception_path_receipt_id": "takeover-receipt",
                    "reason_class": "test_exception_takeover",
                    "active_bounded_unit": "task-takeover-completed-lane",
                    "owned_write_scope": ["crates/vida/src/task_surface.rs"],
                    "why_delegated_or_rerouted_path_is_not_currently_lawful": "test delegated path blocked",
                    "why_local_write_is_the_smallest_safe_bounded_workaround": "test bounded write scope",
                    "return_to_normal_posture_condition": "test verification completes",
                    "verification_plan": ["test"],
                    "recorded_at": "2026-05-13T00:00:00Z"
                })
                .to_string(),
            )
            .expect("metadata should write");
            let status = state_store::RunGraphStatus {
                run_id: run_id.to_string(),
                task_id: task.id.clone(),
                task_class: "implementation".to_string(),
                active_node: "closure".to_string(),
                next_node: None,
                status: "completed".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: "implementer".to_string(),
                lifecycle_stage: "closure_complete".to_string(),
                policy_gate: "not_required".to_string(),
                handoff_state: "none".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: "none".to_string(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
            store
                .record_run_graph_status(&status)
                .await
                .expect("run graph status should persist");
            store
                .record_run_graph_dispatch_receipt(&state_store::RunGraphDispatchReceipt {
                    run_id: status.run_id.clone(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "executed".to_string(),
                    lane_status: "lane_completed".to_string(),
                    supersedes_receipt_id: Some("takeover-receipt".to_string()),
                    exception_path_receipt_id: Some("takeover-receipt".to_string()),
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: None,
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("junior".to_string()),
                    activation_runtime_role: Some("implementer".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-23T00:00:00Z".to_string(),
                })
                .await
                .expect("dispatch receipt should persist");

            let receipt =
                task_takeover_status_receipt(&store, &task, Some(status), Some("run_id"), true)
                    .await;

            assert_eq!(receipt.status, "blocked");
            assert_eq!(
                receipt.local_exception_takeover_state,
                "admissible_not_active"
            );
            assert_eq!(
                receipt.reason,
                "exception takeover is not active for this task"
            );
            assert_eq!(
                receipt.blocker_codes,
                vec!["exception_takeover_not_active".to_string()]
            );
            assert!(!receipt.allowed);
            assert!(!receipt.root_local_write_allowed);
            assert!(receipt.paths.is_empty());
            assert_eq!(receipt.lane["lane_status"], "lane_completed");
            assert_eq!(
                receipt.root_write_guard["root_local_write_allowed_for_only_these_paths"],
                serde_json::json!([])
            );
        });
    }

    #[test]
    fn task_takeover_status_for_requested_task_does_not_fall_back_to_foreign_latest_lane() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let requested =
                owned_task_record("requested-task-without-lane", vec!["crates/vida/src"]);
            let foreign_status = state_store::RunGraphStatus {
                run_id: "foreign-latest-run".to_string(),
                task_id: "foreign-latest-task".to_string(),
                task_class: "implementation".to_string(),
                active_node: "implementer".to_string(),
                next_node: None,
                status: "blocked".to_string(),
                route_task_class: "implementation".to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: "implementer".to_string(),
                lifecycle_stage: "implementer_blocked".to_string(),
                policy_gate: "blocked_open_delegated_cycle".to_string(),
                handoff_state: "bridge_request_pending".to_string(),
                context_state: "ready".to_string(),
                checkpoint_kind: "runtime_dispatch".to_string(),
                resume_target: "none".to_string(),
                recovery_ready: false,
            };
            store
                .record_run_graph_status(&foreign_status)
                .await
                .expect("foreign run graph status should persist");
            let scoped_status = store
                .latest_run_graph_status_for_task(&requested.id)
                .await
                .expect("task-scoped status lookup should succeed");
            assert!(scoped_status.is_none());

            let receipt = task_takeover_status_receipt(
                &store,
                &requested,
                scoped_status,
                Some("task_id"),
                false,
            )
            .await;

            assert_eq!(receipt.status, "blocked");
            assert_eq!(receipt.lane["source"], "task_id");
            assert_eq!(receipt.lane["run_id"], serde_json::Value::Null);
            assert_eq!(
                receipt.blocker_codes,
                vec!["missing_lane_receipt".to_string()]
            );
            assert!(!receipt
                .blocker_codes
                .contains(&"latest_lane_task_mismatch".to_string()));
            assert_ne!(receipt.lane["task_id"], "foreign-latest-task");
        });
    }

    #[test]
    fn takeover_status_blocked_json_operator_contract() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record(
                "requested-task-without-lane-contract",
                vec!["crates/vida/src/task_surface.rs"],
            );

            let receipt =
                task_takeover_status_receipt(&store, &task, None, Some("task_id"), false).await;

            assert_eq!(receipt.status, "blocked");
            assert_eq!(receipt.blocker_codes, vec!["missing_lane_receipt"]);
            assert_eq!(
                receipt.artifact_refs["surface"],
                "vida task takeover status"
            );
            assert_eq!(
                receipt.artifact_refs["task_id"],
                "requested-task-without-lane-contract"
            );
            assert_eq!(receipt.artifact_refs["run_id"], serde_json::Value::Null);
            assert_eq!(receipt.artifact_refs["lane_source"], "task_id");
            assert_eq!(receipt.artifact_refs["root_local_write_allowed"], false);
            assert_eq!(
                receipt.artifact_refs["local_exception_takeover_state"],
                "not_recorded"
            );
            assert_eq!(receipt.shared_fields["status"], receipt.status);
            assert_eq!(
                receipt.shared_fields["artifact_refs"],
                receipt.operator_contracts["artifact_refs"]
            );
            assert_eq!(
                receipt.operator_contracts["contract_id"],
                "release-1-operator-contracts"
            );
            assert_eq!(receipt.operator_contracts["status"], "blocked");
            assert_eq!(
                receipt.operator_contracts["blocker_codes"],
                serde_json::json!(["missing_lane_receipt"])
            );
            assert_eq!(
                receipt.operator_contracts["next_actions"],
                serde_json::json!(receipt.next_actions.clone())
            );
        });
    }

    #[test]
    fn task_takeover_status_default_output_includes_blocker_codes() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let task = owned_task_record("requested-task-without-lane", vec!["crates/vida/src"]);
            let receipt =
                task_takeover_status_receipt(&store, &task, None, Some("task_id"), false).await;
            let lines = task_takeover_status_default_lines(&receipt);

            assert!(lines.iter().any(|(label, value)| *label == "blocker_codes"
                && value.contains("missing_lane_receipt")));
            assert!(lines.iter().any(|(label, value)| *label == "next action"
                && value.contains("vida lane show")
                && value.contains("--latest")));
            assert!(!lines
                .iter()
                .any(|(_, value)| value.contains("vida lane show requested-task-without-lane")));
        });
    }

    #[test]
    fn task_block_cli_accepts_reason_evidence_and_repeated_recovery_fields() {
        let parsed = cli(&[
            "task",
            "block",
            "task-1",
            "--reason",
            "runtime bridge unavailable",
            "--evidence",
            "agent-init receipt path",
            "--evidence",
            "dispatch result path",
            "--blocker",
            "host_tool_capability_missing,bridge_request_pending",
            "--next-action",
            "run host bridge repair",
            "--next-action",
            "retry agent-init",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Block(command) = args.command else {
            panic!("block command should parse");
        };

        assert_eq!(command.task_id, "task-1");
        assert_eq!(command.reason, "runtime bridge unavailable");
        assert_eq!(
            command.evidence,
            vec![
                "agent-init receipt path".to_string(),
                "dispatch result path".to_string()
            ]
        );
        assert_eq!(
            command.blockers,
            vec![
                "host_tool_capability_missing".to_string(),
                "bridge_request_pending".to_string()
            ]
        );
        assert_eq!(
            command.next_actions,
            vec![
                "run host bridge repair".to_string(),
                "retry agent-init".to_string()
            ]
        );
        assert!(command.json);
    }

    #[test]
    fn task_verify_cli_accepts_partial_proof_flags() {
        let parsed = cli(&[
            "task",
            "verify",
            "task-1",
            "--source-fixed",
            "--tests-green",
            "--proof-blocked",
            "--proof-blocker",
            "browser proof unavailable",
            "--evidence",
            "cargo test -p vida task_verify",
            "--evidence",
            "target/debug/vida task verify smoke",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Verify(command) = args.command else {
            panic!("verify command should parse");
        };

        assert_eq!(command.task_id, "task-1");
        assert!(command.source_fixed);
        assert!(command.tests_green);
        assert!(command.proof_blocked);
        assert_eq!(
            command.proof_blocker.as_deref(),
            Some("browser proof unavailable")
        );
        assert_eq!(command.evidence.len(), 2);
        assert!(command.json);
    }

    #[test]
    fn task_verify_command_records_partial_state_without_closing() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "verify-task",
                "Verify task",
                "task",
                "in_progress",
                2,
                Some("parent-epic"),
            )
            .await;
        }));

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Verify(crate::TaskVerifyArgs {
                    task_id: "verify-task".to_string(),
                    source_fixed: true,
                    tests_green: true,
                    proof_blocked: true,
                    proof_blocker: Some("browser proof unavailable".to_string()),
                    evidence: vec!["cargo test -p vida task_verify".to_string()],
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::SUCCESS
        );

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("verify-task")
                .await
                .expect("verify task should load");
            assert_eq!(task.status, "in_progress");
            assert_eq!(task.closed_at, None);
            assert_eq!(task.close_reason, None);
            assert!(task.labels.contains(&"source-fixed".to_string()));
            assert!(task.labels.contains(&"tests-green".to_string()));
            assert!(task
                .labels
                .contains(&"proof-blocked-by-runtime".to_string()));
            assert!(!task.labels.contains(&"runtime-proof-blocked".to_string()));
            assert_eq!(
                task.planner_metadata.proof_targets,
                vec!["cargo test -p vida task_verify".to_string()]
            );
            let notes = task
                .notes
                .expect("partial verification note should persist");
            assert!(notes.contains("task_partial_verification:"));
            assert!(notes.contains("source_fixed: true"));
            assert!(notes.contains("tests_green: true"));
            assert!(notes.contains("proof_blocked: true"));
            assert!(notes.contains("browser proof unavailable"));

            let progress = store
                .task_progress_summary("verify-task")
                .await
                .expect("progress should compute");
            assert!(progress.proof_blocked_by_runtime);
            assert!(progress.blocked_by_runtime);
        }));
    }

    #[test]
    fn task_verify_command_rejects_closed_task_without_mutation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "closed-verify-task",
                "Closed verify task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("closed-verify-task", "done")
                .await
                .expect("task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Verify(crate::TaskVerifyArgs {
                    task_id: "closed-verify-task".to_string(),
                    source_fixed: true,
                    tests_green: true,
                    proof_blocked: true,
                    proof_blocker: Some("browser proof unavailable".to_string()),
                    evidence: Vec::new(),
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("closed-verify-task")
                .await
                .expect("closed verify task should load");
            assert_eq!(task.status, "closed");
            assert_eq!(task.close_reason.as_deref(), Some("done"));
            assert!(task.labels.is_empty());
            assert!(task.notes.is_none());
        });
    }

    #[test]
    fn task_block_command_marks_task_blocked_without_closing() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "blocked-task",
                "Blocked task",
                "task",
                "in_progress",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .update_task(crate::state_store::UpdateTaskRequest {
                    task_id: "blocked-task",
                    title: None,
                    status: None,
                    priority: None,
                    notes: Some("existing note"),
                    description: None,
                    parent_id: None,
                    add_labels: &[],
                    remove_labels: &[],
                    set_labels: None,
                    execution_mode: None,
                    order_bucket: None,
                    parallel_group: None,
                    conflict_domain: None,
                    planner_metadata: None,
                })
                .await
                .expect("notes update should persist");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Block(crate::TaskBlockArgs {
                    task_id: "blocked-task".to_string(),
                    reason: "runtime bridge unavailable".to_string(),
                    evidence: vec![
                        "agent-init returned host_tool_capability_missing".to_string(),
                        "dispatch result path".to_string(),
                    ],
                    blockers: vec![
                        "Host-Tool-Capability-Missing, bridge request pending".to_string()
                    ],
                    next_actions: vec!["retry after host bridge repair".to_string()],
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
            let task = store
                .show_task("blocked-task")
                .await
                .expect("blocked task should load");
            assert_eq!(task.status, "blocked");
            assert_eq!(task.closed_at, None);
            assert_eq!(task.close_reason, None);
            let notes = task.notes.expect("block note should persist");
            assert!(notes.contains("existing note"));
            assert!(notes.contains("task_block:"));
            assert!(notes.contains("runtime bridge unavailable"));
            assert!(notes.contains("agent-init returned host_tool_capability_missing"));
            assert!(notes.contains("dispatch result path"));
            assert!(notes.contains("host_tool_capability_missing"));
            assert!(notes.contains("bridge_request_pending"));
            assert!(!notes.contains("Host-Tool-Capability-Missing"));
            assert!(!notes.contains("bridge request pending"));
            assert!(notes.contains("retry after host bridge repair"));
        });
    }

    #[test]
    fn task_block_command_rejects_closed_task_without_mutation() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "closed-task",
                "Closed task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("closed-task", "done")
                .await
                .expect("task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Block(crate::TaskBlockArgs {
                    task_id: "closed-task".to_string(),
                    reason: "runtime bridge unavailable".to_string(),
                    evidence: vec!["receipt path".to_string()],
                    blockers: Vec::new(),
                    next_actions: Vec::new(),
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("closed-task")
                .await
                .expect("closed task should load");
            assert_eq!(task.status, "closed");
            assert_eq!(task.close_reason.as_deref(), Some("done"));
            assert!(task.notes.is_none());
        });
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
            notes: None,
            notes_file: None,
            labels: Vec::new(),
            execution_mode: None,
            order_bucket: None,
            parallel_group: None,
            conflict_domain: None,
            owned_paths: Vec::new(),
            owned_path_literals: Vec::new(),
            acceptance_targets: Vec::new(),
            acceptance_target_literals: Vec::new(),
            proof_targets: Vec::new(),
            proof_target_literals: Vec::new(),
            release_proof_template: false,
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
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn task_stage_ensemble_next_command_selects_leaf_guidance_for_container_tasks() {
        let mut task = owned_task_record("epic-runtime", Vec::new());
        task.issue_type = "epic".to_string();

        let command = super::task_stage_ensemble_next_command(&task, None, None, None, true);

        assert_eq!(command, "vida task ready --scope epic-runtime --limit 10");
    }

    #[test]
    fn task_stage_ensemble_next_command_keeps_attempt_dispatch_for_leaf_tasks() {
        let task = owned_task_record("leaf-task", Vec::new());

        let command = super::task_stage_ensemble_next_command(&task, None, None, None, true);

        assert_eq!(
            command,
            "vida task attempt dispatch leaf-task --stage implementation"
        );
    }

    #[test]
    fn task_stage_ensemble_uses_stage_summary_receipt_for_status_guidance() {
        let task = owned_task_record("leaf-task", Vec::new());
        let attempt = crate::state_store::TaskAttemptRecord {
            attempt_id: "attempt-b".to_string(),
            task_id: "leaf-task".to_string(),
            stage_id: "implementation".to_string(),
            backend: "internal".to_string(),
            model_profile: "medium".to_string(),
            isolation: "readonly".to_string(),
            freshness: task.updated_at.clone(),
            status: "accepted".to_string(),
            artifact_refs: vec!["artifact-b".to_string()],
            consolidation_receipt_id: None,
            selected_model_profile_readiness_status: Some("ready".to_string()),
            budget_posture: None,
            cap_posture: None,
            write_scope_classification: None,
            created_at: "2026-06-05T00:01:00Z".to_string(),
            updated_at: "2026-06-05T00:01:00Z".to_string(),
        };
        let stage_summary = crate::state_store::TaskStageSummary {
            task_id: "leaf-task".to_string(),
            stage_id: "implementation".to_string(),
            stage_status: Some("accepted".to_string()),
            attempt_count: 1,
            status_counts: [("accepted".to_string(), 1)].into_iter().collect(),
            latest_attempt_id: Some("attempt-b".to_string()),
            latest_attempt_status: Some("accepted".to_string()),
            latest_consolidation_receipt_id: Some("receipt-b".to_string()),
            artifact_refs: vec!["artifact-b".to_string()],
        };

        let summary =
            super::task_stage_ensemble_operator_summary_value(&task, &[attempt], &[stage_summary]);

        assert_eq!(
            summary["latest_consolidation_receipt_id"].as_str(),
            Some("receipt-b")
        );
        assert_eq!(
            summary["next_command"].as_str(),
            Some("vida task stage status leaf-task --stage implementation")
        );
    }

    #[test]
    fn task_attempt_binding_error_kind_classifies_closed_stale_and_container_bindings() {
        assert_eq!(
            super::task_attempt_binding_error_kind(
                "task attempt binding is stale because task `ldr-041` is closed"
            ),
            "closed_task_binding"
        );
        assert_eq!(
            super::task_attempt_binding_error_kind(
                "stale_task_binding: attempt `a` freshness `old` does not match task `t` updated_at `new`"
            ),
            "stale_task_binding"
        );
        assert_eq!(
            super::task_attempt_binding_error_kind(
                "task attempt binding requires a leaf task, got `epic-a` of type `epic`"
            ),
            "container_task_binding"
        );
    }

    #[test]
    fn task_attempt_binding_next_action_is_specific_for_closed_task_binding() {
        let next_action = super::task_attempt_binding_next_action("closed_task_binding");

        assert!(next_action.contains("already closed"));
        assert!(next_action.contains("vida task progress <task-id>"));
    }

    #[test]
    fn task_closeout_blocker_codes_preserve_graph_and_temp_blockers() {
        let proof = serde_json::json!({
            "configured_proof_target_count": 1,
            "missing_count": 0,
        });
        let closure = serde_json::json!({
            "ready_for_close": false,
            "closure_candidate_state": "already_closed",
        });
        let temp_scan = super::TaskCloseoutTempScan {
            enabled: true,
            status: "blocked".to_string(),
            tracked_match_count: 1,
            tracked_matches: vec!["tmp-proof.txt".to_string()],
            command: "git -C repo ls-files tmp* false true null undefined nul".to_string(),
            repo_root: Some("repo".to_string()),
            error: None,
        };

        let blockers = super::task_closeout_blocker_codes(&proof, &closure, 1, &temp_scan);

        assert!(blockers.contains(&"closeout_task_graph_invalid".to_string()));
        assert!(blockers.contains(&"closeout_tracked_temp_artifacts".to_string()));
        assert!(!blockers.contains(&"closeout_closure_not_ready".to_string()));
    }

    #[test]
    fn task_update_planner_metadata_sets_requested_lists_and_preserves_existing_fields() {
        let existing = crate::state_store::TaskPlannerMetadata {
            owned_paths: vec!["old/path.rs".to_string()],
            acceptance_targets: vec!["old acceptance".to_string()],
            proof_targets: vec!["old proof".to_string()],
            risk: Some("high".to_string()),
            estimate: Some("small".to_string()),
            lane_hint: Some("worker".to_string()),
        };
        let command = crate::TaskUpdateArgs {
            task_id: "task-owned".to_string(),
            owned_paths: vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "crates/vida/src/cli.rs".to_string(),
            ],
            proof_targets: vec![
                "cargo test -p vida task_update_planner_metadata proof_target_values".to_string(),
            ],
            ..Default::default()
        };

        let metadata = task_update_planner_metadata_arg(&existing, &command)
            .expect("metadata update should pass")
            .expect("metadata update should be requested");

        assert_eq!(
            metadata.owned_paths,
            vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "crates/vida/src/cli.rs".to_string(),
            ]
        );
        assert_eq!(metadata.acceptance_targets, existing.acceptance_targets);
        assert_eq!(
            metadata.proof_targets,
            vec![
                "cargo test -p vida task_update_planner_metadata".to_string(),
                "cargo test -p vida proof_target_values".to_string(),
            ]
        );
        assert_eq!(metadata.risk, existing.risk);
        assert_eq!(metadata.estimate, existing.estimate);
        assert_eq!(metadata.lane_hint, existing.lane_hint);
    }

    #[test]
    fn task_update_proof_target_replacement_contract() {
        let existing = crate::state_store::TaskPlannerMetadata {
            proof_targets: vec![
                "cargo test -p vida --lib stale_contract -- --nocapture".to_string()
            ],
            risk: Some("medium".to_string()),
            ..Default::default()
        };
        let replace_command = crate::TaskUpdateArgs {
            task_id: "proof-target-task".to_string(),
            proof_targets: vec![
                "cargo test -p vida --bin vida current_contract -- --nocapture".to_string(),
            ],
            ..Default::default()
        };

        let metadata = task_update_planner_metadata_arg(&existing, &replace_command)
            .expect("proof target replacement should pass")
            .expect("proof target replacement should be requested");

        assert_eq!(
            metadata.proof_targets,
            vec!["cargo test -p vida --bin vida current_contract -- --nocapture".to_string()]
        );
        assert_eq!(metadata.risk, existing.risk);

        let after_stale_evidence = task_evidence_proof_planner_metadata(
            &metadata,
            "cargo test -p vida --lib stale_contract -- --nocapture",
        );
        assert_eq!(
            after_stale_evidence.proof_targets, metadata.proof_targets,
            "attaching historical evidence must not resurrect replaced proof targets"
        );

        let empty_existing = crate::state_store::TaskPlannerMetadata::default();
        let after_first_evidence =
            task_evidence_proof_planner_metadata(&empty_existing, "cargo test -p vida first");
        assert_eq!(
            after_first_evidence.proof_targets,
            vec!["cargo test -p vida first".to_string()],
            "attach-evidence still bootstraps proof targets for unconfigured tasks"
        );

        let clear_command = crate::TaskUpdateArgs {
            task_id: "proof-target-task".to_string(),
            clear_proof_targets: true,
            ..Default::default()
        };
        let metadata = task_update_planner_metadata_arg(&existing, &clear_command)
            .expect("proof target clear should pass")
            .expect("proof target clear should be requested");
        assert!(metadata.proof_targets.is_empty());

        let conflicting_command = crate::TaskUpdateArgs {
            task_id: "proof-target-task".to_string(),
            proof_targets: vec!["cargo test -p vida current_contract".to_string()],
            clear_proof_targets: true,
            ..Default::default()
        };
        let error = task_update_planner_metadata_arg(&existing, &conflicting_command)
            .expect_err("clear plus replacement should fail");
        assert!(error.contains("--clear-proof-targets"));
    }

    #[test]
    fn task_update_close_authority_payload_reports_blocker_code() {
        let payload = super::task_update_close_authority_payload("proof-child");

        assert_eq!(payload["surface"], "vida task update");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            state_store::StateStore::TASK_UPDATE_CLOSE_AUTHORITY_BLOCKER_CODE
        );
        assert_eq!(payload["task_id"], "proof-child");
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("next action")
            .contains("vida task close proof-child --reason <closure-evidence>"));
    }

    #[test]
    fn task_create_planner_metadata_normalizes_proof_targets() {
        let mut command = minimal_task_create_args(Some("Task"), None);
        command.proof_targets = vec![
            "vida diagnostics --json".to_string(),
            "vida docflow protocol-coverage-check --profile active-canon --format jsonl"
                .to_string(),
        ];

        let metadata = task_create_planner_metadata_arg(&command);

        assert_eq!(
            metadata.proof_targets,
            vec![
                "vida diagnostics post-commit --json",
                "vida docflow protocol-coverage-check --profile active-canon",
            ]
        );
    }

    #[test]
    fn task_create_release_proof_template_adds_standard_targets() {
        let mut command = minimal_task_create_args(Some("Runtime defect"), None);
        command.issue_type = "runtime_defect".to_string();
        command.proof_targets = vec![
            "cargo test -p vida --test task_smoke focused_release_template -- --nocapture"
                .to_string(),
        ];
        command.release_proof_template = true;

        let metadata = task_create_planner_metadata_arg(&command);

        assert!(metadata.proof_targets.contains(
            &"cargo test -p vida --test task_smoke focused_release_template -- --nocapture"
                .to_string()
        ));
        assert!(metadata
            .proof_targets
            .contains(&"cargo check -p vida --tests".to_string()));
        assert!(metadata
            .proof_targets
            .contains(&"vida release install --json".to_string()));
        assert!(metadata
            .proof_targets
            .contains(&"vida doctor --json".to_string()));
    }

    #[test]
    fn task_update_release_proof_template_preserves_existing_targets() {
        let existing = crate::state_store::TaskPlannerMetadata {
            proof_targets: vec!["cargo test -p vida existing_focus -- --nocapture".to_string()],
            ..Default::default()
        };
        let command = crate::TaskUpdateArgs {
            task_id: "runtime-defect".to_string(),
            proof_targets: vec!["cargo test -p vida new_focus -- --nocapture".to_string()],
            release_proof_template: true,
            ..Default::default()
        };

        let metadata = task_update_planner_metadata_arg(&existing, &command)
            .expect("release proof template update should pass")
            .expect("release proof template should request metadata");

        assert!(metadata
            .proof_targets
            .contains(&"cargo test -p vida existing_focus -- --nocapture".to_string()));
        assert!(metadata
            .proof_targets
            .contains(&"cargo test -p vida new_focus -- --nocapture".to_string()));
        assert!(metadata
            .proof_targets
            .contains(&"vida task validate-graph --json".to_string()));
        assert!(metadata
            .proof_targets
            .contains(&"vida release install --json".to_string()));

        let clear_conflict = crate::TaskUpdateArgs {
            task_id: "runtime-defect".to_string(),
            release_proof_template: true,
            clear_proof_targets: true,
            ..Default::default()
        };
        let error = task_update_planner_metadata_arg(&existing, &clear_conflict)
            .expect_err("release proof template should conflict with clear");
        assert!(error.contains("--release-proof-template"));
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
            ".".to_string(),
            None,
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "planner_metadata.owned_paths");
        assert_eq!(receipt.owned_files, vec!["crates/vida/src/task_surface.rs"]);
        assert_eq!(
            receipt.matched_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(
            receipt.stageable_files,
            vec!["crates/vida/src/task_surface.rs"]
        );
        assert_eq!(receipt.unowned_files, vec!["README.md"]);
        assert_eq!(receipt.unmatched_files, vec!["README.md"]);
        assert_eq!(receipt.unowned_paths, vec!["README.md"]);
        assert_eq!(receipt.confidence, "mixed");
        assert_eq!(receipt.blocker_codes, vec!["dirty_ownership_ambiguous"]);
    }

    #[test]
    fn ensure_existing_task_rejects_contract_mismatch() {
        let mut task = owned_task_record("task-ensure", vec![]);
        task.title = "Unexpected".to_string();
        task.status = "closed".to_string();
        task.issue_type = "bug".to_string();
        task.labels = vec!["other".to_string()];
        task.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: task.id.clone(),
            depends_on_id: "other-parent".to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        assert_eq!(task_parent_id(&task).as_deref(), Some("other-parent"));
        let reason = ensure_existing_task_mismatch_reason(
            &task,
            "Expected",
            None,
            "task",
            "open",
            Some("expected-parent"),
            &["tracked-pack".to_string()],
        )
        .expect("mismatch reason should exist");
        assert!(reason.contains("title mismatch"));
    }

    #[test]
    fn task_ensure_detects_requested_execution_semantics_backfill() {
        let existing = crate::state_store::TaskExecutionSemantics::default();
        let mut command = minimal_task_create_args(Some("Ensure semantics"), None);
        command.execution_mode = Some("parallel_safe".to_string());
        command.order_bucket = Some("feature-x".to_string());
        command.parallel_group = Some("dev-pack".to_string());
        command.conflict_domain = Some("task-ensure-semantics".to_string());

        assert!(task_create_semantics_requested(&command));
        assert!(task_create_semantics_mismatch(&existing, &command));
    }

    #[test]
    fn task_owned_status_fails_closed_without_ownership_source() {
        let receipt = task_owned_status_receipt(
            "task-owned",
            Vec::new(),
            Vec::new(),
            vec!["crates/vida/src/task_surface.rs".to_string()],
            ".".to_string(),
            None,
            None,
            None,
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.ownership_source, "missing");
        assert_eq!(receipt.blocker_codes, vec!["missing_owned_paths"]);
        assert!(receipt.stageable_files.is_empty());
    }

    #[test]
    fn task_close_epic_progress_summary_reports_epic_percentages_and_child_rows() {
        let mut epic = owned_task_record("epic-a", vec![]);
        epic.title = "Epic A".to_string();
        epic.issue_type = "epic".to_string();
        epic.status = "open".to_string();
        epic.priority = 1;

        let mut closed_child = owned_task_record("child-closed", vec![]);
        closed_child.title = "Closed child".to_string();
        closed_child.status = "closed".to_string();
        closed_child.priority = 1;
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: epic.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        let mut blocked_child = owned_task_record("child-blocked", vec![]);
        blocked_child.title = "Blocked child".to_string();
        blocked_child.status = "open".to_string();
        blocked_child.priority = 2;
        blocked_child.dependencies = vec![
            crate::state_store::TaskDependencyRecord {
                issue_id: blocked_child.id.clone(),
                depends_on_id: epic.id.clone(),
                edge_type: "parent-child".to_string(),
                created_at: "2026-06-02T00:00:00Z".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            },
            crate::state_store::TaskDependencyRecord {
                issue_id: blocked_child.id.clone(),
                depends_on_id: "blocker-task".to_string(),
                edge_type: "blocks".to_string(),
                created_at: "2026-06-02T00:00:00Z".to_string(),
                created_by: "test".to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            },
        ];

        let mut blocker = owned_task_record("blocker-task", vec![]);
        blocker.title = "Blocking task".to_string();
        blocker.status = "open".to_string();

        let mut unrelated_epic = owned_task_record("epic-unrelated", vec![]);
        unrelated_epic.issue_type = "epic".to_string();
        unrelated_epic.status = "open".to_string();

        let rows = vec![
            epic,
            closed_child.clone(),
            blocked_child,
            blocker,
            unrelated_epic,
        ];
        let summary = task_close_epic_progress_summary(&rows, &closed_child.id, false)
            .expect("epic progress summary should build from task graph rows");

        assert_eq!(summary.closed_task_id, "child-closed");
        assert_eq!(summary.epic_count, 1);
        assert_eq!(summary.omitted_epic_count, 1);
        assert_eq!(summary.scope, "closed_task_ancestor_epics");
        let epic_row = &summary.epics[0];
        assert_eq!(epic_row.epic_id, "epic-a");
        assert_eq!(epic_row.closed_count, 1);
        assert_eq!(epic_row.total_count, 2);
        assert_eq!(epic_row.percent_closed, 50.0);
        assert_eq!(epic_row.child_task_count, 2);
        assert_eq!(epic_row.reported_child_task_count, 2);
        let blocked_row = epic_row
            .tasks
            .iter()
            .find(|task| task.task_id == "child-blocked")
            .expect("blocked child should be reported");
        assert_eq!(blocked_row.blocker_state, "blocked");
        assert_eq!(blocked_row.blockers[0].task_id, "blocker-task");
        assert!(blocked_row.next_action.contains("Resolve blocking tasks"));
    }

    #[test]
    fn task_close_result_payload_includes_epic_progress_summary() {
        let mut epic = owned_task_record("epic-a", vec![]);
        epic.issue_type = "epic".to_string();
        let mut closed_child = owned_task_record("child-closed", vec![]);
        closed_child.status = "closed".to_string();
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: epic.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let summary =
            task_close_epic_progress_summary(&[epic, closed_child.clone()], "child-closed", false)
                .expect("summary should build");

        let payload = task_close_result_payload(
            &closed_child,
            &serde_json::json!({"status": "recorded"}),
            None,
            None,
            Some(&summary),
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], false);
        assert_eq!(payload["task"]["id"], "child-closed");
        assert_eq!(
            payload["epic_progress_summary"]["epics"][0]["epic_id"],
            "epic-a"
        );
        assert_eq!(
            payload["epic_progress_summary"]["epics"][0]["closed_count"],
            1
        );
    }

    #[test]
    fn task_proof_status_payload_reports_missing_and_satisfied_targets() {
        let mut task = owned_task_record("proof-task", vec![]);
        task.status = "closed".to_string();
        task.close_reason =
            Some("Proof: cargo test -p vida proof_status_payload passed.".to_string());
        task.planner_metadata.proof_targets = vec![
            "cargo test -p vida proof_status_payload".to_string(),
            "cargo build -p vida".to_string(),
        ];

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["task_id"], "proof-task");
        assert_eq!(payload["configured_proof_target_count"], 2);
        assert_eq!(payload["satisfied_count"], 0);
        assert_eq!(payload["missing_count"], 2);
        assert_eq!(payload["missing_proof"], true);
        assert_eq!(payload["proof_targets"][0]["status"], "missing_evidence");
        assert_eq!(
            payload["proof_targets"][0]["legacy_close_reason_match"],
            true
        );
        assert!(payload["proof_targets"][0]["evidence_detail"]
            .as_str()
            .expect("evidence detail should render")
            .contains("structured proof evidence is required"));
        assert_eq!(payload["proof_targets"][1]["status"], "missing_evidence");
        assert!(payload["proof_targets"][1]["next_action"]
            .as_str()
            .expect("closed missing target next action should render")
            .contains("already closed task"));
        assert!(payload["next_required_command"]
            .as_str()
            .expect("closed missing next required command should render")
            .contains("already closed task"));
        assert_eq!(
            payload["missing_targets"],
            serde_json::json!([
                "cargo test -p vida proof_status_payload",
                "cargo build -p vida"
            ])
        );

        task.notes = Some(super::append_task_proof_evidence_note(
            None,
            "cargo test -p vida proof_status_payload",
            Some("cargo test -p vida proof_status_payload"),
            "pass",
            "command",
            Some("artifacts/proof-status-payload.json"),
            &["test passed".to_string()],
        ));
        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_eq!(
            payload["proof_targets"][0]["legacy_close_reason_match"],
            true
        );
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "task_proof_evidence_registry"
        );
        assert_eq!(payload["missing_targets"][0], "cargo build -p vida");
    }

    #[test]
    fn task_close_structured_proof_gate_shell_quotes_missing_targets() {
        let mut task = owned_task_record("proof-task", vec![]);
        let target = "ok $(touch /tmp/vida-pwned) `touch /tmp/vida-pwned2`";
        task.planner_metadata.proof_targets = vec![target.to_string()];

        let payload = super::task_close_structured_proof_gate_payload(&task, None)
            .expect("missing structured proof should block close");
        let action = payload["next_actions"][0]
            .as_str()
            .expect("next action should render");

        assert!(
            action
                .contains("--proof-target 'ok $(touch /tmp/vida-pwned) `touch /tmp/vida-pwned2`'"),
            "rendered action should shell-quote proof target: {action}"
        );
        assert!(!action.contains("--proof-target \"ok $(touch"));
        assert_eq!(payload["missing_targets"][0], target);
    }

    #[test]
    fn task_proof_status_payload_inherits_single_closed_child_evidence() {
        let target = "cargo test -p vida inherited_parent_proof";
        let mut parent = owned_task_record("parent-proof-task", vec![]);
        parent.status = "closed".to_string();
        parent.planner_metadata.proof_targets = vec![target.to_string()];

        let mut child = owned_task_record("closed-child-proof-task", vec![]);
        child.status = "closed".to_string();
        child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-23T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        child.notes = Some(super::append_task_proof_evidence_note(
            None,
            target,
            Some(target),
            "pass",
            "command",
            Some("artifacts/child-proof.json"),
            &["child proof passed".to_string()],
        ));

        let rows = vec![parent.clone(), child];
        let payload = super::task_proof_status_payload_with_inheritance(&parent, None, Some(&rows));

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 0);
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "inherited_child_task_proof_evidence"
        );
        assert!(payload["proof_targets"][0]["evidence_detail"]
            .as_str()
            .expect("inherited detail should render")
            .contains("closed-child-proof-task"));
    }

    #[test]
    fn task_proof_status_payload_fails_closed_for_ambiguous_child_inheritance() {
        let target = "cargo test -p vida ambiguous_parent_proof";
        let mut parent = owned_task_record("ambiguous-parent-proof", vec![]);
        parent.status = "closed".to_string();
        parent.planner_metadata.proof_targets = vec![target.to_string()];

        let mut closed_child = owned_task_record("closed-child-proof", vec![]);
        closed_child.status = "closed".to_string();
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-23T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        closed_child.notes = Some(super::append_task_proof_evidence_note(
            None,
            target,
            Some(target),
            "pass",
            "command",
            Some("artifacts/closed-child-proof.json"),
            &["closed child proof passed".to_string()],
        ));

        let mut open_child = owned_task_record("open-child-proof", vec![]);
        open_child.status = "open".to_string();
        open_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: open_child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-23T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        let rows = vec![parent.clone(), closed_child, open_child];
        let payload = super::task_proof_status_payload_with_inheritance(&parent, None, Some(&rows));

        assert_eq!(payload["satisfied_count"], 0);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["proof_targets"][0]["status"], "missing_evidence");
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "planner_metadata.proof_targets"
        );
    }

    #[test]
    fn task_proof_attach_browser_cli_accepts_artifact_fields() {
        let parsed = cli(&[
            "task",
            "proof",
            "attach-browser",
            "proof-task",
            "--route",
            "/odoo",
            "--expect",
            "My Tasks",
            "--result",
            "pass",
            "--screenshot",
            "artifacts/proof.png",
            "--evidence",
            "console clean",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Proof(proof) = args.command else {
            panic!("task proof command should parse");
        };
        let crate::TaskProofCommand::AttachBrowser(command) = proof.command else {
            panic!("attach-browser command should parse");
        };

        assert_eq!(command.task_id, "proof-task");
        assert_eq!(command.route, "/odoo");
        assert_eq!(command.expect.as_deref(), Some("My Tasks"));
        assert_eq!(command.result, "pass");
        assert_eq!(command.screenshot.as_deref(), Some("artifacts/proof.png"));
        assert_eq!(command.evidence, vec!["console clean".to_string()]);
        assert!(command.json);
    }

    #[test]
    fn task_proof_attach_browser_receipt_serializes_versioned_artifact() {
        let artifact = TaskBrowserProofArtifact::new(
            "/odoo",
            "pass",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &["console clean".to_string()],
        )
        .expect("browser proof artifact should build");
        let receipt = TaskProofAttachBrowserReceipt {
            surface: "vida task proof attach-browser",
            status: task_json_success_status(),
            task_id: "proof-task".to_string(),
            route: "/odoo".to_string(),
            result: "pass".to_string(),
            expect: Some("My Tasks".to_string()),
            screenshot: Some("artifacts/proof.png".to_string()),
            evidence: artifact.evidence.clone(),
            proof_target: artifact.proof_target.clone(),
            artifact,
            notes_appended: true,
            task: owned_task_record("proof-task", vec![]),
        };

        let payload =
            serde_json::to_value(&receipt).expect("browser proof receipt should serialize");

        assert_eq!(
            payload["artifact"]["schema_version"],
            TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(
            payload["artifact"]["proof_target"],
            "vida proof browser --route /odoo --expect My Tasks"
        );
        assert_eq!(payload["artifact"]["result"], "pass");
        assert_eq!(payload["proof_target"], payload["artifact"]["proof_target"]);
    }

    #[test]
    fn task_proof_attach_evidence_cli_accepts_structured_fields() {
        let parsed = cli(&[
            "task",
            "proof",
            "attach-evidence",
            "proof-task",
            "--proof-target",
            "cargo test -p vida proof_registry",
            "--result",
            "pass",
            "--command",
            "cargo test -p vida proof_registry",
            "--artifact-ref",
            "artifacts/proof.json",
            "--evidence",
            "tests green",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Proof(proof) = args.command else {
            panic!("task proof command should parse");
        };
        let crate::TaskProofCommand::AttachEvidence(command) = proof.command else {
            panic!("attach-evidence command should parse");
        };

        assert_eq!(command.task_id, "proof-task");
        assert_eq!(
            command.proof_target,
            vec!["cargo test -p vida proof_registry".to_string()]
        );
        assert_eq!(command.result, "pass");
        assert_eq!(
            command.command.as_deref(),
            Some("cargo test -p vida proof_registry")
        );
        assert_eq!(
            command.artifact_ref,
            vec!["artifacts/proof.json".to_string()]
        );
        assert_eq!(command.evidence, vec!["tests green".to_string()]);
        assert!(command.json);
    }

    #[test]
    fn task_proof_attach_evidence_cli_accepts_repeated_targets_and_artifacts() {
        let parsed = cli(&[
            "task",
            "proof",
            "attach-evidence",
            "proof-task",
            "--proof-target",
            "cargo test -p vida proof_registry_a",
            "--proof-target",
            "cargo test -p vida proof_registry_b",
            "--artifact-ref",
            "artifacts/a.json",
            "--artifact-ref",
            "artifacts/b.json",
            "--result",
            "pass",
            "--evidence",
            "tests green",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Proof(proof) = args.command else {
            panic!("task proof command should parse");
        };
        let crate::TaskProofCommand::AttachEvidence(command) = proof.command else {
            panic!("attach-evidence command should parse");
        };

        assert_eq!(
            command.proof_target,
            vec![
                "cargo test -p vida proof_registry_a".to_string(),
                "cargo test -p vida proof_registry_b".to_string()
            ]
        );
        assert_eq!(
            command.artifact_ref,
            vec![
                "artifacts/a.json".to_string(),
                "artifacts/b.json".to_string()
            ]
        );
    }

    #[test]
    fn task_proof_attach_release_bundle_cli_accepts_bundle_fields() {
        let parsed = cli(&[
            "task",
            "proof",
            "attach-release-bundle",
            "proof-task",
            "--artifact-ref",
            "artifacts/release-proof.json",
            "--evidence",
            "release proof green",
            "--json",
        ]);
        let Some(crate::Command::Task(args)) = parsed.command else {
            panic!("task command should parse");
        };
        let crate::TaskCommand::Proof(proof) = args.command else {
            panic!("task proof command should parse");
        };
        let crate::TaskProofCommand::AttachReleaseBundle(command) = proof.command else {
            panic!("attach-release-bundle command should parse");
        };

        assert_eq!(command.task_id, "proof-task");
        assert_eq!(
            command.artifact_ref,
            vec!["artifacts/release-proof.json".to_string()]
        );
        assert_eq!(command.result, "pass");
        assert_eq!(command.evidence, vec!["release proof green".to_string()]);
        assert!(command.json);
    }

    #[test]
    fn task_proof_attach_evidence_bulk_notes_satisfy_targets() {
        let targets = vec![
            "cargo test -p vida bulk_proof_a".to_string(),
            "cargo test -p vida bulk_proof_b".to_string(),
        ];
        let mut task = owned_task_record("bulk-proof-task", vec![]);
        let mut notes = String::new();
        let mut planner_metadata = task.planner_metadata.clone();

        for proof_target in &targets {
            notes = super::append_task_proof_evidence_note(
                if notes.trim().is_empty() {
                    None
                } else {
                    Some(notes.as_str())
                },
                proof_target,
                Some(proof_target),
                "pass",
                "command",
                Some("artifacts/bulk-proof.json"),
                &["bulk proof passed".to_string()],
            );
            planner_metadata = task_browser_proof_planner_metadata(&planner_metadata, proof_target);
        }
        task.notes = Some(notes);
        task.planner_metadata = planner_metadata;

        assert_eq!(task.planner_metadata.proof_targets, targets);
        let payload = super::task_proof_status_payload(&task, None);
        assert_eq!(payload["satisfied_count"], 2);
        assert_eq!(payload["missing_count"], 0);
    }

    #[test]
    fn task_proof_attach_evidence_receipt_serializes_bulk_targets() {
        let receipt = TaskProofAttachEvidenceReceipt {
            surface: "vida task proof attach-evidence",
            status: task_json_success_status(),
            task_id: "proof-task".to_string(),
            proof_target: "target a | target b".to_string(),
            proof_targets: vec!["target a".to_string(), "target b".to_string()],
            command: "target a".to_string(),
            result: "pass".to_string(),
            artifact_ref: Some("artifacts/proof.json".to_string()),
            artifact_refs: vec![
                "artifacts/proof.json".to_string(),
                "artifacts/extra.json".to_string(),
            ],
            evidence: vec!["tests green".to_string()],
            notes_appended: true,
            task: owned_task_record("proof-task", vec![]),
        };

        let payload =
            serde_json::to_value(&receipt).expect("evidence proof receipt should serialize");

        assert_eq!(
            payload["proof_targets"],
            serde_json::json!(["target a", "target b"])
        );
        assert_eq!(payload["proof_target"], "target a | target b");
        assert_eq!(
            payload["artifact_refs"],
            serde_json::json!(["artifacts/proof.json", "artifacts/extra.json"])
        );
    }

    #[test]
    fn task_proof_status_payload_accepts_browser_attach_registry_note() {
        let mut task = owned_task_record("proof-task", vec![]);
        let artifact = TaskBrowserProofArtifact::new(
            "/odoo",
            "pass",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &["console clean".to_string()],
        )
        .expect("browser proof artifact should build");
        let proof_target = artifact.proof_target.clone();
        task.planner_metadata.proof_targets = vec![proof_target.clone()];
        let browser_notes = append_task_browser_proof_note(None, &artifact);
        task.notes = Some(super::append_task_proof_evidence_note(
            Some(&browser_notes),
            &proof_target,
            Some(&proof_target),
            "pass",
            "browser",
            Some("artifacts/proof.png"),
            &["console clean".to_string()],
        ));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 0);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_eq!(
            payload["proof_targets"][0]["evidence_source"],
            "task_proof_evidence_registry"
        );
        assert_eq!(
            payload["evidence_model"]["artifact_registry"],
            "task_notes.task_proof_evidence|task_notes.task_browser_proof"
        );
        assert_eq!(
            payload["evidence_model"]["browser_proof_artifact_schema"],
            TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(
            payload["evidence_model"]["browser_proof_note_schema"],
            TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION
        );
    }

    #[test]
    fn task_proof_status_payload_rejects_failed_browser_note_with_pass_text_in_evidence() {
        let mut task = owned_task_record("proof-task", vec![]);
        let artifact = TaskBrowserProofArtifact::new(
            "/secure",
            "fail",
            Some("OK"),
            Some("artifacts/proof.png"),
            &["console included result: pass text".to_string()],
        )
        .expect("browser proof artifact should build");
        task.planner_metadata.proof_targets = vec![artifact.proof_target.clone()];
        task.notes = Some(append_task_browser_proof_note(None, &artifact));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 0);
        assert_eq!(payload["missing_count"], 1);
        assert_ne!(payload["proof_targets"][0]["status"], "satisfied");
    }

    #[test]
    fn task_proof_status_payload_scopes_browser_pass_to_matching_target_record() {
        let mut task = owned_task_record("proof-task", vec![]);
        let other_artifact = TaskBrowserProofArtifact::new("/other", "pass", None, None, &[])
            .expect("browser proof artifact should build");
        let secure_artifact = TaskBrowserProofArtifact::new("/secure", "fail", None, None, &[])
            .expect("browser proof artifact should build");
        let other_target = task_browser_proof_target("/other", None);
        let secure_target = task_browser_proof_target("/secure", None);
        task.planner_metadata.proof_targets = vec![other_target.clone(), secure_target.clone()];
        let notes = append_task_browser_proof_note(None, &other_artifact);
        task.notes = Some(append_task_browser_proof_note(
            Some(&notes),
            &secure_artifact,
        ));

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 1);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["proof_targets"][0]["status"], "satisfied");
        assert_ne!(payload["proof_targets"][1]["status"], "satisfied");
        assert_eq!(payload["missing_targets"][0], secure_target);
    }

    #[test]
    fn append_task_browser_proof_note_normalizes_newlines_in_untrusted_fields() {
        let artifact = TaskBrowserProofArtifact::new(
            "/secure",
            "fail",
            Some("OK\n  result: pass"),
            Some("artifacts/proof.png\n  result: pass"),
            &["first line\n  result: pass".to_string()],
        )
        .expect("browser proof artifact should build");
        let note = append_task_browser_proof_note(None, &artifact);

        assert!(note.contains("  result: fail\n"));
        assert!(note.contains("  schema_version: task_browser_proof.v1\n"));
        assert!(!note.contains("\n  expect: OK\n  result: pass"));
        assert!(!note.contains("\n  screenshot: artifacts/proof.png\n  result: pass"));
        assert!(!note.contains("\n  evidence: first line\n  result: pass"));
    }

    #[test]
    fn task_proof_status_payload_rejects_unversioned_browser_note() {
        let mut task = owned_task_record("proof-task", vec![]);
        let proof_target = task_browser_proof_target("/odoo", Some("My Tasks"));
        task.planner_metadata.proof_targets = vec![proof_target.clone()];
        task.notes = Some(
            "task_browser_proof:\n  proof_target: vida proof browser --route /odoo --expect My Tasks\n  command: vida proof browser --route /odoo --expect My Tasks\n  route: /odoo\n  result: pass\n  screenshot: artifacts/proof.png"
                .to_string(),
        );

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["satisfied_count"], 0);
        assert_eq!(payload["missing_count"], 1);
        assert_eq!(payload["missing_targets"][0], proof_target);
    }

    #[test]
    fn task_proof_status_payload_reports_unconfigured_targets() {
        let task = owned_task_record("proofless-task", vec![]);

        let payload = super::task_proof_status_payload(&task, None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["configured_proof_target_count"], 0);
        assert_eq!(payload["missing_proof"], false);
        assert!(payload["next_required_command"]
            .as_str()
            .expect("next command should render")
            .contains("vida task update proofless-task --proof-target"));
    }

    #[test]
    fn task_proof_status_payload_quotes_unconfigured_task_id_command_hint() {
        let task = owned_task_record("safe; touch /tmp/vida_pwned #", vec![]);

        let payload = super::task_proof_status_payload(&task, None);
        let next_required_command = payload["next_required_command"]
            .as_str()
            .expect("next command should render");

        assert!(next_required_command
            .contains("vida task update 'safe; touch /tmp/vida_pwned #' --proof-target"));
        assert!(!next_required_command.contains("vida task update safe; touch"));
    }

    #[test]
    fn task_proof_status_payload_quotes_missing_proof_task_id_command_hint() {
        let mut task = owned_task_record("safe; touch /tmp/vida_pwned #", vec![]);
        task.status = "closed".to_string();
        task.planner_metadata.proof_targets = vec!["cargo test -p vida".to_string()];

        let payload = super::task_proof_status_payload(&task, None);
        let next_required_command = payload["next_required_command"]
            .as_str()
            .expect("next command should render");

        assert!(next_required_command.contains(
            "Structured proof evidence is missing on already closed task `safe; touch /tmp/vida_pwned #`"
        ));
        assert!(!next_required_command.contains("vida task proof status"));
    }

    #[test]
    fn task_close_result_payload_keeps_success_status_when_continuation_is_blocked() {
        let mut closed_task = owned_task_record("closed-with-blocker", vec![]);
        closed_task.status = "closed".to_string();
        let telemetry = serde_json::json!({
            "status": "recorded",
            "reason": "feedback recorded after close"
        });
        let blockers = (
            vec!["post_close_feedback_blocked".to_string()],
            vec!["Inspect continuation blocker separately.".to_string()],
        );

        let payload =
            task_close_result_payload(&closed_task, &telemetry, None, Some(&blockers), None);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], true);
        assert_eq!(payload["feedback_blocked"], true);
        assert_eq!(payload["automation_blocked"], false);
        assert_eq!(payload["blocker_codes"][0], "post_close_feedback_blocked");
    }

    #[test]
    fn task_close_result_payload_reports_blocked_status_for_blocked_automation() {
        let mut closed_task = owned_task_record("closed-with-automation-blocker", vec![]);
        closed_task.status = "closed".to_string();
        let telemetry = serde_json::json!({
            "status": "recorded",
            "reason": "feedback recorded after close"
        });
        let automation = TaskCloseAutomationReceipt {
            status: "blocked".to_string(),
            blocker_codes: vec!["push_requires_commit".to_string()],
            next_actions: vec!["Pass `--commit --commit-file <path>` with `--push`.".to_string()],
            release_build: None,
            release_install: None,
            git: None,
        };

        let payload =
            task_close_result_payload(&closed_task, &telemetry, Some(&automation), None, None);

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["closed"], true);
        assert_eq!(payload["continuation_blocked"], true);
        assert_eq!(payload["automation_blocked"], true);
        assert_eq!(payload["feedback_blocked"], false);
        assert_eq!(payload["blocker_codes"][0], "push_requires_commit");
        assert!(task_close_automation_is_blocked(Some(&automation)));
    }

    #[test]
    fn task_close_commit_allowlist_reports_ignored_dirty_files_diagnostically() {
        let next_actions = task_close_commit_allowlist_next_actions(&[
            "AGENTS.md".to_string(),
            "crates/vida/src/taskflow_proxy.rs".to_string(),
        ]);

        assert_eq!(
            next_actions,
            vec![
                "Ignored 2 unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied."
            ]
        );
        assert!(task_close_commit_allowlist_next_actions(&[]).is_empty());
    }

    #[test]
    fn task_close_commit_allowlist_ignores_unrelated_dirty_files() {
        let ignored = task_close_ignored_dirty_files_for_explicit_commit(
            vec![
                "crates/vida/src/task_surface.rs".to_string(),
                "AGENTS.md".to_string(),
                "docs/process/index.md".to_string(),
            ],
            &["crates/vida/src/task_surface.rs".to_string()],
        );

        assert_eq!(
            ignored,
            vec!["AGENTS.md".to_string(), "docs/process/index.md".to_string()]
        );
        assert_eq!(
            task_close_commit_allowlist_next_actions(&ignored),
            vec![
                "Ignored 2 unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied."
            ]
        );
    }

    #[test]
    fn task_close_commit_rejects_broad_explicit_pathspec() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let repo_root = std::env::temp_dir().join(format!(
            "vida-task-close-commit-broad-pathspec-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&repo_root).expect("create temp repo");
        let run_git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(&repo_root)
                .output()
                .expect("git command should run")
        };
        assert!(run_git(&["init"]).status.success());
        assert!(
            run_git(&["config", "user.email", "vida-test@example.invalid"])
                .status
                .success()
        );
        assert!(run_git(&["config", "user.name", "VIDA Test"])
            .status
            .success());
        std::fs::write(repo_root.join("secret.txt"), "old\n").expect("write secret");
        assert!(run_git(&["add", "."]).status.success());
        assert!(run_git(&["commit", "-m", "initial"]).status.success());
        std::fs::write(repo_root.join("secret.txt"), "new secret\n").expect("modify secret");

        let task = owned_task_record("task-owned", vec!["."]);
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: Some("done".to_string()),
                reason_file: None,
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
                stage_owned: true,
                commit_files: Vec::new(),
                commit_message: Some("reject broad pathspec".to_string()),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            Some(&repo_root),
            Some(&task),
        );

        assert_eq!(receipt.status, "blocked");
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "blocked");
        assert_eq!(git.blocker_codes, vec!["dirty_ownership_ambiguous"]);
        assert!(git.explicit_files.is_empty());
        let status = String::from_utf8(run_git(&["status", "--short"]).stdout)
            .expect("git status should be utf8");
        assert!(status.contains("secret.txt"));
        let _ = std::fs::remove_dir_all(&repo_root);
    }

    #[test]
    fn task_close_commit_with_explicit_file_ignores_unrelated_dirty_worktree() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let repo_root = std::env::temp_dir().join(format!(
            "vida-task-close-commit-allowlist-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(repo_root.join("src")).expect("create temp repo");
        let run_git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(&repo_root)
                .output()
                .expect("git command should run")
        };
        assert!(run_git(&["init"]).status.success());
        assert!(
            run_git(&["config", "user.email", "vida-test@example.invalid"])
                .status
                .success()
        );
        assert!(run_git(&["config", "user.name", "VIDA Test"])
            .status
            .success());
        std::fs::write(repo_root.join("src/owned.txt"), "old\n").expect("write owned");
        std::fs::write(repo_root.join("unrelated.txt"), "old\n").expect("write unrelated");
        assert!(run_git(&["add", "."]).status.success());
        assert!(run_git(&["commit", "-m", "initial"]).status.success());
        std::fs::write(repo_root.join("src/owned.txt"), "new\n").expect("modify owned");
        std::fs::write(repo_root.join("unrelated.txt"), "new\n").expect("modify unrelated");

        let task = owned_task_record("task-owned", vec!["src/owned.txt"]);
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: Some("done".to_string()),
                reason_file: None,
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
                stage_owned: false,
                commit_files: vec![std::path::PathBuf::from("src/owned.txt")],
                commit_message: Some("close explicit file".to_string()),
                state_dir: None,
                render: crate::RenderMode::Plain,
                json: true,
            },
            Some(&repo_root),
            Some(&task),
        );

        assert_eq!(receipt.status, "pass");
        let git = receipt.git.expect("git receipt should be present");
        assert_eq!(git.status, "pass");
        assert_eq!(git.blocker_codes, Vec::<String>::new());
        assert_eq!(
            git.next_actions,
            vec![
                "Ignored 1 unrelated dirty file(s) because explicit `--commit-file` allowlist was supplied."
            ]
        );
        let status = String::from_utf8(run_git(&["status", "--short"]).stdout)
            .expect("git status should be utf8");
        assert!(!status.contains("src/owned.txt"));
        assert!(status.contains("unrelated.txt"));
        let _ = std::fs::remove_dir_all(&repo_root);
    }

    #[test]
    fn task_close_git_stage_failure_classifies_read_only_or_sandbox_stderr() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: Unable to create '/repo/.git/index.lock': Read-only file system",
            None,
        );

        assert_eq!(
            failure.blocker_code,
            "git_stage_read_only_or_sandbox_blocked"
        );
        assert!(failure.detail.contains("Read-only file system"));
        assert!(failure.next_action.contains("writable"));
    }

    #[test]
    fn task_close_git_stage_failure_classifies_index_lock_stderr() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: Unable to create '/repo/.git/index.lock': File exists.",
            None,
        );

        assert_eq!(failure.blocker_code, "git_stage_index_lock_blocked");
        assert!(failure.detail.contains(".git/index.lock"));
        assert!(failure.next_action.contains(".git/index.lock"));
    }

    #[test]
    fn task_close_git_stage_failure_preserves_fallback_stderr_detail() {
        let failure = classify_task_close_git_stage_failure(
            "fatal: pathspec 'missing-file' did not match any files",
            None,
        );

        assert_eq!(failure.blocker_code, "git_stage_failed");
        assert_eq!(
            failure.detail,
            "fatal: pathspec 'missing-file' did not match any files"
        );
        assert_eq!(
            failure.next_action,
            "Verify the explicit commit files exist and can be staged."
        );
    }

    #[test]
    fn task_close_stage_owned_without_commit_fails_closed() {
        let task = owned_task_record("task-owned", vec!["crates/vida/src"]);
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "task-owned".to_string(),
                reason: Some("done".to_string()),
                reason_file: None,
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: false,
                include_global_progress: false,
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
            .replace('\\', "/")
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
        std::thread::Builder::new()
            .name("task-handoff-isolated-state-dir-receipt".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
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
                        "epic",
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

                let (receipt_root, isolation) =
                    task_handoff_receipt_root(&isolated_state_dir, true);
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
                    .replace('\\', "/")
                    .starts_with(
                        isolated_handoff_receipts
                            .to_str()
                            .expect("receipt dir should be utf8")
                            .replace('\\', "/")
                            .as_str()
                    ));
            })
            .expect("high-stack receipt test thread should spawn")
            .join()
            .expect("high-stack receipt test thread should complete");
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
        assert_eq!(receipt.binding_source, None);
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida task next-lawful"));
    }

    #[test]
    fn task_next_lawful_selects_unique_top_priority_ready_candidate() {
        let mut top = owned_task_record("task-top", vec![]);
        top.status = "open".to_string();
        top.priority = 1;
        let mut lower = owned_task_record("task-lower", vec![]);
        lower.status = "open".to_string();
        lower.priority = 2;
        let ready = vec![
            super::task_continuation_candidate(&top, false),
            super::task_continuation_candidate(&lower, false),
        ];

        let receipt = task_next_lawful_receipt(&[top, lower], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-top");
        assert_eq!(
            receipt.why_this_unit,
            "unique highest-priority ready TaskFlow candidate after close/release automation"
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_unique_top_priority_candidate"
        );
        assert_eq!(
            receipt.bind_command.as_deref(),
            Some("vida taskflow run-graph dispatch-init task-top --json")
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_blocks_multiple_ready_candidates() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        first.priority = 1;
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        second.priority = 1;
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
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-a")
        );
        assert_eq!(
            receipt.bind_command.as_deref(),
            Some("vida taskflow run-graph dispatch-init task-a --json")
        );
        assert!(receipt
            .why_not_auto_bound
            .as_deref()
            .is_some_and(|reason| reason.contains("multiple ready candidates")));
    }

    #[test]
    fn next_lawful_blocked_json_operator_contract() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        first.priority = 1;
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        second.priority = 1;
        let ready = vec![
            super::task_continuation_candidate(&first, false),
            super::task_continuation_candidate(&second, false),
        ];

        let receipt = task_next_lawful_receipt(&[first, second], ready, None);
        let payload = serde_json::to_value(&receipt).expect("receipt should serialize");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["ambiguous_ready_task_candidates"]
        );
        assert!(receipt
            .ambiguity_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("multiple ready candidates")));
        assert_eq!(receipt.artifact_refs["surface"], "vida task next-lawful");
        assert_eq!(
            receipt.artifact_refs["active_bounded_unit"],
            serde_json::Value::Null
        );
        assert_eq!(receipt.artifact_refs["ready_task_candidate_count"], 2);
        assert_eq!(
            receipt.artifact_refs["recommended_primary_task_id"],
            "task-a"
        );
        assert_eq!(receipt.shared_fields["status"], receipt.status);
        assert_eq!(
            receipt.shared_fields["artifact_refs"],
            receipt.operator_contracts["artifact_refs"]
        );
        assert_eq!(
            receipt.operator_contracts["contract_id"],
            "release-1-operator-contracts"
        );
        assert_eq!(receipt.operator_contracts["status"], "blocked");
        assert_eq!(
            receipt.operator_contracts["blocker_codes"],
            serde_json::json!(["ambiguous_ready_task_candidates"])
        );
        assert_eq!(
            receipt.operator_contracts["next_actions"],
            serde_json::json!(receipt.next_actions.clone())
        );
        assert!(payload.get("artifact_refs").is_some());
        assert!(payload.get("shared_fields").is_some());
        assert!(payload.get("operator_contracts").is_some());
        assert!(payload.get("ambiguity_reason").is_some());
    }

    #[test]
    fn task_next_lawful_epic_sequential_strategy_keeps_primary_epic_candidates() {
        let mut epic_a = owned_task_record("epic-a", vec![]);
        epic_a.issue_type = "epic".to_string();
        let mut epic_b = owned_task_record("epic-b", vec![]);
        epic_b.issue_type = "epic".to_string();
        let mut a_first = owned_task_record("task-a-first", vec![]);
        a_first.status = "open".to_string();
        a_first.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: a_first.id.clone(),
            depends_on_id: epic_a.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let mut a_second = owned_task_record("task-a-second", vec![]);
        a_second.status = "open".to_string();
        a_second.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: a_second.id.clone(),
            depends_on_id: epic_a.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let mut b_first = owned_task_record("task-b-first", vec![]);
        b_first.status = "open".to_string();
        b_first.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: b_first.id.clone(),
            depends_on_id: epic_b.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let ready = vec![
            task_continuation_candidate(&a_first, false),
            task_continuation_candidate(&a_second, false),
            task_continuation_candidate(&b_first, false),
        ];

        let filtered = task_next_lawful_apply_strategy(
            &[epic_a, epic_b, a_first, a_second, b_first],
            ready,
            Some("epic-sequential"),
        );

        let ids = filtered
            .iter()
            .map(|candidate| candidate.task_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["task-a-first", "task-a-second"]);
    }

    #[test]
    fn task_next_lawful_select_ready_candidate_returns_selected_bind_command() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("task-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            task_continuation_candidate(&first, false),
            task_continuation_candidate(&second, true),
        ];

        let receipt =
            task_next_lawful_select_ready_candidate_receipt(&[first, second], ready, "task-b");

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-b");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("operator_selected_ready_candidate")
        );
        assert_eq!(
            receipt.bind_command.as_deref(),
            Some("vida taskflow run-graph dispatch-init task-b --json")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "parallel_safe_operator_selected_candidate"
        );
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-b")
        );
    }

    #[test]
    fn task_next_lawful_select_missing_candidate_fails_closed() {
        let mut first = owned_task_record("task-a", vec![]);
        first.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&first, false)];

        let receipt =
            task_next_lawful_select_ready_candidate_receipt(&[first], ready, "task-missing");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["selected_task_not_ready"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(
            receipt
                .recommended_primary
                .as_ref()
                .map(|candidate| candidate.task_id.as_str()),
            Some("task-a")
        );
    }

    #[test]
    fn task_next_lawful_select_blocks_when_taskflow_task_is_active() {
        let active = owned_task_record("task-active", vec![]);
        let mut ready_task = owned_task_record("task-ready", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, true)];

        let receipt = task_next_lawful_select_ready_candidate_receipt(
            &[active, ready_task],
            ready,
            "task-ready",
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["select_conflicts_with_active_taskflow_task"]
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "task-active");
        assert!(receipt
            .next_action
            .as_deref()
            .is_some_and(|action| action.contains("task-active")));
    }

    #[test]
    fn task_next_lawful_select_blocks_when_multiple_taskflow_tasks_are_active() {
        let active_a = owned_task_record("task-active-a", vec![]);
        let active_b = owned_task_record("task-active-b", vec![]);
        let mut ready_task = owned_task_record("task-ready", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, true)];

        let receipt = task_next_lawful_select_ready_candidate_receipt(
            &[active_a, active_b, ready_task],
            ready,
            "task-ready",
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["multiple_active_tasks"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
    }

    #[test]
    fn task_next_lawful_selects_in_progress_child_leaf_over_active_parent() {
        let mut parent = owned_task_record("generic-runtime-foundation-release-readiness", vec![]);
        parent.title = "Release readiness".to_string();
        let mut child = owned_task_record(
            "todo-repair-status-cold-after-mutation-timing-20260602",
            vec![],
        );
        child.title = "Repair cold timing".to_string();
        child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];

        let receipt = task_next_lawful_receipt(&[parent, child], Vec::new(), None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "todo-repair-status-cold-after-mutation-timing-20260602"
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_taskflow_active"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_ignores_in_progress_parent_with_only_closed_child() {
        let mut parent = owned_task_record("runtime-defect-action-economy-batch-surfaces", vec![]);
        parent.title = "Action economy batch surfaces".to_string();
        parent.issue_type = "defect".to_string();
        let mut closed_child = owned_task_record("closed-docs-step", vec![]);
        closed_child.status = "closed".to_string();
        closed_child.issue_type = "step".to_string();
        closed_child.dependencies = vec![crate::state_store::TaskDependencyRecord {
            issue_id: closed_child.id.clone(),
            depends_on_id: parent.id.clone(),
            edge_type: "parent-child".to_string(),
            created_at: "2026-07-03T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }];
        let mut ready_task = owned_task_record("parallel-session-ready-task", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, true)];

        let receipt = task_next_lawful_receipt(&[parent, closed_child, ready_task], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "parallel-session-ready-task"
        );
        assert_eq!(
            receipt.why_this_unit,
            "single ready TaskFlow candidate after close/release automation"
        );
        assert_ne!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_blocks_runtime_derived_taskflow_active_conflict() {
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
            binding_source: "latest_run_graph_status".to_string(),
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

    #[test]
    fn task_next_lawful_ignores_missing_single_source_binding_for_taskflow_active() {
        let active_task = owned_task_record("active-task", vec![]);
        let binding = test_continuation_binding(
            "missing-run",
            "missing-runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            std::slice::from_ref(&active_task),
            Some(&binding),
            None,
        )
        .expect("missing binding should not fail source selection");
        assert!(selected.is_none());

        let receipt = task_next_lawful_receipt(&[active_task], Vec::new(), selected);

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(receipt.active_bounded_unit["task_id"], "active-task");
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_keeps_missing_single_source_binding_without_taskflow_active() {
        let binding = test_continuation_binding(
            "missing-run",
            "missing-runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&binding), None)
            .expect("single stale binding should remain selectable without active fallback");

        assert_eq!(
            selected.map(|binding| binding.task_id.as_str()),
            Some("missing-runtime-task")
        );
    }

    #[test]
    fn task_next_lawful_blocks_explicit_task_binding_with_parallel_active_tasks() {
        let mut runtime_task = owned_task_record("runtime-task", vec![]);
        runtime_task.status = "open".to_string();
        let active_task = owned_task_record("active-task", vec![]);
        let ready = vec![super::task_continuation_candidate(&runtime_task, false)];
        let binding = test_continuation_binding(
            "run-1",
            "runtime-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

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
    fn task_next_lawful_prefers_current_same_task_over_stale_explicit_run_binding() {
        let task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[task], Some(&explicit), Some(&current))
            .expect("same-task current run should supersede stale explicit task binding")
            .expect("current binding should select");

        assert_eq!(selected.run_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(
            selected.binding_source,
            "consume_continue_deferred_agent_handoff"
        );
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_over_unscoped_dispatch_init_projection() {
        let explicit_task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let current_task =
            owned_task_record("agent-mode-dev-team-test-first-operating-model", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "agent-mode-dev-team-test-first-operating-model",
            "agent-mode-dev-team-test-first-operating-model",
            "run_graph_dispatch_init",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect(
            "unscoped dispatch-init latest projection should not override live explicit binding",
        )
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_over_unrelated_prelaunch_blocked_projection() {
        let explicit_task = owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        let current_task =
            owned_task_record("agent-mode-dev-team-test-first-operating-model", vec![]);
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "agent-mode-dev-team-test-first-operating-model",
            "agent-mode-dev-team-test-first-operating-model",
            "dispatch_prelaunch_blocked",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("unrelated prelaunch-blocked projection should not override live explicit binding")
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "taskflow-case-18-rollout-regression-gate");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_prefers_live_explicit_binding_over_unrelated_ready_candidates() {
        let mut explicit_task =
            owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        explicit_task.status = "open".to_string();
        let mut ready_task = owned_task_record(
            "agent-mode-defect-model-not-pinned-after-dispatch-init",
            vec![],
        );
        ready_task.status = "open".to_string();
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let ready = vec![super::task_continuation_candidate(&ready_task, false)];

        let receipt =
            task_next_lawful_receipt(&[explicit_task, ready_task], ready, Some(&explicit));

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "taskflow-case-18-rollout-regression-gate"
        );
        assert_eq!(
            receipt.binding_source,
            Some("explicit_continuation_bind_task".to_string())
        );
    }

    #[test]
    fn task_next_lawful_blocks_paused_runtime_binding_with_concrete_resume_action() {
        let mut explicit_task =
            owned_task_record("taskflow-case-18-rollout-regression-gate", vec![]);
        explicit_task.status = "paused".to_string();
        let explicit = test_continuation_binding(
            "codebase-audit-runtime-helper-dedup-refactor",
            "taskflow-case-18-rollout-regression-gate",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[explicit_task], Vec::new(), Some(&explicit));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_paused"]);
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains(
                "vida task update taskflow-case-18-rollout-regression-gate --status in_progress",
            ) && action.contains("vida taskflow continuation bind codebase-audit-runtime-helper-dedup-refactor --task-id <task-id>")
                && !action.contains("--json")
        }));
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
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains("vida taskflow recovery status current-run")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_prefers_newer_explicit_task_override_over_current_run_binding() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let current_task = owned_task_record("current-task", vec![]);
        let mut explicit = test_continuation_binding(
            "parent-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        explicit.recorded_at = "2026-05-22T21:48:46Z".to_string();
        let mut current = test_continuation_binding(
            "current-run",
            "current-task",
            "explicit_continuation_bind",
            "task_graph_task",
        );
        current.recorded_at = "2026-05-22T21:47:08Z".to_string();

        let selected = select_task_next_lawful_binding(
            &[explicit_task, current_task],
            Some(&explicit),
            Some(&current),
        )
        .expect("newer explicit task binding should override prior current-run binding")
        .expect("explicit binding should select");

        assert_eq!(selected.task_id, "explicit-task");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_ignores_both_stale_source_drift_bindings() {
        let explicit = test_continuation_binding(
            "old-run",
            "missing-explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&explicit), Some(&current))
            .expect("stale explicit/current disagreement should defer to TaskFlow selection");

        assert!(selected.is_none());
    }

    #[test]
    fn task_next_lawful_selects_live_explicit_over_missing_current_binding() {
        let explicit_task = owned_task_record("explicit-task", vec![]);
        let explicit = test_continuation_binding(
            "old-run",
            "explicit-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected =
            select_task_next_lawful_binding(&[explicit_task], Some(&explicit), Some(&current))
                .expect("live explicit binding should win over stale current binding")
                .expect("explicit binding should select");

        assert_eq!(selected.task_id, "explicit-task");
        assert_eq!(selected.binding_source, "explicit_continuation_bind_task");
    }

    #[test]
    fn task_next_lawful_keeps_downstream_dispatch_target_live_during_source_drift() {
        let explicit = test_continuation_binding(
            "old-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );
        let current = test_continuation_binding(
            "current-run",
            "missing-current-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let selected = select_task_next_lawful_binding(&[], Some(&explicit), Some(&current))
            .expect("downstream dispatch target should remain live without an open task")
            .expect("explicit downstream dispatch target should select");

        assert_eq!(selected.task_id, "closed-feature-task");
        assert_eq!(selected.binding_source, "task_close_reconcile");
    }

    #[test]
    fn task_next_lawful_allows_downstream_dispatch_target_from_current_binding() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );

        let receipt = task_next_lawful_receipt(&[closed_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(receipt.active_bounded_unit, serde_json::Value::Null);
        assert_eq!(receipt.binding_source, None);
        assert!(receipt.ready_task_candidates.is_empty());
    }

    #[test]
    fn task_next_lawful_allows_active_task_over_closed_downstream_dispatch_target() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let active_task = owned_task_record("live-active-task", vec![]);
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "task_close_reconcile",
            "downstream_dispatch_target",
        );

        let receipt =
            task_next_lawful_receipt(&[closed_task, active_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "live-active-task");
        assert_eq!(
            receipt.why_this_unit,
            "Single TaskFlow in_progress task is the authoritative active bounded unit."
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_prefers_single_active_task_over_other_ready_candidates() {
        let active_task = owned_task_record("live-active-task", vec![]);
        let mut ready_task = owned_task_record("other-ready-task", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![task_continuation_candidate(&ready_task, false)];

        let receipt = task_next_lawful_receipt(&[active_task, ready_task], ready, None);

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["task_id"], "live-active-task");
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_taskflow_active"
        );
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(receipt.ready_task_candidates.len(), 1);
    }

    #[test]
    fn task_next_lawful_blocks_closed_run_graph_binding_with_concrete_recovery_action() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[closed_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_closed"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run")
                && action.contains("closed-feature-task")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_blocks_closed_run_graph_binding_with_single_ready_candidate() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let mut ready_task = owned_task_record("ready-only", vec![]);
        ready_task.status = "open".to_string();
        let ready = vec![super::task_continuation_candidate(&ready_task, false)];
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[closed_task, ready_task], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_closed"]);
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "closed-feature-task"
        );
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run")
                && action.contains("closed-feature-task")
                && !action.contains("Continue ready task `ready-only`")
        }));
    }

    #[test]
    fn task_next_lawful_blocks_closed_run_graph_binding_before_ready_ambiguity() {
        let mut closed_task = owned_task_record("closed-feature-task", vec![]);
        closed_task.status = "closed".to_string();
        let mut first = owned_task_record("ready-a", vec![]);
        first.status = "open".to_string();
        let mut second = owned_task_record("ready-b", vec![]);
        second.status = "open".to_string();
        let ready = vec![
            super::task_continuation_candidate(&first, false),
            super::task_continuation_candidate(&second, false),
        ];
        let binding = test_continuation_binding(
            "current-run",
            "closed-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt =
            task_next_lawful_receipt(&[closed_task, first, second], ready, Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_closed"]);
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "closed-feature-task"
        );
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run")
                && action.contains("closed-feature-task")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_blocks_missing_run_graph_binding_with_concrete_recovery_action() {
        let binding = test_continuation_binding(
            "current-run",
            "missing-feature-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["runtime_binding_task_missing"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida taskflow recovery status current-run")
                && action
                    .contains("vida taskflow continuation bind current-run --task-id <task-id>")
                && action.contains("missing-feature-task")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_prefers_single_active_task_over_missing_runtime_binding() {
        let active_task = owned_task_record("authoritative-active-task", vec![]);
        let binding = test_continuation_binding(
            "stale-run",
            "missing-feature-task",
            "explicit_continuation_bind_task",
            "task_graph_task",
        );

        let receipt = task_next_lawful_receipt(&[active_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.active_bounded_unit["task_id"],
            "authoritative-active-task"
        );
        assert_eq!(
            receipt.binding_source,
            Some("taskflow_single_in_progress".to_string())
        );
    }

    #[test]
    fn task_next_lawful_blocks_open_delegated_cycle_binding() {
        let runtime_task = owned_task_record("running-runtime-task", vec![]);
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "open".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: None,
                lifecycle_stage: "analysis_active".to_string(),
            },
        };

        assert!(runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            None
        ));
        let receipt = blocked_task_next_lawful_receipt(
            binding.active_bounded_unit.clone(),
            Vec::new(),
            "open_delegated_cycle",
            &runtime_binding_open_delegated_cycle_next_action(&binding),
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["open_delegated_cycle"]);
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida lane show running-run")
                && action.contains("vida taskflow recovery status running-run")
                && !action.contains("--json")
        }));
        assert_eq!(
            task_next_lawful_receipt(&[runtime_task], Vec::new(), Some(&binding)).status,
            "pass",
            "baseline receipt still represents the raw binding; command-level recovery gate blocks it"
        );
    }

    #[test]
    fn task_next_lawful_preserves_timeout_blocker_without_bind_command() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "consume_continue_after_downstream_chain",
            "run_graph_task",
        );
        let ready_candidate = TaskContinuationCandidate {
            task_id: "unrelated-ready-task".to_string(),
            title: "Unrelated ready task".to_string(),
            status: "open".to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            ready_parallel_safe: false,
        };
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_blocked".to_string(),
            resume_node: None,
            resume_status: "blocked".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_blocked".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "analysis_blocked".to_string(),
            },
        };
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "running-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-06-07T01:00:00Z".to_string(),
        };

        let receipt = blocked_runtime_recovery_task_next_lawful_receipt(
            &binding,
            vec![ready_candidate],
            Some(&recovery),
            Some(&dispatch),
        );

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec![
                "open_delegated_cycle".to_string(),
                "timeout_without_takeover_authority".to_string()
            ]
        );
        assert!(receipt.recommended_primary.is_some());
        assert_eq!(
            receipt.bind_command, None,
            "runtime recovery blockers must not expose unrelated ready-task bind guidance"
        );
        assert!(receipt.next_actions.iter().any(|action| {
            action.contains("vida lane show running-run")
                && action.contains("vida taskflow recovery status running-run")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_allows_ready_downstream_handoff_despite_open_cycle_gate() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: Some("writer".to_string()),
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "handoff_pending".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "analysis_active".to_string(),
            },
        };
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "running-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        };

        assert!(!runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&dispatch)
        ));
        let receipt = pass_ready_downstream_handoff_task_next_lawful_receipt(
            &binding,
            Vec::new(),
            None,
            None,
        );

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert!(receipt.next_action.as_deref().is_some_and(|action| {
            action.contains("vida taskflow consume continue --run-id running-run")
                && !action.contains("--json")
        }));
    }

    #[test]
    fn task_next_lawful_allows_completed_lane_despite_stale_open_cycle_gate() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution_started",
            "run_graph_task",
        );
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_active".to_string(),
            },
        };
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "running-run".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-26T01:00:00Z".to_string(),
        };

        assert!(!runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&dispatch)
        ));
        let receipt = pass_completed_lane_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_completed_dispatch_receipt")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_completed_lane_reconciled"
        );
    }

    #[test]
    fn task_next_lawful_does_not_allow_unrelated_ready_downstream_handoff() {
        let recovery = state_store::RunGraphRecoverySummary {
            run_id: "running-run".to_string(),
            task_id: "running-runtime-task".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: Some("writer".to_string()),
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "awaiting_writer".to_string(),
            recovery_ready: true,
            delegation_gate: state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "open".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "analysis_active".to_string(),
            },
        };
        let unrelated_dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "newer-unrelated-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: Some("result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("writer".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("downstream-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-22T01:00:00Z".to_string(),
        };

        assert!(runtime_recovery_blocks_task_next_lawful(
            Some(&recovery),
            Some(&unrelated_dispatch)
        ));
    }

    #[test]
    fn task_next_lawful_uses_downstream_execute_command_after_terminal_ready_downstream_handoff() {
        let binding = test_continuation_binding(
            "running-run",
            "running-runtime-task",
            "dispatch_execution",
            "run_graph_task",
        );

        let receipt = pass_ready_downstream_handoff_task_next_lawful_receipt(
            &binding,
            Vec::new(),
            Some("running-run"),
            Some("vida agent-init --downstream-packet packet.json --execute-dispatch --json"),
        );

        assert_eq!(receipt.status, "pass");
        assert!(receipt.blocker_codes.is_empty());
        let next_action = receipt
            .next_action
            .as_deref()
            .expect("ready downstream handoff should return a next action");
        assert!(next_action
            .contains("vida agent-init --downstream-packet packet.json --execute-dispatch"));
        assert!(
            !next_action.contains("--json"),
            "human next-action guidance should prefer the default output mode"
        );
    }

    #[test]
    fn task_next_lawful_surfaces_exception_takeover_binding_source() {
        let runtime_task = owned_task_record("exception-task", vec![]);
        let mut binding = test_continuation_binding(
            "exception-run",
            "exception-task",
            "latest_run_graph_exception_takeover_dispatch",
            "run_graph_task",
        );
        binding.active_bounded_unit = serde_json::json!({
            "active_node": "specification",
            "kind": "run_graph_task",
            "run_id": "exception-run",
            "task_id": "exception-task",
        });

        let receipt = task_next_lawful_receipt(&[runtime_task], Vec::new(), Some(&binding));

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.active_bounded_unit["active_node"], "specification");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
    }

    #[test]
    fn task_next_lawful_exception_takeover_bypasses_open_cycle_blocker() {
        let binding = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("case18-previous-takeover".to_string()),
            exception_path_receipt_id: Some("case18-current-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-21T12:28:00Z".to_string(),
        };

        assert!(runtime_binding_has_active_exception_takeover(
            &binding,
            Some(&dispatch)
        ));
        let receipt = pass_exception_takeover_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_exception_takeover"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_accepts_active_exception_takeover_with_recorded_lane_status() {
        let binding = test_continuation_binding(
            "taskflow-case-18-rollout-regression-gate",
            "taskflow-case-18-rollout-regression-gate",
            "consume_continue_deferred_agent_handoff",
            "run_graph_task",
        );
        let dispatch = state_store::RunGraphDispatchReceiptSummary {
            run_id: "taskflow-case-18-rollout-regression-gate".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: Some("case18-supersession-evidence".to_string()),
            exception_path_receipt_id: Some("case18-exception-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["configured_backend_dispatch_failed".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-05-21T14:47:00Z".to_string(),
        };

        assert!(runtime_binding_has_active_exception_takeover(
            &binding,
            Some(&dispatch)
        ));
        let receipt = pass_exception_takeover_task_next_lawful_receipt(&binding, Vec::new());

        assert_eq!(receipt.status, "pass");
        assert_eq!(
            receipt.binding_source.as_deref(),
            Some("latest_run_graph_exception_takeover_dispatch")
        );
        assert_eq!(
            receipt.sequential_vs_parallel_posture,
            "sequential_only_exception_takeover"
        );
        assert!(receipt.blocker_codes.is_empty());
    }

    #[test]
    fn task_next_lawful_command_runs_with_single_ready_task() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-ready",
                "Ready task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let code = run_cli_on_runtime_stack_for_test(vec![
            "task".to_string(),
            "next-lawful".to_string(),
            "--state-dir".to_string(),
            harness
                .path()
                .to_str()
                .expect("state path should be utf8")
                .to_string(),
            "--json".to_string(),
        ]);

        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn task_ready_prefers_authoritative_store() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-ready",
                "Ready task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
            let snapshot_path =
                crate::StateStore::canonical_task_snapshot_path_for_state_root(harness.path());
            fs::write(&snapshot_path, "").expect("snapshot should be writable");
            drop(store);

            let (tasks, metadata) =
                task_ready_authoritative_first(harness.path().to_path_buf(), None)
                    .await
                    .expect("ready tasks should load from authoritative store");

            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].id, "task-ready");
            assert_eq!(metadata.mode, "authoritative_live");
            assert!(!metadata.degraded);
            assert!(metadata.snapshot_path.is_none());
        });
    }

    #[test]
    fn task_critical_path_prefers_authoritative_store() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "critical-ready",
                "Critical ready",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
            let snapshot_path =
                crate::StateStore::canonical_task_snapshot_path_for_state_root(harness.path());
            fs::write(&snapshot_path, "").expect("snapshot should be writable");
            drop(store);

            let path = task_critical_path_snapshot_first(harness.path().to_path_buf())
                .await
                .expect("critical path should load from authoritative store");

            assert_eq!(path.length, 1);
            assert_eq!(path.root_task_id.as_deref(), Some("critical-ready"));
            assert_eq!(path.terminal_task_id.as_deref(), Some("critical-ready"));
        });
    }

    #[test]
    fn task_next_lawful_command_selects_ready_task_over_closed_downstream_closure_marker() {
        run_on_runtime_stack_for_test(
            task_next_lawful_command_selects_ready_task_over_closed_downstream_closure_marker_body,
        );
    }

    fn task_next_lawful_command_selects_ready_task_over_closed_downstream_closure_marker_body() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "taskflow-case-11-actual-agent-autonomy",
                "Actual ready candidate",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "taskflow-defect-case-11-closed-downstream-binding-blocks-ready",
                "Closed downstream marker",
                "task",
                "closed",
                1,
                Some("parent-epic"),
            )
            .await;
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "run-closed-downstream-marker".to_string(),
                    task_id: "taskflow-defect-case-11-closed-downstream-binding-blocks-ready"
                        .to_string(),
                    task_class: "worker".to_string(),
                    active_node: "closure".to_string(),
                    next_node: None,
                    status: "ready".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "taskflow_state_store".to_string(),
                    lane_id: "closure_lane".to_string(),
                    lifecycle_stage: "closure_active".to_string(),
                    policy_gate: "not_required".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "execution_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: true,
                })
                .await
                .expect("run graph status should record");
            let binding = test_continuation_binding(
                "run-closed-downstream-marker",
                "taskflow-defect-case-11-closed-downstream-binding-blocks-ready",
                "task_close_reconcile",
                "downstream_dispatch_target",
            );
            store
                .record_run_graph_continuation_binding(&binding)
                .await
                .expect("continuation binding should record");
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
        let projection_path = harness
            .path()
            .join("operator-projections")
            .join("task-next-lawful-latest.json");
        let projection: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(projection_path).expect("next-lawful projection should be written"),
        )
        .expect("next-lawful projection should parse");
        assert_eq!(projection["status"], task_json_success_status());
        assert_eq!(
            projection["active_bounded_unit"]["task_id"],
            "taskflow-case-11-actual-agent-autonomy"
        );
        assert_eq!(projection["binding_source"], serde_json::Value::Null);
        assert!(projection["blocker_codes"]
            .as_array()
            .expect("blockers should be an array")
            .is_empty());
    }

    #[test]
    fn task_next_lawful_command_ignores_reconciled_terminal_closure_latest_run() {
        run_on_runtime_stack_for_test(
            task_next_lawful_command_ignores_reconciled_terminal_closure_latest_run_body,
        );
    }

    fn task_next_lawful_command_ignores_reconciled_terminal_closure_latest_run_body() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "task-ready-after-stale-closure",
                "Ready after stale closure",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "closed-runtime-task",
                "Closed runtime task",
                "task",
                "closed",
                1,
                Some("parent-epic"),
            )
            .await;
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "terminal-closure-run".to_string(),
                    task_id: "closed-runtime-task".to_string(),
                    task_class: "worker".to_string(),
                    active_node: "closure".to_string(),
                    next_node: None,
                    status: "completed".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "taskflow_state_store".to_string(),
                    lane_id: "closure_lane".to_string(),
                    lifecycle_stage: "closure_complete".to_string(),
                    policy_gate: "historical_closed_task_stale_run_retired".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "execution_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: false,
                })
                .await
                .expect("run graph status should record");
            let binding = test_continuation_binding(
                "terminal-closure-run",
                "closed-runtime-task",
                "consume_continue_after_downstream_chain",
                "run_graph_task",
            );
            store
                .record_run_graph_continuation_binding(&binding)
                .await
                .expect("continuation binding should record");
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
        let projection_path = harness
            .path()
            .join("operator-projections")
            .join("task-next-lawful-latest.json");
        let projection: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(projection_path).expect("next-lawful projection should be written"),
        )
        .expect("next-lawful projection should parse");
        assert_eq!(projection["status"], task_json_success_status());
        assert_eq!(
            projection["active_bounded_unit"]["task_id"],
            "task-ready-after-stale-closure"
        );
        assert_eq!(projection["binding_source"], serde_json::Value::Null);
        assert!(projection["blocker_codes"]
            .as_array()
            .expect("blockers should be an array")
            .is_empty());
    }

    #[test]
    fn task_next_lawful_command_preserves_timeout_blocker_without_bind_command() {
        run_on_runtime_stack_for_test(
            task_next_lawful_command_preserves_timeout_blocker_without_bind_command_body,
        );
    }

    fn task_next_lawful_command_preserves_timeout_blocker_without_bind_command_body() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "running-runtime-task",
                "Running runtime task",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "unrelated-ready-task",
                "Unrelated ready task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .record_run_graph_status(&crate::state_store::RunGraphStatus {
                    run_id: "running-run".to_string(),
                    task_id: "running-runtime-task".to_string(),
                    task_class: "worker".to_string(),
                    active_node: "analysis".to_string(),
                    next_node: None,
                    status: "blocked".to_string(),
                    route_task_class: "analysis".to_string(),
                    selected_backend: "internal_subagents".to_string(),
                    lane_id: "analysis_lane".to_string(),
                    lifecycle_stage: "analysis_blocked".to_string(),
                    policy_gate: "targeted_verification".to_string(),
                    handoff_state: "none".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "execution_cursor".to_string(),
                    resume_target: "none".to_string(),
                    recovery_ready: false,
                })
                .await
                .expect("run graph status should record");
            store
                .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                    run_id: "running-run".to_string(),
                    dispatch_target: "analysis".to_string(),
                    dispatch_status: "blocked".to_string(),
                    lane_status: "lane_blocked".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("internal_cli:codex".to_string()),
                    dispatch_command: Some("codex exec".to_string()),
                    dispatch_packet_path: Some("packet.json".to_string()),
                    dispatch_result_path: Some("result.json".to_string()),
                    blocker_code: Some("timeout_without_takeover_authority".to_string()),
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("middle".to_string()),
                    activation_runtime_role: Some("analyst".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-07T01:00:00Z".to_string(),
                })
                .await
                .expect("dispatch receipt should record");
            let binding = test_continuation_binding(
                "running-run",
                "running-runtime-task",
                "consume_continue_after_downstream_chain",
                "run_graph_task",
            );
            store
                .record_run_graph_continuation_binding(&binding)
                .await
                .expect("continuation binding should record");
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

        assert_eq!(code, ExitCode::from(1));
        let projection_path = harness
            .path()
            .join("operator-projections")
            .join("task-next-lawful-latest.json");
        let projection: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(projection_path).expect("next-lawful projection should be written"),
        )
        .expect("next-lawful projection should parse");
        assert_eq!(projection["status"], "blocked");
        assert_eq!(
            projection["blocker_codes"],
            serde_json::json!(["open_delegated_cycle", "timeout_without_takeover_authority"])
        );
        assert_eq!(projection["bind_command"], serde_json::Value::Null);
        assert!(projection["ready_task_candidates"]
            .as_array()
            .expect("ready candidates should be an array")
            .iter()
            .any(|candidate| candidate["task_id"] == "unrelated-ready-task"));
        assert!(projection["next_action"]
            .as_str()
            .is_some_and(|action| action.contains("vida lane show running-run")
                && !action.contains("--json")));
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
        let cwd = std::env::current_dir().expect("current dir should resolve");
        let outside_base = cwd
            .ancestors()
            .find(|path| crate::looks_like_project_root(path))
            .and_then(|project_root| project_root.parent())
            .or_else(|| cwd.parent())
            .expect("project parent should exist");
        let outside_root = outside_base.join(
            harness
                .path()
                .file_name()
                .expect("temp harness should have a leaf name"),
        );
        let isolated_state_dir = outside_root.join("isolated-state");
        fs::create_dir_all(&isolated_state_dir)
            .expect("isolated state directory should initialize");
        let task_value = serde_json::json!({
            "id": "audit-p1-task-close-state-dir-feedback-isolation",
            "status": "closed",
        });

        let inferred_project_root =
            crate::taskflow_task_bridge::infer_project_root_from_state_root(&isolated_state_dir);
        assert!(
            task_close_uses_isolated_state_dir(&isolated_state_dir, true),
            "expected isolated explicit state dir; state_dir={}, cwd={:?}, inferred_project_root={:?}",
            isolated_state_dir.display(),
            std::env::current_dir().ok(),
            inferred_project_root
        );
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
        let _ = fs::remove_dir_all(outside_root);
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
    fn task_close_feedback_keeps_noncanonical_feedback_recorded_by_default() {
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
        let task_value = serde_json::json!({
            "id": "audit-p1-fast-task-close-feedback",
            "status": "closed",
        });
        let telemetry = task_close_host_agent_telemetry(
            &project_root.join(crate::state_store::default_state_dir()),
            false,
            Some(project_root),
            &task_value,
            "fixed_by_commit_abc_tests_pass",
            "vida task close",
        );

        assert_eq!(telemetry["status"], "recorded");
        assert_eq!(
            telemetry["feedback"]["mode"],
            "lightweight_task_close_feedback"
        );
    }

    #[test]
    fn task_close_feedback_blocker_summary_surfaces_deferred_canonical_close() {
        let telemetry = serde_json::json!({
            "status": "skipped",
            "reason": "feedback_deferred_for_canonical_close_status",
            "canonical_status": "blocked",
            "canonical_gate": "blocked",
        });

        let (blocker_codes, next_actions) = task_close_feedback_blocker_summary(&telemetry)
            .expect("deferred canonical close should produce blocker summary");

        assert_eq!(
            blocker_codes,
            vec![
                "close_feedback_canonical_status_blocked".to_string(),
                "canonical_gate_blocked".to_string()
            ]
        );
        assert!(next_actions[0].contains("Resolve the blocked condition"));
    }

    #[test]
    fn task_close_feedback_blocker_summary_ignores_historical_blocker_proof_context() {
        let reason = "Closed after proof: previous task close JSON returned close_feedback_canonical_status_blocked/canonical_gate_blocked as historical blocker context; proof passed.";
        let telemetry = task_close_host_agent_telemetry(
            std::path::Path::new(".vida/data/state"),
            false,
            None,
            &serde_json::json!({"id": "task-close-feedback-regression"}),
            reason,
            "test",
        );

        assert_ne!(
            telemetry["reason"],
            "feedback_deferred_for_canonical_close_status"
        );
        assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
    }

    #[test]
    fn task_close_feedback_blocker_summary_ignores_verifier_blockers_folded_in_context() {
        for reason in [
            "Verifier blockers folded in: in-flight read/parse now emit structured envelopes; proof passed.",
            "Stale compile blocker cleared by current mainline proof; proof passed.",
        ] {
            let telemetry = task_close_host_agent_telemetry(
                std::path::Path::new(".vida/data/state"),
                false,
                None,
                &serde_json::json!({"id": "task-close-feedback-verifier-blockers-folded-in"}),
                reason,
                "test",
            );

            assert_ne!(
                telemetry["reason"],
                "feedback_deferred_for_canonical_close_status"
            );
            assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
        }
    }

    fn task_close_feedback_project_telemetry(task_id: &str, reason: &str) -> serde_json::Value {
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
        let task_value = serde_json::json!({
            "id": task_id,
            "status": "closed",
        });

        task_close_host_agent_telemetry(
            &project_root.join(crate::state_store::default_state_dir()),
            false,
            Some(project_root),
            &task_value,
            reason,
            "vida task close",
        )
    }

    #[test]
    fn task_close_feedback_records_advisory_for_historical_failure_state_evidence() {
        let reason = "Closed after verification: implementation and tests passed. Evidence: prior close attempt output quoted blocker details: close_feedback_canonical_status_blocked/canonical_gate_blocked and failure-state wording.";

        let telemetry = task_close_feedback_project_telemetry(
            "runtime-task-close-feedback-literal-trigger-false-positive-worktree-todo",
            reason,
        );

        assert_eq!(telemetry["status"], "recorded");
        assert_eq!(telemetry.get("canonical_status"), None);
        assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
        assert_eq!(
            telemetry["feedback_outcome_inference"]["outcome"],
            "success"
        );
        assert_eq!(
            telemetry["feedback_outcome_inference"]["failure_markers"],
            serde_json::json!([])
        );
    }

    #[test]
    fn task_close_feedback_records_advisory_for_negated_blocker_baseline_context() {
        let reason = "PR #470 processed and merged. Evidence: gh pr view reports state MERGED, mergedAt 2026-06-23T22:25:19Z, mergeCommit 9b92c28b0ca7df81c4dfcd15dd77bac520dbf91a, head 2384afec2fcfb3ba089f121f93a5da005cfd10c1. Local merged-base proof before merge: git diff --check passed; cargo test -p vida --bin vida runtime_defect_design_backed_seed_uses_configured_first_step --locked -- --test-threads=1 passed. rustfmt --check on taskflow_run_graph.rs failed equally on origin/main baseline, so not a PR-specific blocker. vida task validate-graph passed after merge.";

        let telemetry = task_close_feedback_project_telemetry(
            "task-close-feedback-negated-blocker-baseline",
            reason,
        );

        assert_eq!(telemetry["status"], "recorded");
        assert_eq!(telemetry.get("canonical_status"), None);
        assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
        assert_eq!(
            telemetry["feedback_outcome_inference"]["outcome"],
            "success"
        );
        assert_eq!(
            telemetry["feedback_outcome_inference"]["failure_markers"],
            serde_json::json!([])
        );
    }

    #[test]
    fn task_close_feedback_records_proved_blocked_receipt_rejection_policy() {
        let reason = "Rejected materialization-only blocked task-ensure receipts before terminal closure and persisted final-snapshot resume paths. Proof: cargo test -p vida taskflow_consume_continue_rejects_materialization_only_receipt_before_final_snapshot_replay passed.";

        let telemetry = task_close_feedback_project_telemetry(
            "task-close-feedback-blocked-receipt-policy",
            reason,
        );

        assert_eq!(telemetry["status"], "recorded");
        assert_eq!(telemetry.get("canonical_status"), None);
        assert!(task_close_feedback_blocker_summary(&telemetry).is_none());
        assert_eq!(
            telemetry["feedback_outcome_inference"]["outcome"],
            "success"
        );
        assert_eq!(
            telemetry["feedback_outcome_inference"]["failure_markers"],
            serde_json::json!([])
        );
    }

    #[test]
    fn task_close_commit_automation_requires_explicit_owned_files() {
        let receipt = task_close_automation_receipt(
            &crate::TaskCloseArgs {
                task_id: "audit-p1-task-close-release-options".to_string(),
                reason: Some("close bounded task".to_string()),
                reason_file: None,
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: true,
                push: false,
                include_global_progress: false,
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
                reason: Some("close bounded task".to_string()),
                reason_file: None,
                source: None,
                release: false,
                install: false,
                install_target: "current".to_string(),
                skip_release_build: false,
                source_binary: None,
                install_root: None,
                commit: false,
                push: true,
                include_global_progress: false,
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
            "next_actions": ["Run `vida task import-jsonl`"],
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
    fn task_state_store_open_diagnostic_payload_exposes_wal_replay_guidance() {
        let state_dir = std::path::Path::new("C:/project/vida_mobile/.vida/data/state");
        let error = state_store::StateStoreError::InvalidStorageMetadata {
            reason: "failed to open bounded SurrealKV datastore: Failed to flush memtable to SST table_id=4: Keys are not in order".to_string(),
        };
        let diagnostic = error
            .open_diagnostic(state_dir)
            .expect("WAL replay corruption should classify");
        let payload = crate::release1_operator_output::build_release1_operator_output_payload(
            "vida task reset",
            vec![diagnostic.blocker_code.clone()],
            vec![diagnostic.recovery_guidance.clone()],
            serde_json::json!({
                "state_dir": diagnostic.state_dir,
                "suspected_wal_or_sst_hint": diagnostic.suspected_wal_or_sst_hint,
            }),
            serde_json::json!({
                "state_access": {
                    "mode": "blocked_storage_corruption",
                    "state_dir": diagnostic.state_dir,
                    "corruption_state": diagnostic.corruption_state,
                    "suspected_wal_or_sst_hint": diagnostic.suspected_wal_or_sst_hint,
                    "recovery_guidance": diagnostic.recovery_guidance,
                    "silent_delete_allowed": diagnostic.silent_delete_allowed,
                    "error": error.to_string(),
                },
            }),
        )
        .expect("diagnostic payload should preserve release-1 contract");

        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["state_store_surrealkv_wal_replay_corruption"])
        );
        assert_eq!(
            payload["state_access"]["state_dir"],
            state_dir.display().to_string()
        );
        assert_eq!(payload["state_access"]["silent_delete_allowed"], false);
        assert!(payload["state_access"]["suspected_wal_or_sst_hint"]
            .as_str()
            .expect("hint should be text")
            .contains("WAL/SST files are suspects"));
        assert!(payload["state_access"]["recovery_guidance"]
            .as_str()
            .expect("guidance should be text")
            .contains("Create a backup copy of the whole state directory first"));
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
    fn proof_target_values_split_multi_filter_cargo_test_commands() {
        let proof_targets = parse_proof_target_values(&[
            "cargo test -p vida work_item_taxonomy operator_contracts development_flow_catalog -- --nocapture --test-threads=1".to_string(),
        ]);

        assert_eq!(
            proof_targets,
            vec![
                "cargo test -p vida work_item_taxonomy -- --nocapture --test-threads=1",
                "cargo test -p vida operator_contracts -- --nocapture --test-threads=1",
                "cargo test -p vida development_flow_catalog -- --nocapture --test-threads=1",
            ]
        );
    }

    #[test]
    fn proof_target_values_preserve_cargo_target_selector_and_filter_commands() {
        let proof_targets = parse_proof_target_values(&[
            "cargo test -p vida --test boot_smoke task_proof_target_cargo_filter_preservation -- --nocapture".to_string(),
            "cargo test -p vida taskflow_packet_latest_happy_path_selects_latest_run_graph_dispatch_packet --test boot_smoke -- --nocapture".to_string(),
        ]);

        assert_eq!(
            proof_targets,
            vec![
                "cargo test -p vida --test boot_smoke task_proof_target_cargo_filter_preservation -- --nocapture",
                "cargo test -p vida taskflow_packet_latest_happy_path_selects_latest_run_graph_dispatch_packet --test boot_smoke -- --nocapture",
            ]
        );
    }

    #[test]
    fn proof_target_values_normalize_stale_diagnostics_and_docflow_flags() {
        let proof_targets = parse_proof_target_values(&[
            "vida diagnostics --json".to_string(),
            "vida docflow protocol-coverage-check --profile active-canon --format jsonl"
                .to_string(),
        ]);

        assert_eq!(
            proof_targets,
            vec![
                "vida diagnostics post-commit --json",
                "vida docflow protocol-coverage-check --profile active-canon",
            ]
        );
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
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "dep-task",
                "Dependency",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
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
    fn task_split_command_reopens_closed_source_with_new_children() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "sibling-task",
                "Sibling",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
            store
                .close_task("source-task", "completed")
                .await
                .expect("source task should close");
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Split(crate::TaskSplitArgs {
                    task_id: "source-task".to_string(),
                    children: vec![
                        "source-task-a:First reopened slice".to_string(),
                        "source-task-b:Second reopened slice".to_string(),
                    ],
                    reason: "new work found after closure".to_string(),
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
            assert_eq!(source.status, "in_progress");
            assert!(source.closed_at.is_none());
            assert!(source.close_reason.is_none());

            let first_child = store
                .show_task("source-task-a")
                .await
                .expect("first split child should load");
            assert!(first_child.dependencies.iter().any(|dependency| {
                dependency.depends_on_id == "source-task" && dependency.edge_type == "parent-child"
            }));
            assert!(store
                .validate_task_graph()
                .await
                .expect("validate")
                .is_empty());
        });
    }

    #[test]
    fn task_close_child_does_not_auto_close_parent_task() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(
                &store,
                "parent-epic",
                "Parent epic",
                "epic",
                "open",
                1,
                None,
            )
            .await;
            create_task_for_test(
                &store,
                "parent-task",
                "Parent",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "child-todo",
                "Child TODO",
                "todo",
                "in_progress",
                2,
                Some("parent-task"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Close(crate::TaskCloseArgs {
                    task_id: "child-todo".to_string(),
                    reason: Some("implementation proof passed".to_string()),
                    reason_file: None,
                    source: Some("task_close_child_regression".to_string()),
                    release: false,
                    install: false,
                    install_target: "current".to_string(),
                    skip_release_build: false,
                    source_binary: None,
                    install_root: None,
                    commit: false,
                    push: false,
                    include_global_progress: false,
                    stage_owned: false,
                    commit_files: vec![],
                    commit_message: None,
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
            let parent = store
                .show_task("parent-task")
                .await
                .expect("parent task should still exist");
            let child = store
                .show_task("child-todo")
                .await
                .expect("child task should still exist");

            assert_eq!(child.status, "closed");
            assert_eq!(
                parent.status, "open",
                "closing a child TODO must not implicitly close its parent task"
            );
        });
    }

    #[test]
    fn task_close_returns_failure_when_requested_automation_is_blocked() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "root-epic", "Root epic", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "close-with-push-blocked",
                "Close with blocked push",
                "task",
                "in_progress",
                2,
                Some("root-epic"),
            )
            .await;
        });

        assert_eq!(
            runtime.block_on(super::run_task(crate::TaskArgs {
                command: crate::TaskCommand::Close(crate::TaskCloseArgs {
                    task_id: "close-with-push-blocked".to_string(),
                    reason: Some("implementation proof passed".to_string()),
                    reason_file: None,
                    source: Some("task_close_automation_regression".to_string()),
                    release: false,
                    install: false,
                    install_target: "current".to_string(),
                    skip_release_build: false,
                    source_binary: None,
                    install_root: None,
                    commit: false,
                    push: true,
                    include_global_progress: false,
                    stage_owned: false,
                    commit_files: vec![],
                    commit_message: None,
                    state_dir: Some(harness.path().to_path_buf()),
                    render: crate::RenderMode::Plain,
                    json: true,
                }),
            })),
            ExitCode::from(1)
        );

        runtime.block_on(async {
            let store = crate::StateStore::open_existing(harness.path().to_path_buf())
                .await
                .expect("state store should reopen");
            let task = store
                .show_task("close-with-push-blocked")
                .await
                .expect("task should still close even when automation is blocked");
            assert_eq!(task.status, "closed");
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
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "dep-task",
                "Dependency",
                "task",
                "open",
                1,
                Some("parent-epic"),
            )
            .await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
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
            let child_specs = taskflow_core::task::split::parse_split_child_specs(&[
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
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
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

        runtime.block_on(Box::pin(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            create_task_for_test(&store, "parent-epic", "Parent", "epic", "open", 1, None).await;
            create_task_for_test(
                &store,
                "source-task",
                "Source task",
                "task",
                "open",
                2,
                Some("parent-epic"),
            )
            .await;
        }));

        assert_eq!(
            runtime.block_on(Box::pin(crate::taskflow_proxy::run_taskflow_proxy(
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
            ))),
            ExitCode::SUCCESS
        );

        runtime.block_on(Box::pin(async {
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
        }));
    }
}
