//! Run-graph model defaults for TaskFlow runtime decomposition.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultRunGraphStatusFields {
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub route_task_class: String,
    pub selected_backend: String,
    pub lane_id: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

#[must_use]
pub fn default_run_graph_status_fields(
    task_id: impl Into<String>,
    task_class: impl Into<String>,
    route_task_class: impl Into<String>,
) -> DefaultRunGraphStatusFields {
    let task_id = task_id.into();
    let task_class = task_class.into();
    DefaultRunGraphStatusFields {
        run_id: task_id.clone(),
        task_id,
        task_class: task_class.clone(),
        active_node: task_class,
        next_node: None,
        status: "pending".to_string(),
        route_task_class: route_task_class.into(),
        selected_backend: "unknown".to_string(),
        lane_id: "unassigned".to_string(),
        lifecycle_stage: "initialized".to_string(),
        policy_gate: "not_required".to_string(),
        handoff_state: "none".to_string(),
        context_state: "open".to_string(),
        checkpoint_kind: "none".to_string(),
        resume_target: "none".to_string(),
        recovery_ready: false,
    }
}

#[cfg(test)]
mod tests {
    use super::default_run_graph_status_fields;

    #[test]
    fn default_run_graph_status_fields_keep_legacy_defaults() {
        let fields = default_run_graph_status_fields("task-1", "developer", "implementation");

        assert_eq!(fields.run_id, "task-1");
        assert_eq!(fields.task_id, "task-1");
        assert_eq!(fields.task_class, "developer");
        assert_eq!(fields.active_node, "developer");
        assert_eq!(fields.route_task_class, "implementation");
        assert_eq!(fields.status, "pending");
        assert_eq!(fields.selected_backend, "unknown");
        assert_eq!(fields.lane_id, "unassigned");
        assert_eq!(fields.lifecycle_stage, "initialized");
        assert_eq!(fields.policy_gate, "not_required");
        assert_eq!(fields.handoff_state, "none");
        assert_eq!(fields.context_state, "open");
        assert_eq!(fields.checkpoint_kind, "none");
        assert_eq!(fields.resume_target, "none");
        assert!(!fields.recovery_ready);
        assert!(fields.next_node.is_none());
    }
}
