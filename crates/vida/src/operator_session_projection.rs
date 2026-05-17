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
    let active_claims = store.active_orchestrator_claims().await?;
    let project_foreign_claims = active_claims
        .iter()
        .filter(|claim| claim.orchestrator_session_id != current_session_id)
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
