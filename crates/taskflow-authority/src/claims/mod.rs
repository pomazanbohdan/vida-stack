use crate::scheduler_claim::{
    OrchestratorClaimActiveInput, OrchestratorClaimRequestInput, claim_is_expired,
    decide_orchestrator_claim_conflict, validate_orchestrator_claim_request,
};
use time::{Duration, OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub const MODULE: &str = "claims";
pub const CLAIM_AGGREGATE_SCHEMA_VERSION: u32 = 1;
const DEFAULT_ACQUIRE_LEASE_SECONDS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ClaimLeaseCommand {
    Acquire {
        request: OrchestratorClaimRequestInput,
    },
    Renew {
        claim_id: String,
        lease_expires_at: String,
    },
    Block {
        claim_id: String,
        blocker_code: String,
    },
    Release {
        claim_id: String,
    },
    Expire {
        claim_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimLeaseEvent {
    Acquired {
        claim_id: String,
        lease_expires_at: String,
    },
    Renewed {
        claim_id: String,
        lease_expires_at: String,
    },
    Blocked {
        claim_id: String,
        blocker_code: String,
    },
    Released {
        claim_id: String,
    },
    Expired {
        claim_id: String,
    },
    Rejected {
        claim_id: String,
        blocker_code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimLeaseDecision {
    pub schema_version: u32,
    pub admitted: bool,
    pub events: Vec<ClaimLeaseEvent>,
    pub blocker_codes: Vec<String>,
}

impl ClaimLeaseDecision {
    fn admitted(events: Vec<ClaimLeaseEvent>) -> Self {
        Self {
            schema_version: CLAIM_AGGREGATE_SCHEMA_VERSION,
            admitted: true,
            events,
            blocker_codes: Vec::new(),
        }
    }

    fn rejected(claim_id: impl Into<String>, blocker_code: impl Into<String>) -> Self {
        let claim_id = claim_id.into();
        let blocker_code = blocker_code.into();
        Self {
            schema_version: CLAIM_AGGREGATE_SCHEMA_VERSION,
            admitted: false,
            events: vec![ClaimLeaseEvent::Rejected {
                claim_id,
                blocker_code: blocker_code.clone(),
            }],
            blocker_codes: vec![blocker_code],
        }
    }
}

#[must_use]
pub fn decide_claim_lease(
    command: ClaimLeaseCommand,
    active_claims: &[OrchestratorClaimActiveInput],
    now: &str,
) -> ClaimLeaseDecision {
    match command {
        ClaimLeaseCommand::Acquire { request } => decide_acquire(request, active_claims, now),
        ClaimLeaseCommand::Renew {
            claim_id,
            lease_expires_at,
        } => ClaimLeaseDecision::admitted(vec![ClaimLeaseEvent::Renewed {
            claim_id,
            lease_expires_at,
        }]),
        ClaimLeaseCommand::Block {
            claim_id,
            blocker_code,
        } => ClaimLeaseDecision::admitted(vec![ClaimLeaseEvent::Blocked {
            claim_id,
            blocker_code,
        }]),
        ClaimLeaseCommand::Release { claim_id } => {
            ClaimLeaseDecision::admitted(vec![ClaimLeaseEvent::Released { claim_id }])
        }
        ClaimLeaseCommand::Expire { claim_id } => {
            ClaimLeaseDecision::admitted(vec![ClaimLeaseEvent::Expired { claim_id }])
        }
    }
}

fn decide_acquire(
    request: OrchestratorClaimRequestInput,
    active_claims: &[OrchestratorClaimActiveInput],
    now: &str,
) -> ClaimLeaseDecision {
    if let Err(blocker_code) = validate_orchestrator_claim_request(&request) {
        return ClaimLeaseDecision::rejected(request.claim_id, blocker_code);
    }

    for claim in active_claims
        .iter()
        .filter(|claim| !claim_is_expired(claim, now))
    {
        if let Some(conflict) = decide_orchestrator_claim_conflict(&request, claim) {
            return ClaimLeaseDecision::rejected(request.claim_id, conflict.blocker_code);
        }
    }

    let Ok(lease_expires_at) = acquire_lease_expires_at(now) else {
        return ClaimLeaseDecision::rejected(request.claim_id, "orchestrator_claim_invalid_now");
    };

    ClaimLeaseDecision::admitted(vec![ClaimLeaseEvent::Acquired {
        claim_id: request.claim_id,
        lease_expires_at,
    }])
}

fn acquire_lease_expires_at(now: &str) -> Result<String, ()> {
    let expires_at = OffsetDateTime::parse(now, &Rfc3339).map_err(|_| ())?
        + Duration::seconds(DEFAULT_ACQUIRE_LEASE_SECONDS);
    expires_at
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{
        CLAIM_AGGREGATE_SCHEMA_VERSION, ClaimLeaseCommand, ClaimLeaseEvent, MODULE,
        decide_claim_lease,
    };
    use crate::scheduler_claim::{OrchestratorClaimActiveInput, OrchestratorClaimRequestInput};
    use proptest::prelude::*;

    #[test]
    fn exclusive_claim_on_same_bounded_unit_is_rejected() {
        let request = claim_request("claim-2", "exclusive", "task-1", "run-1", "domain-b");
        let active = vec![claim("claim-1", "exclusive", "task-1", "run-1", "domain-a")];

        let decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire { request },
            &active,
            "2026-06-22T00:00:00Z",
        );

        assert_eq!(decision.schema_version, CLAIM_AGGREGATE_SCHEMA_VERSION);
        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec!["orchestrator_claim_conflict_task"]
        );
    }

    #[test]
    fn expired_claim_does_not_block_new_acquire() {
        let request = claim_request("claim-2", "exclusive", "task-1", "run-1", "domain-b");
        let mut expired = claim("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        expired.lease_expires_at = "2026-06-21T00:00:00Z".to_string();

        let decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire { request },
            &[expired],
            "2026-06-22T00:00:00Z",
        );

        assert!(decision.admitted);
        assert_eq!(
            decision.events,
            vec![ClaimLeaseEvent::Acquired {
                claim_id: "claim-2".to_string(),
                lease_expires_at: "2026-06-22T00:05:00Z".to_string(),
            }]
        );
    }

    #[test]
    fn acquired_claim_remains_active_at_acquire_timestamp() {
        let request = claim_request("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        let decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire { request },
            &[],
            "2026-06-22T00:00:00Z",
        );

        assert!(decision.admitted);
        assert_eq!(
            decision.events,
            vec![ClaimLeaseEvent::Acquired {
                claim_id: "claim-1".to_string(),
                lease_expires_at: "2026-06-22T00:05:00Z".to_string(),
            }]
        );

        let mut acquired = claim("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        acquired.lease_expires_at = "2026-06-22T00:05:00Z".to_string();
        let conflicting_request =
            claim_request("claim-2", "exclusive", "task-1", "run-1", "domain-b");

        let conflicting_decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire {
                request: conflicting_request,
            },
            &[acquired],
            "2026-06-22T00:00:00Z",
        );

        assert!(!conflicting_decision.admitted);
        assert_eq!(
            conflicting_decision.blocker_codes,
            vec!["orchestrator_claim_conflict_task"]
        );
    }

    #[test]
    fn acquired_claim_lease_expiry_is_normalized_to_utc() {
        let request = claim_request("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        let decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire { request },
            &[],
            "2026-06-22T00:00:00-05:00",
        );

        assert_eq!(
            decision.events,
            vec![ClaimLeaseEvent::Acquired {
                claim_id: "claim-1".to_string(),
                lease_expires_at: "2026-06-22T05:05:00Z".to_string(),
            }]
        );

        let mut acquired = claim("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        acquired.lease_expires_at = "2026-06-22T05:05:00Z".to_string();
        let conflicting_request =
            claim_request("claim-2", "exclusive", "task-1", "run-1", "domain-b");

        let conflicting_decision = decide_claim_lease(
            ClaimLeaseCommand::Acquire {
                request: conflicting_request,
            },
            &[acquired],
            "2026-06-22T04:59:00Z",
        );

        assert!(!conflicting_decision.admitted);
        assert_eq!(
            conflicting_decision.blocker_codes,
            vec!["orchestrator_claim_conflict_task"]
        );
    }

    #[test]
    fn invalid_acquire_timestamp_fails_closed() {
        let request = claim_request("claim-1", "exclusive", "task-1", "run-1", "domain-a");
        let decision = decide_claim_lease(ClaimLeaseCommand::Acquire { request }, &[], "now");

        assert!(!decision.admitted);
        assert_eq!(
            decision.blocker_codes,
            vec!["orchestrator_claim_invalid_now"]
        );
    }

    #[test]
    fn claim_lease_plans_lifecycle_events() {
        assert_eq!(
            decide_claim_lease(
                ClaimLeaseCommand::Renew {
                    claim_id: "claim-1".to_string(),
                    lease_expires_at: "2026-06-23T00:00:00Z".to_string(),
                },
                &[],
                "2026-06-22T00:00:00Z",
            )
            .events,
            vec![ClaimLeaseEvent::Renewed {
                claim_id: "claim-1".to_string(),
                lease_expires_at: "2026-06-23T00:00:00Z".to_string(),
            }]
        );
        assert_eq!(
            decide_claim_lease(
                ClaimLeaseCommand::Release {
                    claim_id: "claim-1".to_string(),
                },
                &[],
                "2026-06-22T00:00:00Z",
            )
            .events,
            vec![ClaimLeaseEvent::Released {
                claim_id: "claim-1".to_string(),
            }]
        );
    }

    #[test]
    fn claims_module_is_registered() {
        assert_eq!(MODULE, "claims");
    }

    proptest! {
        #[test]
        fn exclusive_claims_for_same_bounded_unit_never_both_admit(
            task_id in "[a-z][a-z0-9-]{0,16}",
            run_id in "[a-z][a-z0-9-]{0,16}",
            conflict_domain in "[a-z][a-z0-9-]{0,16}",
        ) {
            let request = claim_request("claim-2", "exclusive", &task_id, &run_id, &conflict_domain);
            let active = vec![claim("claim-1", "exclusive", &task_id, &run_id, &conflict_domain)];

            let decision = decide_claim_lease(
                ClaimLeaseCommand::Acquire { request },
                &active,
                "2026-06-22T00:00:00Z",
            );

            prop_assert!(!decision.admitted);
            prop_assert_eq!(
                decision.blocker_codes,
                vec!["orchestrator_claim_conflict_task".to_string()]
            );
        }
    }

    fn claim_request(
        claim_id: &str,
        lease_mode: &str,
        task_id: &str,
        run_id: &str,
        conflict_domain: &str,
    ) -> OrchestratorClaimRequestInput {
        OrchestratorClaimRequestInput {
            claim_id: claim_id.to_string(),
            state_root_id: "state-root".to_string(),
            worktree_environment_id: "worktree".to_string(),
            orchestrator_session_id: "session-2".to_string(),
            process_id: None,
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            claim_kind: "write".to_string(),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: vec!["crates/taskflow-authority/src/claims".to_string()],
            read_only_paths: Vec::new(),
            lease_mode: lease_mode.to_string(),
        }
    }

    fn claim(
        claim_id: &str,
        lease_mode: &str,
        task_id: &str,
        run_id: &str,
        conflict_domain: &str,
    ) -> OrchestratorClaimActiveInput {
        OrchestratorClaimActiveInput {
            claim_id: claim_id.to_string(),
            orchestrator_session_id: "session-1".to_string(),
            process_id: None,
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: vec!["crates/taskflow-authority/src/claims".to_string()],
            read_only_paths: Vec::new(),
            lease_mode: lease_mode.to_string(),
            status: "active".to_string(),
            lease_expires_at: "2026-06-23T00:00:00Z".to_string(),
        }
    }
}
