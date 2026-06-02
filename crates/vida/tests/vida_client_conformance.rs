#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_fixture.rs"]
mod vida_client_fixture;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;

use serde_json::json;
use vida_client::VidaClient;
use vida_client_fixture::FixtureVidaClient;
use vida_client_inprocess::InProcessVidaClient;
use vida_contracts::{
    mvp_operation_registry, operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope,
    VidaCommandResponse, VidaIdempotencyKey, VidaOperation, VidaOperationPosture,
    VidaOperationScope, VidaProjectId, VidaProjectRef, VidaRequestId, VidaResponseStatus,
    VidaSessionId, VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("test-session".to_string()),
        request_id: VidaRequestId(format!("request-{operation}")),
        client_kind: VidaClientKind::Cli,
        project_ref: None,
        claim_kind: Some(VidaClaimKind::Observe),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("idem-{operation}"))),
        apply_token: None,
    }
}

fn envelope_with_payload(operation: &str, payload: serde_json::Value) -> VidaCommandEnvelope {
    let mut envelope = envelope(operation);
    envelope.payload = payload;
    envelope
}

fn envelope_with_project_ref(operation: &str, project_ref: VidaProjectRef) -> VidaCommandEnvelope {
    let mut envelope = envelope(operation);
    envelope.project_ref = Some(project_ref);
    envelope
}

fn assert_same_response(operation: &str) -> VidaCommandResponse {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let fixture_response = fixture.execute(envelope(operation));
    let in_process_response = in_process.execute(envelope(operation));
    assert_eq!(fixture_response, in_process_response);
    fixture_response
}

#[test]
fn vida_client_fixture_and_inprocess_match_service_read_operations() {
    for operation in [
        operations::SERVICE_HELLO,
        operations::SERVICE_STATUS,
        operations::SERVICE_CAPABILITIES,
        operations::SERVICE_ENDPOINT_STATUS,
        operations::EVENTS_SINCE,
        operations::SESSION_RESOLVE,
    ] {
        let response = assert_same_response(operation);
        assert_eq!(response.status, VidaResponseStatus::Pass);
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}

#[test]
fn service_status_reports_session_and_event_cursor() {
    let response = assert_same_response(operations::SERVICE_STATUS);
    let result = response.result.expect("status result");

    assert_eq!(result["service"], "vida");
    assert_eq!(result["status"], "ready");
    assert_eq!(result["session"]["status"], "active");
    assert_eq!(result["event_cursor"]["current"], "fixture-cursor-1");
}

#[test]
fn service_capabilities_and_endpoints_are_read_only() {
    let capabilities = assert_same_response(operations::SERVICE_CAPABILITIES)
        .result
        .expect("capabilities result");
    assert_eq!(capabilities["service"], "vida");
    assert_eq!(capabilities["mutation_apply_supported"], false);
    assert!(capabilities["capabilities"]
        .as_array()
        .expect("capabilities should be array")
        .iter()
        .any(|capability| capability == "read_status"));
    assert!(capabilities["capabilities"]
        .as_array()
        .expect("capabilities should be array")
        .iter()
        .any(|capability| capability == "project_registry_read"));

    let endpoints = assert_same_response(operations::SERVICE_ENDPOINT_STATUS)
        .result
        .expect("endpoint status result");
    let endpoint_rows = endpoints["endpoints"]
        .as_array()
        .expect("endpoints should be array");
    assert!(endpoint_rows.iter().any(|row| {
        row["operation"] == operations::SERVICE_HELLO && row["posture"] == "read_only"
    }));
    for registry_operation in [
        operations::PROJECT_REGISTRY_LIST,
        operations::PROJECT_REGISTRY_GET,
        operations::PROJECT_REGISTRY_DISCOVER,
    ] {
        let row = endpoint_rows
            .iter()
            .find(|row| row["operation"] == registry_operation)
            .expect("registry operation endpoint row");
        assert_eq!(row["scope"], "service");
        assert_eq!(row["posture"], "read_only");
        assert_eq!(row["requires_project_ref"], false);
        assert!(row["required_capabilities"]
            .as_array()
            .expect("required capabilities array")
            .iter()
            .any(|capability| capability == "project_registry_read"));
    }
    assert!(endpoint_rows
        .iter()
        .all(|row| row["posture"] != "apply" && row["posture"] != "admin"));
}

#[test]
fn events_since_reports_current_cursor() {
    let response = assert_same_response(operations::EVENTS_SINCE);
    let result = response.result.expect("events result");

    assert_eq!(result["current_cursor"], "fixture-cursor-1");
    assert_eq!(result["events"][0]["cursor"], "fixture-cursor-1");
}

#[test]
fn session_resolve_reports_active_session_status() {
    let response = assert_same_response(operations::SESSION_RESOLVE);
    let result = response.result.expect("session result");

    assert_eq!(result["session_id"], "test-session");
    assert_eq!(result["status"], "active");
    assert_eq!(result["service_status"], "ready");
}

#[test]
fn service_home_registry_exposes_no_mutation_capable_apply_operations() {
    let service_specs: Vec<_> = mvp_operation_registry()
        .into_iter()
        .filter(|spec| spec.scope == VidaOperationScope::Service)
        .collect();
    assert!(!service_specs.is_empty());
    assert!(service_specs.iter().all(|spec| {
        spec.posture != VidaOperationPosture::Apply
            && spec.posture != VidaOperationPosture::Admin
            && !spec.requires_apply_token
    }));

    let response = assert_same_response("vida.service.apply");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.error.expect("unsupported apply operation").code,
        "unsupported_operation"
    );
}

