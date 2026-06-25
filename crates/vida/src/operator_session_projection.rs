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
    let tasks = store.list_tasks(None, true).await?;
    let active_claims = store.active_orchestrator_claims().await?;
    build_operator_session_projection_from_rows_and_claims(
        store,
        &owner_evidence,
        &tasks,
        &active_claims,
    )
    .await
}

pub(crate) async fn build_operator_session_projection_from_rows_and_claims(
    store: &crate::state_store::StateStore,
    owner_evidence: &serde_json::Value,
    tasks: &[crate::state_store::TaskRecord],
    active_claims: &[crate::state_store::OrchestratorClaim],
) -> Result<serde_json::Value, crate::state_store::StateStoreError> {
    let current_session = owner_evidence["current_session"].clone();
    let current_session_id = current_session["session_id"].as_str().unwrap_or_default();
    let stale_session_ids =
        crate::orchestrator_session_surface::stale_orchestrator_session_ids_from_evidence(
            owner_evidence,
        );
    let auto_claim_summary = ensure_current_session_claims_for_active_task_rows(
        store,
        &current_session,
        tasks,
        active_claims,
    )
    .await?;
    let active_task_ids = tasks
        .iter()
        .filter(|task| {
            taskflow_core::canonical_task_status(&task.status) == Some("in_progress")
                && crate::state_store::work_item_is_active_bounded_unit_candidate(&task.issue_type)
        })
        .map(|task| task.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let is_active_task_claim = |claim: &crate::state_store::OrchestratorClaim| {
        claim
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|task_id| !task_id.is_empty())
            .map(|task_id| active_task_ids.contains(task_id))
            .unwrap_or(true)
    };
    let inactive_task_claims = active_claims
        .iter()
        .filter(|claim| !is_active_task_claim(claim))
        .map(|claim| {
            serde_json::json!({
                "claim_id": claim.claim_id,
                "orchestrator_session_id": claim.orchestrator_session_id,
                "task_id": claim.task_id,
                "run_id": claim.run_id,
                "conflict_domain": claim.conflict_domain,
                "lease_mode": claim.lease_mode,
                "status": claim.status,
                "classification": "inactive_task_claim",
            })
        })
        .collect::<Vec<_>>();
    let mut current_session_task_claims = active_claims
        .iter()
        .filter(|claim| claim.orchestrator_session_id == current_session_id)
        .filter(|claim| is_active_task_claim(claim))
        .filter(|claim| {
            claim
                .task_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
        .map(|claim| {
            serde_json::json!({
                "claim_id": claim.claim_id,
                "task_id": claim.task_id,
                "run_id": claim.run_id,
                "lane_id": claim.lane_id,
                "conflict_domain": claim.conflict_domain,
                "lease_mode": claim.lease_mode,
                "status": claim.status,
                "lease_expires_at": claim.lease_expires_at,
            })
        })
        .collect::<Vec<_>>();
    if let Some(auto_claimed) = auto_claim_summary["auto_claimed_active_tasks"].as_array() {
        for claim in auto_claimed {
            let Some(task_id) = claim["task_id"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if current_session_task_claims
                .iter()
                .any(|existing| existing["task_id"].as_str() == Some(task_id))
            {
                continue;
            }
            current_session_task_claims.push(serde_json::json!({
                "claim_id": claim["claim_id"].clone(),
                "task_id": task_id,
                "run_id": null,
                "lane_id": null,
                "conflict_domain": null,
                "lease_mode": "observe",
                "status": claim["status"].clone(),
                "lease_expires_at": null,
            }));
        }
    }
    let project_foreign_claims = active_claims
        .iter()
        .filter(|claim| claim.orchestrator_session_id != current_session_id)
        .filter(|claim| !stale_session_ids.contains(&claim.orchestrator_session_id))
        .filter(|claim| is_active_task_claim(claim))
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
        "current_session_task_claims": current_session_task_claims,
        "auto_claimed_active_tasks": auto_claim_summary["auto_claimed_active_tasks"].clone(),
        "active_task_claim_blockers": auto_claim_summary["active_task_claim_blockers"].clone(),
        "inactive_task_claims": inactive_task_claims,
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

pub(crate) fn is_optional_task_worktree_assignment_missing_error(
    error: &dyn std::fmt::Display,
) -> bool {
    let normalized = error.to_string().to_ascii_lowercase();
    normalized.contains("task_worktree_assignment")
        && (normalized.contains("does not exist")
            || normalized.contains("not found")
            || normalized.contains("missing"))
}

pub(crate) fn degraded_operator_session_projection(
    state_root: &std::path::Path,
    reason: &str,
) -> serde_json::Value {
    let owner_evidence =
        crate::orchestrator_session_surface::build_runtime_owner_evidence(state_root, false)
            .unwrap_or_else(|error| {
                serde_json::json!({
                    "mutation_gate": "unknown",
                    "current_session": {
                        "session_id": "unknown-session",
                        "worktree_environment_id": "unknown-worktree",
                        "state": "unknown",
                        "identity_source": "runtime_owner_evidence_unavailable",
                    },
                    "live_other_sessions": [],
                    "stale_sessions": [],
                    "legacy_ownerless_rows": [],
                    "blocker_codes": ["runtime_owner_evidence_unavailable"],
                    "error": error,
                })
            });

    serde_json::json!({
        "schema_version": "operator-session-projection-v1",
        "projection_state": "degraded",
        "degraded": true,
        "degradation_reason": reason,
        "degradation_blocker_code": "optional_task_worktree_assignment_projection_unavailable",
        "current_session": owner_evidence["current_session"].clone(),
        "project_foreign_runs": [],
        "project_foreign_blockers": [],
        "current_session_task_claims": [],
        "auto_claimed_active_tasks": [],
        "active_task_claim_blockers": [{
            "blocker_code": "optional_task_worktree_assignment_projection_unavailable",
            "error": reason,
            "next_action": "Run `vida doctor --json` and migration/preflight diagnostics; status remains available with degraded worktree-assignment projection evidence.",
        }],
        "inactive_task_claims": [],
        "global_blockers": ["optional_task_worktree_assignment_projection_unavailable"],
        "claim_conflicts": [],
        "runtime_owner_evidence": {
            "mutation_gate": owner_evidence["mutation_gate"].clone(),
            "live_other_sessions": owner_evidence["live_other_sessions"].clone(),
            "stale_sessions": owner_evidence["stale_sessions"].clone(),
            "legacy_ownerless_rows": owner_evidence["legacy_ownerless_rows"].clone(),
        }
    })
}

async fn ensure_current_session_claims_for_active_task_rows(
    store: &crate::state_store::StateStore,
    current_session: &serde_json::Value,
    tasks: &[crate::state_store::TaskRecord],
    existing_claims: &[crate::state_store::OrchestratorClaim],
) -> Result<serde_json::Value, crate::state_store::StateStoreError> {
    let current_session_id = current_session["session_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown-session");
    let worktree_environment_id = current_session["worktree_environment_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown-worktree");
    let state_root_id = store.root().display().to_string();
    let active_tasks = tasks
        .iter()
        .filter(|task| task.status == "in_progress" && task.issue_type != "epic")
        .collect::<Vec<_>>();
    let mut auto_claimed = Vec::new();
    let mut blockers = Vec::new();

    for task in active_tasks {
        if let Some(existing) = existing_claims
            .iter()
            .find(|claim| claim.task_id.as_deref() == Some(task.id.as_str()))
        {
            if existing.orchestrator_session_id != current_session_id {
                blockers.push(serde_json::json!({
                    "task_id": task.id,
                    "claim_id": existing.claim_id,
                    "orchestrator_session_id": existing.orchestrator_session_id,
                    "blocker_code": "active_task_claimed_by_foreign_session",
                    "next_action": format!(
                        "Run `vida orchestrator-session transfer {} --to-current --json` to continue task `{}` from the current session when the handoff is intentional.",
                        existing.orchestrator_session_id,
                        task.id
                    ),
                }));
            }
            continue;
        }

        let claim_id = format!(
            "active-task-{}-{}",
            sanitize_projection_claim_id(current_session_id),
            sanitize_projection_claim_id(&task.id)
        );
        let conflict_domain = task
            .execution_semantics
            .conflict_domain
            .clone()
            .unwrap_or_else(|| format!("task:{}", task.id));
        match store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: claim_id.clone(),
                state_root_id: state_root_id.clone(),
                worktree_environment_id: worktree_environment_id.to_string(),
                orchestrator_session_id: current_session_id.to_string(),
                process_id: Some(std::process::id()),
                task_id: Some(task.id.clone()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some(conflict_domain),
                owned_paths: task.planner_metadata.owned_paths.clone(),
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Observe,
                lease_seconds: 3600,
            })
            .await
        {
            Ok(claim) => auto_claimed.push(serde_json::json!({
                "task_id": task.id,
                "claim_id": claim.claim_id,
                "orchestrator_session_id": claim.orchestrator_session_id,
                "status": claim.status,
            })),
            Err(error) => blockers.push(serde_json::json!({
                "task_id": task.id,
                "blocker_code": "active_task_claim_acquire_failed",
                "error": error.to_string(),
            })),
        }
    }

    Ok(serde_json::json!({
        "auto_claimed_active_tasks": auto_claimed,
        "active_task_claim_blockers": blockers,
    }))
}

fn sanitize_projection_claim_id(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('-').chars().take(96).collect()
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
        other["identity_source"] = serde_json::Value::String("VIDA_SESSION_ID".to_string());
        other["fallback_replaces_legacy_stable_worktree_state_hash"] = serde_json::Value::Null;
        other["process_id"] = serde_json::Value::Number(process_id.into());
        other["owner_annotation"] = serde_json::Value::String("foreign_session".to_string());
        other
    }

    fn task_record(task_id: &str, status: &str) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-05-22T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-05-22T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn optional_task_worktree_assignment_missing_table_degrades_projection() {
        let message = "Failed to build operator session projection: The table 'task_worktree_assignment' does not exist";
        assert!(is_optional_task_worktree_assignment_missing_error(&message));

        let root = temp_state_dir("missing-task-worktree-assignment");
        let projection = degraded_operator_session_projection(&root, message);

        assert_eq!(
            projection["schema_version"],
            "operator-session-projection-v1"
        );
        assert_eq!(projection["projection_state"], "degraded");
        assert_eq!(
            projection["degradation_blocker_code"],
            "optional_task_worktree_assignment_projection_unavailable"
        );
        assert_eq!(
            projection["global_blockers"][0],
            "optional_task_worktree_assignment_projection_unavailable"
        );
        assert_eq!(
            projection["active_task_claim_blockers"][0]["blocker_code"],
            "optional_task_worktree_assignment_projection_unavailable"
        );
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
    async fn projection_reports_live_foreign_session_without_global_blocker() {
        let root = temp_state_dir("foreign-visible-nonblocking");
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
            .is_empty());
        assert!(projection["runtime_owner_evidence"]["live_other_sessions"]
            .as_array()
            .expect("live other sessions")
            .iter()
            .any(|session| session["session_id"] == "foreign-live-session"));
        assert_eq!(
            projection["runtime_owner_evidence"]["mutation_gate"],
            "current_session_allowed"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn projection_auto_claims_ownerless_active_tasks_for_current_session() {
        let root = temp_state_dir("auto-claim-active-task");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        store
            .persist_task_record(task_record("active-ownerless", "in_progress"))
            .await
            .expect("persist active task");

        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        assert!(projection["auto_claimed_active_tasks"]
            .as_array()
            .expect("auto claimed")
            .iter()
            .any(|claim| claim["task_id"] == "active-ownerless"));
        assert!(projection["current_session_task_claims"]
            .as_array()
            .expect("current claims")
            .iter()
            .any(|claim| claim["task_id"] == "active-ownerless"));
        assert!(projection["active_task_claim_blockers"]
            .as_array()
            .expect("claim blockers")
            .is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn projection_reports_foreign_active_task_claim_for_transfer() {
        let root = temp_state_dir("foreign-active-task-transfer");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        store
            .persist_task_record(task_record("active-foreign", "in_progress"))
            .await
            .expect("persist active task");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "foreign-active-task-claim".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: "worktree".to_string(),
                orchestrator_session_id: "foreign-session".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("active-foreign".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:active-foreign".to_string()),
                owned_paths: Vec::new(),
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Exclusive,
                lease_seconds: 3600,
            })
            .await
            .expect("foreign claim");

        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        assert!(projection["auto_claimed_active_tasks"]
            .as_array()
            .expect("auto claimed")
            .is_empty());
        assert!(projection["active_task_claim_blockers"]
            .as_array()
            .expect("claim blockers")
            .iter()
            .any(|blocker| {
                blocker["task_id"] == "active-foreign"
                    && blocker["blocker_code"] == "active_task_claimed_by_foreign_session"
            }));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn projection_auto_claims_same_session_active_tasks_with_overlapping_paths() {
        let root = temp_state_dir("auto-claim-overlapping-active-tasks");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let mut first = task_record("active-first", "in_progress");
        first.planner_metadata.owned_paths = vec!["crates/vida/src/runtime.rs".to_string()];
        let mut second = task_record("active-second", "in_progress");
        second.planner_metadata.owned_paths = vec!["crates/vida/src".to_string()];
        store
            .persist_task_record(first)
            .await
            .expect("persist first active task");
        store
            .persist_task_record(second)
            .await
            .expect("persist second active task");

        let projection = build_operator_session_projection(&store)
            .await
            .expect("projection");

        let current_claims = projection["current_session_task_claims"]
            .as_array()
            .expect("current claims");
        assert!(current_claims
            .iter()
            .any(|claim| claim["task_id"] == "active-first"));
        assert!(current_claims
            .iter()
            .any(|claim| claim["task_id"] == "active-second"));
        assert!(current_claims
            .iter()
            .all(|claim| claim["lease_mode"] == "observe"));
        assert!(projection["active_task_claim_blockers"]
            .as_array()
            .expect("claim blockers")
            .is_empty());

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
                process_id: Some(std::process::id()),
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

    #[tokio::test]
    async fn projection_omits_claim_conflicts_for_inactive_task_claims() {
        let root = temp_state_dir("inactive-task-claim");
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        store
            .persist_task_record(task_record("closed-task", "closed"))
            .await
            .expect("persist closed task");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "claim-closed-task".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: "worktree".to_string(),
                orchestrator_session_id: "foreign-session".to_string(),
                process_id: Some(std::process::id()),
                task_id: Some("closed-task".to_string()),
                run_id: None,
                lane_id: None,
                claim_kind: "active_task_session_claim".to_string(),
                conflict_domain: Some("task:closed-task".to_string()),
                owned_paths: vec!["crates/vida/src/operator_session_projection.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Observe,
                lease_seconds: 3600,
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
        assert!(projection["inactive_task_claims"]
            .as_array()
            .expect("inactive task claims")
            .iter()
            .any(|claim| claim["claim_id"] == "claim-closed-task"));

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
