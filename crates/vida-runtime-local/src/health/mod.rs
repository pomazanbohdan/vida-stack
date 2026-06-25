use taskflow_state_redb::{RedbJournalHealth, RedbProjectionFailureRecord};
use vida_contracts::{
    CompletionOutcome, ProjectionDriftClass, ProjectionDriftFinding, ProjectionRepairPlan,
    ProjectionRepairReceipt,
};

pub const JUNE_2026_PASS_RESULT_BLOCKED_RUN: &str = "june_2026_pass_result_blocked_run";
pub const PROJECTION_FAILURE_RECORDED: &str = "projection_failure_recorded";
pub const SAFE_PROJECTION_LAG: &str = "safe_projection_lag";
pub const PASS_RESULT_LEGACY_CONTRADICTION: &str = "pass_result_legacy_contradiction";
pub const UNREPAIRABLE_PROJECTION_FAILURE: &str = "unrepairable_projection_failure";
pub const REPAIR_PLAN_REQUIRES_EVENT_BACKING: &str = "repair_plan_requires_event_backing";
pub const REPAIR_PLAN_REQUIRES_CANONICAL_PASSED_EVIDENCE: &str =
    "repair_plan_requires_canonical_passed_evidence";
pub const REPAIR_PLAN_REQUIRES_AUTHORIZED_REPAIR_CLASS: &str =
    "repair_plan_requires_authorized_repair_class";
pub const VERIFY_EXISTING_SOURCE_EVENT_CURSOR: &str = "verify_existing_source_event_cursor";
pub const REBUILD_PROJECTION_FROM_EXISTING_JOURNAL_EVENTS: &str =
    "rebuild_projection_from_existing_journal_events";
pub const REBUILD_LAGGING_PROJECTION_FROM_EXISTING_JOURNAL_EVENTS: &str =
    "rebuild_lagging_projection_from_existing_journal_events";
pub const VALIDATE_CANONICAL_COMPLETION_OUTCOME_PASSED: &str =
    "validate_canonical_completion_outcome_passed";
pub const REPAIR_LEGACY_PASS_RESULT_PROJECTION_CONTRADICTION: &str =
    "repair_legacy_pass_result_projection_contradiction";
pub const REPORT_UNREPAIRABLE_PROJECTION_FAILURE: &str = "report_unrepairable_projection_failure";

pub fn classify_projection_drift(
    _health: &RedbJournalHealth,
    failures: &[RedbProjectionFailureRecord],
) -> Vec<ProjectionDriftFinding> {
    failures
        .iter()
        .map(|failure| {
            let haystack = format!(
                "{} {} {}",
                failure.failure_kind,
                failure.failure_message,
                failure.repair_plan_ref.clone().unwrap_or_default()
            )
            .to_ascii_lowercase();
            let drift_class = if failure.source_event_cursor.is_none() {
                ProjectionDriftClass::UnrepairableProjectionFailure
            } else if haystack.contains("pass-result") && haystack.contains("blocked-run") {
                ProjectionDriftClass::June2026PassResultBlockedRun
            } else if haystack.contains("legacy")
                && haystack.contains("passed")
                && haystack.contains("blocked")
            {
                ProjectionDriftClass::PassResultLegacyContradiction
            } else if haystack.contains("lag") || haystack.contains("behind") {
                ProjectionDriftClass::SafeProjectionLag
            } else {
                ProjectionDriftClass::ProjectionFailureRecorded
            };
            ProjectionDriftFinding {
                drift_class,
                blocker_code: blocker_code(drift_class).to_string(),
                projection_id: failure.projection_id.0.clone(),
                source_event_cursor: failure.source_event_cursor.clone(),
                failure_hash: failure.content_hash.clone(),
            }
        })
        .collect()
}

pub fn plan_projection_repair(finding: &ProjectionDriftFinding) -> ProjectionRepairPlan {
    ProjectionRepairPlan {
        plan_id: format!("projection-repair:{}", finding.failure_hash),
        drift_class: finding.drift_class,
        state_mutation_allowed: false,
        required_existing_event_cursors: finding.source_event_cursor.clone().into_iter().collect(),
        actions: repair_actions(finding.drift_class),
        auto_repair_allowed: matches!(finding.drift_class, ProjectionDriftClass::SafeProjectionLag)
            && finding.source_event_cursor.is_some(),
        canonical_passed_evidence_required: matches!(
            finding.drift_class,
            ProjectionDriftClass::June2026PassResultBlockedRun
                | ProjectionDriftClass::PassResultLegacyContradiction
        ),
    }
}

