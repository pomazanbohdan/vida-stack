pub const MODULE: &str = "exception_takeover";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionTakeoverReceipt<'a> {
    pub lane_status: &'a str,
    pub exception_path_receipt_id: Option<&'a str>,
    pub supersedes_receipt_id: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionTakeoverRecovery<'a> {
    pub local_exception_takeover_gate: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionTakeoverStateLabel {
    Active,
    AdmissibleNotActive,
    ReceiptRecorded,
}

impl ExceptionTakeoverStateLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::AdmissibleNotActive => "admissible_not_active",
            Self::ReceiptRecorded => "receipt_recorded",
        }
    }
}

pub fn exception_takeover_state_label(
    latest_receipt: Option<&ExceptionTakeoverReceipt<'_>>,
    latest_recovery: Option<&ExceptionTakeoverRecovery<'_>>,
) -> Option<ExceptionTakeoverStateLabel> {
    let receipt = latest_receipt?;
    if !has_nonempty_value(receipt.exception_path_receipt_id) {
        return None;
    }

    if has_nonempty_value(receipt.supersedes_receipt_id) {
        return Some(ExceptionTakeoverStateLabel::Active);
    }

    let gate_clear = latest_recovery.is_some_and(|recovery| {
        let gate = recovery.local_exception_takeover_gate.trim();
        !gate.is_empty() && gate != "blocked_open_delegated_cycle"
    });

    if gate_clear && receipt.lane_status == "lane_exception_recorded" {
        return Some(ExceptionTakeoverStateLabel::AdmissibleNotActive);
    }
    if receipt.lane_status == "lane_exception_takeover" {
        return Some(ExceptionTakeoverStateLabel::AdmissibleNotActive);
    }

    Some(ExceptionTakeoverStateLabel::ReceiptRecorded)
}

pub fn exception_takeover_is_lawfully_active(
    latest_receipt: Option<&ExceptionTakeoverReceipt<'_>>,
    latest_recovery: Option<&ExceptionTakeoverRecovery<'_>>,
) -> bool {
    exception_takeover_state_label(latest_receipt, latest_recovery)
        == Some(ExceptionTakeoverStateLabel::Active)
}

fn has_nonempty_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        ExceptionTakeoverReceipt, ExceptionTakeoverRecovery, ExceptionTakeoverStateLabel,
        exception_takeover_is_lawfully_active, exception_takeover_state_label,
    };

    fn receipt() -> ExceptionTakeoverReceipt<'static> {
        ExceptionTakeoverReceipt {
            lane_status: "lane_exception_recorded",
            exception_path_receipt_id: Some("receipt-1"),
            supersedes_receipt_id: None,
        }
    }

    fn recovery(gate: &'static str) -> ExceptionTakeoverRecovery<'static> {
        ExceptionTakeoverRecovery {
            local_exception_takeover_gate: gate,
        }
    }

    #[test]
    fn exception_takeover_state_label_rejects_missing_receipt_or_evidence() {
        assert_eq!(exception_takeover_state_label(None, None), None);

        let receipt = ExceptionTakeoverReceipt {
            exception_path_receipt_id: None,
            ..receipt()
        };
        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt),
                Some(&recovery("delegated_cycle_clear"))
            ),
            None
        );
    }

    #[test]
    fn exception_takeover_state_label_keeps_recorded_receipts_blocked_when_gate_blocks() {
        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt()),
                Some(&recovery("blocked_open_delegated_cycle"))
            ),
            Some(ExceptionTakeoverStateLabel::ReceiptRecorded)
        );
    }

    #[test]
    fn exception_takeover_state_label_marks_recorded_clear_gate_admissible_not_active() {
        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt()),
                Some(&recovery("delegated_cycle_clear"))
            ),
            Some(ExceptionTakeoverStateLabel::AdmissibleNotActive)
        );
    }

    #[test]
    fn exception_takeover_state_label_requires_supersession_for_active_takeover() {
        let receipt = ExceptionTakeoverReceipt {
            lane_status: "lane_exception_takeover",
            supersedes_receipt_id: Some("supersede-1"),
            ..receipt()
        };

        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt),
                Some(&recovery("delegated_cycle_clear"))
            ),
            Some(ExceptionTakeoverStateLabel::Active)
        );
        assert!(exception_takeover_is_lawfully_active(
            Some(&receipt),
            Some(&recovery("delegated_cycle_clear"))
        ));
    }

    #[test]
    fn exception_takeover_state_label_fails_closed_without_recovery_or_supersession() {
        let receipt = ExceptionTakeoverReceipt {
            lane_status: "lane_exception_takeover",
            ..receipt()
        };

        assert_eq!(
            exception_takeover_state_label(Some(&receipt), None),
            Some(ExceptionTakeoverStateLabel::AdmissibleNotActive)
        );
        assert!(!exception_takeover_is_lawfully_active(Some(&receipt), None));
    }

    #[test]
    fn exception_takeover_state_label_keeps_blocked_takeover_admissible_not_active() {
        let receipt = ExceptionTakeoverReceipt {
            lane_status: "lane_exception_takeover",
            ..receipt()
        };

        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt),
                Some(&recovery("blocked_open_delegated_cycle"))
            ),
            Some(ExceptionTakeoverStateLabel::AdmissibleNotActive)
        );
    }

    #[test]
    fn exception_takeover_state_label_marks_superseded_receipt_active() {
        let receipt = ExceptionTakeoverReceipt {
            supersedes_receipt_id: Some("supersede-1"),
            ..receipt()
        };

        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt),
                Some(&recovery("blocked_open_delegated_cycle"))
            ),
            Some(ExceptionTakeoverStateLabel::Active)
        );
    }

    #[test]
    fn exception_takeover_state_label_strings_match_operator_contract() {
        assert_eq!(ExceptionTakeoverStateLabel::Active.as_str(), "active");
        assert_eq!(
            ExceptionTakeoverStateLabel::AdmissibleNotActive.as_str(),
            "admissible_not_active"
        );
        assert_eq!(
            ExceptionTakeoverStateLabel::ReceiptRecorded.as_str(),
            "receipt_recorded"
        );
    }
}
