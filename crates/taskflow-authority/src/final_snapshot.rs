pub const MODULE: &str = "final_snapshot";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalSnapshotReleaseAdmission<'a> {
    pub admitted: bool,
    pub blockers_empty: bool,
    pub status: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalSnapshotCandidate<'a> {
    pub file_name: &'a str,
    pub release_admission: Option<FinalSnapshotReleaseAdmission<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalConsumeContinueSnapshot<'a> {
    pub file_name: &'a str,
    pub surface: &'a str,
    pub status: &'a str,
    pub top_level_next_actions_empty: bool,
    pub operator_next_actions_empty: bool,
    pub blockers_empty: bool,
    pub deferred_handoff_projection: bool,
    pub source_run_id: Option<&'a str>,
}

pub fn final_snapshot_has_admissible_release_admission(
    release_admission: Option<&FinalSnapshotReleaseAdmission<'_>>,
) -> bool {
    let Some(release_admission) = release_admission else {
        return false;
    };

    release_admission.admitted
        && release_admission.blockers_empty
        && !matches!(release_admission.status, "" | "block" | "blocked")
}

pub fn recorded_final_snapshot_candidate(candidate: &FinalSnapshotCandidate<'_>) -> bool {
    candidate.file_name.starts_with("final-")
}

pub fn admissible_final_snapshot_candidate(candidate: &FinalSnapshotCandidate<'_>) -> bool {
    recorded_final_snapshot_candidate(candidate)
        && final_snapshot_has_admissible_release_admission(candidate.release_admission.as_ref())
}

pub fn terminal_consume_continue_snapshot_run_id(
    snapshot: &TerminalConsumeContinueSnapshot<'_>,
) -> Option<String> {
    if !snapshot.file_name.starts_with("final-") {
        return None;
    }
    if snapshot.surface != "vida taskflow consume continue" {
        return None;
    }
    if snapshot.status != "pass" {
        return None;
    }

    let clean_terminal = snapshot.top_level_next_actions_empty
        && snapshot.operator_next_actions_empty
        && snapshot.blockers_empty;
    let deferred_handoff_terminal = snapshot.deferred_handoff_projection && snapshot.blockers_empty;

    if !(clean_terminal || deferred_handoff_terminal) {
        return None;
    }

    snapshot
        .source_run_id
        .map(str::trim)
        .filter(|run_id| !run_id.is_empty())
        .map(str::to_string)
}

