use std::path::Path;

use crate::project_activator_surface::{
    canonical_project_activation_status_truth, ProjectActivationStatusTruth,
};
use crate::status_surface_host_agents::build_host_agent_status_summary;
use crate::status_surface_write_guard::root_session_write_guard_summary_from_snapshot_path;

pub(crate) struct StatusTruthInputs {
    pub(crate) host_agents: Option<serde_json::Value>,
    pub(crate) latest_final_snapshot_path: Option<String>,
    pub(crate) latest_recorded_final_snapshot_path: Option<String>,
    pub(crate) root_session_write_guard: serde_json::Value,
    pub(crate) activation_truth: Option<ProjectActivationStatusTruth>,
    pub(crate) project_activation_status: Option<String>,
    pub(crate) project_activation_pending: bool,
}

pub(crate) fn build_status_truth_inputs(
    state_root: &Path,
    runtime_consumption_latest_snapshot_path: Option<&str>,
) -> StatusTruthInputs {
    let status_project_root = crate::resolve_status_project_root(state_root);
    let mut host_agents = status_project_root
        .as_deref()
        .and_then(build_host_agent_status_summary);
    let latest_final_snapshot_path =
        crate::runtime_consumption_state::latest_final_runtime_consumption_snapshot_path(
            state_root,
        )
        .ok()
        .flatten();
    let latest_recorded_final_snapshot_path =
        crate::runtime_consumption_state::latest_recorded_final_runtime_consumption_snapshot_path(
            state_root,
        )
        .ok()
        .flatten();
    let root_session_write_guard = root_session_write_guard_summary_from_snapshot_path(
        latest_final_snapshot_path
            .as_deref()
            .or(runtime_consumption_latest_snapshot_path),
    );
    if let Some(host_agents_value) = host_agents.as_mut() {
        if let Some(object) = host_agents_value.as_object_mut() {
            object.insert(
                "root_session_write_guard".to_string(),
                root_session_write_guard.clone(),
            );
        }
    }
    let activation_truth = status_project_root
        .as_deref()
        .map(canonical_project_activation_status_truth);
    let project_activation_status = activation_truth.as_ref().map(|truth| {
        crate::activation_status::canonical_activation_status(
            Some(truth.status.as_str()),
            truth.activation_pending,
        )
        .to_string()
    });
    let project_activation_pending = project_activation_status.as_deref() == Some("pending");

    StatusTruthInputs {
        host_agents,
        latest_final_snapshot_path,
        latest_recorded_final_snapshot_path,
        root_session_write_guard,
        activation_truth,
        project_activation_status,
        project_activation_pending,
    }
}
