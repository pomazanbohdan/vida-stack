//! Core helpers for task spawned-blocker command planning.

pub fn merged_blocker_labels(source_labels: &[String], extra_labels: &[String]) -> Vec<String> {
    let mut labels = source_labels.to_vec();
    labels.extend(extra_labels.iter().cloned());
    labels.sort();
    labels.dedup();
    labels
}

#[must_use]
pub fn blocker_priority(source_priority: u32, override_priority: Option<u32>) -> u32 {
    override_priority.unwrap_or(source_priority)
}

#[must_use]
pub fn blocker_description(
    source_task_id: &str,
    reason: &str,
    explicit_description: Option<&str>,
) -> String {
    explicit_description
        .map(str::to_string)
        .unwrap_or_else(|| format!("Blocker for `{source_task_id}`: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::{blocker_description, blocker_priority, merged_blocker_labels};

    #[test]
    fn blocker_labels_are_sorted_and_deduplicated() {
        let labels = merged_blocker_labels(
            &["runtime".to_string(), "wave-9".to_string()],
            &["wave-9".to_string(), "blocker".to_string()],
        );
        assert_eq!(labels, vec!["blocker", "runtime", "wave-9"]);
    }

    #[test]
    fn blocker_priority_and_description_follow_overrides() {
        assert_eq!(blocker_priority(2, None), 2);
        assert_eq!(blocker_priority(2, Some(1)), 1);
        assert_eq!(
            blocker_description("task-a", "needs proof", None),
            "Blocker for `task-a`: needs proof"
        );
        assert_eq!(
            blocker_description("task-a", "needs proof", Some("Explicit")),
            "Explicit"
        );
    }
}
