use std::{env, path::PathBuf};

use serde_json::json;
use vida_contracts::{
    mvp_operation_registry, operations, VidaCommandEnvelope, VidaCommandResponse, VidaProblem,
    VidaProblemSeverity, VidaProjectRef,
};
use vida_runtime_local::engine::local_runtime_capabilities;
use vida_runtime_local::jobs::{
    job_status_payload, plan_outbox_job_from_redb, unavailable_job_status, RetryPolicy,
};

use crate::{
    command_pipeline::VidaCommandPipeline,
    vida_client::{pass_response, problem_response, unsupported_operation_response, VidaClient},
};

#[derive(Debug, Clone)]
pub(crate) struct InProcessVidaClient {
    pipeline: VidaCommandPipeline<LocalRuntimeVidaClient>,
}

impl InProcessVidaClient {
    pub(crate) fn new_ready() -> Self {
        Self {
            pipeline: VidaCommandPipeline::new(LocalRuntimeVidaClient::new_ready()),
        }
    }
}

impl VidaClient for InProcessVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        self.pipeline.execute(envelope)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LocalRuntimeVidaClient {
    project_root: PathBuf,
    job_journal_path: Option<PathBuf>,
}

impl LocalRuntimeVidaClient {
    pub(crate) fn new_ready() -> Self {
        Self {
            project_root: local_project_root(),
            job_journal_path: local_job_journal_path(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_job_journal_path(project_root: PathBuf, job_journal_path: PathBuf) -> Self {
        Self {
            project_root,
            job_journal_path: Some(job_journal_path),
        }
    }

    fn project_id(&self) -> String {
        self.project_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("vida-stack")
            .to_string()
    }

    fn project_entry(&self) -> serde_json::Value {
        let project_id = self.project_id();
        json!({
            "registry_entry_id": project_id,
            "project_id": project_id,
            "display_name": project_id,
            "worktree_environment_id": self.project_root.display().to_string(),
            "root_path": self.project_root.display().to_string(),
            "activation_status": "ready_enough_for_normal_work",
            "service_binding_status": "local_inprocess",
            "health": {
                "status": "ready",
                "source": "local_runtime_projection"
            }
        })
    }

    fn resolve_local_project(&self, envelope: &VidaCommandEnvelope) -> Result<(), VidaProblem> {
        let Some(project_ref) = envelope.project_ref.as_ref() else {
            return Err(project_resolution_ambiguous_problem());
        };
        let local_project_id = self.project_id();
        let root_path = self.project_root.display().to_string();
        let matches_local_project = match project_ref {
            VidaProjectRef::ProjectId { project_id } => project_id.0 == local_project_id,
            VidaProjectRef::RegistryEntry { registry_entry_id } => {
                registry_entry_id == &local_project_id
            }
            VidaProjectRef::RootPath {
                root_path: requested,
            } => requested == &root_path,
        };
        if matches_local_project {
            Ok(())
        } else {
            Err(project_not_found_problem(project_ref))
        }
    }

    fn requested_project(&self, envelope: &VidaCommandEnvelope) -> Option<String> {
        envelope
            .project_ref
            .as_ref()
            .map(|project_ref| match project_ref {
                VidaProjectRef::ProjectId { project_id } => project_id.0.clone(),
                VidaProjectRef::RegistryEntry { registry_entry_id } => registry_entry_id.clone(),
                VidaProjectRef::RootPath { root_path } => root_path.clone(),
            })
    }

    fn service_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": "ready",
                "session": {
                    "session_id": envelope.session_id,
                    "status": "active"
                },
                "event_cursor": {
                    "current": "local-runtime-current"
                },
                "project_root": self.project_root.display().to_string()
            }),
        )
    }

    fn service_capabilities(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let engine_capabilities = serde_json::to_value(local_runtime_capabilities())
            .expect("serialize engine capabilities");
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": "ready",
                "mutation_apply_supported": false,
                "engine_capabilities": engine_capabilities,
                "capabilities": [
                    "read_status",
                    "read_events",
                    "project_registry_read",
                    "wizard_read",
                    "wizard_plan",
                    "job_read",
                    "receipt_read"
                ],
                "projection_source": "local_runtime_projection"
            }),
        )
    }

    fn endpoint_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let endpoints: Vec<_> = mvp_operation_registry()
            .into_iter()
            .map(|spec| {
                json!({
                    "operation": spec.operation.0,
                    "scope": spec.scope,
                    "posture": spec.posture,
                    "requires_project_ref": spec.requires_project_ref,
                    "requires_apply_token": spec.requires_apply_token,
                    "required_capabilities": spec.required_capabilities
                })
            })
            .collect();
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": "ready",
                "endpoints": endpoints
            }),
        )
    }

    fn lifecycle_plan(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "mode": envelope
                    .payload
                    .get("mode")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("dry_run"),
                "native_service_apply_supported": false,
                "platform_plans": [{
                    "platform": env::consts::OS,
                    "adapter": "local_inprocess",
                    "install_target": "current_user_session",
                    "start_mode": "foreground",
                    "dry_run": true
                }],
                "apply_gate": {
                    "required": true,
                    "default": "blocked"
                }
            }),
        )
    }

    fn lifecycle_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "lifecycle": {
                    "state": "ready",
                    "running_mode": "local_inprocess",
                    "native_service_installed": false
                },
                "binary": {
                    "name": "vida",
                    "profile": "local",
                    "fingerprint": "local-runtime",
                    "fingerprint_algorithm": "local"
                },
                "project_root": self.project_root.display().to_string()
            }),
        )
    }

    fn events_since(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "current_cursor": "local-runtime-current",
                "events": [{
                    "event_id": "local-runtime-ready",
                    "request_id": envelope.request_id,
                    "session_id": envelope.session_id,
                    "project_id": self.project_id(),
                    "job_id": null,
                    "kind": "service.ready",
                    "payload": {
                        "status": "ready",
                        "project_root": self.project_root.display().to_string()
                    },
                    "cursor": "local-runtime-current"
                }]
            }),
        )
    }

    fn project_registry_list(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "projects": [self.project_entry()]
            }),
        )
    }

    fn project_resolve(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "project": self.project_entry(),
                "requested_project": self.requested_project(envelope)
            }),
        )
    }

    fn project_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        let project = self.project_entry();
        pass_response(
            envelope,
            json!({
                "project_id": project["project_id"],
                "registry_entry_id": project["registry_entry_id"],
                "worktree_environment_id": project["worktree_environment_id"],
                "status": "ready",
                "activation_status": "ready_enough_for_normal_work",
                "service_binding_status": "local_inprocess",
                "actor": {
                    "actor_id": format!("project-actor-{}", self.project_id()),
                    "mutation_queue_mode": "serialized",
                    "read_only_concurrency": true,
                    "mutation_intent_serialization": {
                        "enabled": true,
                        "queue_owner": self.project_id(),
                        "apply_execution_supported": false
                    }
                }
            }),
        )
    }

    fn wizard_schema(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "schema_id": "vida.project_init.local_runtime.v1",
                "wizard_kind": envelope
                    .payload
                    .get("wizard_kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("project_init"),
                "current_step": "inspect",
                "apply_supported": false,
                "option_graph": [{
                    "option_id": "project_root",
                    "label": "Project root",
                    "value_type": "path",
                    "required": true,
                    "value": self.project_root.display().to_string()
                }],
                "transitions": [
                    {"from": "inspect", "to": "draft", "operation": operations::WIZARD_SESSION_START},
                    {"from": "draft", "to": "validate", "operation": operations::WIZARD_SESSION_VALIDATE},
                    {"from": "validate", "to": "diff", "operation": operations::WIZARD_SESSION_DIFF}
                ],
                "disabled_apply_reason": "apply-token and durable local runtime apply handlers are not enabled"
            }),
        )
    }

    fn wizard_session(&self, envelope: &VidaCommandEnvelope, step: &str) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        let inputs =
            envelope.payload.get("inputs").cloned().unwrap_or_else(
                || json!({ "project_root": self.project_root.display().to_string() }),
            );
        pass_response(
            envelope,
            json!({
                "wizard_session": {
                    "session_id": envelope.session_id,
                    "wizard_kind": envelope
                        .payload
                        .get("wizard_kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("project_init"),
                    "step": step,
                    "revision": 1,
                    "inputs": inputs
                },
                "apply_supported": false
            }),
        )
    }

    fn materialization_manifest(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "manifest_id": "local-runtime-materialization",
                "config_schema_version": "vida-config-v1",
                "config_generator_version": "local-runtime",
                "config_file_hash": "local-runtime-config",
                "config_semantic_hash": "local-runtime-semantic",
                "artifacts": self.materialization_artifacts(),
                "receipt_refs": self.materialization_receipts()
            }),
        )
    }

    fn materialization_drift(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "manifest_id": "local-runtime-materialization",
                "classifications": [
                    {
                        "artifact_id": "vida-config",
                        "drift_status": "clean",
                        "update_mode": "report_only",
                        "reason": "Local runtime projection has no pending generated update."
                    }
                ],
                "summary": {
                    "clean": 1,
                    "safe_update": 0,
                    "manual_conflict": 0,
                    "report_only": 1
                }
            }),
        )
    }

    fn materialization_update_plan(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "plan_id": "local-runtime-update-plan",
                "mode": envelope
                    .payload
                    .get("mode")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("report_only"),
                "apply_supported": false,
                "planned_actions": [{
                    "artifact_id": "vida-config",
                    "mode": "report_only",
                    "receipt_ref": "local-runtime-receipt",
                    "safe_to_apply": false
                }],
                "receipt_evidence": self.materialization_receipts(),
                "manual_conflict_count": 0
            }),
        )
    }

    fn orchestration_control_plane_summary(
        &self,
        envelope: &VidaCommandEnvelope,
    ) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "project": {
                    "project_id": self.project_id(),
                    "registry_entry_id": self.project_id(),
                    "root_path": self.project_root.display().to_string()
                },
                "tracker_control_plane": {
                    "active_unit_source": "taskflow_tasks",
                    "state_machine_source": "task_status",
                    "agent_assignment_unit": "bounded_task",
                    "agent_may_create_follow_up_tasks": true
                },
                "workspace_model": {
                    "isolated_workspace_per_task": true,
                    "workspace_owner": "task_worktree_assignment",
                    "preserve_workspace_across_runs": true
                },
                "scheduling": {
                    "bounded_concurrency": true,
                    "parallelism_source": "taskflow_execution_semantics",
                    "blocked_tasks_are_not_dispatched": true
                },
                "observability": {
                    "structured_runtime_logs": true,
                    "tui_projection": true,
                    "service_projection": true
                },
                "safety": {
                    "apply_supported": false,
                    "admin_supported": false,
                    "approval_policy_source": "vida_runtime_policy"
                }
            }),
        )
    }

    fn jobs_get(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let job_id = envelope
            .payload
            .get("job_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("latest");
        let job = match self.job_journal_path.as_ref() {
            Some(path) if path.exists() => {
                match plan_outbox_job_from_redb(path, job_id, &RetryPolicy::default()) {
                    Ok(Some(plan)) => job_status_payload(&plan),
                    Ok(None) => unavailable_job_status(
                        job_id,
                        format!(
                            "job `{job_id}` was not found in redb outbox `{}`",
                            path.display()
                        ),
                    ),
                    Err(error) => unavailable_job_status(job_id, error),
                }
            }
            Some(path) => unavailable_job_status(
                job_id,
                format!("redb outbox journal `{}` does not exist", path.display()),
            ),
            None => unavailable_job_status(
                job_id,
                "no redb outbox journal path is configured for the local runtime",
            ),
        };
        let status = job
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unavailable")
            .to_string();
        pass_response(
            envelope,
            json!({
                "job_id": job["job_id"].clone(),
                "status": status,
                "operation": envelope.operation,
                "receipt_available": true,
                "source": "local_runtime_projection",
                "authority": "redb_outbox",
                "runner": "effectum",
                "job": job
            }),
        )
    }

    fn receipts_get(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Err(problem) = self.resolve_local_project(envelope) {
            return problem_response(envelope, problem);
        }
        pass_response(
            envelope,
            json!({
                "receipt_scope": "project",
                "receipt_id": envelope
                    .payload
                    .get("receipt_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("latest"),
                "receipts": [{
                    "receipt_id": "local-runtime-ready",
                    "kind": "local_runtime_projection",
                    "status": "recorded",
                    "evidence_kind": "local_runtime_projection",
                    "project_root": self.project_root.display().to_string()
                }]
            }),
        )
    }

    fn materialization_artifacts(&self) -> serde_json::Value {
        json!([{
            "artifact_id": "vida-config",
            "path": "vida.config.yaml",
            "artifact_kind": "config",
            "owner": "vida_generated",
            "template_version": "local-runtime",
            "schema_revision": "vida-config-v1",
            "source_config_revision": "local-runtime-semantic",
            "generator_revision": "local-runtime",
            "last_generated_hash": "local-runtime-config",
            "current_hash": "local-runtime-config",
            "drift_status": "clean",
            "update_mode": "report_only",
            "receipt_refs": ["local-runtime-receipt"]
        }])
    }

    fn materialization_receipts(&self) -> serde_json::Value {
        json!([{
            "receipt_id": "local-runtime-receipt",
            "artifact_id": "vida-config",
            "mode": "report_only",
            "status": "recorded",
            "evidence_kind": "local_runtime_projection"
        }])
    }
}

fn local_project_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.ancestors()
        .find(|path| path.join("AGENTS.sidecar.md").is_file() || path.join(".vida").is_dir())
        .map(PathBuf::from)
        .unwrap_or(cwd)
}

