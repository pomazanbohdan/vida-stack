use std::path::PathBuf;

fn repo_yaml(relative_path: &str) -> serde_yaml::Value {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative_path);
    serde_yaml::from_str(
        &std::fs::read_to_string(&config_path).expect("repo yaml file should read"),
    )
    .expect("repo yaml file should parse")
}

fn project_config() -> serde_yaml::Value {
    repo_yaml("vida.config.yaml")
}

fn route<'a>(config: &'a serde_yaml::Value, route_id: &str) -> &'a serde_yaml::Value {
    &config["agent_system"]["routing"][route_id]
}

fn subagent<'a>(config: &'a serde_yaml::Value, backend_id: &str) -> &'a serde_yaml::Value {
    &config["agent_system"]["subagents"][backend_id]
}

fn yaml_string(value: &serde_yaml::Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn yaml_string_list(value: &serde_yaml::Value) -> Vec<String> {
    value
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn yaml_bool(value: &serde_yaml::Value) -> Option<bool> {
    value.as_bool()
}

fn assert_contains_all(actual: &[String], expected: &[&str], context: &str) {
    for item in expected {
        assert!(
            actual.iter().any(|actual_item| actual_item == item),
            "{context} should include {item}; actual={actual:?}",
        );
    }
}

fn assert_route_backend(config: &serde_yaml::Value, route_id: &str, field: &str, expected: &str) {
    assert_eq!(
        yaml_string(&route(config, route_id)[field]),
        Some(expected),
        "{route_id}.{field} should follow the active project routing policy",
    );
}

fn assert_no_legacy_external_backends(backends: &[String]) {
    for legacy_backend in ["opencode_cli", "hermes_cli", "qwen_cli"] {
        assert!(
            !backends.iter().any(|backend| backend == legacy_backend),
            "active route fanout must not require legacy external backend {legacy_backend}",
        );
    }
}

fn assert_stage_attempt_policy_catalog(
    config: &serde_yaml::Value,
    expected_stage_ids: &[&str],
    context: &str,
) {
    let policies = &config["agent_system"]["stage_attempt_policies"];
    assert!(
        !policies.is_null(),
        "{context} should define agent_system.stage_attempt_policies",
    );
    for stage_id in expected_stage_ids {
        let policy = &policies[*stage_id];
        assert!(
            !policy.is_null(),
            "{context} should define stage attempt policy {stage_id}",
        );
        assert!(
            policy["fanout"]["max_attempts"]
                .as_i64()
                .unwrap_or_default()
                > 0,
            "{context} {stage_id} should define positive fanout.max_attempts",
        );
        let attempts = policy["attempts"]
            .as_sequence()
            .expect("stage policy attempts should render");
        assert!(
            !attempts.is_empty(),
            "{context} {stage_id} should define at least one attempt",
        );
        for attempt in attempts {
            assert!(
                yaml_string(&attempt["carrier_id"]).is_some(),
                "{context} {stage_id} attempts should bind carrier_id",
            );
            assert!(
                yaml_string(&attempt["model_profile_id"]).is_some(),
                "{context} {stage_id} attempts should bind model_profile_id",
            );
            assert!(
                yaml_string(&attempt["isolation"]).is_some(),
                "{context} {stage_id} attempts should bind isolation",
            );
        }
        let consolidator = &policy["consolidator"];
        assert!(
            yaml_string(&consolidator["carrier_id"]).is_some(),
            "{context} {stage_id} should bind consolidator carrier_id",
        );
        assert!(
            yaml_string(&consolidator["model_profile_id"]).is_some(),
            "{context} {stage_id} should bind consolidator model_profile_id",
        );
    }
}

fn assert_configured_flow_catalog(config: &serde_yaml::Value, context: &str) {
    let enabled_hooks = yaml_string_list(&config["agent_extensions"]["enabled_hook_templates"]);
    assert_contains_all(
        &enabled_hooks,
        &[
            "command_timing_summary",
            "approval_wait",
            "approval_complete",
        ],
        &format!("{context} enabled hook templates"),
    );

    for (work_item, flow_id) in [
        ("pull_request", "pr_processing_team"),
        ("pr_repair", "pr_processing_team"),
        ("runtime_defect", "runtime_defect_remediation"),
        ("architecture", "architecture_design"),
        ("release_readiness", "release_readiness_gate"),
        ("service_tui", "service_tui_orchestration"),
        (
            "internal_agent_development",
            "hook_enabled_internal_agent_development",
        ),
    ] {
        assert_eq!(
            yaml_string(&config["dev_team"]["work_item_flow_bindings"][work_item]),
            Some(flow_id),
            "{context} should bind {work_item} to {flow_id}",
        );
        assert_configured_flow(config, flow_id, context);
    }

    assert_pr_repair_flow(config, context);
    assert_approval_gated_architecture_flow(config, context);
}

fn assert_configured_flow(config: &serde_yaml::Value, flow_id: &str, context: &str) {
    let flow = &config["dev_team"]["flows"][flow_id];
    assert!(
        !flow.is_null(),
        "{context} should define dev_team.flows.{flow_id}",
    );
    assert_eq!(
        yaml_bool(&flow["enabled"]),
        Some(true),
        "{context} {flow_id} should be enabled",
    );
    assert_eq!(
        yaml_string(&flow["adapter_projection"]["host_agent_bridge_contract"]),
        Some("required"),
        "{context} {flow_id} should require host-agent bridge projection",
    );
    assert_eq!(
        yaml_bool(&flow["adapter_projection"]["process_carrier_requires_explicit_backend"]),
        Some(true),
        "{context} {flow_id} should keep process carriers explicit",
    );
    assert!(
        flow["steps"]
            .as_sequence()
            .map(|steps| !steps.is_empty())
            .unwrap_or(false),
        "{context} {flow_id} should have ordered steps",
    );
    assert!(
        flow["steps"]
            .as_sequence()
            .into_iter()
            .flatten()
            .any(
                |step| yaml_string(&step["command_template"]["surface"]) == Some("vida agent-init")
            ),
        "{context} {flow_id} should dispatch through vida agent-init templates",
    );
    assert_contains_all(
        &yaml_string_list(&flow["lifecycle_hook_templates"]),
        &["command_timing_summary"],
        &format!("{context} {flow_id} lifecycle hooks"),
    );
}

fn assert_pr_repair_flow(config: &serde_yaml::Value, context: &str) {
    let flow = &config["dev_team"]["flows"]["pr_repair_verified"];
    assert_eq!(
        yaml_string_list(&flow["work_item_bindings"]),
        vec!["pull_request".to_string(), "pr_repair".to_string()],
        "{context} PR flow should bind both pull_request and pr_repair work items",
    );
    let steps = flow["steps"]
        .as_sequence()
        .expect("PR flow should expose ordered steps");
    let step_ids = steps
        .iter()
        .filter_map(|step| yaml_string(&step["step_id"]))
        .collect::<Vec<_>>();
    assert_eq!(
        step_ids,
        vec![
            "pr-triage",
            "pr-ci-review",
            "pr-repair-or-integrate",
            "pr-coach",
            "pr-proof",
        ],
        "{context} PR flow should keep the configured PR command sequence",
    );
    let runtime_roles = steps
        .iter()
        .filter_map(|step| yaml_string(&step["runtime_role"]))
        .collect::<Vec<_>>();
    assert_eq!(
        runtime_roles,
        vec!["business_analyst", "verifier", "worker", "coach", "prover"],
        "{context} PR flow should keep triage, CI review, repair, coach, and proof roles",
    );
}

fn assert_approval_gated_architecture_flow(config: &serde_yaml::Value, context: &str) {
    let flow = &config["dev_team"]["flows"]["architecture_design"];
    let architect_role = &config["dev_team"]["roles"]["architect"];
    assert_eq!(
        yaml_string(&architect_role["runtime_role"]),
        Some("solution_architect"),
        "{context} architecture flow should use a configured architect role contract",
    );
    assert_contains_all(
        &yaml_string_list(&architect_role["task_classes"]),
        &["architecture", "execution_preparation"],
        &format!("{context} architect role task classes"),
    );
    assert_contains_all(
        &yaml_string_list(&flow["lifecycle_hook_templates"]),
        &["approval_wait", "approval_complete"],
        &format!("{context} architecture_design lifecycle hooks"),
    );
    let steps = flow["steps"]
        .as_sequence()
        .expect("architecture flow should expose ordered steps");
    let first_step = &steps[0];
    assert_eq!(
        yaml_bool(&first_step["requires_user_approval"]),
        Some(true),
        "{context} architecture_design should pause after analysis for user approval",
    );
    let execution_prep_step = steps
        .iter()
        .find(|step| yaml_string(&step["task_class"]) == Some("execution_preparation"))
        .expect("architecture flow should include execution preparation");
    assert_eq!(
        yaml_string(&execution_prep_step["role_id"]),
        Some("architect"),
        "{context} architecture_design execution preparation should use the configured architect flow role",
    );
    assert_eq!(
        yaml_string(&execution_prep_step["runtime_role"]),
        Some("solution_architect"),
        "{context} architecture_design execution preparation should dispatch as solution_architect",
    );
    assert_eq!(
        yaml_string_list(&execution_prep_step["command_template"]["args"]),
        vec![
            "--role".to_string(),
            "solution_architect".to_string(),
            "{{task_id}}".to_string(),
            "--json".to_string(),
        ],
        "{context} architecture_design execution preparation command should bind solution_architect",
    );
}

fn assert_enabled_hooks_have_templates(
    config: &serde_yaml::Value,
    hook_registry: &serde_yaml::Value,
) {
    let registered_hooks = hook_registry["hook_templates"]
        .as_sequence()
        .expect("hook registry should expose hook_templates")
        .iter()
        .filter_map(|hook| yaml_string(&hook["template_id"]))
        .collect::<Vec<_>>();

    for hook_id in yaml_string_list(&config["agent_extensions"]["enabled_hook_templates"]) {
        assert!(
            registered_hooks
                .iter()
                .any(|registered| *registered == hook_id),
            "enabled hook {hook_id} should be declared in docs/product/spec/hook-templates.yaml",
        );
    }
}

#[test]
fn project_routing_shape_separates_internal_host_agents_from_codex_cli_exec() {
    let config = project_config();
    let codex_system = &config["host_environment"]["systems"]["codex"];
    let internal = subagent(&config, "internal_subagents");
    let cli_exec = subagent(&config, "codex_cli_exec");

    assert_eq!(
        yaml_string(&codex_system["execution_class"]),
        Some("internal")
    );
    assert_eq!(
        yaml_string(&codex_system["execution_boundary"]),
        Some("parent_host_session")
    );
    assert_eq!(
        yaml_string(&codex_system["dispatch_transport"]),
        Some("host_tool_bridge")
    );
    assert!(
        codex_system["dispatch"].is_null(),
        "internal codex host posture must not carry a codex exec dispatch command"
    );

    assert_eq!(
        yaml_string(&internal["subagent_backend_class"]),
        Some("internal")
    );
    assert_eq!(
        yaml_string(&internal["execution_boundary"]),
        Some("parent_host_session")
    );
    assert_eq!(
        yaml_string(&internal["dispatch_transport"]),
        Some("host_tool_bridge")
    );
    assert_eq!(
        yaml_string(&internal["receipt_mode"]),
        Some("host_bridge_receipt")
    );

    assert_eq!(
        yaml_string(&cli_exec["subagent_backend_class"]),
        Some("external_cli")
    );
    assert_eq!(
        yaml_string(&cli_exec["execution_boundary"]),
        Some("child_process")
    );
    assert_eq!(
        yaml_string(&cli_exec["dispatch_transport"]),
        Some("codex_cli_exec")
    );
    assert_eq!(yaml_string(&cli_exec["dispatch"]["command"]), Some("codex"));
    assert_eq!(
        yaml_string_list(&cli_exec["dispatch"]["static_args"]),
        vec!["exec".to_string(), "--json".to_string()]
    );
}

#[test]
fn project_routing_shape_uses_internal_defaults_and_configured_vibe_coach() {
    let config = project_config();

    for route_id in ["default", "analysis", "review", "ui_review", "verification"] {
        assert_route_backend(&config, route_id, "executor_backend", "internal_subagents");
        if yaml_string(&route(&config, route_id)["fallback_executor_backend"]).is_some() {
            assert_route_backend(
                &config,
                route_id,
                "fallback_executor_backend",
                "internal_subagents",
            );
        }
        assert_eq!(
            yaml_string(&route(&config, route_id)["external_first_required"]),
            Some("no"),
            "{route_id} should not require an external-first route",
        );
    }

    assert_route_backend(&config, "coach", "executor_backend", "vibe_cli");
    assert_route_backend(
        &config,
        "coach",
        "fallback_executor_backend",
        "internal_subagents",
    );
}

#[test]
fn project_routing_shape_matches_current_internal_fanout_policy() {
    let config = project_config();

    let research_fanout = yaml_string_list(&route(&config, "research")["fanout_executor_backends"]);
    assert_eq!(research_fanout, vec!["internal_subagents".to_string()]);
    assert_no_legacy_external_backends(&research_fanout);

    let review_ensemble_fanout =
        yaml_string_list(&route(&config, "review_ensemble")["fanout_executor_backends"]);
    assert_eq!(
        review_ensemble_fanout,
        vec!["internal_subagents".to_string()]
    );
    assert_no_legacy_external_backends(&review_ensemble_fanout);

    let verification_ensemble_fanout =
        yaml_string_list(&route(&config, "verification_ensemble")["fanout_executor_backends"]);
    assert_eq!(
        verification_ensemble_fanout,
        vec!["internal_subagents".to_string()]
    );
    assert_no_legacy_external_backends(&verification_ensemble_fanout);
}

#[test]
fn project_routing_shape_keeps_write_routes_internal_fallback_with_diversified_read_only_prep() {
    let config = project_config();

    for route_id in [
        "small_patch",
        "small_patch_write",
        "ui_patch",
        "implementation",
    ] {
        let route = route(&config, route_id);
        assert_eq!(
            yaml_string(&route["fallback_executor_backend"]),
            Some("internal_subagents"),
            "{route_id} should retain internal fallback",
        );
        assert_eq!(
            yaml_string(&route["coach_executor_backend"]),
            Some("vibe_cli"),
            "{route_id} should route coach review through the configured coach backend",
        );
    }

    for route_id in ["small_patch", "ui_patch", "implementation"] {
        assert_route_backend(
            &config,
            route_id,
            "analysis_executor_backend",
            "internal_subagents",
        );
    }
}

#[test]
fn project_routing_shape_defines_stage_attempt_policies() {
    let project = project_config();
    let template = repo_yaml("docs/framework/templates/vida.config.yaml.template");

    assert_stage_attempt_policy_catalog(
        &project,
        &[
            "analysis",
            "specification",
            "implementation",
            "regression_test",
            "review",
        ],
        "root project config",
    );
    assert_stage_attempt_policy_catalog(
        &template,
        &["analysis", "specification", "implementation", "review"],
        "project config template",
    );
}

#[test]
fn project_routing_shape_defines_configurable_pr_and_specialized_flow_presets() {
    let project = project_config();
    let template = repo_yaml("docs/framework/templates/vida.config.yaml.template");
    let hook_registry = repo_yaml("docs/product/spec/hook-templates.yaml");

    assert_configured_flow_catalog(&project, "root project config");
    assert_configured_flow_catalog(&template, "project config template");
    assert_enabled_hooks_have_templates(&project, &hook_registry);
    assert_enabled_hooks_have_templates(&template, &hook_registry);
}
