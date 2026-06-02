use crate::vida_client::{
    pass_response, problem_response, unsupported_operation_response, VidaClient,
};
use serde_json::json;
use vida_contracts::{
    mvp_operation_registry, operations, VidaCommandEnvelope, VidaCommandResponse, VidaEvent,
    VidaEventCursor, VidaProblem, VidaProblemSeverity, VidaProjectId, VidaProjectRef,
    VidaRequestId, VidaSessionId,
};

#[derive(Debug, Clone)]
pub(crate) struct FixtureVidaClient {
    service_status: String,
    session_status: String,
    current_cursor: VidaEventCursor,
    events: Vec<VidaEvent>,
    projects: Vec<vida_contracts::ServiceProjectRegistryEntry>,
}

impl FixtureVidaClient {
    pub(crate) fn new_ready() -> Self {
        let session_id = VidaSessionId("fixture-session".to_string());
        let request_id = VidaRequestId("fixture-request".to_string());
        let current_cursor = VidaEventCursor("fixture-cursor-1".to_string());
        Self {
            service_status: "ready".to_string(),
            session_status: "active".to_string(),
            current_cursor: current_cursor.clone(),
            events: vec![VidaEvent {
                event_id: "fixture-event-1".to_string(),
                request_id,
                session_id,
                project_id: None,
                job_id: None,
                kind: "service.ready".to_string(),
                payload: json!({ "status": "ready" }),
                cursor: current_cursor,
            }],
            projects: fixture_projects(),
        }
    }

    fn hello(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "protocol_version": envelope.protocol_version,
                "schema_version": envelope.schema_version
            }),
        )
    }

    fn status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "session": {
                    "session_id": envelope.session_id,
                    "status": self.session_status
                },
                "event_cursor": {
                    "current": self.current_cursor
                }
            }),
        )
    }

    fn capabilities(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "mutation_apply_supported": false,
                "capabilities": [
                    "read_status",
                    "read_events",
                    "project_registry_read"
                ]
            }),
        )
    }

    fn endpoint_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let endpoints: Vec<_> = mvp_operation_registry()
            .into_iter()
            .filter(|spec| matches!(spec.scope, vida_contracts::VidaOperationScope::Service))
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
                "status": self.service_status,
                "endpoints": endpoints
            }),
        )
    }

    fn events_since(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "current_cursor": self.current_cursor,
                "events": self.events
            }),
        )
    }

    fn session_resolve(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "session_id": envelope.session_id,
                "status": self.session_status,
                "service_status": self.service_status,
                "event_cursor": self.current_cursor
            }),
        )
    }

    fn registry_list(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "projects": self.projects
            }),
        )
    }

    fn registry_get(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let registry_entry_id = envelope
            .payload
            .get("registry_entry_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        match self
            .projects
            .iter()
            .find(|project| project.registry_entry_id == registry_entry_id)
        {
            Some(project) => pass_response(envelope, json!({ "project": project })),
            None => project_not_found_response(envelope, registry_entry_id),
        }
    }

    fn registry_discover(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "discovered_projects": self.projects,
                "discovery_mode": "fixture"
            }),
        )
    }

    fn project_resolve(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        match self.resolve_project(envelope) {
            Ok(project) => pass_response(envelope, json!({ "project": project })),
            Err(problem) => problem_response(envelope, problem),
        }
    }

    fn project_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        match self.resolve_project(envelope) {
            Ok(project) => pass_response(
                envelope,
                json!({
                    "project_id": project.project_id,
                    "registry_entry_id": project.registry_entry_id,
                    "worktree_environment_id": project.worktree_environment_id,
                    "status": project.health.status,
                    "activation_status": project.activation_status,
                    "service_binding_status": project.service_binding_status,
                    "actor": {
                        "actor_id": format!("project-actor-{}", project.project_id.0),
                        "mutation_queue_mode": "serialized",
                        "read_only_concurrency": true,
                        "mutation_intent_serialization": {
                            "enabled": true,
                            "queue_owner": project.project_id.0,
                            "apply_execution_supported": false
                        }
                    }
                }),
            ),
            Err(problem) => problem_response(envelope, problem),
        }
    }

    fn resolve_project(
        &self,
        envelope: &VidaCommandEnvelope,
    ) -> Result<&vida_contracts::ServiceProjectRegistryEntry, VidaProblem> {
        let Some(project_ref) = envelope.project_ref.as_ref() else {
            return Err(project_resolution_ambiguous_problem());
        };
        let matched = match project_ref {
            VidaProjectRef::ProjectId { project_id } => self
                .projects
                .iter()
                .find(|project| project.project_id == *project_id),
            VidaProjectRef::RegistryEntry { registry_entry_id } => self
                .projects
                .iter()
                .find(|project| project.registry_entry_id == *registry_entry_id),
            VidaProjectRef::RootPath { root_path } => self
                .projects
                .iter()
                .find(|project| project.root_path == *root_path),
        };
        matched.ok_or_else(|| project_not_found_problem(project_ref))
    }
}

