pub(crate) use crate::runtime_lane_summary::{
    build_runtime_lane_selection_with_store, RuntimeConsumptionLaneSelection,
};

fn canonicalize_moved_test_request(request: &str) -> String {
    const MOVED_TEST_MOVE_PREFIX: &str = "move ";
    const MOVED_TEST_MOVE_SUFFIX: &str =
        " from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs";
    const BARE_PROOF_TARGET_PREFIX: &str = "cargo test -p vida ";
    const BARE_PROOF_TARGET_SUFFIX: &str = " -- --nocapture";
    const MODULE_QUALIFIED_PREFIX: &str = "project_activator_surface::tests::";

    let Some(move_start) = request.find(MOVED_TEST_MOVE_PREFIX) else {
        return request.to_string();
    };
    let move_start = move_start + MOVED_TEST_MOVE_PREFIX.len();
    let Some(move_end) = request[move_start..].find(MOVED_TEST_MOVE_SUFFIX) else {
        return request.to_string();
    };
    let move_end = move_start + move_end;
    let test_name = request[move_start..move_end].trim();
    if test_name.is_empty() {
        return request.to_string();
    }

    let bare_proof_target =
        format!("{BARE_PROOF_TARGET_PREFIX}{test_name}{BARE_PROOF_TARGET_SUFFIX}");
    let canonical_proof_target = format!(
        "{BARE_PROOF_TARGET_PREFIX}{MODULE_QUALIFIED_PREFIX}{test_name} -- --exact --nocapture"
    );
    if request.contains(&bare_proof_target) {
        request.replace(&bare_proof_target, &canonical_proof_target)
    } else {
        request.to_string()
    }
}

pub(crate) fn build_design_first_tracked_flow_bootstrap(request: &str) -> serde_json::Value {
    let canonical_request = canonicalize_moved_test_request(request);
    let feature_slug = crate::infer_feature_request_slug(request)
        .trim()
        .trim_matches('-')
        .to_string();
    let feature_slug = if feature_slug.is_empty() {
        "feature-request".to_string()
    } else {
        feature_slug
    };
    let feature_title = crate::infer_feature_request_title(request);
    let design_doc_path = format!("docs/product/spec/{feature_slug}-design.md");
    let artifact_path = format!("product/spec/{feature_slug}-design");
    let epic_task_id = format!("feature-{feature_slug}");
    let spec_task_id = format!("{epic_task_id}-spec");
    let work_pool_task_id = format!("{epic_task_id}-work-pool");
    let dev_task_id = format!("{epic_task_id}-dev");
    let epic_title = format!("Feature epic: {feature_title}");
    let spec_title = format!("Spec pack: {feature_title}");
    let work_pool_title = format!("Work-pool pack: {feature_title}");
    let dev_title = format!("Dev pack: {feature_title}");
    let quoted_request = crate::shell_quote(&canonical_request);
    let work_pool_semantics = crate::launcher_task_commands::TaskExecutionSemanticsCommandArgs {
        execution_mode: "container_only",
        order_bucket: &epic_task_id,
        parallel_group: "work-pool-pack",
        conflict_domain: &work_pool_task_id,
    };
    let dev_semantics = crate::launcher_task_commands::TaskExecutionSemanticsCommandArgs {
        execution_mode: "parallel_safe",
        order_bucket: &epic_task_id,
        parallel_group: "dev-pack",
        conflict_domain: &dev_task_id,
    };

    serde_json::json!({
        "required": true,
        "status": "pending",
        "bootstrap_command": format!(
            "vida taskflow bootstrap-spec {} --json",
            quoted_request,
        ),
        "feature_slug": feature_slug,
        "feature_title": feature_title,
        "design_doc_path": design_doc_path,
        "design_artifact_path": artifact_path,
        "epic": {
            "required": true,
            "task_id": epic_task_id,
            "title": epic_title,
            "runtime": "vida taskflow",
            "create_command": crate::build_task_create_command(
                &epic_task_id,
                &epic_title,
                "epic",
                None,
                &["feature-request", "spec-first"],
                Some(&quoted_request),
                None,
            ),
            "close_command": crate::build_task_close_command(
                &epic_task_id,
                "feature delivery closed after proof and runtime handoff",
            )
        },
        "spec_task": {
            "required": true,
            "task_id": spec_task_id,
            "title": spec_title,
            "runtime": "vida taskflow",
            "inspect_command": crate::build_task_show_command(&spec_task_id),
            "ensure_command": crate::build_task_ensure_command(
                &spec_task_id,
                &spec_title,
                "task",
                Some(&epic_task_id),
                &["spec-pack", "documentation"],
                Some(&crate::shell_quote("bounded design/spec packet for the feature request")),
                None,
            ),
            "create_command": crate::build_task_create_command(
                &spec_task_id,
                &spec_title,
                "task",
                Some(&epic_task_id),
                &["spec-pack", "documentation"],
                Some(&crate::shell_quote("bounded design/spec packet for the feature request")),
                None,
            ),
            "close_command": crate::build_task_close_command(
                &spec_task_id,
                "design packet finalized and handed off into tracked work-pool shaping",
            )
        },
        "work_pool_task": {
            "required": true,
            "task_id": work_pool_task_id,
            "title": work_pool_title,
            "runtime": "vida taskflow",
            "inspect_command": crate::build_task_show_command(&work_pool_task_id),
            "ensure_command": crate::build_task_ensure_command(
                &work_pool_task_id,
                &work_pool_title,
                "task",
                Some(&epic_task_id),
                &["work-pool-pack"],
                None,
                Some(work_pool_semantics),
            ),
            "create_command": crate::build_task_create_command(
                &work_pool_task_id,
                &work_pool_title,
                "task",
                Some(&epic_task_id),
                &["work-pool-pack"],
                None,
                Some(work_pool_semantics),
            ),
            "close_command": crate::build_task_close_command(
                &work_pool_task_id,
                "work-pool packet closed after delegated execution packet was shaped",
            )
        },
        "dev_task": {
            "required": false,
            "task_id": dev_task_id,
            "title": dev_title,
            "runtime": "vida taskflow",
            "inspect_command": crate::build_task_show_command(&dev_task_id),
            "ensure_command": crate::build_task_ensure_command(
                &dev_task_id,
                &dev_title,
                "task",
                Some(&epic_task_id),
                &["dev-pack"],
                None,
                Some(dev_semantics),
            ),
            "create_command": crate::build_task_create_command(
                &dev_task_id,
                &dev_title,
                "task",
                Some(&epic_task_id),
                &["dev-pack"],
                None,
                Some(dev_semantics),
            ),
            "close_command": crate::build_task_close_command(
                &dev_task_id,
                "delegated development packet reached proof-ready closure",
            )
        },
        "docflow": {
            "required": true,
            "runtime": "vida docflow",
            "init_command": format!(
                "vida docflow init {} {} product_spec {}",
                design_doc_path,
                artifact_path,
                crate::shell_quote("initialize bounded feature design"),
            ),
            "finalize_command": format!(
                "vida docflow finalize-edit {} {}",
                design_doc_path,
                crate::shell_quote("record bounded feature design"),
            ),
            "check_command": format!(
                "vida docflow check --root . {}",
                design_doc_path,
            )
        },
        "handoff_sequence": [
            "create epic",
            "open spec task",
            "open work-pool shaping task",
            "initialize bounded design document",
            "finalize and validate bounded design document",
            "close spec task",
            "shape dev packet in TaskFlow before delegated implementation"
        ]
    })
}

