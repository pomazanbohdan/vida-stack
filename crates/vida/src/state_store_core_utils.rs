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
