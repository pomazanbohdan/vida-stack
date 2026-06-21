//! Task takeover command helpers.

use enum_map::{Enum, EnumMap, enum_map};
use strum::IntoStaticStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ExceptionTakeoverStateLabel {
    Active,
    AdmissibleNotActive,
    ReceiptRecorded,
}

impl ExceptionTakeoverStateLabel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ExceptionTakeoverEvidenceCase {
    MissingReceiptEvidence,
    SupersededReceipt,
    RecordedWithClearGate,
    TakeoverLane,
    Recorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionTakeoverDecisionInput<'a> {
    pub lane_status: &'a str,
    pub exception_path_receipt_id: Option<&'a str>,
    pub supersedes_receipt_id: Option<&'a str>,
    pub local_exception_takeover_gate: Option<&'a str>,
}

#[must_use]
pub fn exception_takeover_transition_table()
-> EnumMap<ExceptionTakeoverEvidenceCase, Option<ExceptionTakeoverStateLabel>> {
    enum_map! {
        ExceptionTakeoverEvidenceCase::MissingReceiptEvidence => None,
        ExceptionTakeoverEvidenceCase::SupersededReceipt => Some(ExceptionTakeoverStateLabel::Active),
        ExceptionTakeoverEvidenceCase::RecordedWithClearGate => Some(ExceptionTakeoverStateLabel::AdmissibleNotActive),
        ExceptionTakeoverEvidenceCase::TakeoverLane => Some(ExceptionTakeoverStateLabel::AdmissibleNotActive),
        ExceptionTakeoverEvidenceCase::Recorded => Some(ExceptionTakeoverStateLabel::ReceiptRecorded),
    }
}

#[must_use]
pub fn classify_exception_takeover_evidence(
    input: ExceptionTakeoverDecisionInput<'_>,
) -> ExceptionTakeoverEvidenceCase {
    if !has_nonempty_value(input.exception_path_receipt_id) {
        return ExceptionTakeoverEvidenceCase::MissingReceiptEvidence;
    }
    if has_nonempty_value(input.supersedes_receipt_id) {
        return ExceptionTakeoverEvidenceCase::SupersededReceipt;
    }

    let gate_clear = input.local_exception_takeover_gate.is_some_and(|gate| {
        let gate = gate.trim();
        !gate.is_empty() && gate != "blocked_open_delegated_cycle"
    });
    if gate_clear && input.lane_status == "lane_exception_recorded" {
        return ExceptionTakeoverEvidenceCase::RecordedWithClearGate;
    }
    if input.lane_status == "lane_exception_takeover" {
        return ExceptionTakeoverEvidenceCase::TakeoverLane;
    }

    ExceptionTakeoverEvidenceCase::Recorded
}

#[must_use]
pub fn exception_takeover_state(
    input: ExceptionTakeoverDecisionInput<'_>,
) -> Option<ExceptionTakeoverStateLabel> {
    exception_takeover_transition_table()[classify_exception_takeover_evidence(input)]
}

#[must_use]
pub fn exception_takeover_state_label(active: bool, receipt_recorded: bool) -> &'static str {
    let input = ExceptionTakeoverDecisionInput {
        lane_status: if active {
            "lane_exception_takeover"
        } else if receipt_recorded {
            "lane_exception_recorded"
        } else {
            ""
        },
        exception_path_receipt_id: (active || receipt_recorded).then_some("receipt"),
        supersedes_receipt_id: active.then_some("superseded"),
        local_exception_takeover_gate: None,
    };
    if let Some(state) = exception_takeover_state(input) {
        state.as_str()
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

fn has_nonempty_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        ExceptionTakeoverDecisionInput, ExceptionTakeoverEvidenceCase, ExceptionTakeoverStateLabel,
        classify_exception_takeover_evidence, exception_takeover_state,
        exception_takeover_state_label, root_write_guard_status, takeover_ready_state,
    };

    #[test]
    fn exception_takeover_state_label_prefers_active_over_recorded() {
        assert_eq!(exception_takeover_state_label(false, false), "not_recorded");
        assert_eq!(
            exception_takeover_state_label(false, true),
            "receipt_recorded"
        );
        assert_eq!(exception_takeover_state_label(true, false), "active");
        assert_eq!(exception_takeover_state_label(true, true), "active");
    }

    #[test]
    fn exception_takeover_transition_table_preserves_operator_labels() {
        let recorded = ExceptionTakeoverDecisionInput {
            lane_status: "lane_exception_recorded",
            exception_path_receipt_id: Some("receipt-1"),
            supersedes_receipt_id: None,
            local_exception_takeover_gate: Some("blocked_open_delegated_cycle"),
        };
        assert_eq!(
            classify_exception_takeover_evidence(recorded),
            ExceptionTakeoverEvidenceCase::Recorded
        );
        assert_eq!(
            exception_takeover_state(recorded),
            Some(ExceptionTakeoverStateLabel::ReceiptRecorded)
        );
        assert_eq!(
            exception_takeover_state(recorded).map(ExceptionTakeoverStateLabel::as_str),
            Some("receipt_recorded")
        );

        let active = ExceptionTakeoverDecisionInput {
            supersedes_receipt_id: Some("supersede-1"),
            ..recorded
        };
        assert_eq!(
            exception_takeover_state(active),
            Some(ExceptionTakeoverStateLabel::Active)
        );
        assert_eq!(
            ExceptionTakeoverStateLabel::AdmissibleNotActive.as_str(),
            "admissible_not_active"
        );
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