fn request_requires_execution_preparation(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> bool {
    let selected_flow = compiled_bundle["default_flow_set"]
        .as_str()
        .and_then(|flow_id| compiled_bundle["all_project_flow_catalog"].get(flow_id));
    if let Some(policy) = selected_flow.and_then(|flow| flow.get("execution_preparation_policy")) {
        let mode = policy["mode"].as_str().unwrap_or_default();
        let gated_task_classes = policy["task_classes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        let task_class = crate::runtime_assignment_from_execution_plan(&selection.execution_plan)
            ["task_class"]
            .as_str()
            .unwrap_or("implementation");
        let validation_gate = if crate::json_bool(policy.get("honor_validation_gate"), false) {
            crate::json_bool(
                compiled_bundle["autonomous_execution"]
                    .get("validation_report_required_before_implementation"),
                false,
            )
        } else {
            false
        };
        match mode {
            "always" => return true,
            "never" => return false,
            "required_for_task_classes" => {
                return gated_task_classes.contains(&task_class);
            }
            "required_for_code_shaped_work" => {
                if gated_task_classes.contains(&task_class) {
                    return validation_gate || task_class == "implementation";
                }
                return false;
            }
            _ => {}
        }
    }
    let normalized_request = selection.request.to_lowercase();
    let architecture_signals = crate::contains_keywords(
        &normalized_request,
        &[
            "architecture".to_string(),
            "architect".to_string(),
            "cross-cutting".to_string(),
            "cross cutting".to_string(),
            "migration".to_string(),
            "refactor".to_string(),
            "topology".to_string(),
            "boundary".to_string(),
            "cross-scope".to_string(),
            "cross scope".to_string(),
        ],
    );
    let write_signals = crate::contains_keywords(
        &normalized_request,
        &[
            "implement".to_string(),
            "implementation".to_string(),
            "write code".to_string(),
            "write the code".to_string(),
            "patch".to_string(),
            "refactor".to_string(),
            "build".to_string(),
        ],
    );
    let task_class = crate::json_string(
        compiled_bundle["role_selection"]
            .get("selected_task_class")
            .or_else(|| {
                crate::runtime_assignment_from_execution_plan(&selection.execution_plan)
                    .get("task_class")
            }),
    )
    .unwrap_or_default();
    let validation_gate = crate::json_bool(
        compiled_bundle["autonomous_execution"]
            .get("validation_report_required_before_implementation"),
        false,
    );
    task_class == "implementation"
        && (validation_gate || !architecture_signals.is_empty() || !write_signals.is_empty())
}

fn legacy_development_flow_templates() -> Vec<serde_json::Value> {
    let pending_specification_evidence = crate::blocker_code_str(
        crate::release1_contracts::BlockerCode::PendingSpecificationEvidence,
    );
    let pending_execution_preparation_evidence = crate::blocker_code_str(
        crate::release1_contracts::BlockerCode::PendingExecutionPreparationEvidence,
    );
    let pending_implementation_evidence = crate::blocker_code_str(
        crate::release1_contracts::BlockerCode::PendingImplementationEvidence,
    );
    let pending_review_clean_evidence =
        crate::blocker_code_str(crate::release1_contracts::BlockerCode::PendingReviewCleanEvidence);
    let pending_verification_evidence = crate::blocker_code_str(
        crate::release1_contracts::BlockerCode::PendingVerificationEvidence,
    );
    vec![
        serde_json::json!({
            "lane_id": "specification",
            "dispatch_target": "specification",
            "dispatch_alias": "development_specification",
            "task_class": "specification",
            "packet_template_kind": "delivery_task_packet",
            "closure_class": "law",
            "stage": "design_gate",
            "inclusion_rule": "when_design_gate",
            "completion_blocker": pending_specification_evidence,
        }),
        serde_json::json!({
            "lane_id": "execution_preparation",
            "dispatch_target": "execution_preparation",
            "dispatch_alias": "development_execution_preparation",
            "task_class": "execution_preparation",
            "packet_template_kind": "escalation_packet",
            "closure_class": "refactor",
            "stage": "execution",
            "inclusion_rule": "when_execution_preparation_required",
            "completion_blocker": pending_execution_preparation_evidence,
        }),
        serde_json::json!({
            "lane_id": "implementation",
            "dispatch_target": "implementer",
            "dispatch_alias": "development_implementer",
            "task_class": "implementation",
            "packet_template_kind": "delivery_task_packet",
            "closure_class": "implementation",
            "stage": "execution",
            "inclusion_rule": "always",
            "completion_blocker": pending_implementation_evidence,
        }),
        serde_json::json!({
            "lane_id": "coach",
            "dispatch_target": "coach",
            "dispatch_alias": "development_coach",
            "task_class": "coach",
            "packet_template_kind": "coach_review_packet",
            "closure_class": "proof",
            "stage": "execution",
            "inclusion_rule": "when_flow_requires_coach",
            "completion_blocker": pending_review_clean_evidence,
        }),
        serde_json::json!({
            "lane_id": "verification",
            "dispatch_target": "verification",
            "dispatch_alias": "development_verifier",
            "task_class": "verification",
            "packet_template_kind": "verifier_proof_packet",
            "closure_class": "proof",
            "stage": "execution",
            "inclusion_rule": "when_flow_requires_verification",
            "completion_blocker": pending_verification_evidence,
        }),
    ]
}

fn resolved_development_flow_templates(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> Vec<serde_json::Value> {
    let configured_templates = configured_dev_team_flow_templates(compiled_bundle, selection);
    if !configured_templates.is_empty() {
        return configured_templates;
    }
    let flow_id = compiled_bundle["default_flow_set"]
        .as_str()
        .unwrap_or_default();
    if let Some(flow) = compiled_bundle["all_project_flow_catalog"]
        .get(flow_id)
        .or_else(|| compiled_bundle["project_flow_catalog"].get(flow_id))
    {
        if flow["flow_class"].as_str() == Some("development") {
            let templates = flow["lane_templates"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if !templates.is_empty() {
                return templates;
            }
        }
    }
    legacy_development_flow_templates()
}

fn configured_dev_team_flow_templates(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> Vec<serde_json::Value> {
    let sequence = selection
        .matched_terms
        .iter()
        .find_map(|term| term.strip_prefix("dev_team_flow_id:"))
        .map(|flow_id| {
            crate::dev_team_sequence_contract::dev_team_sequence_for_flow_id(
                &compiled_bundle["dev_team_readiness"],
                flow_id,
            )
        })
        .filter(|sequence| !sequence.is_empty())
        .unwrap_or_else(|| crate::dev_team_sequence_contract::dev_team_sequence(compiled_bundle));
    if sequence.is_empty() {
        return Vec::new();
    }
    sequence
        .into_iter()
        .filter(|step| step.requires_task)
        .map(|step| {
            let role_label = step.role_label;
            let runtime_role = step.runtime_role;
            let task_class = step.task_class;
            let mut fallback_fields = Vec::new();
            let packet_template_kind = dev_team_policy_field_or_fallback(
                step.packet_template_kind.as_deref(),
                packet_template_kind_for_dev_team_task_class(&task_class),
                "packet_template_kind",
                &mut fallback_fields,
            );
            let closure_class = dev_team_policy_field_or_fallback(
                step.closure_class.as_deref(),
                closure_class_for_dev_team_task_class(&task_class),
                "closure_class",
                &mut fallback_fields,
            );
            let stage = dev_team_policy_field_or_fallback(
                step.stage.as_deref(),
                stage_for_dev_team_task_class(&task_class),
                "stage",
                &mut fallback_fields,
            );
            let fallback_completion_blocker =
                completion_blocker_for_dev_team_task_class(&task_class);
            let completion_blocker = dev_team_policy_field_or_fallback(
                step.completion_blocker.as_deref(),
                &fallback_completion_blocker,
                "completion_blocker",
                &mut fallback_fields,
            );
            let inclusion_rule = dev_team_policy_field_or_fallback(
                step.inclusion_rule.as_deref(),
                "always",
                "inclusion_rule",
                &mut fallback_fields,
            );
            let fallback_used = !fallback_fields.is_empty();
            serde_json::json!({
                "lane_id": role_label.clone(),
                "dispatch_target": role_label,
                "dispatch_alias": "",
                "task_class": task_class.clone(),
                "runtime_role": runtime_role,
                "packet_template_kind": packet_template_kind,
                "closure_class": closure_class,
                "stage": stage,
                "inclusion_rule": inclusion_rule,
                "completion_blocker": completion_blocker,
                "policy_diagnostics": {
                    "source": "dev_team_readiness",
                    "fallback_used": fallback_used,
                    "fallback_fields": fallback_fields,
                    "fallback_reason": if fallback_used {
                        serde_json::Value::String(
                            "missing_configured_dev_team_policy_fields".to_string(),
                        )
                    } else {
                        serde_json::Value::Null
                    },
                },
                "requires_user_approval": step.requires_user_approval,
                "approval_policy": step.approval_policy,
                "lifecycle_hook_templates": step.lifecycle_hook_templates,
                "resume_transitions": step.resume_transitions,
                "rework_transitions": step.rework_transitions,
            })
        })
        .collect()
}

fn dev_team_policy_field_or_fallback(
    configured: Option<&str>,
    fallback: &str,
    field: &'static str,
    fallback_fields: &mut Vec<&'static str>,
) -> String {
    if let Some(configured) = configured.map(str::trim).filter(|value| !value.is_empty()) {
        return configured.to_string();
    }
    fallback_fields.push(field);
    fallback.to_string()
}

pub(crate) fn packet_template_kind_for_dev_team_task_class(task_class: &str) -> &'static str {
    match task_class {
        "coach" | "review" | "validation" => "coach_review_packet",
        "verification" | "quality_gate" | "release_readiness" => "verifier_proof_packet",
        "architecture" | "execution_preparation" => "escalation_packet",
        _ => "delivery_task_packet",
    }
}

fn closure_class_for_dev_team_task_class(task_class: &str) -> &'static str {
    match task_class {
        "coach" | "review" | "validation" | "verification" | "quality_gate"
        | "release_readiness" => "proof",
        "architecture" | "execution_preparation" => "refactor",
        "specification" | "planning" => "law",
        _ => "implementation",
    }
}

fn stage_for_dev_team_task_class(task_class: &str) -> &'static str {
    match task_class {
        "specification" | "planning" => "design_gate",
        _ => "execution",
    }
}

fn completion_blocker_for_dev_team_task_class(task_class: &str) -> String {
    let blocker = match task_class {
        "specification" | "planning" => {
            crate::release1_contracts::BlockerCode::PendingSpecificationEvidence
        }
        "architecture" | "execution_preparation" => {
            crate::release1_contracts::BlockerCode::PendingExecutionPreparationEvidence
        }
        "coach" | "review" | "validation" => {
            crate::release1_contracts::BlockerCode::PendingReviewCleanEvidence
        }
        "verification" | "quality_gate" | "release_readiness" => {
            crate::release1_contracts::BlockerCode::PendingVerificationEvidence
        }
        _ => crate::release1_contracts::BlockerCode::PendingImplementationEvidence,
    };
    crate::blocker_code_str(blocker).to_string()
}

fn copy_non_empty_route_value(
    target: &mut serde_json::Value,
    target_key: &str,
    source: &serde_json::Value,
    source_key: &str,
) {
    let Some(value) = source.get(source_key) else {
        return;
    };
    let configured = match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(raw) => !raw.trim().is_empty(),
        serde_json::Value::Array(rows) => !rows.is_empty(),
        serde_json::Value::Object(entries) => !entries.is_empty(),
        _ => true,
    };
    if configured {
        target[target_key] = value.clone();
    }
}

fn apply_implementation_analysis_route_overrides(
    analysis: &mut serde_json::Value,
    implementation: &serde_json::Value,
) {
    copy_non_empty_route_value(
        analysis,
        "executor_backend",
        implementation,
        "analysis_executor_backend",
    );
    copy_non_empty_route_value(
        analysis,
        "external_first_required",
        implementation,
        "analysis_external_first_required",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_executor_backends",
        implementation,
        "analysis_fanout_executor_backends",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_min_results",
        implementation,
        "analysis_fanout_min_results",
    );
    copy_non_empty_route_value(
        analysis,
        "fanout_subagents",
        implementation,
        "analysis_fanout_subagents",
    );
    copy_non_empty_route_value(
        analysis,
        "merge_policy",
        implementation,
        "analysis_merge_policy",
    );
}

fn lane_template_included(
    lane_template: &serde_json::Value,
    requires_design_gate: bool,
    requires_execution_preparation: bool,
) -> bool {
    match lane_template["inclusion_rule"].as_str().unwrap_or("always") {
        "when_design_gate" => requires_design_gate,
        "when_execution_preparation_required" => requires_execution_preparation,
        "when_flow_requires_coach" => true,
        "when_flow_requires_verification" => true,
        _ => true,
    }
}

fn build_resolved_development_dispatch_contract(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> serde_json::Value {
    let flow_id = compiled_bundle["default_flow_set"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let requires_execution_preparation =
        request_requires_execution_preparation(compiled_bundle, selection);
    let resolved_lanes = resolved_development_flow_templates(compiled_bundle, selection)
        .into_iter()
        .filter(|lane| {
            lane_template_included(lane, requires_design_gate, requires_execution_preparation)
        })
        .map(|lane_template| {
            let preferred_dispatch_alias = lane_template["dispatch_alias"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let task_class = lane_template["task_class"]
                .as_str()
                .unwrap_or("implementation");
            let dispatch_alias = crate::resolve_dispatch_alias_id(
                compiled_bundle,
                &preferred_dispatch_alias,
                task_class,
            )
            .unwrap_or_default();
            let activation = if dispatch_alias.is_empty() {
                serde_json::json!({
                    "enabled": false,
                    "reason": "dispatch_alias_missing_from_lane_template",
                })
            } else {
                crate::build_runtime_assignment_preview_from_dispatch_alias(
                    compiled_bundle,
                    &dispatch_alias,
                    task_class,
                )
            };
            let runtime_role = activation["activation_runtime_role"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| lane_template["runtime_role"].as_str())
                .unwrap_or_default();
            serde_json::json!({
                "lane_id": lane_template["lane_id"],
                "dispatch_target": lane_template["dispatch_target"],
                "dispatch_alias": dispatch_alias,
                "task_class": task_class,
                "runtime_role": runtime_role,
                "packet_template_kind": lane_template["packet_template_kind"],
                "closure_class": lane_template["closure_class"],
                "stage": lane_template["stage"],
                "inclusion_rule": lane_template["inclusion_rule"],
                "completion_blocker": lane_template["completion_blocker"],
                "policy_diagnostics": lane_template["policy_diagnostics"],
                "activation": activation,
            })
        })
        .collect::<Vec<_>>();
    let lane_sequence = resolved_lanes
        .iter()
        .filter_map(|lane| lane["dispatch_target"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let execution_lane_sequence = resolved_lanes
        .iter()
        .filter(|lane| lane["stage"].as_str() != Some("design_gate"))
        .filter_map(|lane| lane["dispatch_target"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let lane_catalog = resolved_lanes
        .iter()
        .fold(serde_json::Map::new(), |mut acc, lane| {
            if let Some(dispatch_target) = lane["dispatch_target"].as_str() {
                acc.insert(dispatch_target.to_string(), lane.clone());
            }
            acc
        });
    serde_json::json!({
        "selected_flow_set": flow_id,
        "execution_preparation_required": requires_execution_preparation,
        "root_session_must_remain_orchestrator": true,
        "packet_family_required": [
            "delivery_task_packet",
            "execution_block_packet",
            "coach_review_packet",
            "verifier_proof_packet",
            "escalation_packet"
        ],
        "resolved_lanes": resolved_lanes,
        "lane_sequence": lane_sequence,
        "execution_lane_sequence": execution_lane_sequence,
        "lane_catalog": lane_catalog,
        "specification_activation": crate::dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "specification"
        ).map(crate::dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "implementer_activation": crate::dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "implementer"
        ).map(crate::dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "coach_activation": crate::dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "coach"
        ).map(crate::dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "verifier_activation": crate::dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "verification"
        ).map(crate::dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
        "escalation_activation": crate::dispatch_contract_lane(
            &serde_json::json!({"development_flow": {"dispatch_contract": {"lane_catalog": lane_catalog.clone()}}}),
            "execution_preparation"
        ).map(crate::dispatch_contract_lane_activation).cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn orchestration_lane_step_label(dispatch_target: &str) -> &'static str {
    match dispatch_target {
        "specification" => "delegate_specification_or_research_lane",
        "implementer" => "delegate_implementer_lane",
        "coach" => "delegate_coach_lane",
        "verification" => "delegate_verifier_lane",
        "execution_preparation" => "delegate_execution_preparation_lane",
        _ => "delegate_lane",
    }
}

fn dispatch_contract_lane_logical_class<'a>(
    dispatch_contract: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a str> {
    let lane = dispatch_contract["lane_catalog"].get(dispatch_target)?;
    lane["task_class"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            lane["runtime_role"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
        })
}

fn orchestration_lane_step_label_for_contract(
    dispatch_contract: &serde_json::Value,
    dispatch_target: &str,
) -> &'static str {
    match dispatch_contract_lane_logical_class(dispatch_contract, dispatch_target) {
        Some("specification" | "business_analyst" | "analyst") => {
            "delegate_specification_or_research_lane"
        }
        Some("implementation" | "worker" | "implementer" | "developer") => {
            "delegate_implementer_lane"
        }
        Some("review" | "coach" | "implementation_review") => "delegate_coach_lane",
        Some("verification" | "verifier" | "proof") => "delegate_verifier_lane",
        Some("architecture" | "solution_architect" | "execution_preparation") => {
            "delegate_execution_preparation_lane"
        }
        _ => orchestration_lane_step_label(dispatch_target),
    }
}

fn orchestration_checkpoint_label(dispatch_target: &str) -> &'static str {
    match dispatch_target {
        "implementer" => "after_implementation_evidence",
        "coach" => "after_review_evidence",
        "verification" => "after_verification_evidence",
        "specification" => "after_design_gate",
        "execution_preparation" => "after_execution_preparation_evidence",
        _ => "after_lane_evidence",
    }
}

fn orchestration_checkpoint_label_for_contract(
    dispatch_contract: &serde_json::Value,
    dispatch_target: &str,
) -> &'static str {
    match dispatch_contract_lane_logical_class(dispatch_contract, dispatch_target) {
        Some("specification" | "business_analyst" | "analyst") => "after_design_gate",
        Some("implementation" | "worker" | "implementer" | "developer") => {
            "after_implementation_evidence"
        }
        Some("review" | "coach" | "implementation_review") => "after_review_evidence",
        Some("verification" | "verifier" | "proof") => "after_verification_evidence",
        Some("architecture" | "solution_architect" | "execution_preparation") => {
            "after_execution_preparation_evidence"
        }
        _ => orchestration_checkpoint_label(dispatch_target),
    }
}

fn build_runtime_orchestration_contract(
    requires_design_gate: bool,
    agent_only_development: bool,
    dispatch_contract: &serde_json::Value,
) -> serde_json::Value {
    let execution_lane_sequence = dispatch_contract["execution_lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let active_cycle = if requires_design_gate {
        let mut cycle = vec![
            "publish_initial_execution_plan".to_string(),
            "delegate_specification_or_research_lane".to_string(),
            "replan_after_design_gate".to_string(),
            "shape_work_pool_and_dev_packets".to_string(),
        ];
        cycle.extend(execution_lane_sequence.iter().map(|lane| {
            orchestration_lane_step_label_for_contract(dispatch_contract, lane).to_string()
        }));
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    } else {
        let mut cycle = vec!["publish_initial_execution_plan".to_string()];
        cycle.extend(execution_lane_sequence.iter().map(|lane| {
            orchestration_lane_step_label_for_contract(dispatch_contract, lane).to_string()
        }));
        cycle.push("synthesize_closure_or_replan".to_string());
        serde_json::json!(cycle)
    };
    let replanning_checkpoints = if requires_design_gate {
        let mut checkpoints = vec![
            "after_design_gate".to_string(),
            "after_work_pool_shape".to_string(),
            "after_dev_packet_shape".to_string(),
        ];
        checkpoints.extend(execution_lane_sequence.iter().map(|lane| {
            orchestration_checkpoint_label_for_contract(dispatch_contract, lane).to_string()
        }));
        serde_json::json!(checkpoints)
    } else {
        let mut checkpoints = vec!["after_packet_shape".to_string()];
        checkpoints.extend(execution_lane_sequence.iter().map(|lane| {
            orchestration_checkpoint_label_for_contract(dispatch_contract, lane).to_string()
        }));
        serde_json::json!(checkpoints)
    };

    serde_json::json!({
        "mode": "delegated_orchestration_cycle",
        "root_session_role": "orchestrator",
        "root_session_must_remain_orchestrator": true,
        "root_session_write_guard": build_root_session_write_guard(),
        "initial_response": {
            "plan_required_before_substantive_execution": true,
            "plan_scope": "one bounded active cycle",
            "must_happen_before": [
                "design_doc_mutation",
                "packet_dispatch",
                "implementation_work"
            ],
            "minimum_fields": [
                "active_bounded_unit",
                "next_steps",
                "delegation_targets",
                "proof_target"
            ],
            "operator_message": "publish a concise execution plan before mutating docs, dispatching work, or entering implementation"
        },
        "delegation_policy": {
            "normal_write_producing_work": "delegated_by_default",
            "agent_only_development_required": agent_only_development,
            "canonical_project_delegated_execution_surface": "vida agent-init",
            "host_subagent_apis_are_backend_details": true,
            "host_local_write_capability_is_not_authority": true,
            "generic_single_worker_dispatch_forbidden": true,
            "local_implementation_without_exception_path_forbidden": true,
            "required_lanes": dispatch_contract["lane_sequence"]
        },
        "replanning": {
            "required": true,
            "checkpoints": replanning_checkpoints,
            "trigger_rule": "replan after each bounded gate or delegated evidence return before the next write-producing step"
        },
        "continuation_binding": {
            "required_for_continue_development": true,
            "fail_closed_without_explicit_binding": true,
            "required_fields": [
                "active_bounded_unit",
                "why_this_unit",
                "primary_path",
                "sequential_vs_parallel_posture"
            ],
            "forbidden_fallbacks": [
                "ready_head[0]",
                "first_ready_backlog_candidate",
                "adjacent_sibling_slice"
            ]
        },
        "active_cycle": active_cycle
    })
}

fn build_root_session_write_guard() -> serde_json::Value {
    serde_json::json!({
        "status": "blocked_by_default",
        "root_session_role": "orchestrator",
        "local_write_requires_exception_path": true,
        "lawful_write_surface": "vida agent-init",
        "explicit_user_ordered_agent_mode_is_sticky": true,
        "saturation_recovery_required_before_local_fallback": true,
        "local_fallback_without_lane_recovery_forbidden": true,
        "host_local_write_capability_is_not_authority": true,
        "required_exception_evidence": crate::status_surface_write_guard::root_session_write_guard_required_exception_evidence(),
        "pre_write_checkpoint_required": true,
    })
}

fn supported_autonomous_execution_settings(
    compiled_bundle: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "agent_only_development": crate::json_bool(
            compiled_bundle["autonomous_execution"].get("agent_only_development"),
            false,
        ),
        "validation_report_required_before_implementation": crate::json_bool(
            compiled_bundle["autonomous_execution"]
                .get("validation_report_required_before_implementation"),
            false,
        ),
    })
}

pub(crate) fn build_runtime_execution_plan_from_snapshot(
    compiled_bundle: &serde_json::Value,
    selection: &crate::RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let agent_system = &compiled_bundle["agent_system"];
    let implementation = crate::runtime_lane_summary::summarize_agent_route_from_snapshot(
        compiled_bundle,
        agent_system,
        "implementation",
    );
    let analysis_route_id = implementation["analysis_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("analysis");
    let mut analysis = crate::runtime_lane_summary::summarize_agent_route_from_snapshot(
        compiled_bundle,
        agent_system,
        analysis_route_id,
    );
    apply_implementation_analysis_route_overrides(&mut analysis, &implementation);
    let coach_route_id = implementation["coach_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("coach");
    let verification_route_id = implementation["verification_route_task_class"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("verification");
    let feature_design_terms =
        crate::feature_delivery_design_terms(&selection.request.to_lowercase());
    let suppress_fresh_design_gate =
        selection.reason == "auto_existing_design_backed_implementation_request_override";
    let requires_design_gate = !suppress_fresh_design_gate
        && (selection.tracked_flow_entry.as_deref() == Some("spec-pack")
            || !feature_design_terms.is_empty());
    let tracked_flow_bootstrap = if requires_design_gate {
        build_design_first_tracked_flow_bootstrap(&selection.request)
    } else {
        serde_json::Value::Null
    };
    let autonomous_execution = supported_autonomous_execution_settings(compiled_bundle);
    let agent_only_development =
        crate::json_bool(autonomous_execution.get("agent_only_development"), false);
    let dispatch_contract = build_resolved_development_dispatch_contract(
        compiled_bundle,
        selection,
        requires_design_gate,
    );
    let orchestration_contract = build_runtime_orchestration_contract(
        requires_design_gate,
        agent_only_development,
        &dispatch_contract,
    );
    let runtime_assignment =
        crate::build_runtime_assignment(compiled_bundle, selection, requires_design_gate);
    let lane_sequence = dispatch_contract["lane_sequence"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let backend_admissibility_matrix =
        crate::runtime_lane_summary::build_executor_backend_admissibility_matrix(agent_system);
    let mut execution_plan = serde_json::json!({
        "status": if requires_design_gate {
            "design_first"
        } else {
            "ready_for_runtime_routing"
        },
        "system_mode": crate::json_string(crate::json_lookup(agent_system, &["mode"])).unwrap_or_default(),
        "state_owner": crate::json_string(crate::json_lookup(agent_system, &["state_owner"])).unwrap_or_default(),
        "max_parallel_agents": crate::json_lookup(agent_system, &["max_parallel_agents"]).cloned().unwrap_or(serde_json::Value::Null),
        "autonomous_execution": autonomous_execution,
        "backend_admissibility_matrix": backend_admissibility_matrix,
        "orchestration_contract": orchestration_contract,
        "default_route": crate::runtime_lane_summary::summarize_agent_route_from_snapshot(compiled_bundle, agent_system, "default"),
        "conversation_stage": {
            "selected_role": selection.selected_role,
            "conversational_mode": selection.conversational_mode,
            "tracked_flow_entry": selection.tracked_flow_entry,
            "allow_freeform_chat": selection.allow_freeform_chat,
            "single_task_only": selection.single_task_only,
        },
        "pre_execution_design_gate": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "not_required"
            },
            "developer_handoff_packet_required": requires_design_gate,
            "developer_handoff_packet_status": if requires_design_gate {
                "blocked_pending_developer_handoff_packet"
            } else {
                "not_required"
            },
            "design_runtime": "vida docflow",
            "design_template": crate::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            "intake_runtime": if requires_design_gate {
                serde_json::Value::String("vida taskflow consume final <request> --json".to_string())
            } else {
                serde_json::Value::Null
            },
            "tracked_handoff": if requires_design_gate {
                serde_json::Value::String("spec-pack".to_string())
            } else {
                serde_json::Value::Null
            },
            "todo_sequence": if requires_design_gate {
                serde_json::json!([
                    "capture research, specification scope, and implementation plan in one bounded design document",
                    "create one epic and one spec task in vida taskflow before code execution",
                    "keep the design artifact canonical through vida docflow init/finalize-edit/check",
                    "close the spec task and shape one bounded execution packet from the approved design before delegated development"
                ])
            } else {
                serde_json::json!([])
            },
            "taskflow_sequence": if requires_design_gate {
                serde_json::json!(["spec-pack", "work-pool-pack", "dev-pack"])
            } else {
                serde_json::json!([])
            }
        },
        "pre_execution_todo": {
            "required": requires_design_gate,
            "status": if requires_design_gate {
                "open"
            } else {
                "not_required"
            },
            "items": if requires_design_gate {
                serde_json::json!([
                    {
                        "id": "taskflow_epic_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one epic that will own the feature-level tracked flow before documentation or implementation begins"
                    },
                    {
                        "id": "taskflow_spec_task_open",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "open one spec-pack task under the epic before authoring the design artifact"
                    },
                    {
                        "id": "design_doc_scope",
                        "owner": "business_analyst",
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "capture research, specification scope, and implementation plan in one bounded design document"
                    },
                    {
                        "id": "design_doc_finalize",
                        "owner": "orchestrator",
                        "runtime": "vida docflow",
                        "status": "pending",
                        "note": "finalize and validate the bounded design artifact canonically"
                    },
                    {
                        "id": "taskflow_spec_task_close",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "close the spec-pack task only after the design artifact is finalized and validated"
                    },
                    {
                        "id": "taskflow_packet_shape",
                        "owner": "orchestrator",
                        "runtime": "vida taskflow",
                        "status": "pending",
                        "note": "shape TaskFlow handoff from spec-pack through work-pool-pack and dev-pack before delegated implementation dispatch"
                    }
                ])
            } else {
                serde_json::json!([])
            }
        },
        "tracked_flow_bootstrap": tracked_flow_bootstrap,
        "development_flow": {
            "activation_status": if requires_design_gate {
                "blocked_pending_design_packet"
            } else {
                "eligible_after_runtime_routing"
            },
            "lane_sequence": lane_sequence,
            "generic_single_worker_dispatch_forbidden": true,
            "dispatch_contract": dispatch_contract,
            "timeout_policy": {
                "worker_wait_timeout_is_not_root_write_permission": true,
                "generic_internal_worker_fallback_forbidden": true,
                "root_session_takeover_requires_exception_receipt": true,
                "next_actions": [
                    "continue_lawful_waiting_or_polling",
                    "inspect_open_delegated_lane_state",
                    "reuse_or_reclaim_eligible_lane_if_lawful",
                    "dispatch_coach_or_verifier_or_escalation_when_route_requires_it",
                    "record_explicit_blocker_or_exception_path_before_any_root_session_write"
                ]
            },
            "implementation": implementation,
            "analysis": analysis,
            "coach": crate::runtime_lane_summary::summarize_agent_route_from_snapshot(compiled_bundle, agent_system, coach_route_id),
            "verification": crate::runtime_lane_summary::summarize_agent_route_from_snapshot(compiled_bundle, agent_system, verification_route_id),
        },
    });
    if let Some(plan) = execution_plan.as_object_mut() {
        plan.insert(
            "root_session_write_guard".to_string(),
            build_root_session_write_guard(),
        );
        plan.extend(crate::runtime_assignment_alias_fields(&runtime_assignment));
    }
    execution_plan
}

#[cfg(test)]
mod tests {
    use super::{
        apply_implementation_analysis_route_overrides, build_design_first_tracked_flow_bootstrap,
        build_resolved_development_dispatch_contract, build_runtime_orchestration_contract,
        supported_autonomous_execution_settings,
    };
    use crate::RuntimeConsumptionLaneSelection;
    use serde_json::json;

    #[test]
    fn design_first_bootstrap_canonicalizes_moved_project_activator_test_proof_target() {
        let request = "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation. Proof target: cargo test -p vida project_activator_command_accepts_json_output -- --nocapture. After a green bounded result, continue with the normal commit, push, release build, and system binary update cycle.";

        let bootstrap = build_design_first_tracked_flow_bootstrap(request);
        let bootstrap_command = bootstrap["bootstrap_command"]
            .as_str()
            .expect("bootstrap command should render");

        assert!(
            bootstrap_command.contains(
                "cargo test -p vida project_activator_surface::tests::project_activator_command_accepts_json_output -- --exact --nocapture"
            ),
            "bootstrap command should carry the canonical exact module-qualified proof target"
        );
        assert!(
            !bootstrap_command.contains(
                "cargo test -p vida project_activator_command_accepts_json_output -- --nocapture"
            ),
            "bootstrap command should not retain the bare moved-test proof target"
        );
    }

    #[test]
    fn design_first_work_packet_commands_carry_execution_semantics() {
        let bootstrap = build_design_first_tracked_flow_bootstrap(
            "Research the feature, write detailed specifications, create a plan, and implement runtime flow",
        );
        let work_pool_ensure = bootstrap["work_pool_task"]["ensure_command"]
            .as_str()
            .expect("work-pool ensure command should render");
        let work_pool_task_id = bootstrap["work_pool_task"]["task_id"]
            .as_str()
            .expect("work-pool task id should render");
        let dev_ensure = bootstrap["dev_task"]["ensure_command"]
            .as_str()
            .expect("dev ensure command should render");
        let dev_task_id = bootstrap["dev_task"]["task_id"]
            .as_str()
            .expect("dev task id should render");

        assert!(work_pool_ensure.contains("--execution-mode container_only"));
        assert!(work_pool_ensure.contains("--parallel-group work-pool-pack"));
        assert!(work_pool_ensure.contains(&format!("--conflict-domain {work_pool_task_id}")));
        assert!(dev_ensure.contains("--execution-mode parallel_safe"));
        assert!(dev_ensure.contains("--parallel-group dev-pack"));
        assert!(dev_ensure.contains(&format!("--conflict-domain {dev_task_id}")));
    }

    #[test]
    fn implementation_analysis_route_overrides_materialize_analysis_policy_knobs() {
        let implementation = json!({
            "analysis_executor_backend": "opencode_cli",
            "analysis_external_first_required": "yes",
            "analysis_fanout_executor_backends": ["hermes_cli", "opencode_cli"],
            "analysis_fanout_min_results": 2,
            "analysis_fanout_subagents": "hermes_cli,opencode_cli",
            "analysis_merge_policy": "consensus_with_conflict_flag",
        });
        let mut analysis = json!({
            "executor_backend": "internal_subagents",
            "fanout_min_results": 1,
            "merge_policy": "first_success",
        });

        apply_implementation_analysis_route_overrides(&mut analysis, &implementation);

        assert_eq!(analysis["executor_backend"], "opencode_cli");
        assert_eq!(analysis["external_first_required"], "yes");
        assert_eq!(
            analysis["fanout_executor_backends"],
            json!(["hermes_cli", "opencode_cli"])
        );
        assert_eq!(analysis["fanout_min_results"], 2);
        assert_eq!(analysis["fanout_subagents"], "hermes_cli,opencode_cli");
        assert_eq!(analysis["merge_policy"], "consensus_with_conflict_flag");
    }

    #[test]
    fn dispatch_contract_uses_configured_dev_team_flow_lane_sequence() {
        let bundle = json!({
            "dev_team_readiness": {
                "roles": [
                    {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "task_classes": ["implementation"],
                        "packet_template_kind": "delivery_task_packet",
                        "closure_class": "implementation",
                        "stage": "execution",
                        "completion_blocker": "configured_implementation_blocker",
                        "inclusion_rule": "always"
                    },
                    {"role_id": "reviewer", "runtime_role": "verifier", "task_classes": ["review"]}
                ],
                "flows": [
                    {
                        "flow_id": "configured_flow",
                        "enabled": true,
                        "ordered_steps": [
                            {"role_id": "analyst"},
                            {"role_id": "developer"},
                            {
                                "role_id": "reviewer",
                                "packet_template_kind": "verifier_proof_packet",
                                "closure_class": "proof",
                                "stage": "execution",
                                "completion_blocker": "configured_review_blocker",
                                "inclusion_rule": "when_flow_requires_verification"
                            }
                        ]
                    }
                ]
            },
            "carrier_runtime": {
                "dispatch_aliases": []
            }
        });
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "configured".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "configured flow".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec!["dev_team_flow_id:configured_flow".to_string()],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, true);

        assert_eq!(
            contract["lane_sequence"],
            json!(["analyst", "developer", "reviewer"])
        );
        assert_eq!(
            contract["execution_lane_sequence"],
            json!(["developer", "reviewer"])
        );
        let orchestration_contract = build_runtime_orchestration_contract(true, true, &contract);
        assert!(orchestration_contract["active_cycle"]
            .as_array()
            .expect("active cycle should be an array")
            .iter()
            .any(|step| step == "delegate_implementer_lane"));
        assert!(orchestration_contract["replanning"]["checkpoints"]
            .as_array()
            .expect("replanning checkpoints should be an array")
            .iter()
            .any(|step| step == "after_implementation_evidence"));
        assert_eq!(
            contract["lane_catalog"]["developer"]["runtime_role"],
            "worker"
        );
        assert_eq!(
            contract["lane_catalog"]["developer"]["completion_blocker"],
            "configured_implementation_blocker"
        );
        assert_eq!(
            contract["lane_catalog"]["developer"]["policy_diagnostics"]["fallback_used"],
            false
        );
        assert_eq!(contract["lane_catalog"]["reviewer"]["task_class"], "review");
        assert_eq!(
            contract["lane_catalog"]["reviewer"]["packet_template_kind"],
            "verifier_proof_packet"
        );
        assert_eq!(
            contract["lane_catalog"]["reviewer"]["completion_blocker"],
            "configured_review_blocker"
        );
        assert_eq!(
            contract["lane_catalog"]["reviewer"]["inclusion_rule"],
            "when_flow_requires_verification"
        );
        assert_eq!(
            contract["lane_catalog"]["reviewer"]["policy_diagnostics"]["fallback_used"],
            false
        );
        assert_eq!(
            contract["lane_catalog"]["analyst"]["policy_diagnostics"]["fallback_used"],
            true
        );
        assert!(
            contract["lane_catalog"]["analyst"]["policy_diagnostics"]["fallback_fields"]
                .as_array()
                .expect("fallback fields should be reported")
                .contains(&json!("packet_template_kind")),
            "missing configured policy fields must be reported explicitly"
        );
    }

    #[test]
    fn dispatch_contract_keeps_legacy_flow_when_dev_team_readiness_is_absent() {
        let bundle = json!({});
        let selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "legacy".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "legacy flow".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "test".to_string(),
            matched_terms: vec![],
            compiled_bundle: bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        let contract = build_resolved_development_dispatch_contract(&bundle, &selection, false);

        assert_eq!(
            contract["lane_sequence"],
            json!(["implementer", "coach", "verification"])
        );
    }

    #[test]
    fn supported_autonomous_execution_settings_excludes_unwired_overlay_toggles() {
        let settings = supported_autonomous_execution_settings(&json!({
            "autonomous_execution": {
                "agent_only_development": true,
                "validation_report_required_before_implementation": true,
                "spec_ready_auto_development": true,
                "resume_after_validation_gate": true
            }
        }));

        assert_eq!(
            settings,
            json!({
                "agent_only_development": true,
                "validation_report_required_before_implementation": true
            })
        );
    }
}