fn local_job_journal_path() -> Option<PathBuf> {
    env::var_os("VIDA_JOB_JOURNAL_PATH")
        .map(PathBuf::from)
        .or_else(|| {
            let path = local_project_root()
                .join(".vida")
                .join("data")
                .join("state")
                .join("operational-journal.redb");
            path.exists().then_some(path)
        })
}

fn project_resolution_ambiguous_problem() -> VidaProblem {
    VidaProblem {
        problem_type: "https://vida.dev/problems/project-resolution-ambiguous".to_string(),
        title: "Project resolution is ambiguous".to_string(),
        detail: "Provide a project_ref before reading project-scoped state.".to_string(),
        code: "project_resolution_ambiguous".to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: false,
        blockers: vec![vida_contracts::VidaBlocker {
            code: "project_ref_required".to_string(),
            scope: Some("project_ref".to_string()),
            next_actions: vec![
                "Retry with a concrete project_id, registry_entry_id, or root_path.".to_string(),
            ],
        }],
        remediation: vec![
            "Call vida.project.registry.list before project-scoped reads.".to_string(),
        ],
        instance: None,
        related_receipt: None,
    }
}

fn project_not_found_problem(project_ref: &VidaProjectRef) -> VidaProblem {
    VidaProblem {
        problem_type: "https://vida.dev/problems/project-not-found".to_string(),
        title: "Project was not found".to_string(),
        detail: format!("No local project registry entry matched `{project_ref:?}`."),
        code: "project_not_found".to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: false,
        blockers: vec![vida_contracts::VidaBlocker {
            code: "project_not_registered".to_string(),
            scope: Some("project_ref".to_string()),
            next_actions: vec![
                "Register or discover the project before reading project status.".to_string(),
            ],
        }],
        remediation: vec!["Call vida.project.registry.discover and retry.".to_string()],
        instance: None,
        related_receipt: None,
    }
}

