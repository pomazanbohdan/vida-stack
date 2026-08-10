use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn escape_surql_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

pub(super) fn sanitize_record_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) fn task_sort_key(
    left: &super::TaskRecord,
    right: &super::TaskRecord,
) -> std::cmp::Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.id.cmp(&right.id))
}

pub(super) fn task_ready_sort_key(
    left: &super::TaskRecord,
    right: &super::TaskRecord,
) -> std::cmp::Ordering {
    let left_rank = if left.status == "in_progress" {
        0u8
    } else {
        1u8
    };
    let right_rank = if right.status == "in_progress" {
        0u8
    } else {
        1u8
    };
    left_rank
        .cmp(&right_rank)
        .then_with(|| left.priority.cmp(&right.priority))
        .then_with(|| left.id.cmp(&right.id))
}

pub(super) fn compare_task_paths(left: &[String], right: &[String]) -> std::cmp::Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.join("->").cmp(&right.join("->")))
}

pub fn default_state_dir() -> PathBuf {
    PathBuf::from(super::DEFAULT_STATE_DIR)
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(super::REPO_ROOT)
}

pub(super) fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub(super) fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{
        compare_task_paths, default_state_dir, escape_surql_literal, repo_root, sanitize_record_id,
        task_ready_sort_key, task_sort_key,
    };
    use crate::state_store::TaskRecord;

    fn task(id: &str, status: &str, priority: u32) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: id.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority,
            issue_type: "task".to_string(),
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
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn core_utils_escape_and_sanitize_literals_without_collapsing_safe_chars() {
        assert_eq!(escape_surql_literal(r"O'Reilly\team"), "O\\'Reilly\\\\team");
        assert_eq!(sanitize_record_id("safe_ID-1.2"), "safe_ID-1.2");
        assert_eq!(
            sanitize_record_id("needs space/slash?"),
            "needs-space-slash-"
        );
        assert_eq!(sanitize_record_id(""), "");
        assert_eq!(
            compare_task_paths(&["a".into()], &["a".into(), "b".into()]),
            Ordering::Less
        );
        assert_eq!(
            compare_task_paths(&["a".into(), "b".into()], &["a".into(), "c".into()]),
            Ordering::Less
        );
    }

    #[test]
    fn core_utils_sort_tasks_by_readiness_priority_and_id() {
        let mut tasks = [
            task("z", "todo", 1),
            task("a", "in_progress", 9),
            task("b", "in_progress", 1),
        ];
        tasks.sort_by(task_ready_sort_key);
        assert_eq!(
            tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a", "z"]
        );

        let mut priority_order = [
            task("z", "todo", 2),
            task("a", "todo", 1),
            task("b", "todo", 1),
        ];
        priority_order.sort_by(task_sort_key);
        assert_eq!(
            priority_order
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "z"]
        );
    }

    #[test]
    fn core_utils_public_paths_follow_state_store_constants() {
        assert_eq!(
            default_state_dir(),
            std::path::PathBuf::from(super::super::DEFAULT_STATE_DIR)
        );
        assert_eq!(
            repo_root(),
            std::path::PathBuf::from(super::super::REPO_ROOT)
        );
    }
}
