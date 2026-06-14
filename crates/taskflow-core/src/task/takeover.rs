//! Task takeover command helpers.

#[must_use]
pub fn exception_takeover_state_label(active: bool, receipt_recorded: bool) -> &'static str {
    if active {
        "active"
    } else if receipt_recorded {
        "receipt_recorded"
    } else {
        "not_recorded"
    }
}

#[must_use]
pub fn takeover_ready_state(
    allowed: bool,
    receipt_recorded: bool,
    task_matches_lane: bool,
) -> &'static str {
    if allowed {
        "active"
    } else if receipt_recorded {
        "supersession_required"
    } else if task_matches_lane {
        "not_ready"
    } else {
        "stale_task_blocked"
    }
}

#[must_use]
pub fn root_write_guard_status(root_local_write_allowed: bool) -> &'static str {
    if root_local_write_allowed {
        "exception_takeover_active"
    } else {
        "blocked_by_default"
    }
}

#[cfg(test)]
mod tests {
    use super::{exception_takeover_state_label, root_write_guard_status, takeover_ready_state};

    #[test]
    fn exception_takeover_state_label_prefers_active_over_recorded() {
        assert_eq!(exception_takeover_state_label(false, false), "not_recorded");
        assert_eq!(
            exception_takeover_state_label(false, true),
            "receipt_recorded"
        );
        assert_eq!(exception_takeover_state_label(true, true), "active");
    }

    #[test]
    fn takeover_ready_state_reports_operator_next_gate() {
        assert_eq!(takeover_ready_state(true, true, true), "active");
        assert_eq!(
            takeover_ready_state(false, true, true),
            "supersession_required"
        );
        assert_eq!(takeover_ready_state(false, false, true), "not_ready");
        assert_eq!(
            takeover_ready_state(false, false, false),
            "stale_task_blocked"
        );
    }

    #[test]
    fn root_write_guard_status_tracks_write_allowance() {
        assert_eq!(root_write_guard_status(true), "exception_takeover_active");
        assert_eq!(root_write_guard_status(false), "blocked_by_default");
    }
}