pub fn apply_projection_repair_plan(
    plan: &ProjectionRepairPlan,
    idempotency_key: impl Into<String>,
    before_health: &RedbJournalHealth,
    after_health: &RedbJournalHealth,
    canonical_outcome: Option<&CompletionOutcome>,
) -> Result<ProjectionRepairReceipt, &'static str> {
    if plan.required_existing_event_cursors.is_empty() {
        return Err(REPAIR_PLAN_REQUIRES_EVENT_BACKING);
    }
    if plan.canonical_passed_evidence_required && !canonical_passed_evidence_gate(canonical_outcome)
    {
        return Err(REPAIR_PLAN_REQUIRES_CANONICAL_PASSED_EVIDENCE);
    }
    if !repair_plan_has_authorized_apply_path(plan, canonical_outcome) {
        return Err(REPAIR_PLAN_REQUIRES_AUTHORIZED_REPAIR_CLASS);
    }
    Ok(ProjectionRepairReceipt {
        plan_id: plan.plan_id.clone(),
        applied: true,
        idempotency_key: idempotency_key.into(),
        event_backing_cursors: plan.required_existing_event_cursors.clone(),
        applied_actions: plan.actions.clone(),
        before_health_hash: health_receipt_hash(before_health),
        after_health_hash: health_receipt_hash(after_health),
    })
}

pub fn guarded_projection_repair(
    health: &RedbJournalHealth,
    failures: &[RedbProjectionFailureRecord],
    idempotency_key: impl Into<String>,
    canonical_outcome: Option<&CompletionOutcome>,
) -> Result<Option<ProjectionRepairReceipt>, &'static str> {
    let Some(finding) = classify_projection_drift(health, failures)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let plan = plan_projection_repair(&finding);
    apply_projection_repair_plan(&plan, idempotency_key, health, health, canonical_outcome)
        .map(Some)
}

fn blocker_code(drift_class: ProjectionDriftClass) -> &'static str {
    match drift_class {
        ProjectionDriftClass::June2026PassResultBlockedRun => JUNE_2026_PASS_RESULT_BLOCKED_RUN,
        ProjectionDriftClass::ProjectionFailureRecorded => PROJECTION_FAILURE_RECORDED,
        ProjectionDriftClass::SafeProjectionLag => SAFE_PROJECTION_LAG,
        ProjectionDriftClass::PassResultLegacyContradiction => PASS_RESULT_LEGACY_CONTRADICTION,
        ProjectionDriftClass::UnrepairableProjectionFailure => UNREPAIRABLE_PROJECTION_FAILURE,
    }
}

pub fn canonical_passed_evidence_gate(outcome: Option<&CompletionOutcome>) -> bool {
    matches!(
        outcome,
        Some(CompletionOutcome::Passed { evidence_refs, .. }) if !evidence_refs.is_empty()
    )
}

fn repair_plan_has_authorized_apply_path(
    plan: &ProjectionRepairPlan,
    canonical_outcome: Option<&CompletionOutcome>,
) -> bool {
    match plan.drift_class {
        ProjectionDriftClass::SafeProjectionLag => plan.auto_repair_allowed,
        ProjectionDriftClass::June2026PassResultBlockedRun
        | ProjectionDriftClass::PassResultLegacyContradiction => {
            plan.canonical_passed_evidence_required
                && canonical_passed_evidence_gate(canonical_outcome)
        }
        ProjectionDriftClass::ProjectionFailureRecorded
        | ProjectionDriftClass::UnrepairableProjectionFailure => false,
    }
}

fn repair_actions(drift_class: ProjectionDriftClass) -> Vec<String> {
    match drift_class {
        ProjectionDriftClass::SafeProjectionLag => vec![
            VERIFY_EXISTING_SOURCE_EVENT_CURSOR.to_string(),
            REBUILD_LAGGING_PROJECTION_FROM_EXISTING_JOURNAL_EVENTS.to_string(),
        ],
        ProjectionDriftClass::June2026PassResultBlockedRun
        | ProjectionDriftClass::PassResultLegacyContradiction => vec![
            VERIFY_EXISTING_SOURCE_EVENT_CURSOR.to_string(),
            VALIDATE_CANONICAL_COMPLETION_OUTCOME_PASSED.to_string(),
            REPAIR_LEGACY_PASS_RESULT_PROJECTION_CONTRADICTION.to_string(),
        ],
        ProjectionDriftClass::ProjectionFailureRecorded => vec![
            VERIFY_EXISTING_SOURCE_EVENT_CURSOR.to_string(),
            REBUILD_PROJECTION_FROM_EXISTING_JOURNAL_EVENTS.to_string(),
        ],
        ProjectionDriftClass::UnrepairableProjectionFailure => {
            vec![REPORT_UNREPAIRABLE_PROJECTION_FAILURE.to_string()]
        }
    }
}

