pub const MODULE: &str = "exception_takeover";

pub use taskflow_core::task::takeover::ExceptionTakeoverStateLabel;

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

pub fn exception_takeover_state_label(
    latest_receipt: Option<&ExceptionTakeoverReceipt<'_>>,
    latest_recovery: Option<&ExceptionTakeoverRecovery<'_>>,
) -> Option<ExceptionTakeoverStateLabel> {
    let receipt = latest_receipt?;
    taskflow_core::task::takeover::exception_takeover_state(
        taskflow_core::task::takeover::ExceptionTakeoverDecisionInput {
            lane_status: receipt.lane_status,
            exception_path_receipt_id: receipt.exception_path_receipt_id,
            supersedes_receipt_id: receipt.supersedes_receipt_id,
            local_exception_takeover_gate: latest_recovery
                .map(|recovery| recovery.local_exception_takeover_gate),
        },
    )
}

pub fn exception_takeover_is_lawfully_active(
    latest_receipt: Option<&ExceptionTakeoverReceipt<'_>>,
    latest_recovery: Option<&ExceptionTakeoverRecovery<'_>>,
) -> bool {
    exception_takeover_state_label(latest_receipt, latest_recovery)
        == Some(ExceptionTakeoverStateLabel::Active)
}

#[cfg(test)]
mod tests {
    use super::{
        ExceptionTakeoverReceipt, ExceptionTakeoverRecovery, ExceptionTakeoverStateLabel,
        exception_takeover_is_lawfully_active, exception_takeover_state_label,
    };
    use proptest::prelude::*;

    fn lane_status_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just(""),
            Just("lane_exception_recorded"),
            Just("lane_exception_takeover"),
            Just("lane_completed"),
            Just("unknown_lane_status"),
        ]
    }

    fn receipt_id_strategy() -> impl Strategy<Value = Option<&'static str>> {
        prop_oneof![
            Just(None),
            Just(Some("")),
            Just(Some("   ")),
            Just(Some("receipt-1"))
        ]
    }

    fn recovery_gate_strategy() -> impl Strategy<Value = Option<&'static str>> {
        prop_oneof![
            Just(None),
            Just(Some("")),
            Just(Some("   ")),
            Just(Some("blocked_open_delegated_cycle")),
            Just(Some("delegated_cycle_clear")),
        ]
    }

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

    proptest! {
        #[test]
        fn exception_takeover_authority_transition_matches_core_for_generated_receipts(
            lane_status in lane_status_strategy(),
            receipt_id in receipt_id_strategy(),
            supersedes_id in receipt_id_strategy(),
            gate in recovery_gate_strategy(),
        ) {
            let receipt = ExceptionTakeoverReceipt {
                lane_status,
                exception_path_receipt_id: receipt_id,
                supersedes_receipt_id: supersedes_id,
            };
            let recovery = gate.map(|local_exception_takeover_gate| ExceptionTakeoverRecovery {
                local_exception_takeover_gate,
            });
            let expected = taskflow_core::task::takeover::exception_takeover_state(
                taskflow_core::task::takeover::ExceptionTakeoverDecisionInput {
                    lane_status,
                    exception_path_receipt_id: receipt_id,
                    supersedes_receipt_id: supersedes_id,
                    local_exception_takeover_gate: gate,
                },
            );

            prop_assert_eq!(
                exception_takeover_state_label(Some(&receipt), recovery.as_ref()),
                expected
            );
            prop_assert_eq!(
                exception_takeover_is_lawfully_active(Some(&receipt), recovery.as_ref()),
                expected == Some(ExceptionTakeoverStateLabel::Active)
            );
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
    fn exception_takeover_state_label_rejects_completed_lane_as_active() {
        let receipt = ExceptionTakeoverReceipt {
            lane_status: "lane_completed",
            supersedes_receipt_id: Some("supersede-1"),
            ..receipt()
        };

        assert_eq!(
            exception_takeover_state_label(
                Some(&receipt),
                Some(&recovery("delegated_cycle_clear"))
            ),
            Some(ExceptionTakeoverStateLabel::AdmissibleNotActive)
        );
        assert!(!exception_takeover_is_lawfully_active(
            Some(&receipt),
            Some(&recovery("delegated_cycle_clear"))
        ));
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
