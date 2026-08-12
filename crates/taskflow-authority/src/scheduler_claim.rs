use std::collections::BTreeSet;

pub const MODULE: &str = "scheduler_claim";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerReservationRequestInput {
    pub reservation_id: String,
    pub task_id: String,
    pub conflict_domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerReservationActiveInput {
    pub reservation_id: String,
    pub task_id: String,
    pub conflict_domain: Option<String>,
    pub lease_status: String,
    pub lease_expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorClaimRequestInput {
    pub claim_id: String,
    pub state_root_id: String,
    pub worktree_environment_id: String,
    pub orchestrator_session_id: String,
    pub process_id: Option<u32>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub claim_kind: String,
    pub conflict_domain: Option<String>,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
    pub lease_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorClaimActiveInput {
    pub claim_id: String,
    pub orchestrator_session_id: String,
    pub process_id: Option<u32>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub conflict_domain: Option<String>,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
    pub lease_mode: String,
    pub status: String,
    pub lease_expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorClaimConflictDecision {
    pub conflict_kind: String,
    pub claim_id: String,
    pub orchestrator_session_id: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub conflict_domain: Option<String>,
    pub path: Option<String>,
    pub blocker_code: String,
}

pub type OrchestratorClaimInput = OrchestratorClaimActiveInput;
pub type OrchestratorClaimConflict = OrchestratorClaimConflictDecision;

#[must_use]
pub fn decide_scheduler_reservation_collision(
    request: &SchedulerReservationRequestInput,
    active: &[SchedulerReservationActiveInput],
) -> Option<String> {
    for reservation in active {
        if reservation.task_id == request.task_id {
            return Some(format!(
                "scheduler_task_already_reserved:{}:{}",
                request.task_id, reservation.reservation_id
            ));
        }
        if let (Some(left), Some(right)) = (
            request.conflict_domain.as_deref(),
            reservation.conflict_domain.as_deref(),
        ) {
            if !left.trim().is_empty() && left == right {
                return Some(format!(
                    "scheduler_conflict_domain_reserved:{}:{}",
                    left, reservation.reservation_id
                ));
            }
        }
    }
    None
}

#[must_use]
pub fn normalize_scheduler_reservation_blocker_codes(blocker_codes: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    blocker_codes
        .iter()
        .map(|code| code.trim())
        .filter(|code| !code.is_empty())
        .filter_map(|code| seen.insert(code.to_string()).then(|| code.to_string()))
        .collect()
}

#[must_use]
pub fn scheduler_reservation_is_active(status: &str) -> bool {
    matches!(status, "reserved" | "executing")
}

#[must_use]
pub fn scheduler_reservation_is_expired(
    reservation: &SchedulerReservationActiveInput,
    now: &str,
) -> bool {
    scheduler_reservation_is_active(&reservation.lease_status)
        && !reservation.lease_expires_at.trim().is_empty()
        && reservation.lease_expires_at.as_str() <= now
}

pub fn validate_orchestrator_claim_request(
    request: &OrchestratorClaimRequestInput,
) -> Result<(), String> {
    let missing = [
        ("claim_id", request.claim_id.as_str()),
        ("state_root_id", request.state_root_id.as_str()),
        (
            "worktree_environment_id",
            request.worktree_environment_id.as_str(),
        ),
        (
            "orchestrator_session_id",
            request.orchestrator_session_id.as_str(),
        ),
        ("claim_kind", request.claim_kind.as_str()),
    ]
    .into_iter()
    .find_map(|(name, value)| value.trim().is_empty().then_some(name));
    if let Some(name) = missing {
        return Err(format!("orchestrator_claim_missing_required_field:{name}"));
    }
    if request.lease_mode != "observe" && request.task_id.is_none() && request.run_id.is_none() {
        return Err("orchestrator_claim_missing_bounded_unit".to_string());
    }
    Ok(())
}

#[must_use]
pub fn claim_is_active(status: &str) -> bool {
    matches!(status, "active" | "renewed" | "blocked")
}

#[must_use]
pub fn claim_is_expired(claim: &OrchestratorClaimInput, now: &str) -> bool {
    claim_is_active(&claim.status)
        && !claim.lease_expires_at.trim().is_empty()
        && claim.lease_expires_at.as_str() <= now
}

#[must_use]
pub fn normalize_claim_path(path: &str) -> Option<String> {
    let mut value = path.trim().replace('\\', "/");
    while let Some(stripped) = value.strip_prefix("./") {
        value = stripped.to_string();
    }
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    value = parts.join("/");
    #[cfg(windows)]
    {
        value = value.to_ascii_lowercase();
    }
    value = value.trim_matches('/').to_string();
    (!value.is_empty()).then_some(value)
}

#[must_use]
pub fn claim_paths_intersect(left: &str, right: &str) -> bool {
    let Some(left) = normalize_claim_path(left) else {
        return false;
    };
    let Some(right) = normalize_claim_path(right) else {
        return false;
    };
    left == right
        || left
            .strip_prefix(right.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[must_use]
pub fn decide_orchestrator_claim_conflict(
    request: &OrchestratorClaimRequestInput,
    claim: &OrchestratorClaimActiveInput,
) -> Option<OrchestratorClaimConflictDecision> {
    if !claim_modes_conflict(&request.lease_mode, &claim.lease_mode) {
        return None;
    }
    if let (Some(req_pid), Some(claim_pid)) = (request.process_id, claim.process_id) {
        if req_pid == claim_pid {
            return Some(claim_conflict_payload("process", claim, None));
        }
    }
    if request.task_id.is_some() && request.task_id == claim.task_id {
        return Some(claim_conflict_payload("task", claim, None));
    }
    if request.run_id.is_some() && request.run_id == claim.run_id {
        return Some(claim_conflict_payload("run", claim, None));
    }

    let active_write_paths = claim
        .owned_paths
        .iter()
        .map(|path| (path, "owned_path"))
        .collect::<Vec<_>>();
    let active_paths = claim
        .owned_paths
        .iter()
        .map(|path| (path, "owned_path"))
        .chain(
            claim
                .read_only_paths
                .iter()
                .map(|path| (path, "read_only_path")),
        )
        .collect::<Vec<_>>();
    for requested_path in &request.owned_paths {
        for (active_path, conflict_kind) in &active_paths {
            if claim_paths_intersect(requested_path, active_path) {
                return Some(claim_conflict_payload(
                    conflict_kind,
                    claim,
                    normalize_claim_path(requested_path),
                ));
            }
        }
    }
    for requested_path in &request.read_only_paths {
        for (active_path, conflict_kind) in &active_write_paths {
            if claim_paths_intersect(requested_path, active_path) {
                return Some(claim_conflict_payload(
                    conflict_kind,
                    claim,
                    normalize_claim_path(requested_path),
                ));
            }
        }
    }
    if let (Some(left), Some(right)) = (
        request.conflict_domain.as_deref(),
        claim.conflict_domain.as_deref(),
    ) {
        if !left.trim().is_empty() && left == right {
            return Some(claim_conflict_payload("conflict_domain", claim, None));
        }
    }
    None
}

fn claim_modes_conflict(request_mode: &str, active_mode: &str) -> bool {
    if request_mode == "observe" || active_mode == "observe" {
        return false;
    }
    request_mode == "exclusive" || active_mode == "exclusive"
}

fn claim_conflict_payload(
    conflict_kind: &str,
    claim: &OrchestratorClaimActiveInput,
    path: Option<String>,
) -> OrchestratorClaimConflictDecision {
    OrchestratorClaimConflictDecision {
        conflict_kind: conflict_kind.to_string(),
        claim_id: claim.claim_id.clone(),
        orchestrator_session_id: claim.orchestrator_session_id.clone(),
        task_id: claim.task_id.clone(),
        run_id: claim.run_id.clone(),
        conflict_domain: claim.conflict_domain.clone(),
        path,
        blocker_code: format!("orchestrator_claim_conflict_{conflict_kind}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OrchestratorClaimActiveInput, OrchestratorClaimRequestInput,
        SchedulerReservationActiveInput, SchedulerReservationRequestInput, claim_paths_intersect,
        decide_orchestrator_claim_conflict, decide_scheduler_reservation_collision,
        normalize_claim_path, normalize_scheduler_reservation_blocker_codes,
    };

    #[test]
    fn scheduler_reservation_collision_classifies_duplicate_task_and_domain() {
        let active = vec![SchedulerReservationActiveInput {
            reservation_id: "reservation-1".to_string(),
            task_id: "task-1".to_string(),
            conflict_domain: Some("domain-a".to_string()),
            lease_status: "reserved".to_string(),
            lease_expires_at: "2026-06-22T00:00:00Z".to_string(),
        }];

        assert_eq!(
            decide_scheduler_reservation_collision(
                &SchedulerReservationRequestInput {
                    reservation_id: "reservation-2".to_string(),
                    task_id: "task-1".to_string(),
                    conflict_domain: Some("domain-b".to_string()),
                },
                &active,
            )
            .as_deref(),
            Some("scheduler_task_already_reserved:task-1:reservation-1")
        );
        assert_eq!(
            decide_scheduler_reservation_collision(
                &SchedulerReservationRequestInput {
                    reservation_id: "reservation-3".to_string(),
                    task_id: "task-2".to_string(),
                    conflict_domain: Some("domain-a".to_string()),
                },
                &active,
            )
            .as_deref(),
            Some("scheduler_conflict_domain_reserved:domain-a:reservation-1")
        );
    }

    #[test]
    fn scheduler_reservation_blocker_codes_are_canonical() {
        assert_eq!(
            normalize_scheduler_reservation_blocker_codes(&[
                " scheduler_agent_init_activation_view_only ".to_string(),
                "scheduler_agent_init_activation_view_only".to_string(),
                "".to_string(),
            ]),
            vec!["scheduler_agent_init_activation_view_only"]
        );
    }

    #[test]
    fn scheduler_reservation_expiry_only_applies_to_active_leases() {
        let mut reservation = SchedulerReservationActiveInput {
            reservation_id: "reservation-1".to_string(),
            task_id: "task-1".to_string(),
            conflict_domain: None,
            lease_status: "reserved".to_string(),
            lease_expires_at: "2026-06-21T00:00:00Z".to_string(),
        };
        assert!(super::scheduler_reservation_is_expired(
            &reservation,
            "2026-06-22T00:00:00Z"
        ));

        reservation.lease_status = "released".to_string();
        assert!(!super::scheduler_reservation_is_expired(
            &reservation,
            "2026-06-22T00:00:00Z"
        ));
    }

    #[test]
    fn orchestrator_claim_request_validation_fails_closed() {
        let mut request =
            claim_request("claim-1", "exclusive", None, "task-1", "run-1", "domain-a");
        request.claim_id = " ".to_string();
        assert_eq!(
            super::validate_orchestrator_claim_request(&request).expect_err("missing claim id"),
            "orchestrator_claim_missing_required_field:claim_id"
        );

        let mut request =
            claim_request("claim-2", "exclusive", None, "task-1", "run-1", "domain-a");
        request.task_id = None;
        request.run_id = None;
        assert_eq!(
            super::validate_orchestrator_claim_request(&request).expect_err("missing unit"),
            "orchestrator_claim_missing_bounded_unit"
        );
    }

    #[test]
    fn orchestrator_claim_observe_mode_does_not_conflict() {
        let conflict = decide_orchestrator_claim_conflict(
            &claim_request(
                "claim-2",
                "observe",
                Some(42),
                "task-2",
                "run-2",
                "domain-b",
            ),
            &claim(
                "claim-1",
                "observe",
                Some(42),
                "task-1",
                "run-1",
                "domain-a",
            ),
        );

        assert!(conflict.is_none());
    }

    #[test]
    fn orchestrator_claim_classifies_task_domain_and_path_conflicts() {
        let active = claim("claim-1", "exclusive", None, "task-1", "run-1", "domain-a");

        assert_eq!(
            decide_orchestrator_claim_conflict(
                &claim_request("claim-2", "exclusive", None, "task-1", "run-2", "domain-b"),
                &active,
            )
            .map(|conflict| conflict.blocker_code),
            Some("orchestrator_claim_conflict_task".to_string())
        );
        let mut domain_request =
            claim_request("claim-3", "exclusive", None, "task-3", "run-3", "domain-a");
        domain_request.owned_paths = vec!["crates/other/src/lib.rs".to_string()];
        assert_eq!(
            decide_orchestrator_claim_conflict(&domain_request, &active)
                .map(|conflict| conflict.blocker_code),
            Some("orchestrator_claim_conflict_conflict_domain".to_string())
        );

        let mut request =
            claim_request("claim-4", "exclusive", None, "task-4", "run-4", "domain-b");
        request.owned_paths = vec![".\\crates\\vida\\src\\taskflow_proxy.rs".to_string()];
        assert_eq!(
            decide_orchestrator_claim_conflict(&request, &active)
                .and_then(|conflict| conflict.path),
            Some("crates/vida/src/taskflow_proxy.rs".to_string())
        );
    }

    #[test]
    fn claim_path_intersection_handles_children_and_case() {
        assert!(claim_paths_intersect(
            "crates/vida/src",
            "crates/vida/src/taskflow_proxy.rs"
        ));
        assert!(!claim_paths_intersect(
            "crates/vida/src2",
            "crates/vida/src/taskflow_proxy.rs"
        ));
    }

    #[test]
    fn claim_paths_normalize_empty_segments_and_claim_expiry() {
        assert_eq!(
            normalize_claim_path(" .\\crates//vida/../taskflow/src/./lib.rs "),
            Some("crates/taskflow/src/lib.rs".to_string())
        );
        assert_eq!(normalize_claim_path("foo/../"), None);
        assert_eq!(normalize_claim_path(" /./ "), None);

        let claim = claim("claim-1", "exclusive", None, "task-1", "run-1", "domain-a");
        assert!(super::claim_is_expired(&claim, "2026-06-23T00:00:00Z"));
        assert!(!super::claim_is_expired(&claim, "2026-06-21T00:00:00Z"));
    }

    #[test]
    fn claim_modes_allow_shared_readers_but_block_exclusive_writers() {
        let active = claim("claim-1", "shared", None, "task-1", "run-1", "domain-a");
        let mut request = claim_request("claim-2", "shared", None, "task-2", "run-2", "domain-b");
        request.owned_paths = vec!["crates/other/src/lib.rs".to_string()];
        assert!(decide_orchestrator_claim_conflict(&request, &active).is_none());

        request.lease_mode = "exclusive".to_string();
        request.conflict_domain = Some("domain-a".to_string());
        assert_eq!(
            decide_orchestrator_claim_conflict(&request, &active)
                .map(|decision| decision.blocker_code),
            Some("orchestrator_claim_conflict_conflict_domain".to_string())
        );
        assert!(super::claim_is_active("active"));
        assert!(super::claim_is_active("renewed"));
        assert!(!super::claim_is_active("released"));
    }

    fn claim_request(
        claim_id: &str,
        lease_mode: &str,
        process_id: Option<u32>,
        task_id: &str,
        run_id: &str,
        conflict_domain: &str,
    ) -> OrchestratorClaimRequestInput {
        let _ = claim_id;
        OrchestratorClaimRequestInput {
            claim_id: claim_id.to_string(),
            state_root_id: "state-root".to_string(),
            worktree_environment_id: "worktree".to_string(),
            orchestrator_session_id: "session-2".to_string(),
            process_id,
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            claim_kind: "write".to_string(),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: vec!["crates/vida/src/taskflow_proxy.rs".to_string()],
            read_only_paths: Vec::new(),
            lease_mode: lease_mode.to_string(),
        }
    }

    fn claim(
        claim_id: &str,
        lease_mode: &str,
        process_id: Option<u32>,
        task_id: &str,
        run_id: &str,
        conflict_domain: &str,
    ) -> OrchestratorClaimActiveInput {
        OrchestratorClaimActiveInput {
            claim_id: claim_id.to_string(),
            orchestrator_session_id: "session-1".to_string(),
            process_id,
            task_id: Some(task_id.to_string()),
            run_id: Some(run_id.to_string()),
            conflict_domain: Some(conflict_domain.to_string()),
            owned_paths: vec!["crates/vida/src".to_string()],
            read_only_paths: Vec::new(),
            lease_mode: lease_mode.to_string(),
            status: "active".to_string(),
            lease_expires_at: "2026-06-22T00:00:00Z".to_string(),
        }
    }
}
