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

#[cfg(test)]
mod tests {
    use super::resolved_worker_strategy;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn resolved_worker_strategy_empty_roles_returns_stable_default_shape() {
        let strategy = resolved_worker_strategy(Path::new("unused"), &[], &json!({}));

        assert_eq!(strategy["schema_version"], 1);
        assert_eq!(strategy["store_path"], super::super::WORKER_STRATEGY_STATE);
        assert_eq!(
            strategy["scorecards_path"],
            super::super::WORKER_SCORECARDS_STATE
        );
        assert_eq!(strategy["agents"], json!({}));
    }
}