impl Default for FixtureVidaClient {
    fn default() -> Self {
        Self::new_ready()
    }
}

impl VidaClient for FixtureVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        match envelope.operation.0.as_str() {
            operations::SERVICE_HELLO => self.hello(&envelope),
            operations::SERVICE_STATUS => self.status(&envelope),
            operations::SERVICE_CAPABILITIES => self.capabilities(&envelope),
            operations::SERVICE_ENDPOINT_STATUS => self.endpoint_status(&envelope),
            operations::EVENTS_SINCE => self.events_since(&envelope),
            operations::SESSION_RESOLVE => self.session_resolve(&envelope),
            operations::PROJECT_REGISTRY_LIST => self.registry_list(&envelope),
            operations::PROJECT_REGISTRY_GET => self.registry_get(&envelope),
            operations::PROJECT_REGISTRY_DISCOVER => self.registry_discover(&envelope),
            operations::PROJECT_RESOLVE => self.project_resolve(&envelope),
            operations::PROJECT_STATUS => self.project_status(&envelope),
            _ => unsupported_operation_response(&envelope),
        }
    }
}

fn fixture_projects() -> Vec<vida_contracts::ServiceProjectRegistryEntry> {
    use vida_contracts::{
        ProjectActivationStatus, ProjectHealthSummary, ProjectRegistryStatus, ServiceBindingStatus,
        ServiceProjectRegistryEntry,
    };
    vec![
        ServiceProjectRegistryEntry {
            registry_entry_id: "vida-stack-main".to_string(),
            project_id: VidaProjectId("vida-stack".to_string()),
            worktree_environment_id: "worktree-vida-stack-main".to_string(),
            root_path: "C:/project/vida-stack".to_string(),
            registry_status: ProjectRegistryStatus::Connected,
            activation_status: ProjectActivationStatus::Activated,
            service_binding_status: ServiceBindingStatus::BoundCurrentService,
            health: ProjectHealthSummary {
                status: "pass".to_string(),
                blocker_codes: Vec::new(),
            },
        },
        ServiceProjectRegistryEntry {
            registry_entry_id: "vida-mobile-main".to_string(),
            project_id: VidaProjectId("vida-mobile".to_string()),
            worktree_environment_id: "worktree-vida-mobile-main".to_string(),
            root_path: "C:/project/vida_mobile".to_string(),
            registry_status: ProjectRegistryStatus::Connected,
            activation_status: ProjectActivationStatus::Activated,
            service_binding_status: ServiceBindingStatus::NotBound,
            health: ProjectHealthSummary {
                status: "pass".to_string(),
                blocker_codes: Vec::new(),
            },
        },
    ]
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
        detail: format!("No project registry entry matched `{project_ref:?}`."),
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

fn project_not_found_response(
    envelope: &VidaCommandEnvelope,
    registry_entry_id: &str,
) -> VidaCommandResponse {
    let problem = VidaProblem {
        problem_type: "https://vida.dev/problems/project-not-found".to_string(),
        title: "Project was not found".to_string(),
        detail: format!("No project registry entry matched `{registry_entry_id}`."),
        code: "project_not_found".to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: false,
        blockers: vec![vida_contracts::VidaBlocker {
            code: "project_not_registered".to_string(),
            scope: Some(registry_entry_id.to_string()),
            next_actions: vec![
                "Use a registry_entry_id from vida.project.registry.list.".to_string()
            ],
        }],
        remediation: vec!["Call vida.project.registry.list and retry.".to_string()],
        instance: None,
        related_receipt: None,
    };
    problem_response(envelope, problem)
}