fn health_receipt_hash(health: &RedbJournalHealth) -> String {
    format!(
        "redb-health:{}:{}:{}:{}",
        health.schema_version,
        health.global_event_count,
        health.projection_checkpoint_count,
        health.projection_failure_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vida_contracts::{VidaArtifactRef, VidaEventCursor, VidaProjectionRef, VidaStreamRef};

    fn empty_health() -> RedbJournalHealth {
        RedbJournalHealth {
            schema_version: "1".to_string(),
            stream_event_count: 0,
            global_event_count: 0,
            append_idempotency_count: 0,
            idempotency_count: 0,
            outbox_pending_count: 0,
            outbox_claimed_count: 0,
            outbox_succeeded_count: 0,
            outbox_failed_count: 0,
            projection_checkpoint_count: 0,
            projection_failure_count: 0,
            artifact_count: 0,
        }
    }

    fn failure(message: &str, cursor: Option<&str>) -> RedbProjectionFailureRecord {
        RedbProjectionFailureRecord {
            projection_id: VidaProjectionRef("projection-1".to_string()),
            stream_id: VidaStreamRef("stream-1".to_string()),
            source_event_cursor: cursor.map(|value| VidaEventCursor(value.to_string())),
            failure_kind: "projection_rebuild_failed".to_string(),
            failure_message: message.to_string(),
            retry_after: None,
            repair_plan_ref: Some("redb-projection-repair-plan".to_string()),
            content_hash: "failure-hash-1".to_string(),
        }
    }

    #[test]
    fn detects_june_2026_pass_result_blocked_run_drift_class() {
        let health = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let findings = classify_projection_drift(
            &health,
            &[failure(
                "June 2026 pass-result/blocked-run projection drift",
                Some("global-7"),
            )],
        );
        assert_eq!(
            findings[0].drift_class,
            ProjectionDriftClass::June2026PassResultBlockedRun
        );
        assert_eq!(findings[0].blocker_code, JUNE_2026_PASS_RESULT_BLOCKED_RUN);
    }

    #[test]
    fn planning_is_pure_and_disallows_state_mutation() {
        let health = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let finding =
            classify_projection_drift(&health, &[failure("projector failed", Some("global-7"))])
                .remove(0);
        let first = plan_projection_repair(&finding);
        let second = plan_projection_repair(&finding);
        assert_eq!(first, second);
        assert!(!first.state_mutation_allowed);
    }

    #[test]
    fn safe_projection_lag_applies_idempotently_with_before_after_health_receipt() {
        let before = empty_health();
        let after = RedbJournalHealth {
            projection_checkpoint_count: 1,
            ..empty_health()
        };
        let plan = plan_projection_repair(&ProjectionDriftFinding {
            drift_class: ProjectionDriftClass::SafeProjectionLag,
            blocker_code: SAFE_PROJECTION_LAG.to_string(),
            projection_id: "projection-1".to_string(),
            source_event_cursor: Some(VidaEventCursor("global-7".to_string())),
            failure_hash: "failure-hash-1".to_string(),
        });
        let receipt = apply_projection_repair_plan(&plan, "idem-1", &before, &after, None).unwrap();
        assert_eq!(
            receipt,
            apply_projection_repair_plan(&plan, "idem-1", &before, &after, None).unwrap()
        );
        assert_ne!(receipt.before_health_hash, receipt.after_health_hash);
    }

    #[test]
    fn apply_rejects_impossible_synthetic_domain_events() {
        let plan = ProjectionRepairPlan {
            plan_id: "projection-repair:missing-event".to_string(),
            drift_class: ProjectionDriftClass::ProjectionFailureRecorded,
            state_mutation_allowed: false,
            required_existing_event_cursors: Vec::new(),
            actions: Vec::new(),
            auto_repair_allowed: false,
            canonical_passed_evidence_required: false,
        };
        assert_eq!(
            apply_projection_repair_plan(&plan, "idem-1", &empty_health(), &empty_health(), None),
            Err(REPAIR_PLAN_REQUIRES_EVENT_BACKING)
        );
    }

    #[test]
    fn guarded_repair_returns_no_receipt_for_clean_query_health() {
        let health = empty_health();
        assert_eq!(
            guarded_projection_repair(&health, &[], "query-only", None).unwrap(),
            None
        );
    }

    #[test]
    fn guarded_repair_receipt_links_before_after_health_and_actions() {
        let before = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let outcome =
            CompletionOutcome::passed(vec![VidaArtifactRef("completion-proof".to_string())], None);
        let receipt = guarded_projection_repair(
            &before,
            &[failure(
                "June 2026 pass-result/blocked-run projection drift",
                Some("global-7"),
            )],
            "idem-1",
            Some(&outcome),
        )
        .unwrap()
        .expect("drift should produce a guarded repair receipt");

        assert_eq!(
            receipt.event_backing_cursors,
            vec![VidaEventCursor("global-7".to_string())]
        );
        assert_eq!(
            receipt.applied_actions,
            vec![
                VERIFY_EXISTING_SOURCE_EVENT_CURSOR.to_string(),
                VALIDATE_CANONICAL_COMPLETION_OUTCOME_PASSED.to_string(),
                REPAIR_LEGACY_PASS_RESULT_PROJECTION_CONTRADICTION.to_string(),
            ]
        );
        assert_eq!(receipt.before_health_hash, receipt.after_health_hash);
        assert!(!receipt.before_health_hash.is_empty());
    }

    #[test]
    fn classification_matrix_distinguishes_safe_lag_legacy_and_unrepairable_failures() {
        let health = RedbJournalHealth {
            projection_failure_count: 3,
            ..empty_health()
        };
        let findings = classify_projection_drift(
            &health,
            &[
                failure("projection lag behind global journal", Some("global-7")),
                failure("legacy passed completion remains blocked", Some("global-8")),
                failure("missing domain evidence for rebuild", None),
            ],
        );

        assert_eq!(
            findings[0].drift_class,
            ProjectionDriftClass::SafeProjectionLag
        );
        assert_eq!(
            findings[1].drift_class,
            ProjectionDriftClass::PassResultLegacyContradiction
        );
        assert_eq!(
            findings[2].drift_class,
            ProjectionDriftClass::UnrepairableProjectionFailure
        );
    }

    #[test]
    fn safe_projection_lag_is_the_only_automatic_repair_class() {
        let health = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let safe_finding = classify_projection_drift(
            &health,
            &[failure("projection lag behind", Some("global-7"))],
        )
        .remove(0);
        let safe_plan = plan_projection_repair(&safe_finding);
        assert!(safe_plan.auto_repair_allowed);
        assert!(!safe_plan.canonical_passed_evidence_required);

        let generic_plan = plan_projection_repair(&ProjectionDriftFinding {
            drift_class: ProjectionDriftClass::ProjectionFailureRecorded,
            blocker_code: PROJECTION_FAILURE_RECORDED.to_string(),
            projection_id: "projection-1".to_string(),
            source_event_cursor: Some(VidaEventCursor("global-7".to_string())),
            failure_hash: "failure-hash-1".to_string(),
        });
        assert!(!generic_plan.auto_repair_allowed);
        assert_eq!(
            apply_projection_repair_plan(
                &generic_plan,
                "idem-1",
                &empty_health(),
                &empty_health(),
                None
            ),
            Err(REPAIR_PLAN_REQUIRES_AUTHORIZED_REPAIR_CLASS)
        );
    }

    #[test]
    fn pass_result_repair_requires_canonical_passed_evidence() {
        let health = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let finding = classify_projection_drift(
            &health,
            &[failure(
                "legacy passed completion remains blocked",
                Some("global-7"),
            )],
        )
        .remove(0);
        let plan = plan_projection_repair(&finding);
        assert!(plan.canonical_passed_evidence_required);
        assert!(!plan.auto_repair_allowed);
        assert_eq!(
            apply_projection_repair_plan(&plan, "idem-1", &health, &health, None),
            Err(REPAIR_PLAN_REQUIRES_CANONICAL_PASSED_EVIDENCE)
        );

        let empty_evidence = CompletionOutcome::passed(Vec::new(), None);
        assert_eq!(
            apply_projection_repair_plan(&plan, "idem-1", &health, &health, Some(&empty_evidence)),
            Err(REPAIR_PLAN_REQUIRES_CANONICAL_PASSED_EVIDENCE)
        );

        let passed =
            CompletionOutcome::passed(vec![VidaArtifactRef("completion-proof".to_string())], None);
        assert!(
            apply_projection_repair_plan(&plan, "idem-1", &health, &health, Some(&passed)).is_ok()
        );
    }

    #[test]
    fn classification_and_planning_do_not_mutate_query_inputs() {
        let health = RedbJournalHealth {
            projection_failure_count: 1,
            ..empty_health()
        };
        let failures = vec![failure("projection lag behind", Some("global-7"))];
        let original_health = health.clone();
        let original_failures = failures.clone();

        let findings = classify_projection_drift(&health, &failures);
        let _plan = plan_projection_repair(&findings[0]);

        assert_eq!(health, original_health);
        assert_eq!(failures, original_failures);
    }
}
