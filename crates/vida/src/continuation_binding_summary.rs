pub(crate) fn build_continuation_binding_summary(
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    latest_run_graph_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_dispatch_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    evidence_ambiguous: bool,
) -> serde_json::Value {
    if evidence_ambiguous {
        return serde_json::json!({
            "status": "ambiguous",
            "continuation_allowed": false,
            "active_bounded_unit": serde_json::Value::Null,
            "binding_source": serde_json::Value::Null,
            "why_this_unit": serde_json::Value::Null,
            "primary_path": "diagnosis_path",
            "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
            "ambiguity_reason": "runtime_evidence_ambiguous",
            "next_actions": [
                "Do not continue by heuristic while run-graph continuation evidence is ambiguous.",
                "Refresh continuation evidence with `vida taskflow consume continue --json` and recheck `vida status --json` before selecting the next bounded step."
            ]
        });
    }

    let sequential_vs_parallel_posture = if latest_run_graph_recovery
        .is_some_and(|recovery| recovery.delegation_gate.delegated_cycle_open)
    {
        "sequential_only_open_cycle"
    } else {
        "sequential_only"
    };

    if let Some(status) = latest_run_graph_status {
        if let Some(binding) = explicit_binding {
            if binding.run_id == status.run_id
                && (status.status != "completed"
                    || status
                        .next_node
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty()))
            {
                return serde_json::json!({
                    "status": binding.status,
                    "continuation_allowed": binding.status == "bound",
                    "active_bounded_unit": binding.active_bounded_unit,
                    "binding_source": binding.binding_source,
                    "why_this_unit": binding.why_this_unit,
                    "primary_path": binding.primary_path,
                    "sequential_vs_parallel_posture": binding.sequential_vs_parallel_posture,
                    "ambiguity_reason": serde_json::Value::Null,
                    "next_actions": []
                });
            }
        }

        if status.status != "completed" {
            return serde_json::json!({
                "status": "bound",
                "continuation_allowed": true,
                "active_bounded_unit": {
                    "kind": "run_graph_task",
                    "task_id": status.task_id,
                    "run_id": status.run_id,
                    "active_node": status.active_node,
                },
                "binding_source": "latest_run_graph_status",
                "why_this_unit": format!(
                    "Latest runtime state is still active for task `{}` at node `{}`.",
                    status.task_id, status.active_node
                ),
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": sequential_vs_parallel_posture,
                "ambiguity_reason": serde_json::Value::Null,
                "next_actions": []
            });
        }

        if let Some(receipt) = latest_run_graph_dispatch_receipt {
            let downstream_target = receipt
                .downstream_dispatch_target
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let downstream_status_ready = matches!(
                receipt.downstream_dispatch_status.as_deref(),
                Some("packet_ready") | Some("executed")
            );
            if receipt.downstream_dispatch_ready && downstream_status_ready {
                if let Some(dispatch_target) = downstream_target {
                    return serde_json::json!({
                        "status": "bound",
                        "continuation_allowed": true,
                        "active_bounded_unit": {
                            "kind": "downstream_dispatch_target",
                            "task_id": status.task_id,
                            "run_id": status.run_id,
                            "dispatch_target": dispatch_target,
                        },
                        "binding_source": "latest_run_graph_dispatch_receipt",
                        "why_this_unit": format!(
                            "Latest dispatch receipt explicitly names downstream target `{}` as the next lawful bounded unit.",
                            dispatch_target
                        ),
                        "primary_path": "normal_delivery_path",
                        "sequential_vs_parallel_posture": "sequential_only_downstream_bound",
                        "ambiguity_reason": serde_json::Value::Null,
                        "next_actions": []
                    });
                }
            }
        }

        return serde_json::json!({
            "status": "ambiguous",
            "continuation_allowed": false,
            "active_bounded_unit": serde_json::Value::Null,
            "binding_source": serde_json::Value::Null,
            "why_this_unit": serde_json::Value::Null,
            "primary_path": "diagnosis_path",
            "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
            "ambiguity_reason": "completed_without_explicit_next_bounded_unit",
            "next_actions": [
                "Do not continue by selecting the next ready task heuristically after a completed bounded slice.",
                "Either cite the explicit next bounded unit from the user or refresh continuation evidence with `vida taskflow consume continue --json` before further implementation."
            ]
        });
    }

    serde_json::json!({
        "status": "ambiguous",
        "continuation_allowed": false,
        "active_bounded_unit": serde_json::Value::Null,
        "binding_source": serde_json::Value::Null,
        "why_this_unit": serde_json::Value::Null,
        "primary_path": "diagnosis_path",
        "sequential_vs_parallel_posture": "unknown_until_explicit_binding",
        "ambiguity_reason": "missing_active_bounded_unit_runtime_evidence",
        "next_actions": [
            "Do not continue by plausibility when runtime state does not expose an explicit active bounded unit.",
            "Bind the bounded unit explicitly from user intent or refresh runtime evidence before implementation continues."
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::build_continuation_binding_summary;

    #[test]
    fn active_run_graph_status_binds_current_bounded_unit() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "implementation".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "implementation_active".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);

        assert_eq!(summary["status"], "bound");
        assert_eq!(
            summary["active_bounded_unit"]["task_id"],
            serde_json::Value::String("task-1".to_string())
        );
        assert_eq!(summary["binding_source"], "latest_run_graph_status");
    }

    #[test]
    fn completed_status_without_explicit_next_unit_is_ambiguous() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();

        let summary = build_continuation_binding_summary(None, Some(&status), None, None, false);

        assert_eq!(summary["status"], "ambiguous");
        assert_eq!(
            summary["ambiguity_reason"],
            "completed_without_explicit_next_bounded_unit"
        );
    }

    #[test]
    fn explicit_binding_is_preferred_when_present() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "task-1",
            "implementation",
            "implementation",
        );
        status.active_node = "pm".to_string();
        status.status = "running".to_string();

        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "task-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "task-1",
                "active_node": "pm"
            }),
            binding_source: "explicit_continuation_bind".to_string(),
            why_this_unit: "explicit".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("req".to_string()),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };

        let summary =
            build_continuation_binding_summary(Some(&binding), Some(&status), None, None, false);

        assert_eq!(summary["binding_source"], "explicit_continuation_bind");
        assert_eq!(summary["why_this_unit"], "explicit");
    }
}
