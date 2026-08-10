pub mod engine;
pub mod health;
pub mod jobs;
pub mod shadow;
pub mod workers;

#[cfg(test)]
mod tests {
    #[test]
    fn module_exports_local_engine_capability_contract() {
        let snapshot =
            serde_json::to_value(crate::engine::local_runtime_capabilities()).expect("snapshot");

        assert_eq!(snapshot["engine_id"], crate::engine::LOCAL_ENGINE_ID);
        assert_eq!(snapshot["engine_kind"], "local_redb_effectum");
        assert!(snapshot["capabilities"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["capability"] == "jobs" && entry["supported"] == true)
        }));
    }
}
