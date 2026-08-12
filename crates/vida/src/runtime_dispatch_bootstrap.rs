use crate::{runtime_consumption_run_id, RuntimeConsumptionLaneSelection, StateStore};

pub(crate) async fn build_runtime_consumption_run_graph_bootstrap(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    build_runtime_consumption_run_graph_bootstrap_with_persistence(store, role_selection, true)
        .await
}

pub(crate) async fn build_runtime_consumption_run_graph_bootstrap_read_only(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    build_runtime_consumption_run_graph_bootstrap_with_persistence(store, role_selection, false)
        .await
}

async fn build_runtime_consumption_run_graph_bootstrap_with_persistence(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
    persist_state: bool,
) -> serde_json::Value {
    let run_id = runtime_consumption_run_id(role_selection);
    if !role_selection.ok || role_selection.selected_role == "orchestrator" {
        let status = crate::runtime_dispatch_status::blocking_runtime_consumption_run_graph_status(
            role_selection,
            &run_id,
        );
        let latest_status = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
        let fallback_reason = if role_selection.ok {
            "selected role `orchestrator` is not a dispatchable runtime lane"
        } else {
            role_selection.reason.as_str()
        };
        if persist_state {
            if let Err(error) = store.record_run_graph_status(&status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": "unresolved_lane_selection",
                    "fallback_reason": fallback_reason,
                    "record_error": format!("record_blocked_selection_failed: {error}"),
                });
            }
        }
        return serde_json::json!({
            "status": "blocked",
            "handoff_ready": false,
            "run_id": run_id,
            "seed": serde_json::Value::Null,
            "advanced": serde_json::Value::Null,
            "latest_status": latest_status,
            "reason": "unresolved_lane_selection",
            "fallback_reason": fallback_reason,
        });
    }
    let seed_result = if persist_state {
        crate::taskflow_run_graph::derive_seeded_run_graph_state(
            store,
            &run_id,
            &role_selection.request,
        )
        .await
    } else {
        crate::taskflow_run_graph::derive_seeded_run_graph_state_read_only(
            store,
            &run_id,
            &role_selection.request,
        )
        .await
    };
    match seed_result {
        Ok(seed_payload) => {
            let seed_payload_json =
                serde_json::to_value(&seed_payload).unwrap_or(serde_json::Value::Null);
            let seed_status_json =
                serde_json::to_value(&seed_payload.status).unwrap_or(serde_json::Value::Null);
            if persist_state {
                if let Err(error) = store.record_run_graph_status(&seed_payload.status).await {
                    return serde_json::json!({
                        "status": "blocked",
                        "handoff_ready": false,
                        "run_id": run_id,
                        "reason": format!("record_seed_failed: {error}"),
                    });
                }
            }
            if persist_state {
                if let Err(error) = store
                    .record_run_graph_dispatch_context(
                        &crate::taskflow_run_graph::run_graph_dispatch_context_from_seed_payload(
                            &seed_payload,
                        ),
                    )
                    .await
                {
                    return serde_json::json!({
                        "status": "blocked",
                        "handoff_ready": false,
                        "run_id": run_id,
                        "seed": seed_payload_json,
                        "reason": format!("record_seed_context_failed: {error}"),
                    });
                }
            }
            if persist_state {
                if let Err(error) =
                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                        store,
                        &seed_payload.status,
                        "runtime_consumption_seed",
                    )
                    .await
                {
                    return serde_json::json!({
                        "status": "blocked",
                        "handoff_ready": false,
                        "run_id": run_id,
                        "seed": seed_payload_json,
                        "reason": format!("record_seed_binding_failed: {error}"),
                    });
                }
            }
            let mut latest_status = seed_status_json.clone();
            let mut advanced_payload = serde_json::Value::Null;

            if role_selection.conversational_mode.is_some() {
                let advance_result = if persist_state {
                    crate::taskflow_run_graph::derive_advanced_run_graph_state(
                        store,
                        seed_payload.status,
                    )
                    .await
                } else {
                    crate::taskflow_run_graph::derive_advanced_run_graph_state_read_only(
                        store,
                        seed_payload.status,
                    )
                    .await
                };
                match advance_result {
                    Ok(payload) => {
                        let advanced_status = payload.status.clone();
                        let advanced_status_json = serde_json::to_value(&payload.status)
                            .unwrap_or(serde_json::Value::Null);
                        if persist_state {
                            if let Err(error) = store.record_run_graph_status(&payload.status).await
                            {
                                let blocked_status = crate::runtime_dispatch_status::blocking_runtime_consumption_run_graph_status(
                                role_selection,
                                &run_id,
                            );
                                let blocked_status_json = serde_json::to_value(&blocked_status)
                                    .unwrap_or(serde_json::Value::Null);
                                let blocked_write_error =
                                    store.record_run_graph_status(&blocked_status).await.err();
                                return serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "run_id": run_id,
                                    "seed": seed_payload_json,
                                    "latest_status": blocked_status_json,
                                    "reason": if let Some(blocked_write_error) = blocked_write_error {
                                        format!(
                                            "record_advance_failed: {error}; compensating_blocked_record_failed: {blocked_write_error}"
                                        )
                                    } else {
                                        format!("record_advance_failed: {error}")
                                    },
                                });
                            }
                        }
                        advanced_payload =
                            serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
                        latest_status = advanced_status_json;
                        if persist_state {
                            if let Err(error) =
                                crate::taskflow_continuation::sync_run_graph_continuation_binding(
                                    store,
                                    &advanced_status,
                                    "runtime_consumption_advance",
                                )
                                .await
                            {
                                return serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "run_id": run_id,
                                    "seed": seed_payload_json,
                                    "reason": format!("record_advance_binding_failed: {error}"),
                                });
                            }
                        }
                    }
                    Err(error) => {
                        return serde_json::json!({
                            "status": "blocked",
                            "handoff_ready": false,
                            "run_id": run_id,
                            "seed": seed_payload_json,
                            "reason": format!("advance_failed: {error}"),
                        });
                    }
                }
            }

            serde_json::json!({
                "status": if advanced_payload.is_null() {
                    "seeded"
                } else {
                    "seeded_and_advanced"
                },
                "handoff_ready": true,
                "run_id": run_id,
                "seed": seed_payload_json,
                "advanced": advanced_payload,
                "latest_status": if advanced_payload.is_null() {
                    seed_status_json
                } else {
                    latest_status
                },
            })
        }
        Err(error) => {
            let status =
                crate::runtime_dispatch_status::blocking_runtime_consumption_run_graph_status(
                    role_selection,
                    &run_id,
                );
            let latest_status = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
            if persist_state {
                if let Err(record_error) = store.record_run_graph_status(&status).await {
                    return serde_json::json!({
                        "status": "blocked",
                        "handoff_ready": false,
                        "run_id": run_id,
                        "reason": format!("seed_failed: {error}; fallback_record_failed: {record_error}"),
                    });
                }
            }
            serde_json::json!({
                "status": "blocked",
                "handoff_ready": false,
                "run_id": run_id,
                "seed": serde_json::Value::Null,
                "advanced": serde_json::Value::Null,
                "latest_status": latest_status,
                "fallback_reason": format!("seed_failed: {error}"),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_consumption_run_graph_bootstrap,
        build_runtime_consumption_run_graph_bootstrap_read_only,
    };
    use crate::{RuntimeConsumptionLaneSelection, StateStore};

    #[tokio::test]
    async fn runtime_consumption_bootstrap_fails_closed_with_blocked_fallback_when_seed_derivation_fails(
    ) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-consumption-seed-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));
        let cwd = std::env::temp_dir().join(format!(
            "vida-runtime-consumption-seed-fail-closed-cwd-{}-{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&cwd).expect("create isolated cwd");
        let _cwd = crate::test_cli_support::guard_current_dir(&cwd);
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "seed-fail-parent",
                title: "Ambiguous seed parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create ambiguous seed parent");
        for task_id in ["seed-fail-task-a", "seed-fail-task-b"] {
            store
                .create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title: "Ambiguous seed candidate",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 1,
                    parent_id: Some("seed-fail-parent"),
                    labels: &[],
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create ambiguous seed task");
        }
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implement seed-fail-task-a and seed-fail-task-b".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let readonly_bootstrap =
            build_runtime_consumption_run_graph_bootstrap_read_only(&store, &role_selection).await;
        assert_eq!(readonly_bootstrap["status"], "blocked");
        assert!(matches!(
            store.read_launcher_activation_snapshot().await,
            Err(crate::state_store::StateStoreError::MissingLauncherActivationSnapshot)
        ));
        assert!(store
            .latest_run_graph_status()
            .await
            .expect("read-only bootstrap status lookup should succeed")
            .is_none());

        let bootstrap =
            build_runtime_consumption_run_graph_bootstrap(&store, &role_selection).await;
        assert_eq!(bootstrap["status"], "blocked");
        assert_eq!(bootstrap["handoff_ready"], false);
        assert!(bootstrap["fallback_reason"]
            .as_str()
            .is_some_and(|value| value.contains("seed_failed")));

        assert_eq!(bootstrap["latest_status"]["status"], "blocked");
        assert_eq!(bootstrap["latest_status"]["recovery_ready"], false);
        assert_eq!(bootstrap["latest_status"]["context_state"], "open");

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&cwd);
    }
}
