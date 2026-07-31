pub(crate) const MANAGEMENT_RUNTIME_STATUS: &str = "always_on";
pub(crate) const MANAGEMENT_RUNTIME_AUTHORITY: &str = "task_lifecycle";

pub(crate) fn runtime_metadata() -> serde_json::Value {
    serde_json::json!({
        "status": MANAGEMENT_RUNTIME_STATUS,
        "authority": MANAGEMENT_RUNTIME_AUTHORITY,
        "task_storage": "canonical_shared_state_store",
    })
}
