//! Task handoff command helpers.

#[must_use]
pub fn canonical_nonempty_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
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

#[must_use]
pub fn sanitize_task_handoff_receipt_component(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::{canonical_nonempty_strings, sanitize_task_handoff_receipt_component};

    #[test]
    fn canonical_nonempty_strings_trims_dedupes_and_drops_empty_values() {
        let values = canonical_nonempty_strings([
            " proof-a ".to_string(),
            "".to_string(),
            "proof-a".to_string(),
            "proof-b".to_string(),
        ]);

        assert_eq!(values, vec!["proof-a".to_string(), "proof-b".to_string()]);
    }

    #[test]
    fn handoff_receipt_component_sanitizes_path_like_input() {
        assert_eq!(
            sanitize_task_handoff_receipt_component("task/a:b c"),
            "task-a-b-c"
        );
    }

    #[test]
    fn handoff_receipt_component_falls_back_for_empty_input() {
        assert_eq!(sanitize_task_handoff_receipt_component(" / "), "task");
    }
}
