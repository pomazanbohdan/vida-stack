pub(crate) fn build_project_activator_normal_work_defaults(
    default_agent_topology: Vec<String>,
    carrier_tier_rates: serde_json::Map<String, serde_json::Value>,
    execution_carrier_model: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "documentation_first_for_feature_requests": true,
        "intake_runtime": "vida taskflow consume final <request> --json",
        "local_feature_design_template": crate::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
        "local_product_spec_guide": crate::DEFAULT_PROJECT_PRODUCT_SPEC_README,
        "local_documentation_tooling_map": crate::DEFAULT_PROJECT_DOC_TOOLING_DOC,
        "local_agent_guide": crate::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
        "local_host_agent_guide": crate::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
        "default_agent_topology": default_agent_topology,
        "carrier_tier_rates": carrier_tier_rates,
        "local_agent_score_state": {
            "strategy_store": crate::WORKER_STRATEGY_STATE,
            "scorecards_store": crate::WORKER_SCORECARDS_STATE
        },
        "execution_carrier_model": execution_carrier_model,
        "recommended_flow": [
            "create or update one bounded design document before code execution when the request asks for research/specification/planning and implementation together",
            "open one feature epic and one spec-pack task in vida taskflow before delegated implementation begins",
            "use vida docflow to initialize, finalize, and validate the design document",
            "close the spec-pack task and shape the execution packet from the bounded file set and proof targets recorded in the design document",
            "delegate normal write-producing work through the default carrier tier ladder and let runtime pick the cheapest capable tier with a healthy local score instead of collapsing directly into root-session coding"
        ]
    })
}