#[test]
fn project_registry_list_and_get_return_project_and_worktree_ids() {
    let list_response = assert_same_response(operations::PROJECT_REGISTRY_LIST);
    let list = list_response.result.expect("registry list result");
    let projects = list["projects"].as_array().expect("projects array");
    assert!(projects.len() >= 2);
    assert!(projects.iter().any(|project| {
        project["project_id"] == "vida-stack"
            && project["worktree_environment_id"] == "worktree-vida-stack-main"
    }));

    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let request = envelope_with_payload(
        operations::PROJECT_REGISTRY_GET,
        json!({ "registry_entry_id": "vida-stack-main" }),
    );
    let fixture_response = fixture.execute(request.clone());
    let in_process_response = in_process.execute(request);
    assert_eq!(fixture_response, in_process_response);
    let result = fixture_response.result.expect("registry get result");
    assert_eq!(result["project"]["registry_entry_id"], "vida-stack-main");
    assert_eq!(result["project"]["project_id"], "vida-stack");
}

#[test]
fn project_registry_discover_returns_same_entries_as_registry_list() {
    let list = assert_same_response(operations::PROJECT_REGISTRY_LIST)
        .result
        .expect("registry list result");
    let discover = assert_same_response(operations::PROJECT_REGISTRY_DISCOVER)
        .result
        .expect("registry discover result");

    assert_eq!(discover["service"], "vida");
    assert_eq!(discover["discovery_mode"], "fixture");
    assert_eq!(discover["discovered_projects"], list["projects"]);
}

#[test]
fn project_registry_get_missing_entry_returns_structured_blocker() {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let request = envelope_with_payload(
        operations::PROJECT_REGISTRY_GET,
        json!({ "registry_entry_id": "missing-project" }),
    );

    let fixture_response = fixture.execute(request.clone());
    let in_process_response = in_process.execute(request);
    assert_eq!(fixture_response, in_process_response);
    assert_eq!(fixture_response.status, VidaResponseStatus::Blocked);
    let problem = fixture_response.error.expect("missing project problem");
    assert_eq!(problem.code, "project_not_found");
    assert_eq!(fixture_response.blockers[0].code, "project_not_registered");
}

