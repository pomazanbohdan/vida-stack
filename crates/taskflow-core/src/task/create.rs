//! Pure task creation and ensure-policy helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskCreateTitleInput<'a> {
    pub positional_title: Option<&'a str>,
    pub title_option: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskExecutionSemanticsInput<'a> {
    pub execution_mode: Option<&'a str>,
    pub order_bucket: Option<&'a str>,
    pub parallel_group: Option<&'a str>,
    pub conflict_domain: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExistingTaskActual<'a> {
    pub task_id: &'a str,
    pub title: &'a str,
    pub display_id: Option<&'a str>,
    pub issue_type: &'a str,
    pub status: &'a str,
    pub parent_id: Option<&'a str>,
    pub labels: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExistingTaskExpectation<'a> {
    pub title: &'a str,
    pub display_id: Option<&'a str>,
    pub issue_type: &'a str,
    pub status: &'a str,
    pub parent_id: Option<&'a str>,
    pub labels: &'a [String],
}

pub fn task_create_title(input: TaskCreateTitleInput<'_>) -> Result<String, String> {
    let positional = input
        .positional_title
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let option = input
        .title_option
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

#[must_use]
pub fn task_create_semantics_requested(input: TaskExecutionSemanticsInput<'_>) -> bool {
    input.execution_mode.is_some()
        || input.order_bucket.is_some()
        || input.parallel_group.is_some()
        || input.conflict_domain.is_some()
}

#[must_use]
pub fn task_create_semantics_mismatch(
    existing: TaskExecutionSemanticsInput<'_>,
    requested: TaskExecutionSemanticsInput<'_>,
) -> bool {
    requested
        .execution_mode
        .is_some_and(|expected| existing.execution_mode != Some(expected))
        || requested
            .order_bucket
            .is_some_and(|expected| existing.order_bucket != Some(expected))
        || requested
            .parallel_group
            .is_some_and(|expected| existing.parallel_group != Some(expected))
        || requested
            .conflict_domain
            .is_some_and(|expected| existing.conflict_domain != Some(expected))
}

#[must_use]
pub fn ensure_existing_task_mismatch_reason(
    actual: ExistingTaskActual<'_>,
    expected: ExistingTaskExpectation<'_>,
) -> Option<String> {
    if actual.title != expected.title {
        return Some(format!(
            "existing task '{}' title mismatch (expected '{}', got '{}')",
            actual.task_id, expected.title, actual.title
        ));
    }
    if actual.display_id != expected.display_id {
        return Some(format!(
            "existing task '{}' display_id mismatch (expected '{}', got '{}')",
            actual.task_id,
            expected.display_id.unwrap_or(""),
            actual.display_id.unwrap_or("")
        ));
    }
    if actual.issue_type != expected.issue_type {
        return Some(format!(
            "existing task '{}' issue_type mismatch (expected '{}', got '{}')",
            actual.task_id, expected.issue_type, actual.issue_type
        ));
    }
    if actual.status != expected.status {
        return Some(format!(
            "existing task '{}' status mismatch (expected '{}', got '{}')",
            actual.task_id, expected.status, actual.status
        ));
    }
    if actual.parent_id != expected.parent_id {
        return Some(format!(
            "existing task '{}' parent_id mismatch (expected '{}', got '{}')",
            actual.task_id,
            expected.parent_id.unwrap_or(""),
            actual.parent_id.unwrap_or("")
        ));
    }
    if expected
        .labels
        .iter()
        .any(|label| !actual.labels.iter().any(|existing| existing == label))
    {
        let missing_labels = expected
            .labels
            .iter()
            .filter(|label| !actual.labels.iter().any(|existing| existing == *label))
            .cloned()
            .collect::<Vec<_>>();
        return Some(format!(
            "existing task '{}' missing required labels: {}",
            actual.task_id,
            missing_labels.join(",")
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        ExistingTaskActual, ExistingTaskExpectation, TaskCreateTitleInput,
        TaskExecutionSemanticsInput, ensure_existing_task_mismatch_reason,
        task_create_semantics_mismatch, task_create_semantics_requested, task_create_title,
    };

    #[test]
    fn task_create_title_resolves_positional_or_title_option() {
        assert_eq!(
            task_create_title(TaskCreateTitleInput {
                positional_title: Some(" Positional title "),
                title_option: None,
            })
            .expect("positional title should resolve"),
            "Positional title"
        );
        assert_eq!(
            task_create_title(TaskCreateTitleInput {
                positional_title: None,
                title_option: Some(" Flag title "),
            })
            .expect("--title should resolve"),
            "Flag title"
        );
    }

    #[test]
    fn task_create_title_rejects_missing_or_duplicate_sources() {
        let missing = task_create_title(TaskCreateTitleInput {
            positional_title: None,
            title_option: None,
        })
        .expect_err("missing title should fail");
        assert!(missing.contains("Missing task title"));

        let duplicate = task_create_title(TaskCreateTitleInput {
            positional_title: Some("A"),
            title_option: Some("B"),
        })
        .expect_err("duplicate title sources should fail");
        assert!(duplicate.contains("only one task title source"));
    }

    #[test]
    fn task_create_semantics_detect_requested_backfill() {
        let existing = TaskExecutionSemanticsInput::default();
        let requested = TaskExecutionSemanticsInput {
            execution_mode: Some("parallel_safe"),
            order_bucket: Some("feature-x"),
            parallel_group: Some("dev-pack"),
            conflict_domain: Some("task-ensure-semantics"),
        };

        assert!(task_create_semantics_requested(requested));
        assert!(task_create_semantics_mismatch(existing, requested));
    }

    #[test]
    fn task_create_semantics_accepts_exact_matches_and_reports_single_field_drift() {
        let existing = TaskExecutionSemanticsInput {
            execution_mode: Some("parallel_safe"),
            order_bucket: Some("feature-x"),
            parallel_group: Some("dev-pack"),
            conflict_domain: Some("task-ensure-semantics"),
        };
        assert!(!task_create_semantics_mismatch(existing, existing));
        for requested in [
            TaskExecutionSemanticsInput {
                execution_mode: Some("serial"),
                ..existing
            },
            TaskExecutionSemanticsInput {
                order_bucket: Some("feature-y"),
                ..existing
            },
            TaskExecutionSemanticsInput {
                parallel_group: Some("other-pack"),
                ..existing
            },
            TaskExecutionSemanticsInput {
                conflict_domain: Some("other-domain"),
                ..existing
            },
        ] {
            assert!(task_create_semantics_requested(requested));
            assert!(task_create_semantics_mismatch(existing, requested));
        }
    }

    #[test]
    fn ensure_existing_task_rejects_first_contract_mismatch() {
        let actual_labels = vec!["other".to_string()];
        let expected_labels = vec!["tracked-pack".to_string()];
        let reason = ensure_existing_task_mismatch_reason(
            ExistingTaskActual {
                task_id: "task-ensure",
                title: "Unexpected",
                display_id: None,
                issue_type: "bug",
                status: "closed",
                parent_id: Some("other-parent"),
                labels: &actual_labels,
            },
            ExistingTaskExpectation {
                title: "Expected",
                display_id: None,
                issue_type: "task",
                status: "open",
                parent_id: Some("expected-parent"),
                labels: &expected_labels,
            },
        )
        .expect("mismatch reason should exist");

        assert!(reason.contains("title mismatch"));
    }

    #[test]
    fn ensure_existing_task_reports_missing_required_labels() {
        let actual_labels = vec!["one".to_string()];
        let expected_labels = vec!["one".to_string(), "two".to_string()];
        let reason = ensure_existing_task_mismatch_reason(
            ExistingTaskActual {
                task_id: "task-ensure",
                title: "Expected",
                display_id: Some("VH-1"),
                issue_type: "task",
                status: "open",
                parent_id: Some("parent"),
                labels: &actual_labels,
            },
            ExistingTaskExpectation {
                title: "Expected",
                display_id: Some("VH-1"),
                issue_type: "task",
                status: "open",
                parent_id: Some("parent"),
                labels: &expected_labels,
            },
        )
        .expect("missing label should be reported");

        assert_eq!(
            reason,
            "existing task 'task-ensure' missing required labels: two"
        );
    }

    #[test]
    fn ensure_existing_task_reports_each_scalar_contract_drift() {
        let labels = vec!["tracked".to_string()];
        let actual = ExistingTaskActual {
            task_id: "task-ensure",
            title: "Title",
            display_id: Some("VH-1"),
            issue_type: "task",
            status: "open",
            parent_id: Some("parent"),
            labels: &labels,
        };
        let expected = ExistingTaskExpectation {
            title: "Title",
            display_id: Some("VH-1"),
            issue_type: "task",
            status: "open",
            parent_id: Some("parent"),
            labels: &labels,
        };
        assert!(ensure_existing_task_mismatch_reason(actual, expected).is_none());
        for (expected, fragment) in [
            (ExistingTaskExpectation { display_id: Some("VH-2"), ..expected }, "display_id mismatch"),
            (ExistingTaskExpectation { issue_type: "bug", ..expected }, "issue_type mismatch"),
            (ExistingTaskExpectation { status: "closed", ..expected }, "status mismatch"),
            (ExistingTaskExpectation { parent_id: Some("other"), ..expected }, "parent_id mismatch"),
        ] {
            let reason = ensure_existing_task_mismatch_reason(actual, expected)
                .expect("scalar drift should produce a reason");
            assert!(reason.contains(fragment), "unexpected reason: {reason}");
        }
    }
}