impl Default for LocalRuntimeVidaClient {
    fn default() -> Self {
        Self::new_ready()
    }
}

impl VidaClient for LocalRuntimeVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        match envelope.operation.0.as_str() {
            operations::SERVICE_HELLO | operations::SERVICE_STATUS => {
                self.service_status(&envelope)
            }
            operations::SERVICE_CAPABILITIES => self.service_capabilities(&envelope),
            operations::SERVICE_ENDPOINT_STATUS => self.endpoint_status(&envelope),
            operations::SERVICE_LIFECYCLE_PLAN => self.lifecycle_plan(&envelope),
            operations::SERVICE_LIFECYCLE_STATUS => self.lifecycle_status(&envelope),
            operations::EVENTS_SINCE | operations::SESSION_RESOLVE => self.events_since(&envelope),
            operations::PROJECT_REGISTRY_LIST
            | operations::PROJECT_REGISTRY_GET
            | operations::PROJECT_REGISTRY_DISCOVER => self.project_registry_list(&envelope),
            operations::PROJECT_RESOLVE => self.project_resolve(&envelope),
            operations::PROJECT_STATUS => self.project_status(&envelope),
            operations::WIZARD_SCHEMA_GET => self.wizard_schema(&envelope),
            operations::WIZARD_SESSION_START | operations::WIZARD_SESSION_GET => {
                self.wizard_session(&envelope, "draft")
            }
            operations::WIZARD_SESSION_UPDATE_INPUT
            | operations::WIZARD_SESSION_VALIDATE
            | operations::WIZARD_SESSION_DIFF => self.wizard_session(&envelope, "validate"),
            operations::JOBS_GET => self.jobs_get(&envelope),
            operations::RECEIPTS_GET | operations::MATERIALIZATION_RECEIPTS_LIST => {
                self.receipts_get(&envelope)
            }
            operations::MATERIALIZATION_MANIFEST_GET => self.materialization_manifest(&envelope),
            operations::MATERIALIZATION_DRIFT_CLASSIFY => self.materialization_drift(&envelope),
            operations::MATERIALIZATION_UPDATE_PLAN => self.materialization_update_plan(&envelope),
            operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET => {
                self.orchestration_control_plane_summary(&envelope)
            }
            _ => unsupported_operation_response(&envelope),
        }
    }
}
