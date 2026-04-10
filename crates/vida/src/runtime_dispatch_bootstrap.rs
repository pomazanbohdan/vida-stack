use crate::{runtime_consumption_run_id, RuntimeConsumptionLaneSelection, StateStore};

pub(crate) async fn build_runtime_consumption_run_graph_bootstrap(
    store: &StateStore,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let run_id = runtime_consumption_run_id(role_selection);
    match crate::taskflow_run_graph::derive_seeded_run_graph_status(
        store,
        &run_id,
        &role_selection.request,
    )
    .await
    {
        Ok(seed_payload) => {
            let seed_payload_json =
                serde_json::to_value(&seed_payload).unwrap_or(serde_json::Value::Null);
            let seed_status_json =
                serde_json::to_value(&seed_payload.status).unwrap_or(serde_json::Value::Null);
            if let Err(error) = store.record_run_graph_status(&seed_payload.status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("record_seed_failed: {error}"),
                });
            }
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
            if let Err(error) = crate::taskflow_continuation::sync_run_graph_continuation_binding(
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
            let mut latest_status = seed_status_json.clone();
            let mut advanced_payload = serde_json::Value::Null;

            if role_selection.conversational_mode.is_some() {
                match crate::taskflow_run_graph::derive_advanced_run_graph_status(
                    store,
                    seed_payload.status,
                )
                .await
                {
                    Ok(payload) => {
                        let advanced_status = payload.status.clone();
                        let advanced_status_json = serde_json::to_value(&payload.status)
                            .unwrap_or(serde_json::Value::Null);
                        if let Err(error) = store.record_run_graph_status(&payload.status).await {
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
                        advanced_payload =
                            serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
                        latest_status = advanced_status_json;
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
            if let Err(record_error) = store.record_run_graph_status(&status).await {
                return serde_json::json!({
                    "status": "blocked",
                    "handoff_ready": false,
                    "run_id": run_id,
                    "reason": format!("seed_failed: {error}; fallback_record_failed: {record_error}"),
                });
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