#[test]
fn project_resolve_is_ambiguity_safe_without_project_ref() {
    let response = assert_same_response(operations::PROJECT_RESOLVE);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    let problem = response.error.expect("ambiguous project problem");
    assert_eq!(problem.code, "project_resolution_ambiguous");
    assert_eq!(response.blockers[0].code, "project_ref_required");
}

#[test]
fn project_resolve_and_status_use_project_ref() {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let project_ref = VidaProjectRef::ProjectId {
        project_id: VidaProjectId("vida-stack".to_string()),
    };

    let resolve = envelope_with_project_ref(operations::PROJECT_RESOLVE, project_ref.clone());
    let fixture_resolve = fixture.execute(resolve.clone());
    let in_process_resolve = in_process.execute(resolve);
    assert_eq!(fixture_resolve, in_process_resolve);
    assert_eq!(fixture_resolve.status, VidaResponseStatus::Pass);
    let resolved = fixture_resolve.result.expect("project resolve result");
    assert_eq!(resolved["project"]["project_id"], "vida-stack");
    assert_eq!(
        resolved["project"]["worktree_environment_id"],
        "worktree-vida-stack-main"
    );

    let status = envelope_with_project_ref(operations::PROJECT_STATUS, project_ref);
    let fixture_status = fixture.execute(status.clone());
    let in_process_status = in_process.execute(status);
    assert_eq!(fixture_status, in_process_status);
    let result = fixture_status.result.expect("project status result");
    assert_eq!(result["project_id"], "vida-stack");
    assert_eq!(result["actor"]["mutation_queue_mode"], "serialized");
    assert_eq!(result["actor"]["read_only_concurrency"], true);
    assert_eq!(
        result["actor"]["mutation_intent_serialization"]["enabled"],
        true
    );
    assert_eq!(
        result["actor"]["mutation_intent_serialization"]["apply_execution_supported"],
        false
    );
}

#[test]
fn project_resolve_supports_registry_entry_and_root_path_refs() {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    for project_ref in [
        VidaProjectRef::RegistryEntry {
            registry_entry_id: "vida-mobile-main".to_string(),
        },
        VidaProjectRef::RootPath {
            root_path: "C:/project/vida_mobile".to_string(),
        },
    ] {
        let request = envelope_with_project_ref(operations::PROJECT_RESOLVE, project_ref);
        let fixture_response = fixture.execute(request.clone());
        let in_process_response = in_process.execute(request);
        assert_eq!(fixture_response, in_process_response);
        assert_eq!(fixture_response.status, VidaResponseStatus::Pass);
        let result = fixture_response.result.expect("project resolve result");
        assert_eq!(result["project"]["project_id"], "vida-mobile");
        assert_eq!(
            result["project"]["worktree_environment_id"],
            "worktree-vida-mobile-main"
        );
    }
}

#[test]
fn project_status_unknown_project_ref_returns_structured_blocker() {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let request = envelope_with_project_ref(
        operations::PROJECT_STATUS,
        VidaProjectRef::ProjectId {
            project_id: VidaProjectId("missing-project".to_string()),
        },
    );

    let fixture_response = fixture.execute(request.clone());
    let in_process_response = in_process.execute(request);
    assert_eq!(fixture_response, in_process_response);
    assert_eq!(fixture_response.status, VidaResponseStatus::Blocked);
    let problem = fixture_response.error.expect("missing project problem");
    assert_eq!(problem.code, "project_not_found");
    assert_eq!(fixture_response.blockers[0].code, "project_not_registered");
}

#[test]
fn vida_client_unsupported_operation_returns_structured_problem() {
    let response = assert_same_response("vida.unknown.operation");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    let problem = response.error.expect("unsupported operation problem");
    assert_eq!(problem.code, "unsupported_operation");
    assert_eq!(response.blockers.len(), 1);
    assert_eq!(response.blockers[0].code, "operation_not_registered");
}
