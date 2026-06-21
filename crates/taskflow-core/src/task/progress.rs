//! Pure task progress and closure-readiness helpers.

use std::collections::{BTreeMap, BTreeSet};

use crate::{TaskStatus, parse_task_status, task_status_is_closed_like};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskProgressBasis {
    DescendantsExcludingRoot,
    DirectChildren,
}

impl TaskProgressBasis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DescendantsExcludingRoot => "descendants_excluding_root",
            Self::DirectChildren => "direct_children",
        }
    }
}

pub fn parse_task_progress_basis(value: &str) -> Result<TaskProgressBasis, String> {
    match value.trim() {
        "" | "descendants" | "descendants_excluding_root" => {
            Ok(TaskProgressBasis::DescendantsExcludingRoot)
        }
        "direct-children" | "direct_children" | "children" => Ok(TaskProgressBasis::DirectChildren),
        other => Err(format!(
            "unsupported progress basis `{other}`; expected descendants or direct-children"
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProgressRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub issue_type: String,
    pub priority: u32,
    pub labels: Vec<String>,
    pub proof_targets: Vec<String>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskProgressSummary {
    pub root_task: TaskProgressRow,
    pub progress_basis: String,
    pub direct_child_count: usize,
    pub descendant_count: usize,
    pub open_count: usize,
    pub in_progress_count: usize,
    pub closed_count: usize,
    pub epic_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub percent_closed: f64,
    pub closure_candidate: bool,
    pub closure_candidate_state: String,
    pub closure_candidate_reason: Option<String>,
    pub ready_for_close: bool,
    pub missing_proof: bool,
    pub proof_blocked_by_runtime: bool,
    pub blocked_by_runtime: bool,
    pub next_required_command: Option<String>,
    pub recommended_next_action: String,
    pub canonical_commands: Vec<String>,
}

pub fn task_progress_summary_from_rows(
    rows: &[TaskProgressRow],
    task_id: &str,
    basis: TaskProgressBasis,
    quote_task_id: impl Fn(&str) -> String,
    human_command: impl Fn(&str) -> String,
) -> Result<TaskProgressSummary, String> {
    let root_task = rows
        .iter()
        .find(|task| task.id == task_id)
        .cloned()
        .ok_or_else(|| format!("task is missing: {task_id}"))?;
    let child_ids = rows
        .iter()
        .filter(|task| task.parent_id.as_deref() == Some(task_id))
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let scoped_ids = match basis {
        TaskProgressBasis::DirectChildren => child_ids.iter().cloned().collect::<BTreeSet<_>>(),
        TaskProgressBasis::DescendantsExcludingRoot => {
            descendant_ids_from_rows(rows, task_id, &child_ids)
        }
    };

    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut open_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut closed_count = 0usize;
    let mut epic_count = 0usize;
    for task in rows.iter().filter(|task| scoped_ids.contains(&task.id)) {
        *status_counts.entry(task.status.clone()).or_insert(0) += 1;
        match parse_task_status(&task.status) {
            Some(TaskStatus::Open) => open_count += 1,
            Some(TaskStatus::InProgress) => in_progress_count += 1,
            Some(TaskStatus::Closed) => closed_count += 1,
            _ => {}
        }
        if task_is_program_container(task) {
            epic_count += 1;
        }
    }

    let descendant_count = scoped_ids.len();
    let percent_closed = if descendant_count == 0 {
        0.0
    } else {
        (closed_count as f64 / descendant_count as f64) * 100.0
    };
    let all_scoped_closed_like = rows
        .iter()
        .filter(|task| scoped_ids.contains(&task.id))
        .all(|task| task_status_is_closed_like(&task.status));

    match basis {
        TaskProgressBasis::DirectChildren => Ok(direct_child_progress_summary(
            root_task,
            child_ids.len(),
            descendant_count,
            open_count,
            in_progress_count,
            closed_count,
            epic_count,
            status_counts,
            percent_closed,
            all_scoped_closed_like,
            quote_task_id,
        )),
        TaskProgressBasis::DescendantsExcludingRoot => Ok(descendant_progress_summary(
            root_task,
            child_ids.len(),
            descendant_count,
            open_count,
            in_progress_count,
            closed_count,
            epic_count,
            status_counts,
            percent_closed,
            all_scoped_closed_like,
            quote_task_id,
            human_command,
        )),
    }
}

fn descendant_ids_from_rows(
    rows: &[TaskProgressRow],
    task_id: &str,
    direct_child_ids: &[String],
) -> BTreeSet<String> {
    let mut descendant_ids = BTreeSet::<String>::new();
    let mut frontier = direct_child_ids.to_vec();
    while let Some(parent_id) = frontier.pop() {
        if !descendant_ids.insert(parent_id.clone()) {
            continue;
        }
        frontier.extend(
            rows.iter()
                .filter(|task| task.parent_id.as_deref() == Some(parent_id.as_str()))
                .map(|task| task.id.clone()),
        );
    }
    descendant_ids.remove(task_id);
    descendant_ids
}

#[allow(clippy::too_many_arguments)]
fn direct_child_progress_summary(
    root_task: TaskProgressRow,
    direct_child_count: usize,
    descendant_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    epic_count: usize,
    status_counts: BTreeMap<String, usize>,
    percent_closed: f64,
    all_children_closed_like: bool,
    quote_task_id: impl Fn(&str) -> String,
) -> TaskProgressSummary {
    let root_closed = task_status_is_closed_like(&root_task.status);
    let closure_candidate = !root_closed && descendant_count > 0 && all_children_closed_like;
    let next_required_command = if closure_candidate {
        Some(format!(
            "vida task close {} --reason \"direct children closed\"",
            quote_task_id(&root_task.id)
        ))
    } else if descendant_count == 0 {
        Some("Add child work items or close with an explicit operator reason.".to_string())
    } else if !all_children_closed_like {
        Some("Continue or close remaining direct children before closing the parent.".to_string())
    } else {
        None
    };
    let recommended_next_action = next_required_command.clone().unwrap_or_else(|| {
        "No action; task is already closed or has no direct-child blocker.".to_string()
    });

    TaskProgressSummary {
        root_task,
        progress_basis: TaskProgressBasis::DirectChildren.as_str().to_string(),
        direct_child_count,
        descendant_count,
        open_count,
        in_progress_count,
        closed_count,
        epic_count,
        status_counts,
        percent_closed,
        closure_candidate,
        closure_candidate_state: if closure_candidate {
            "ready_to_close".to_string()
        } else if root_closed {
            "already_closed".to_string()
        } else if descendant_count == 0 {
            "container_without_direct_children".to_string()
        } else {
            "direct_children_remaining".to_string()
        },
        closure_candidate_reason: Some("direct-child basis selected by operator".to_string()),
        ready_for_close: closure_candidate,
        missing_proof: false,
        proof_blocked_by_runtime: false,
        blocked_by_runtime: false,
        next_required_command,
        recommended_next_action,
        canonical_commands: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn descendant_progress_summary(
    root_task: TaskProgressRow,
    direct_child_count: usize,
    descendant_count: usize,
    open_count: usize,
    in_progress_count: usize,
    closed_count: usize,
    epic_count: usize,
    status_counts: BTreeMap<String, usize>,
    percent_closed: f64,
    all_descendants_closed_like: bool,
    quote_task_id: impl Fn(&str) -> String,
    human_command: impl Fn(&str) -> String,
) -> TaskProgressSummary {
    let is_container = task_is_program_container(&root_task);
    let root_closed = task_status_is_closed_like(&root_task.status);
    let is_non_container_work_item = !is_container;
    let non_container_descendants_clear = descendant_count == 0 || all_descendants_closed_like;
    let proof_blocked_by_runtime = !root_closed
        && is_non_container_work_item
        && non_container_descendants_clear
        && !root_task.proof_targets.is_empty()
        && root_task
            .labels
            .iter()
            .any(|label| label == "proof-blocked-by-runtime" || label == "runtime-proof-blocked");
    let blocked_by_runtime = proof_blocked_by_runtime
        || (!root_closed
            && is_non_container_work_item
            && (root_task.status == "blocked"
                || root_task
                    .labels
                    .iter()
                    .any(|label| label == "runtime-blocked" || label == "blocked-by-runtime")));
    let missing_proof = !root_closed
        && is_non_container_work_item
        && non_container_descendants_clear
        && !root_task.proof_targets.is_empty()
        && !proof_blocked_by_runtime;
    let leaf_ready_for_close = !root_closed
        && is_non_container_work_item
        && non_container_descendants_clear
        && !missing_proof
        && !blocked_by_runtime
        && matches!(
            root_task.status.as_str(),
            "in_progress" | "review" | "verified" | "ready_for_close"
        );
    let closure_candidate =
        is_container && !root_closed && descendant_count > 0 && all_descendants_closed_like;

    let (
        closure_candidate_state,
        closure_candidate_reason,
        recommended_next_action,
        canonical_commands,
        next_required_command,
    ) = if closure_candidate {
        let close_command = human_command(&format!(
            "vida task close {} --reason \"all descendants closed\"",
            quote_task_id(&root_task.id)
        ));
        (
            "ready_to_close".to_string(),
            Some("root container is open while all descendants are closed-like".to_string()),
            format!("Close container with `{}`.", close_command),
            vec![close_command.clone()],
            Some(close_command),
        )
    } else if root_closed {
        (
            "already_closed".to_string(),
            Some("root task is already closed-like".to_string()),
            "No action; task is already closed.".to_string(),
            Vec::new(),
            None,
        )
    } else if is_non_container_work_item {
        non_container_progress_state(
            &root_task,
            descendant_count,
            all_descendants_closed_like,
            missing_proof,
            proof_blocked_by_runtime,
            blocked_by_runtime,
            leaf_ready_for_close,
            quote_task_id,
            human_command,
        )
    } else if descendant_count == 0 {
        (
            "container_without_descendants".to_string(),
            Some("container has no descendants to prove closure readiness".to_string()),
            "Add child work items or close with an explicit operator reason.".to_string(),
            Vec::new(),
            Some("Add child work items or close with an explicit operator reason.".to_string()),
        )
    } else {
        (
            "active_descendants_remaining".to_string(),
            Some("one or more descendants are not closed-like".to_string()),
            "Continue or close remaining descendant work before closing the container.".to_string(),
            Vec::new(),
            Some(
                "Continue or close remaining descendant work before closing the container."
                    .to_string(),
            ),
        )
    };

    TaskProgressSummary {
        root_task,
        progress_basis: TaskProgressBasis::DescendantsExcludingRoot
            .as_str()
            .to_string(),
        direct_child_count,
        descendant_count,
        open_count,
        in_progress_count,
        closed_count,
        epic_count,
        status_counts,
        percent_closed,
        closure_candidate,
        closure_candidate_state,
        closure_candidate_reason,
        ready_for_close: closure_candidate || leaf_ready_for_close,
        missing_proof,
        proof_blocked_by_runtime,
        blocked_by_runtime,
        next_required_command,
        recommended_next_action,
        canonical_commands,
    }
}

#[allow(clippy::too_many_arguments)]
fn non_container_progress_state(
    root_task: &TaskProgressRow,
    descendant_count: usize,
    all_descendants_closed_like: bool,
    missing_proof: bool,
    proof_blocked_by_runtime: bool,
    blocked_by_runtime: bool,
    leaf_ready_for_close: bool,
    quote_task_id: impl Fn(&str) -> String,
    human_command: impl Fn(&str) -> String,
) -> (String, Option<String>, String, Vec<String>, Option<String>) {
    let child_work_remaining = descendant_count > 0 && !all_descendants_closed_like;
    let next_required_command = if child_work_remaining {
        Some("Close or complete child work before closing the parent work item.".to_string())
    } else if missing_proof {
        Some(
            "Run declared proof targets, then close the leaf task with explicit evidence."
                .to_string(),
        )
    } else if proof_blocked_by_runtime {
        Some(
            "Record or resolve the runtime proof blocker before closing the leaf task.".to_string(),
        )
    } else if blocked_by_runtime {
        Some("Record or resolve the runtime blocker before closing the leaf task.".to_string())
    } else if leaf_ready_for_close {
        Some(human_command(&format!(
            "vida task close {} --reason \"verified\"",
            quote_task_id(&root_task.id)
        )))
    } else {
        Some("Continue the leaf task until verification evidence is available.".to_string())
    };
    let closure_candidate_state = if child_work_remaining {
        "work_item_child_work_remaining"
    } else if missing_proof {
        "leaf_missing_proof"
    } else if proof_blocked_by_runtime {
        "leaf_proof_blocked_by_runtime"
    } else if blocked_by_runtime {
        "leaf_blocked_by_runtime"
    } else if leaf_ready_for_close {
        "leaf_ready_for_close"
    } else {
        "leaf_in_progress"
    };
    let closure_candidate_reason = if descendant_count == 0 {
        "leaf task uses proof readiness instead of container closure semantics"
    } else {
        "non-container work item uses proof readiness instead of container closure semantics"
    };
    (
        closure_candidate_state.to_string(),
        Some(closure_candidate_reason.to_string()),
        next_required_command
            .clone()
            .unwrap_or_else(|| "Continue normal leaf task execution.".to_string()),
        Vec::new(),
        next_required_command,
    )
}

fn task_is_program_container(task: &TaskProgressRow) -> bool {
    task.issue_type.trim().eq_ignore_ascii_case("epic")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, status: &str, issue_type: &str, parent_id: Option<&str>) -> TaskProgressRow {
        TaskProgressRow {
            id: id.to_string(),
            title: id.to_string(),
            status: status.to_string(),
            issue_type: issue_type.to_string(),
            priority: 1,
            labels: Vec::new(),
            proof_targets: Vec::new(),
            parent_id: parent_id.map(str::to_string),
        }
    }

    #[test]
    fn direct_child_progress_marks_open_parent_ready_when_children_are_closed() {
        let rows = vec![
            row("parent", "open", "task", None),
            row("child-a", "closed", "task", Some("parent")),
            row("child-b", "completed", "task", Some("parent")),
        ];

        let summary = task_progress_summary_from_rows(
            &rows,
            "parent",
            TaskProgressBasis::DirectChildren,
            |value| value.to_string(),
            |value| value.to_string(),
        )
        .unwrap();

        assert!(summary.closure_candidate);
        assert!(summary.ready_for_close);
        assert_eq!(summary.closure_candidate_state, "ready_to_close");
        assert_eq!(summary.closed_count, 2);
        assert_eq!(summary.descendant_count, 2);
        assert_eq!(summary.percent_closed, 100.0);
    }

    #[test]
    fn direct_child_progress_counts_done_alias_but_rejects_cancelled_as_open_work() {
        let rows = vec![
            row("parent", "open", "task", None),
            row("child-a", "done", "task", Some("parent")),
            row("child-b", "cancelled", "task", Some("parent")),
        ];

        let summary = task_progress_summary_from_rows(
            &rows,
            "parent",
            TaskProgressBasis::DirectChildren,
            |value| value.to_string(),
            |value| value.to_string(),
        )
        .unwrap();

        assert!(!summary.closure_candidate);
        assert!(!summary.ready_for_close);
        assert_eq!(summary.closure_candidate_state, "direct_children_remaining");
        assert_eq!(summary.closed_count, 1);
        assert_eq!(summary.percent_closed, 50.0);
    }

    #[test]
    fn descendant_progress_uses_normalized_epic_as_container_taxonomy() {
        let rows = vec![
            row("parent", "open", "EPIC", None),
            row("child", "closed", "ePiC", Some("parent")),
        ];

        let summary = task_progress_summary_from_rows(
            &rows,
            "parent",
            TaskProgressBasis::DescendantsExcludingRoot,
            |value| value.to_string(),
            |value| value.to_string(),
        )
        .unwrap();

        assert_eq!(summary.epic_count, 1);
        assert!(summary.ready_for_close);
        assert_eq!(summary.closure_candidate_state, "ready_to_close");
    }

    #[test]
    fn descendant_progress_does_not_treat_program_or_milestone_as_containers() {
        for issue_type in ["program", "milestone"] {
            let rows = vec![row("parent", "open", issue_type, None)];

            let summary = task_progress_summary_from_rows(
                &rows,
                "parent",
                TaskProgressBasis::DescendantsExcludingRoot,
                |value| value.to_string(),
                |value| value.to_string(),
            )
            .unwrap();

            assert!(!summary.ready_for_close);
            assert_eq!(summary.closure_candidate_state, "leaf_in_progress");
        }
    }

    #[test]
    fn descendant_progress_reports_leaf_missing_proof() {
        let mut parent = row("leaf", "in_progress", "task", None);
        parent.proof_targets = vec!["cargo test".to_string()];
        let summary = task_progress_summary_from_rows(
            &[parent],
            "leaf",
            TaskProgressBasis::DescendantsExcludingRoot,
            |value| value.to_string(),
            |value| value.replace(" --json", ""),
        )
        .unwrap();

        assert!(!summary.ready_for_close);
        assert!(summary.missing_proof);
        assert_eq!(summary.closure_candidate_state, "leaf_missing_proof");
    }
}
