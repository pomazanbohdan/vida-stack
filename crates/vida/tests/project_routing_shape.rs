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

#[test]
fn project_routing_shape_diversifies_primary_external_backends() {
    let config = project_config();

    assert_eq!(
        yaml_string(&route(&config, "default")["executor_backend"]),
        Some("opencode_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "analysis")["executor_backend"]),
        Some("opencode_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "coach")["executor_backend"]),
        Some("hermes_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "review")["executor_backend"]),
        Some("hermes_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "ui_review")["executor_backend"]),
        Some("vibe_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "verification")["executor_backend"]),
        Some("opencode_cli")
    );
}

#[test]
fn project_routing_shape_matches_current_research_and_ensemble_fanout() {
    let config = project_config();

    let research_fanout = yaml_string_list(&route(&config, "research")["fanout_executor_backends"]);
    assert!(research_fanout
        .iter()
        .any(|backend| backend == "hermes_cli"));
    assert!(research_fanout
        .iter()
        .any(|backend| backend == "opencode_cli"));
    assert!(research_fanout.iter().any(|backend| backend == "kilo_cli"));
    assert!(research_fanout.iter().any(|backend| backend == "vibe_cli"));
    assert!(!research_fanout.iter().any(|backend| backend == "qwen_cli"));

    let review_ensemble_fanout =
        yaml_string_list(&route(&config, "review_ensemble")["fanout_executor_backends"]);
    assert_eq!(review_ensemble_fanout.len(), 2);
    assert!(review_ensemble_fanout
        .iter()
        .any(|backend| backend == "hermes_cli"));
    assert!(review_ensemble_fanout
        .iter()
        .any(|backend| backend == "opencode_cli"));
    assert!(!review_ensemble_fanout
        .iter()
        .any(|backend| backend == "qwen_cli"));

    let verification_ensemble_fanout =
        yaml_string_list(&route(&config, "verification_ensemble")["fanout_executor_backends"]);
    assert!(verification_ensemble_fanout.len() >= 4);
    assert!(verification_ensemble_fanout
        .iter()
        .any(|backend| backend == "opencode_cli"));
    assert!(verification_ensemble_fanout
        .iter()
        .any(|backend| backend == "kilo_cli"));
    assert!(verification_ensemble_fanout
        .iter()
        .any(|backend| backend == "hermes_cli"));
    assert!(verification_ensemble_fanout
        .iter()
        .any(|backend| backend == "vibe_cli"));
    assert!(!verification_ensemble_fanout
        .iter()
        .any(|backend| backend == "qwen_cli"));
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
            Some("hermes_cli"),
            "{route_id} should route coach review through hermes",
        );
    }

    assert_eq!(
        yaml_string(&route(&config, "small_patch")["analysis_executor_backend"]),
        Some("opencode_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "ui_patch")["analysis_executor_backend"]),
        Some("vibe_cli")
    );
    assert_eq!(
        yaml_string(&route(&config, "implementation")["analysis_executor_backend"]),
        Some("opencode_cli")
    );
}
