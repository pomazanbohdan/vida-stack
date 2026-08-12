//! Task blocking command helpers.

pub fn normalize_task_block_list(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for code in values
        .iter()
        .flat_map(|value| value.split(','))
        .filter_map(canonical_task_blocker_code)
    {
        if !normalized.iter().any(|existing| existing == &code) {
            normalized.push(code);
        }
    }
    normalized
}

#[must_use]
pub fn canonical_task_blocker_code(value: &str) -> Option<String> {
    let value = value.trim();
    let (code, suffix) = value
        .split_once(':')
        .map_or((value, None), |(code, suffix)| (code, Some(suffix.trim())));
    let code = canonical_task_blocker_code_segment(code)?;
    match suffix.filter(|suffix| !suffix.is_empty()) {
        Some(suffix) => Some(format!("{code}:{suffix}")),
        None => Some(code),
    }
}

fn canonical_task_blocker_code_segment(value: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut previous_separator = false;
    for character in value.trim().chars() {
        let next = match character {
            '-' | ' ' | '\t' | '\n' | '\r' => '_',
            other => other.to_ascii_lowercase(),
        };
        if next == '_' {
            if previous_separator {
                continue;
            }
            previous_separator = true;
        } else {
            previous_separator = false;
        }
        normalized.push(next);
    }
    let normalized = normalized.trim_matches('_').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

#[must_use]
pub fn append_task_block_note_with_timestamp(
    existing_notes: Option<&str>,
    reason: &str,
    evidence: &[String],
    blocker_codes: &[String],
    next_actions: &[String],
    recorded_at_unix_nanos: i128,
) -> String {
    let mut note = format!(
        "task_block:\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  reason: {}",
        reason.trim()
    );
    let evidence = evidence
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if evidence.len() == 1 {
        note.push_str("\n  evidence: ");
        note.push_str(evidence[0]);
    } else if !evidence.is_empty() {
        note.push_str("\n  evidence:");
        for item in evidence {
            note.push_str("\n    - ");
            note.push_str(item);
        }
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
    evidence: &[String],
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
    use super::{
        append_task_block_note_with_timestamp, canonical_task_blocker_code,
        normalize_task_block_list,
    };

    #[test]
    fn block_list_splits_commas_and_canonicalizes_entries() {
        let values = normalize_task_block_list(&[
            "Runtime Blocked, bridge-request-pending".to_string(),
            " , proof\tblocked ,runtime_blocked ".to_string(),
        ]);

        assert_eq!(
            values,
            vec![
                "runtime_blocked".to_string(),
                "bridge_request_pending".to_string(),
                "proof_blocked".to_string()
            ]
        );
    }

    #[test]
    fn parameterized_blocker_code_punctuation_is_preserved() {
        assert_eq!(
            canonical_task_blocker_code(" Selected-Lane:task=VH-42 "),
            Some("selected_lane:task=VH-42".to_string())
        );
        assert_eq!(
            canonical_task_blocker_code("Runtime Blocked"),
            Some("runtime_blocked".to_string())
        );
        assert_eq!(canonical_task_blocker_code(" --- "), None);
        assert_eq!(canonical_task_blocker_code("a--b"), Some("a_b".to_string()));
    }

    #[test]
    fn block_note_preserves_existing_notes_and_structured_fields() {
        let note = append_task_block_note_with_timestamp(
            Some("existing"),
            " bridge unavailable ",
            &[" dispatch receipt ".to_string()],
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

    #[test]
    fn block_note_preserves_repeated_evidence_as_list() {
        let note = append_task_block_note_with_timestamp(
            None,
            "bridge unavailable",
            &[
                " receipt-a ".to_string(),
                "receipt-b".to_string(),
                " ".to_string(),
            ],
            &[],
            &[],
            42,
        );

        assert_eq!(
            note,
            "task_block:\n  recorded_at_unix_nanos: 42\n  reason: bridge unavailable\n  evidence:\n    - receipt-a\n    - receipt-b"
        );

        let without_evidence =
            append_task_block_note_with_timestamp(None, "bridge unavailable", &[], &[], &[], 42);
        assert!(!without_evidence.contains("evidence:"));
    }
}
