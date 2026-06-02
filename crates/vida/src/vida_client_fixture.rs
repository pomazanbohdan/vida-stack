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
                    "project_registry_read",
                    "wizard_read",
                    "wizard_plan",
                    "materialization_read",
                    "materialization_plan",
                    "orchestration_control_plane_read"
                ]
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

    fn wizard_schema(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "schema_id": "vida.project_init.fixture.v1",
                "wizard_kind": envelope
                    .payload
                    .get("wizard_kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("project_init"),
                "current_step": "inspect",
                "apply_supported": false,
                "option_graph": fixture_wizard_option_graph(),
                "transitions": [
                    {
                        "from": "inspect",
                        "to": "draft",
                        "operation": operations::WIZARD_SESSION_START
                    },
                    {
                        "from": "draft",
                        "to": "validate",
                        "operation": operations::WIZARD_SESSION_VALIDATE
                    },
                    {
                        "from": "validate",
                        "to": "diff",
                        "operation": operations::WIZARD_SESSION_DIFF
                    }
                ],
                "disabled_apply_reason": "apply-token and claim-proof execution are not implemented"
            }),
        )
    }

    fn wizard_start(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "wizard_session": fixture_wizard_state(envelope, 1, "draft", json!({})),
                "idempotency_key": envelope.idempotency_key,
                "state_machine": {
                    "from": "inspect",
                    "to": "draft",
                    "transition": operations::WIZARD_SESSION_START
                }
            }),
        )
    }

    fn wizard_get(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "wizard_session": fixture_wizard_state(envelope, 1, "draft", json!({}))
            }),
        )
    }

    fn wizard_update_input(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Some(problem) = wizard_stale_revision_problem(envelope, 1) {
            return problem_response(envelope, problem);
        }
        let inputs = envelope
            .payload
            .get("inputs")
            .cloned()
            .unwrap_or_else(|| json!({}));
        pass_response(
            envelope,
            json!({
                "wizard_session": fixture_wizard_state(envelope, 2, "draft", inputs),
                "idempotency_key": envelope.idempotency_key,
                "state_machine": {
                    "from": "draft",
                    "to": "draft",
                    "transition": operations::WIZARD_SESSION_UPDATE_INPUT
                }
            }),
        )
    }

    fn wizard_validate(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let inputs = envelope
            .payload
            .get("inputs")
            .cloned()
            .unwrap_or_else(|| json!({}));
        pass_response(
            envelope,
            json!({
                "wizard_session": fixture_wizard_state(envelope, 2, "validate", inputs.clone()),
                "validation": fixture_wizard_validation(&inputs),
                "readiness": fixture_wizard_readiness(),
                "apply_supported": false
            }),
        )
    }

    fn wizard_diff(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        if let Some(problem) = wizard_stale_revision_problem(envelope, 2) {
            return problem_response(envelope, problem);
        }
        let inputs = envelope
            .payload
            .get("inputs")
            .cloned()
            .unwrap_or_else(|| json!({}));
        pass_response(
            envelope,
            json!({
                "wizard_session": fixture_wizard_state(envelope, 2, "diff", inputs.clone()),
                "plan_ref": {
                    "plan_id": "wizard-plan-fixture-1"
                },
                "diff_summary": fixture_wizard_diff(&inputs),
                "apply_supported": false,
                "disabled_apply_reason": "apply-token and claim-proof execution are not implemented"
            }),
        )
    }

    fn materialization_manifest(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "manifest_id": "materialization-manifest-fixture-1",
                "config_schema_version": "vida-config-v1",
                "config_generator_version": "fixture-generator-v1",
                "config_file_hash": "fixture-file-hash",
                "config_semantic_hash": "fixture-semantic-hash",
                "artifacts": fixture_materialization_artifacts(),
                "receipt_refs": fixture_materialization_receipts()
            }),
        )
    }

    fn materialization_drift(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "manifest_id": "materialization-manifest-fixture-1",
                "classifications": fixture_materialization_drift_classifications(),
                "summary": {
                    "clean": 1,
                    "safe_update": 1,
                    "manual_conflict": 1,
                    "report_only": 1
                }
            }),
        )
    }

    fn materialization_update_plan(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "plan_id": "materialization-update-plan-fixture-1",
                "mode": envelope
                    .payload
                    .get("mode")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("report_only"),
                "apply_supported": false,
                "planned_actions": fixture_materialization_update_actions(),
                "receipt_evidence": fixture_materialization_receipts(),
                "manual_conflict_count": 1
            }),
        )
    }

    fn materialization_receipts(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "receipt_scope": "materialization",
                "receipts": fixture_materialization_receipts()
            }),
        )
    }

    fn orchestration_control_plane_summary(
        &self,
        envelope: &VidaCommandEnvelope,
    ) -> VidaCommandResponse {
        match self.resolve_project(envelope) {
            Ok(project) => pass_response(
                envelope,
                json!({
                    "service": "vida",
                    "project": {
                        "project_id": project.project_id,
                        "registry_entry_id": project.registry_entry_id,
                        "root_path": project.root_path
                    },
                    "source_pattern": {
                        "kind": "external_reference_inspiration",
                        "authority": "vida_runtime_law",
                        "source_refs": [
                            "https://openai.com/ru-RU/index/open-source-codex-orchestration-symphony/",
                            "https://github.com/openai/symphony/blob/main/SPEC.md"
                        ]
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
                    "retry_reconciliation": {
                        "restart_recovery": true,
                        "transient_failure_strategy": "exponential_backoff",
                        "state_change_stops_ineligible_runs": true
                    },
                    "workflow_contract": {
                        "repo_owned_policy_file": "WORKFLOW.md",
                        "vida_equivalent_sources": [
                            "flows.yaml",
                            "vida.config.yaml",
                            "AGENTS.sidecar.md"
                        ],
                        "prompt_template_source": "configured_development_flow"
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
            operations::WIZARD_SCHEMA_GET => self.wizard_schema(&envelope),
            operations::WIZARD_SESSION_START => self.wizard_start(&envelope),
            operations::WIZARD_SESSION_GET => self.wizard_get(&envelope),
            operations::WIZARD_SESSION_UPDATE_INPUT => self.wizard_update_input(&envelope),
            operations::WIZARD_SESSION_VALIDATE => self.wizard_validate(&envelope),
            operations::WIZARD_SESSION_DIFF => self.wizard_diff(&envelope),
            operations::MATERIALIZATION_MANIFEST_GET => self.materialization_manifest(&envelope),
            operations::MATERIALIZATION_DRIFT_CLASSIFY => self.materialization_drift(&envelope),
            operations::MATERIALIZATION_UPDATE_PLAN => self.materialization_update_plan(&envelope),
            operations::MATERIALIZATION_RECEIPTS_LIST => self.materialization_receipts(&envelope),
            operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET => {
                self.orchestration_control_plane_summary(&envelope)
            }
            _ => unsupported_operation_response(&envelope),
        }
    }
}

fn fixture_wizard_option_graph() -> serde_json::Value {
    json!([
        {
            "option_id": "project_root",
            "label": "Project root",
            "value_type": "path",
            "required": true,
            "depends_on": [],
            "conflicts_with": []
        },
        {
            "option_id": "enable_tui",
            "label": "Enable TUI",
            "value_type": "boolean",
            "required": false,
            "depends_on": ["project_root"],
            "conflicts_with": []
        },
        {
            "option_id": "service_mode",
            "label": "Service mode",
            "value_type": "enum_one",
            "required": true,
            "depends_on": ["project_root"],
            "conflicts_with": []
        }
    ])
}

fn fixture_wizard_state(
    envelope: &VidaCommandEnvelope,
    revision: u64,
    current_step: &str,
    inputs: serde_json::Value,
) -> serde_json::Value {
    json!({
        "wizard_session_id": "wizard-session-fixture-1",
        "wizard_kind": envelope
            .payload
            .get("wizard_kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("project_init"),
        "session_id": envelope.session_id,
        "project_ref": envelope.project_ref,
        "current_step": current_step,
        "revision": revision,
        "semantic_revision": format!("wizard-semantic-revision-{revision}"),
        "inputs": fixture_wizard_option_states(inputs),
        "validation_findings": [],
        "readiness_findings": fixture_wizard_readiness(),
        "apply_supported": false
    })
}

fn fixture_wizard_option_states(inputs: serde_json::Value) -> serde_json::Value {
    let project_root = inputs
        .get("project_root")
        .cloned()
        .unwrap_or_else(|| json!(""));
    let enable_tui = inputs
        .get("enable_tui")
        .cloned()
        .unwrap_or_else(|| json!(false));
    let service_mode = inputs
        .get("service_mode")
        .cloned()
        .unwrap_or_else(|| json!("read_only"));
    json!([
        {
            "option_id": "project_root",
            "value": project_root,
            "effective_value": project_root,
            "source": "operator_input",
            "visible": true,
            "enabled": true,
            "required": true,
            "dirty": project_root != "",
            "valid": project_root != "",
            "dependency_inputs": [],
            "affected_materialization_targets": ["vida.config.yaml"]
        },
        {
            "option_id": "enable_tui",
            "value": enable_tui,
            "effective_value": enable_tui,
            "source": "operator_input",
            "visible": true,
            "enabled": project_root != "",
            "required": false,
            "dirty": enable_tui != false,
            "valid": true,
            "dependency_inputs": ["project_root"],
            "affected_materialization_targets": ["flows.yaml"]
        },
        {
            "option_id": "service_mode",
            "value": service_mode,
            "effective_value": service_mode,
            "source": "operator_input",
            "visible": true,
            "enabled": project_root != "",
            "required": true,
            "dirty": service_mode != "read_only",
            "valid": service_mode == "read_only" || service_mode == "read_write_plan_only",
            "dependency_inputs": ["project_root"],
            "affected_materialization_targets": ["vida.config.yaml"]
        }
    ])
}

fn fixture_wizard_validation(inputs: &serde_json::Value) -> serde_json::Value {
    let project_root_missing = inputs
        .get("project_root")
        .and_then(serde_json::Value::as_str)
        .is_none_or(str::is_empty);
    if project_root_missing {
        json!({
            "status": "blocked",
            "findings": [
                {
                    "option_id": "project_root",
                    "code": "required_option_missing",
                    "message": "Project root is required before diff planning.",
                    "severity": "error"
                }
            ]
        })
    } else {
        json!({
            "status": "pass",
            "findings": []
        })
    }
}

fn fixture_wizard_readiness() -> serde_json::Value {
    json!([
        {
            "code": "apply_disabled_until_claim_proof",
            "message": "Apply remains disabled until apply-token and claim-proof execution are implemented.",
            "blocker": true
        }
    ])
}

fn fixture_wizard_diff(inputs: &serde_json::Value) -> serde_json::Value {
    let enable_tui = inputs
        .get("enable_tui")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let service_mode = inputs
        .get("service_mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("read_only");
    json!({
        "diff_hash": format!("fixture-diff-enable-tui-{enable_tui}-mode-{service_mode}"),
        "config_changes": ["project_root"],
        "registry_changes": ["project_registry_binding"],
        "materialization_changes": if enable_tui {
            json!(["tui_wizard_surface"])
        } else {
            json!([])
        },
        "service_changes": [service_mode],
        "runtime_impacts": ["plan_only_no_apply"]
    })
}

fn wizard_stale_revision_problem(
    envelope: &VidaCommandEnvelope,
    current_revision: u64,
) -> Option<VidaProblem> {
    let expected_revision = envelope
        .payload
        .get("expected_revision")
        .and_then(serde_json::Value::as_u64)?;
    if expected_revision == current_revision {
        return None;
    }
    Some(VidaProblem {
        problem_type: "https://vida.dev/problems/wizard-stale-revision".to_string(),
        title: "Wizard draft revision is stale".to_string(),
        detail: format!(
            "Expected revision `{expected_revision}` does not match current revision `{current_revision}`."
        ),
        code: "wizard_stale_revision".to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: true,
        blockers: vec![vida_contracts::VidaBlocker {
            code: "wizard_revision_mismatch".to_string(),
            scope: Some("expected_revision".to_string()),
            next_actions: vec![
                "Reload the wizard session and retry with the latest revision.".to_string()
            ],
        }],
        remediation: vec!["Call vida.wizard.session.get before updating or diffing.".to_string()],
        instance: None,
        related_receipt: None,
    })
}

fn fixture_materialization_artifacts() -> serde_json::Value {
    json!([
        {
            "artifact_id": "vida-config",
            "path": "vida.config.yaml",
            "artifact_kind": "config",
            "owner": "vida_generated",
            "template_version": "fixture-template-v1",
            "schema_revision": "vida-config-v1",
            "source_config_revision": "fixture-semantic-hash",
            "generator_revision": "fixture-generator-v1",
            "last_generated_hash": "hash-config-old",
            "current_hash": "hash-config-new",
            "drift_status": "generated_changed_by_version",
            "update_mode": "safe_update",
            "receipt_refs": ["receipt-config-safe-update"]
        },
        {
            "artifact_id": "flows",
            "path": "flows.yaml",
            "artifact_kind": "flow_config",
            "owner": "vida_generated",
            "template_version": "fixture-template-v1",
            "schema_revision": "vida-flow-v1",
            "source_config_revision": "fixture-semantic-hash",
            "generator_revision": "fixture-generator-v1",
            "last_generated_hash": "hash-flows",
            "current_hash": "hash-flows",
            "drift_status": "clean",
            "update_mode": "report_only",
            "receipt_refs": ["receipt-flows-report-only"]
        },
        {
            "artifact_id": "agents-sidecar",
            "path": "AGENTS.sidecar.md",
            "artifact_kind": "agent_instructions",
            "owner": "user_owned",
            "template_version": "fixture-template-v1",
            "schema_revision": "agent-sidecar-v1",
            "source_config_revision": "fixture-semantic-hash",
            "generator_revision": "fixture-generator-v1",
            "last_generated_hash": "hash-sidecar-old",
            "current_hash": "hash-sidecar-user",
            "drift_status": "user_modified",
            "update_mode": "manual_conflict",
            "receipt_refs": ["receipt-sidecar-manual-conflict"]
        }
    ])
}

fn fixture_materialization_drift_classifications() -> serde_json::Value {
    json!([
        {
            "artifact_id": "vida-config",
            "drift_status": "generated_changed_by_version",
            "update_mode": "safe_update",
            "reason": "VIDA-generated artifact changed only by generator/template revision."
        },
        {
            "artifact_id": "flows",
            "drift_status": "clean",
            "update_mode": "report_only",
            "reason": "Artifact hash matches the latest generated hash."
        },
        {
            "artifact_id": "agents-sidecar",
            "drift_status": "user_modified",
            "update_mode": "manual_conflict",
            "reason": "User-owned artifact changed outside generated ownership."
        }
    ])
}

fn fixture_materialization_update_actions() -> serde_json::Value {
    json!([
        {
            "artifact_id": "flows",
            "mode": "report_only",
            "receipt_ref": "receipt-flows-report-only",
            "safe_to_apply": false
        },
        {
            "artifact_id": "vida-config",
            "mode": "safe_update",
            "receipt_ref": "receipt-config-safe-update",
            "safe_to_apply": true
        },
        {
            "artifact_id": "agents-sidecar",
            "mode": "manual_conflict",
            "receipt_ref": "receipt-sidecar-manual-conflict",
            "safe_to_apply": false
        }
    ])
}

fn fixture_materialization_receipts() -> serde_json::Value {
    json!([
        {
            "receipt_id": "receipt-flows-report-only",
            "artifact_id": "flows",
            "mode": "report_only",
            "status": "recorded",
            "evidence_kind": "artifact_manifest_entry"
        },
        {
            "receipt_id": "receipt-config-safe-update",
            "artifact_id": "vida-config",
            "mode": "safe_update",
            "status": "recorded",
            "evidence_kind": "artifact_update_plan"
        },
        {
            "receipt_id": "receipt-sidecar-manual-conflict",
            "artifact_id": "agents-sidecar",
            "mode": "manual_conflict",
            "status": "recorded",
            "evidence_kind": "manual_conflict_record"
        }
    ])
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
