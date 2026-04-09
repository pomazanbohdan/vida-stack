pub(crate) fn resolved_worker_strategy(
    project_root: &std::path::Path,
    roles: &[serde_json::Value],
    scoring_policy: &serde_json::Value,
) -> serde_json::Value {
    if roles.is_empty() {
        serde_json::json!({
            "schema_version": 1,
            "store_path": super::WORKER_STRATEGY_STATE,
            "scorecards_path": super::WORKER_SCORECARDS_STATE,
            "agents": {}
        })
    } else {
        super::refresh_worker_strategy(project_root, roles, scoring_policy)
    }
}

pub(crate) fn resolved_pricing_policy(
    config: &serde_yaml::Value,
    roles: &[serde_json::Value],
    worker_strategy: &serde_json::Value,
) -> serde_json::Value {
    crate::host_agent_state::build_carrier_pricing_policy(
        roles,
        worker_strategy,
        &crate::carrier_runtime_metadata::pricing_vendor_basis(config),
    )
}
