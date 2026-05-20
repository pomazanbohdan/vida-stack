pub(crate) async fn build_operator_session_projection(
    store: &crate::state_store::StateStore,
) -> Result<serde_json::Value, crate::state_store::StateStoreError> {
    let owner_evidence =
        crate::orchestrator_session_surface::build_runtime_owner_evidence(store.root(), false)
            .map_err(
                |reason| crate::state_store::StateStoreError::InvalidTaskRecord {
                    reason: format!("runtime owner evidence unavailable: {reason}"),
                },
            )?;
    let current_session = owner_evidence["current_session"].clone();
    let current_session_id = current_session["session_id"].as_str().unwrap_or_default();
    let stale_session_ids =
        crate::orchestrator_session_surface::stale_orchestrator_session_ids_from_evidence(
            &owner_evidence,
        );
    let active_claims = store.active_orchestrator_claims().await?;
    let project_foreign_claims = active_claims
        .iter()
        .filter(|claim| claim.orchestrator_session_id != current_session_id)
        .filter(|claim| !stale_session_ids.contains(&claim.orchestrator_session_id))
        .cloned()
        .collect::<Vec<_>>();
    let project_foreign_runs = project_foreign_claims
        .iter()
        .filter(|claim| {
            claim
                .run_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
        .map(|claim| {
            serde_json::json!({
                "claim_id": claim.claim_id,
                "orchestrator_session_id": claim.orchestrator_session_id,
                "run_id": claim.run_id,
                "task_id": claim.task_id,
                "lane_id": claim.lane_id,
                "conflict_domain": claim.conflict_domain,
                "lease_mode": claim.lease_mode,
                "status": claim.status,
                "lease_expires_at": claim.lease_expires_at,
            })
        })
        .collect::<Vec<_>>();
    let project_foreign_blockers = project_foreign_claims
        .iter()
        .filter(|claim| claim.status == "blocked" || !claim.blocker_codes.is_empty())
        .map(|claim| {
            serde_json::json!({
                "claim_id": claim.claim_id,
                "orchestrator_session_id": claim.orchestrator_session_id,
                "task_id": claim.task_id,
                "run_id": claim.run_id,
                "blocker_codes": claim.blocker_codes,
            })
        })
        .collect::<Vec<_>>();
    let claim_conflicts = project_foreign_claims
        .iter()
        .filter(|claim| {
            claim.lease_mode == "exclusive"
                || claim.status == "blocked"
                || !claim.blocker_codes.is_empty()
                || claim.conflict_domain.is_some()
                || !claim.owned_paths.is_empty()
        })
        .map(|claim| {
            serde_json::json!({
                "claim_id": claim.claim_id,
                "orchestrator_session_id": claim.orchestrator_session_id,
                "task_id": claim.task_id,
                "run_id": claim.run_id,
                "conflict_domain": claim.conflict_domain,
                "owned_paths": claim.owned_paths,
                "read_only_paths": claim.read_only_paths,
                "lease_mode": claim.lease_mode,
                "status": claim.status,
                "blocker_codes": claim.blocker_codes,
            })
        })
        .collect::<Vec<_>>();
    let global_blockers = owner_evidence["blocker_codes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "schema_version": "operator-session-projection-v1",
        "current_session": current_session,
        "project_foreign_runs": project_foreign_runs,
        "project_foreign_blockers": project_foreign_blockers,
        "global_blockers": global_blockers,
        "claim_conflicts": claim_conflicts,
        "runtime_owner_evidence": {
            "mutation_gate": owner_evidence["mutation_gate"].clone(),
            "live_other_sessions": owner_evidence["live_other_sessions"].clone(),
            "stale_sessions": owner_evidence["stale_sessions"].clone(),
            "legacy_ownerless_rows": owner_evidence["legacy_ownerless_rows"].clone(),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vida-operator-session-projection-{name}-{nanos}"))
    }

    fn live_other_session(
        current: &serde_json::Value,
        session_id: &str,
        process_id: u32,
    ) -> serde_json::Value {
        let mut other = current.clone();
        other["session_id"] = serde_json::Value::String(session_id.to_string());
        other["process_id"] = serde_json::Value::Number(process_id.into());
        other["owner_annotation"] = serde_json::Value::String("foreign_session".to_string());
        other
    }

    #[tokio::test]
    async fn projection_exposes_required_session_fields_without_foreign_blocker_inheritance() {
        let root = temp_state_dir("required-fields");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        assert_eq!(
            projection["schema_version"],
            "operator-session-projection-v1"
        );
        assert!(projection["current_session"].is_object());
        assert!(projection["project_foreign_runs"].is_array());
        assert!(projection["project_foreign_blockers"].is_array());
        assert!(projection["global_blockers"].is_array());
        assert!(projection["claim_conflicts"].is_array());
        assert!(projection["global_blockers"]
            .as_array()
            .expect("global blockers")
            .is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn projection_reports_owner_evidence_as_global_blocker() {
        let root = temp_state_dir("global-blocker");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let evidence =
            crate::orchestrator_session_surface::build_runtime_owner_evidence(store.root(), true)
                .expect("owner evidence");
        let session_store_path = evidence["session_store_path"]
            .as_str()
            .expect("session store path");
        let current = evidence["current_session"].clone();
        let other = live_other_session(&current, "foreign-live-session", std::process::id());
        let payload = serde_json::json!({
            "schema_version": "runtime-owner-evidence-v1",
            "updated_at_epoch_seconds": evidence["current_session"]["last_heartbeat_epoch_seconds"],
            "sessions": [current, other],
        });
        std::fs::write(
            session_store_path,
            serde_json::to_string_pretty(&payload).expect("serialize sessions"),
        )
        .expect("write session store");

        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        assert!(projection["global_blockers"]
            .as_array()
            .expect("global blockers")
            .iter()
            .any(|value| value.as_str() == Some("live_other_orchestrator_owner")));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn projection_omits_claim_conflicts_owned_by_stale_sessions() {
        let root = temp_state_dir("stale-owner-claim");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let evidence =
            crate::orchestrator_session_surface::build_runtime_owner_evidence(store.root(), true)
                .expect("owner evidence");
        let session_store_path = evidence["session_store_path"]
            .as_str()
            .expect("session store path");
        let current = evidence["current_session"].clone();
        let now = current["last_heartbeat_epoch_seconds"]
            .as_i64()
            .expect("current heartbeat");
        let payload = serde_json::json!({
            "schema_version": "runtime-owner-evidence-v1",
            "updated_at_epoch_seconds": now,
            "sessions": [
                current,
                {
                    "session_id": "stale-foreign-session",
                    "state": "stale",
                    "process_id": 12345,
                    "last_heartbeat_epoch_seconds": now.saturating_sub(3 * 60 * 60),
                }
            ],
        });
        std::fs::write(
            session_store_path,
            serde_json::to_string_pretty(&payload).expect("serialize sessions"),
        )
        .expect("write session store");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "claim-stale-owner".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree".to_string(),
                orchestrator_session_id: "stale-foreign-session".to_string(),
                task_id: Some("task-foreign".to_string()),
                run_id: Some("run-foreign".to_string()),
                lane_id: Some("lane".to_string()),
                claim_kind: "write".to_string(),
                conflict_domain: Some("case15".to_string()),
                owned_paths: vec!["crates/vida/src".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("claim");

        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        assert!(projection["claim_conflicts"]
            .as_array()
            .expect("claim conflicts")
            .is_empty());
        assert!(projection["project_foreign_runs"]
            .as_array()
            .expect("foreign runs")
            .is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}

pub(crate) fn projection_plain_summary(projection: &serde_json::Value) -> String {
    format!(
        "current_session={} foreign_runs={} foreign_blockers={} global_blockers={} claim_conflicts={}",
        projection["current_session"]["session_id"]
            .as_str()
            .unwrap_or("unknown"),
        projection["project_foreign_runs"].as_array().map_or(0, Vec::len),
        projection["project_foreign_blockers"].as_array().map_or(0, Vec::len),
        projection["global_blockers"].as_array().map_or(0, Vec::len),
        projection["claim_conflicts"].as_array().map_or(0, Vec::len),
    )
}

pub(crate) fn projection_operator_blocker_codes(projection: &serde_json::Value) -> Vec<String> {
    let mut blocker_codes = projection["global_blockers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    if json_array_len(&projection["claim_conflicts"]) > 0
        || json_array_len(&projection["project_foreign_blockers"]) > 0
    {
        blocker_codes.push(
            crate::contract_profile_adapter::blocker_code_str(
                crate::contract_profile_adapter::BlockerCode::ConflictDomainCollision,
            )
            .to_string(),
        );
    }

    crate::contract_profile_adapter::canonical_blocker_code_list(
        blocker_codes.iter().map(String::as_str),
    )
}

pub(crate) fn projection_operator_next_actions(blocker_codes: &[String]) -> Vec<String> {
    let mut next_actions = Vec::new();
    if blocker_codes.iter().any(|code| {
        code == crate::contract_profile_adapter::blocker_code_str(
            crate::contract_profile_adapter::BlockerCode::LiveOtherOrchestratorOwner,
        )
    }) {
        next_actions.push(
            "Inspect `operator_session_projection.runtime_owner_evidence.live_other_sessions` and reclaim or transfer stale/foreign orchestrator ownership before reporting a clean operator pass."
                .to_string(),
        );
    }
    if blocker_codes.iter().any(|code| {
        code == crate::contract_profile_adapter::blocker_code_str(
            crate::contract_profile_adapter::BlockerCode::ConflictDomainCollision,
        )
    }) {
        next_actions.push(
            "Inspect `operator_session_projection.claim_conflicts` and resolve or supersede the competing orchestrator claim before continuing the blocked task."
                .to_string(),
        );
    }
    next_actions
}

pub(crate) fn projection_operator_artifact_refs(
    projection: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "operator_session_projection_schema_version": projection["schema_version"],
        "current_session_id": projection["current_session"]["session_id"],
        "project_foreign_run_count": json_array_len(&projection["project_foreign_runs"]),
        "project_foreign_blocker_count": json_array_len(&projection["project_foreign_blockers"]),
        "global_blocker_count": json_array_len(&projection["global_blockers"]),
        "claim_conflict_count": json_array_len(&projection["claim_conflicts"]),
    })
}

fn json_array_len(value: &serde_json::Value) -> usize {
    value.as_array().map_or(0, Vec::len)
}
