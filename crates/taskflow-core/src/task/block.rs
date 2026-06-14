//! Task blocking command helpers.

pub fn normalize_task_block_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[must_use]
pub fn append_task_block_note_with_timestamp(
    existing_notes: Option<&str>,
    reason: &str,
    evidence: Option<&str>,
    blocker_codes: &[String],
    next_actions: &[String],
    recorded_at_unix_nanos: i128,
) -> String {
    let mut note = format!(
        "task_block:\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  reason: {}",
        reason.trim()
    );
    if let Some(evidence) = evidence.map(str::trim).filter(|value| !value.is_empty()) {
        note.push_str("\n  evidence: ");
        note.push_str(evidence);
    }
    if !blocker_codes.is_empty() {
        note.push_str("\n  blocker_codes: ");
        note.push_str(&blocker_codes.join(", "));
    }
    if !next_actions.is_empty() {
        note.push_str("\n  next_actions:");
        for action in next_actions {
            note.push_str("\n    - ");
            note.push_str(action.trim());
        }
    }

    match existing_notes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    }
}

#[must_use]
pub fn append_task_block_note(
    existing_notes: Option<&str>,
    reason: &str,
    evidence: Option<&str>,
    blocker_codes: &[String],
    next_actions: &[String],
) -> String {
    append_task_block_note_with_timestamp(
        existing_notes,
        reason,
        evidence,
        blocker_codes,
        next_actions,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
    )
}

#[cfg(test)]
mod tests {
    use super::{append_task_block_note_with_timestamp, normalize_task_block_list};

    #[test]
    fn block_list_splits_commas_and_omits_empty_entries() {
        let values = normalize_task_block_list(&[
            "runtime, bridge_request_pending".to_string(),
            " , proof ".to_string(),
        ]);

        assert_eq!(
            values,
            vec![
                "runtime".to_string(),
                "bridge_request_pending".to_string(),
                "proof".to_string()
            ]
        );
    }

    #[test]
    fn block_note_preserves_existing_notes_and_structured_fields() {
        let note = append_task_block_note_with_timestamp(
            Some("existing"),
            " bridge unavailable ",
            Some(" dispatch receipt "),
            &["open_delegated_cycle".to_string()],
            &["inspect lane".to_string(), "retry dispatch".to_string()],
            42,
        );

        assert_eq!(
            note,
            "existing\n\n\
task_block:\n  recorded_at_unix_nanos: 42\n  reason: bridge unavailable\n  evidence: dispatch receipt\n  blocker_codes: open_delegated_cycle\n  next_actions:\n    - inspect lane\n    - retry dispatch"
        );
    }
}
