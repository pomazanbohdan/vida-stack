pub(crate) fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    })
}

pub(crate) fn carrier_runtime_section<'a>(
    compiled_bundle: &'a serde_json::Value,
) -> &'a serde_json::Value {
    compiled_bundle
        .get("carrier_runtime")
        .unwrap_or(&serde_json::Value::Null)
}

pub(crate) fn runtime_assignment_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
) -> &'a serde_json::Value {
    execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("carrier_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

pub(crate) fn runtime_assignment_alias_fields(
    runtime_assignment: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "carrier_runtime_assignment".to_string(),
        runtime_assignment.clone(),
    );
    fields.insert("runtime_assignment".to_string(), runtime_assignment.clone());
    fields
}

pub(crate) fn expected_policy_bundle_ref(
    execution_plan: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    execution_plan
        .get("policy_bundle_ref")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| "policy_bundle_pin_missing".to_string())
}

pub(crate) fn validate_policy_bundle_ref(
    execution_plan: &serde_json::Value,
    compiled_bundle: &serde_json::Value,
    assignment: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let expected = expected_policy_bundle_ref(execution_plan)?;
    if crate::runtime_lane_summary::resolve_policy_pin(compiled_bundle) != expected {
        return Err("policy_bundle_pin_mismatch".to_string());
    }
    if assignment.get("policy_bundle_ref") != Some(&expected) {
        return Err("policy_bundle_pin_missing_or_mismatch".to_string());
    }
    Ok(expected)
}

pub(crate) fn apply_run_graph_runtime_assignment_to_selection(
    role_selection: &mut crate::RuntimeConsumptionLaneSelection,
    compiled_bundle: &serde_json::Value,
    run_graph_bootstrap: &serde_json::Value,
    execution_plan_error: &str,
) -> Result<(), String> {
    let latest_status = &run_graph_bootstrap["latest_status"];
    let Some(task_class) = crate::json_string(latest_status.get("task_class"))
        .or_else(|| crate::json_string(latest_status.get("route_task_class")))
    else {
        return Ok(());
    };
    let expected_policy_bundle_ref = expected_policy_bundle_ref(&role_selection.execution_plan)?;
    if crate::runtime_lane_summary::resolve_policy_pin(compiled_bundle)
        != expected_policy_bundle_ref
    {
        return Err("policy_bundle_pin_mismatch".to_string());
    }
    let runtime_role = crate::json_string(latest_status.get("activation_runtime_role"))
        .unwrap_or_else(|| role_selection.selected_role.clone());
    let conversation_role = role_selection.fallback_role.trim();
    let mut assignment = crate::build_runtime_assignment_from_resolved_constraints(
        compiled_bundle,
        conversation_role,
        &task_class,
        &runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        return Ok(());
    }
    crate::runtime_assignment_builder::attach_policy_bundle_ref(
        &mut assignment,
        &expected_policy_bundle_ref,
    );
    validate_policy_bundle_ref(
        &role_selection.execution_plan,
        compiled_bundle,
        &assignment,
    )?;
    let execution_plan = role_selection
        .execution_plan
        .as_object_mut()
        .ok_or_else(|| execution_plan_error.to_string())?;
    execution_plan.extend(runtime_assignment_alias_fields(&assignment));
    Ok(())
}

