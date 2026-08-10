pub(crate) fn canonical_activation_status(
    status: Option<&str>,
    activation_pending: bool,
) -> &'static str {
    let normalized = status.map(|value| value.trim().to_ascii_lowercase());
    if activation_pending
        || matches!(
            normalized.as_deref(),
            Some("pending") | Some("pending_activation")
        )
    {
        "pending"
    } else {
        "ready_enough_for_normal_work"
    }
}

pub(crate) fn activation_status_is_pending(status: Option<&str>) -> bool {
    matches!(
        status
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("pending") | Some("pending_activation")
    )
}

#[cfg(test)]
mod tests {
    use super::{activation_status_is_pending, canonical_activation_status};

    #[test]
    fn activation_status_normalizes_pending_aliases_and_explicit_flag() {
        assert_eq!(
            canonical_activation_status(None, false),
            "ready_enough_for_normal_work"
        );
        assert_eq!(
            canonical_activation_status(Some(" PENDING "), false),
            "pending"
        );
        assert_eq!(
            canonical_activation_status(Some("pending_activation"), false),
            "pending"
        );
        assert_eq!(canonical_activation_status(Some("ready"), true), "pending");

        assert!(activation_status_is_pending(Some(" PENDING_ACTIVATION ")));
        assert!(!activation_status_is_pending(Some("ready")));
        assert!(!activation_status_is_pending(None));
    }
}
