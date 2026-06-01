use std::path::PathBuf;

fn project_config() -> serde_yaml::Value {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("vida.config.yaml");
    serde_yaml::from_str(
        &std::fs::read_to_string(&config_path).expect("project config should read"),
    )
    .expect("project config should parse")
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