pub(crate) fn infer_task_class_from_task_payload(task: &serde_json::Value) -> String {
    let labels = task["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if labels.iter().any(|label| label == "coach") {
        return "coach".to_string();
    }
    if labels
        .iter()
        .any(|label| label == "tester" || label == "prover")
    {
        return "verification".to_string();
    }
    let mut text = String::new();
    if let Some(title) = task["title"].as_str() {
        text.push_str(title);
        text.push(' ');
    }
    if let Some(description) = task["description"].as_str() {
        text.push_str(description);
    }
    let normalized = text.to_ascii_lowercase();

    if labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "architecture" | "solution-architect" | "hard-escalation" | "escalation"
        )
    }) || !crate::contains_keywords(
        &normalized,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "migration".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "hard escalation".to_string(),
        ],
    )
    .is_empty()
    {
        return "architecture".to_string();
    }
    if labels.iter().any(|label| {
        matches!(
            label.as_str(),
            "verification" | "review" | "proof" | "release-readiness"
        )
    }) || !crate::contains_keywords(
        &normalized,
        &[
            "verify".to_string(),
            "verification".to_string(),
            "review".to_string(),
            "audit".to_string(),
            "proof".to_string(),
            "release readiness".to_string(),
        ],
    )
    .is_empty()
    {
        return "verification".to_string();
    }
    if labels
        .iter()
        .any(|label| matches!(label.as_str(), "spec-pack" | "documentation" | "planning"))
        || !crate::contains_keywords(
            &normalized,
            &[
                "spec".to_string(),
                "design".to_string(),
                "research".to_string(),
                "plan".to_string(),
                "requirements".to_string(),
            ],
        )
        .is_empty()
    {
        return "specification".to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_assignment_alias_fields_emits_canonical_and_compat_aliases() {
        let assignment = serde_json::json!({
            "enabled": true,
            "selected_backend_id": "internal_subagents",
            "activation_agent_type": "middle",
        });

        let fields = super::runtime_assignment_alias_fields(&assignment);

        assert_eq!(fields.get("runtime_assignment"), Some(&assignment));
        assert_eq!(fields.get("carrier_runtime_assignment"), Some(&assignment));
    }

    #[test]
    fn run_graph_assignment_helper_ignores_status_without_task_class() {
        let mut selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "fix runtime".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec![],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let compiled_bundle = serde_json::json!({});
        let run_graph_bootstrap = serde_json::json!({
            "latest_status": {
                "activation_runtime_role": "worker"
            }
        });

        super::apply_run_graph_runtime_assignment_to_selection(
            &mut selection,
            &compiled_bundle,
            &run_graph_bootstrap,
            "execution_plan is not an object",
        )
        .expect("assignment helper should update selection");

        assert_eq!(selection.execution_plan, serde_json::json!({}));
    }

    #[test]
    fn activation_a_to_b_policy_pin_race_fails_closed_without_mutation() {
        let pin_a = serde_json::json!({
            "policy_id": "rhai.runtime.authority",
            "version": 1,
            "content_digest": "digest-a"
        });
        let pin_b = serde_json::json!({
            "policy_id": "rhai.runtime.authority",
            "version": 2,
            "content_digest": "digest-b"
        });
        let assignment = serde_json::json!({
            "enabled": true,
            "policy_bundle_ref": pin_a
        });
        let mut selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "test".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "policy_bundle_ref": pin_a,
                "runtime_assignment": assignment,
            }),
            reason: "test".to_string(),
        };
        let before = selection.execution_plan.clone();
        let result = super::apply_run_graph_runtime_assignment_to_selection(
            &mut selection,
            &serde_json::json!({
                "policy_runtime": {"active": pin_b}
            }),
            &serde_json::json!({
                "latest_status": {
                    "task_class": "implementation",
                    "activation_runtime_role": "worker"
                }
            }),
            "execution plan is not an object",
        );
        assert_eq!(result, Err("policy_bundle_pin_mismatch".to_string()));
        assert_eq!(selection.execution_plan, before);
    }

    #[test]
    fn task_payload_class_inference_preserves_priority_and_fails_closed_when_missing() {
        let cases = [
            (serde_json::json!({"labels": ["coach", "prover"]}), "coach"),
            (
                serde_json::json!({"labels": ["prover", "tester"]}),
                "verification",
            ),
            (
                serde_json::json!({"title": "architecture migration"}),
                "architecture",
            ),
            (
                serde_json::json!({"title": "write a verification proof"}),
                "verification",
            ),
            (
                serde_json::json!({"title": "prepare a specification plan"}),
                "specification",
            ),
        ];

        for (task, expected) in cases {
            assert_eq!(super::infer_task_class_from_task_payload(&task), expected);
        }
        assert_eq!(
            super::infer_task_class_from_task_payload(&serde_json::json!({
                "title": "ordinary work"
            })),
            ""
        );
    }
}
