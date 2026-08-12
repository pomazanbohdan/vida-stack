#![cfg(loom)]
#![allow(unexpected_cfgs)]

use std::sync::Arc;

use loom::sync::Mutex;
use taskflow_authority::claims::{decide_claim_lease, ClaimLeaseCommand};
use taskflow_authority::scheduler_claim::{
    OrchestratorClaimActiveInput, OrchestratorClaimRequestInput,
};

fn request(claim_id: &str) -> OrchestratorClaimRequestInput {
    OrchestratorClaimRequestInput {
        claim_id: claim_id.to_string(),
        state_root_id: "loom-state".to_string(),
        worktree_environment_id: "loom-worktree".to_string(),
        orchestrator_session_id: format!("session-{claim_id}"),
        process_id: None,
        task_id: Some("loom-task".to_string()),
        run_id: Some("loom-run".to_string()),
        claim_kind: "execution".to_string(),
        conflict_domain: Some("loom-domain".to_string()),
        owned_paths: vec!["crates/taskflow-authority".to_string()],
        read_only_paths: Vec::new(),
        lease_mode: "exclusive".to_string(),
    }
}

fn active(request: &OrchestratorClaimRequestInput) -> OrchestratorClaimActiveInput {
    OrchestratorClaimActiveInput {
        claim_id: request.claim_id.clone(),
        orchestrator_session_id: request.orchestrator_session_id.clone(),
        process_id: request.process_id,
        task_id: request.task_id.clone(),
        run_id: request.run_id.clone(),
        conflict_domain: request.conflict_domain.clone(),
        owned_paths: request.owned_paths.clone(),
        read_only_paths: request.read_only_paths.clone(),
        lease_mode: request.lease_mode.clone(),
        status: "active".to_string(),
        lease_expires_at: "2099-01-01T00:00:00Z".to_string(),
    }
}

#[test]
fn loom_exhaustively_admits_one_conflicting_reservation() {
    loom::model(|| {
        let claims = Arc::new(Mutex::new(Vec::<OrchestratorClaimActiveInput>::new()));
        let mut handles = Vec::new();

        for claim_id in ["claim-a", "claim-b"] {
            let claims = Arc::clone(&claims);
            handles.push(loom::thread::spawn(move || {
                let candidate = request(claim_id);
                let mut active_claims = claims.lock().expect("loom mutex");
                let decision = decide_claim_lease(
                    ClaimLeaseCommand::Acquire {
                        request: candidate.clone(),
                    },
                    &active_claims,
                    "2026-08-12T00:00:00Z",
                );
                if decision.admitted {
                    active_claims.push(active(&candidate));
                    1usize
                } else {
                    0usize
                }
            }));
        }

        let admitted: usize = handles
            .into_iter()
            .map(|handle| handle.join().expect("loom worker"))
            .sum();
        assert_eq!(admitted, 1);
        assert_eq!(claims.lock().expect("loom mutex").len(), 1);
    });
}