pub fn final_snapshot_dispatch_receipt_authority_is_persisted(
    snapshot_dispatch_run_id: Option<&str>,
    latest_status_run_id: Option<&str>,
    persisted_receipt_run_id: Option<&str>,
) -> bool {
    let Some(snapshot_dispatch_run_id) = nonempty(snapshot_dispatch_run_id) else {
        return false;
    };
    let Some(latest_status_run_id) = nonempty(latest_status_run_id) else {
        return false;
    };
    let Some(persisted_receipt_run_id) = nonempty(persisted_receipt_run_id) else {
        return false;
    };

    snapshot_dispatch_run_id == latest_status_run_id
        && persisted_receipt_run_id == latest_status_run_id
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        FinalSnapshotCandidate, FinalSnapshotReleaseAdmission, TerminalConsumeContinueSnapshot,
        admissible_final_snapshot_candidate,
        final_snapshot_dispatch_receipt_authority_is_persisted,
        final_snapshot_has_admissible_release_admission, recorded_final_snapshot_candidate,
        terminal_consume_continue_snapshot_run_id,
    };

    fn release_admission() -> FinalSnapshotReleaseAdmission<'static> {
        FinalSnapshotReleaseAdmission {
            admitted: true,
            blockers_empty: true,
            status: "pass",
        }
    }

    fn terminal_continue_snapshot() -> TerminalConsumeContinueSnapshot<'static> {
        TerminalConsumeContinueSnapshot {
            file_name: "final-terminal-continue.json",
            surface: "vida taskflow consume continue",
            status: "pass",
            top_level_next_actions_empty: true,
            operator_next_actions_empty: true,
            blockers_empty: true,
            deferred_handoff_projection: false,
            source_run_id: Some("run-terminal"),
        }
    }

    #[test]
    fn final_snapshot_release_admission_accepts_clean_pass() {
        assert!(final_snapshot_has_admissible_release_admission(Some(
            &release_admission()
        )));
    }

    #[test]
    fn final_snapshot_release_admission_rejects_missing_blocked_or_incomplete() {
        assert!(!final_snapshot_has_admissible_release_admission(None));

        let blocked = FinalSnapshotReleaseAdmission {
            status: "blocked",
            ..release_admission()
        };
        assert!(!final_snapshot_has_admissible_release_admission(Some(
            &blocked
        )));

        let with_blockers = FinalSnapshotReleaseAdmission {
            blockers_empty: false,
            ..release_admission()
        };
        assert!(!final_snapshot_has_admissible_release_admission(Some(
            &with_blockers
        )));

        let not_admitted = FinalSnapshotReleaseAdmission {
            admitted: false,
            ..release_admission()
        };
        assert!(!final_snapshot_has_admissible_release_admission(Some(
            &not_admitted
        )));
    }

    #[test]
    fn final_snapshot_candidate_requires_final_prefix_and_release_admission() {
        let candidate = FinalSnapshotCandidate {
            file_name: "final-admissible.json",
            release_admission: Some(release_admission()),
        };
        assert!(recorded_final_snapshot_candidate(&candidate));
        assert!(admissible_final_snapshot_candidate(&candidate));

        let non_final = FinalSnapshotCandidate {
            file_name: "continue-admissible.json",
            ..candidate
        };
        assert!(!recorded_final_snapshot_candidate(&non_final));
        assert!(!admissible_final_snapshot_candidate(&non_final));

        let missing_admission = FinalSnapshotCandidate {
            release_admission: None,
            ..candidate
        };
        assert!(recorded_final_snapshot_candidate(&missing_admission));
        assert!(!admissible_final_snapshot_candidate(&missing_admission));
    }

    #[test]
    fn terminal_consume_continue_snapshot_accepts_clean_terminal_continue() {
        assert_eq!(
            terminal_consume_continue_snapshot_run_id(&terminal_continue_snapshot()),
            Some("run-terminal".to_string())
        );
    }

    #[test]
    fn terminal_consume_continue_snapshot_accepts_deferred_handoff_projection() {
        let snapshot = TerminalConsumeContinueSnapshot {
            top_level_next_actions_empty: false,
            operator_next_actions_empty: false,
            deferred_handoff_projection: true,
            source_run_id: Some("run-deferred-handoff"),
            ..terminal_continue_snapshot()
        };

        assert_eq!(
            terminal_consume_continue_snapshot_run_id(&snapshot),
            Some("run-deferred-handoff".to_string())
        );
    }

    #[test]
    fn terminal_consume_continue_snapshot_rejects_blocked_or_actionable_outputs() {
        let blocked = TerminalConsumeContinueSnapshot {
            status: "blocked",
            ..terminal_continue_snapshot()
        };
        assert_eq!(terminal_consume_continue_snapshot_run_id(&blocked), None);

        let actionable = TerminalConsumeContinueSnapshot {
            top_level_next_actions_empty: false,
            operator_next_actions_empty: true,
            deferred_handoff_projection: false,
            ..terminal_continue_snapshot()
        };
        assert_eq!(terminal_consume_continue_snapshot_run_id(&actionable), None);

        let missing_run = TerminalConsumeContinueSnapshot {
            source_run_id: Some("   "),
            ..terminal_continue_snapshot()
        };
        assert_eq!(
            terminal_consume_continue_snapshot_run_id(&missing_run),
            None
        );
    }

    #[test]
    fn final_snapshot_dispatch_receipt_authority_requires_persisted_matching_receipt() {
        assert!(final_snapshot_dispatch_receipt_authority_is_persisted(
            Some("run-1"),
            Some("run-1"),
            Some("run-1")
        ));
        assert!(!final_snapshot_dispatch_receipt_authority_is_persisted(
            None,
            Some("run-1"),
            Some("run-1")
        ));
        assert!(!final_snapshot_dispatch_receipt_authority_is_persisted(
            Some("run-1"),
            None,
            Some("run-1")
        ));
        assert!(!final_snapshot_dispatch_receipt_authority_is_persisted(
            Some("run-1"),
            Some("run-1"),
            None
        ));
        assert!(!final_snapshot_dispatch_receipt_authority_is_persisted(
            Some("forged-run"),
            Some("run-1"),
            Some("run-1")
        ));
        assert!(!final_snapshot_dispatch_receipt_authority_is_persisted(
            Some("run-1"),
            Some("run-1"),
            Some("other-run")
        ));
    }
}
