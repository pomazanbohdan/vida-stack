use std::{fs, path::Path};

use serde::Serialize;

use crate::state_store::TaskRecord;

pub(crate) const DISPATCH_RUNTIME_DISABLED_CODE: &str = "dispatch_runtime_disabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskRuntimeMode {
    ManagementOnly,
    DispatchEnabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "source")]
pub(crate) enum TaskLifecycleMutationSource {
    Management,
    DispatchReceipt { run_id: String, receipt_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskExecutionBinding {
    ManagementOnly,
    ExecutionBound,
}

pub(crate) struct TaskLifecycleService;

impl TaskLifecycleService {
    pub(crate) fn authorize(
        mode: TaskRuntimeMode,
        binding: TaskExecutionBinding,
        source: &TaskLifecycleMutationSource,
    ) -> Result<(), &'static str> {
        match source {
            TaskLifecycleMutationSource::Management
                if mode == TaskRuntimeMode::DispatchEnabled
                    && binding == TaskExecutionBinding::ExecutionBound =>
            {
                Err("dispatch_runtime_owns_execution_bound_lifecycle")
            }
            TaskLifecycleMutationSource::DispatchReceipt { run_id, receipt_id }
                if mode != TaskRuntimeMode::DispatchEnabled =>
            {
                let _ = (run_id, receipt_id);
                Err(DISPATCH_RUNTIME_DISABLED_CODE)
            }
            TaskLifecycleMutationSource::DispatchReceipt { run_id, receipt_id }
                if run_id.trim().is_empty() || receipt_id.trim().is_empty() =>
            {
                let _ = (run_id, receipt_id);
                Err("dispatch_receipt_required")
            }
            TaskLifecycleMutationSource::DispatchReceipt { .. }
                if binding != TaskExecutionBinding::ExecutionBound =>
            {
                Err("dispatch_requires_execution_bound_task")
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn authorize_close(
        mode: TaskRuntimeMode,
        binding: TaskExecutionBinding,
        source: &TaskLifecycleMutationSource,
        proof_admitted: bool,
    ) -> Result<(), &'static str> {
        Self::authorize(mode, binding, source)?;
        if matches!(source, TaskLifecycleMutationSource::DispatchReceipt { .. }) && !proof_admitted
        {
            return Err("dispatch_receipt_proof_required");
        }
        Ok(())
    }
}

pub(crate) fn taskflow_dispatch_enabled(config: &serde_yaml::Value) -> bool {
    crate::yaml_lookup(config, &["taskflow", "dispatch", "enabled"])
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn taskflow_dispatch_enabled_for_state_root(state_root: &Path) -> bool {
    let Some(project_root) =
        crate::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
    else {
        // An isolated state store has no explicit project configuration. Never infer
        // execution authority from the absence of that configuration.
        return false;
    };
    let config_path = project_root.join("vida.config.yaml");
    if !config_path.exists() {
        return false;
    }
    crate::project_activator_surface::read_yaml_file_checked(&config_path)
        .map(|config| taskflow_dispatch_enabled(&config))
        .unwrap_or(false)
}

pub(crate) fn task_runtime_mode_for_state_root(state_root: &Path) -> TaskRuntimeMode {
    if taskflow_dispatch_enabled_for_state_root(state_root) {
        TaskRuntimeMode::DispatchEnabled
    } else {
        TaskRuntimeMode::ManagementOnly
    }
}

pub(crate) fn task_execution_binding(
    task: &TaskRecord,
    has_active_run: bool,
) -> TaskExecutionBinding {
    let explicit_execution_plan = task.execution_semantics != Default::default();
    if has_active_run || explicit_execution_plan {
        TaskExecutionBinding::ExecutionBound
    } else {
        TaskExecutionBinding::ManagementOnly
    }
}

pub(crate) fn task_is_execution_bound(task: &TaskRecord) -> bool {
    task_execution_binding(task, false) == TaskExecutionBinding::ExecutionBound
}

pub(crate) fn dispatch_runtime_disabled_payload(
    surface: &str,
    mode: TaskRuntimeMode,
) -> serde_json::Value {
    serde_json::json!({
        "surface": surface,
        "status": "blocked",
        "runtime": "task_dispatch",
        "mode": mode,
        "blocker_codes": [DISPATCH_RUNTIME_DISABLED_CODE],
        "next_actions": [
            "Set taskflow.dispatch.enabled: true and rerun the dispatch command."
        ],
        "artifact_refs": {
            "surface": surface,
            "runtime": "task_dispatch",
        },
    })
}

pub(crate) fn management_status_projection() -> serde_json::Value {
    management_status_projection_with_counts(0)
}

pub(crate) fn management_status_projection_with_counts(
    execution_bound_count: usize,
) -> serde_json::Value {
    serde_json::json!({
        "mode": "management",
        "enabled": true,
        "status": "always_on",
        "authority": "task_lifecycle",
        "execution_bound_count": execution_bound_count,
        "adopted_count": 0,
        "unadopted_count": 0,
        "blocker_codes": [],
        "next_actions": [],
        "artifact_refs": {},
    })
}

pub(crate) fn dispatch_status_projection(state_root: &Path) -> serde_json::Value {
    dispatch_status_projection_with_counts(state_root, 0)
}

pub(crate) fn dispatch_status_projection_with_counts(
    state_root: &Path,
    execution_bound_count: usize,
) -> serde_json::Value {
    let enabled = taskflow_dispatch_enabled_for_state_root(state_root);
    let adoption_path = state_root.join("taskflow-dispatch-adoptions.jsonl");
    let adopted_count = fs::read_to_string(&adoption_path)
        .ok()
        .map(|body| body.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0);
    if !enabled {
        let mut payload =
            dispatch_runtime_disabled_payload("taskflow.dispatch", TaskRuntimeMode::ManagementOnly);
        if let Some(object) = payload.as_object_mut() {
            object.insert("enabled".to_string(), serde_json::Value::Bool(false));
            object.insert(
                "authority".to_string(),
                serde_json::Value::String("execution_bound_transitions".to_string()),
            );
            object.insert(
                "execution_bound_count".to_string(),
                serde_json::Value::from(execution_bound_count),
            );
            object.insert("adopted_count".to_string(), serde_json::Value::from(0));
            object.insert("unadopted_count".to_string(), serde_json::Value::from(0));
            object.insert(
                "artifact_refs".to_string(),
                serde_json::json!({"adoption_path": adoption_path}),
            );
        }
        return payload;
    }
    serde_json::json!({
        "mode": "dispatch_enabled",
        "enabled": true,
        "status": "ready",
        "authority": "execution_bound_transitions",
        "execution_bound_count": execution_bound_count,
        "adopted_count": adopted_count,
        "unadopted_count": execution_bound_count.saturating_sub(adopted_count),
        "blocker_codes": [],
        "next_actions": ["Run `vida taskflow dispatch adopt --dry-run` before dispatching existing runs."],
        "artifact_refs": {"adoption_path": adoption_path},
    })
}

#[cfg(test)]
mod tests {
    use super::{
        task_execution_binding, taskflow_dispatch_enabled, TaskExecutionBinding,
        TaskLifecycleMutationSource, TaskLifecycleService, TaskRuntimeMode,
    };

    #[test]
    fn dispatch_defaults_to_management_only_and_requires_boolean_true() {
        assert!(!taskflow_dispatch_enabled(
            &serde_yaml::from_str("{}").unwrap()
        ));
        assert!(!taskflow_dispatch_enabled(
            &serde_yaml::from_str("dev_team:\n  enabled: true\n").unwrap()
        ));
        assert!(!taskflow_dispatch_enabled(
            &serde_yaml::from_str("taskflow:\n  dispatch:\n    enabled: 'true'\n").unwrap()
        ));
        assert!(taskflow_dispatch_enabled(
            &serde_yaml::from_str("taskflow:\n  dispatch:\n    enabled: true\n").unwrap()
        ));
    }

    #[test]
    fn execution_binding_uses_explicit_execution_semantics_or_active_run() {
        let mut task = crate::state_store::TaskRecord {
            id: "task".to_string(),
            display_id: None,
            title: "task".to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 0,
            issue_type: "task".to_string(),
            created_at: String::new(),
            created_by: String::new(),
            updated_at: String::new(),
            closed_at: None,
            close_reason: None,
            source_repo: String::new(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: Default::default(),
            planner_metadata: Default::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        };
        assert_eq!(
            task_execution_binding(&task, false),
            TaskExecutionBinding::ManagementOnly
        );
        task.execution_semantics.execution_mode = Some("exclusive".to_string());
        assert_eq!(
            task_execution_binding(&task, false),
            TaskExecutionBinding::ExecutionBound
        );
        task.execution_semantics.execution_mode = None;
        assert_eq!(
            task_execution_binding(&task, true),
            TaskExecutionBinding::ExecutionBound
        );
        task.planner_metadata.proof_targets = vec!["proof".to_string()];
        assert_eq!(
            task_execution_binding(&task, false),
            TaskExecutionBinding::ManagementOnly
        );
    }

    #[test]
    fn lifecycle_service_separates_management_and_receipt_authority() {
        assert_eq!(
            TaskLifecycleService::authorize(
                TaskRuntimeMode::ManagementOnly,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::Management,
            ),
            Ok(())
        );
        assert_eq!(
            TaskLifecycleService::authorize_close(
                TaskRuntimeMode::ManagementOnly,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::Management,
                false,
            ),
            Ok(())
        );
        assert_eq!(
            TaskLifecycleService::authorize_close(
                TaskRuntimeMode::DispatchEnabled,
                TaskExecutionBinding::ManagementOnly,
                &TaskLifecycleMutationSource::Management,
                false,
            ),
            Ok(())
        );
        assert_eq!(
            TaskLifecycleService::authorize_close(
                TaskRuntimeMode::DispatchEnabled,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::DispatchReceipt {
                    run_id: "run".to_string(),
                    receipt_id: "receipt".to_string(),
                },
                false,
            ),
            Err("dispatch_receipt_proof_required")
        );
        assert_eq!(
            TaskLifecycleService::authorize(
                TaskRuntimeMode::DispatchEnabled,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::Management,
            ),
            Err("dispatch_runtime_owns_execution_bound_lifecycle")
        );
        assert_eq!(
            TaskLifecycleService::authorize(
                TaskRuntimeMode::ManagementOnly,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::DispatchReceipt {
                    run_id: "run".to_string(),
                    receipt_id: "receipt".to_string(),
                },
            ),
            Err("dispatch_runtime_disabled")
        );
        assert_eq!(
            TaskLifecycleService::authorize(
                TaskRuntimeMode::DispatchEnabled,
                TaskExecutionBinding::ExecutionBound,
                &TaskLifecycleMutationSource::DispatchReceipt {
                    run_id: "run".to_string(),
                    receipt_id: "receipt".to_string(),
                },
            ),
            Ok(())
        );
    }
}
