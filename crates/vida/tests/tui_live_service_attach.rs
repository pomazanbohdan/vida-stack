#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_fixture.rs"]
mod vida_client_fixture;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;
#[allow(dead_code)]
#[path = "../src/vida_transport_tarpc.rs"]
mod vida_transport_tarpc;
#[path = "../src/vida_tui_shell.rs"]
mod vida_tui_shell;

use ratatui::{backend::TestBackend, Terminal};
use vida_client::VidaClient;
use vida_contracts::{
    operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaIdempotencyKey,
    VidaOperation, VidaRequestId, VidaResponseStatus, VidaSessionId, VIDA_COMMAND_PROTOCOL_VERSION,
    VIDA_CONTRACTS_SCHEMA_VERSION,
};
use vida_transport_tarpc::TarpcLocalIpcVidaClient;
use vida_tui_shell::{render_app_shell, VidaTuiShellSnapshot};

struct BlockingTarpcVidaClient {
    runtime: tokio::runtime::Runtime,
    client: TarpcLocalIpcVidaClient,
}

impl BlockingTarpcVidaClient {
    fn connect_ready_local_socket() -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let client = runtime
            .block_on(TarpcLocalIpcVidaClient::connect_ready_local_socket())
            .expect("tarpc local socket client");
        Self { runtime, client }
    }

    fn execute_async(&self, envelope: VidaCommandEnvelope) -> vida_contracts::VidaCommandResponse {
        self.runtime
            .block_on(self.client.execute(envelope))
            .expect("tarpc envelope response")
    }
}

impl VidaClient for BlockingTarpcVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> vida_contracts::VidaCommandResponse {
        self.execute_async(envelope)
    }
}

#[test]
fn ratatui_live_attach_renders_operator_console_from_tarpc_local_socket() {
    let client = BlockingTarpcVidaClient::connect_ready_local_socket();
    let hello = client.execute(service_envelope(operations::SERVICE_HELLO));
    assert_eq!(hello.status, VidaResponseStatus::Pass);
    assert_eq!(hello.result.expect("hello result")["service"], "vida");

    let snapshot = VidaTuiShellSnapshot::from_client(&client);
    assert_eq!(snapshot.service_status, "ready");
    assert_eq!(snapshot.active_session, "active");
    assert_eq!(snapshot.project_count, 2);
    assert_eq!(snapshot.project_id, "vida-stack");
    assert_eq!(snapshot.wizard_step, "inspect");
    assert_eq!(snapshot.wizard_validation_findings, 1);
    assert_eq!(snapshot.wizard_diff_change_count, 4);
    assert_eq!(snapshot.job_status, "completed");
    assert_eq!(snapshot.event_count, 1);
    assert_eq!(snapshot.receipt_count, 3);
    assert_eq!(snapshot.lifecycle_state, "ready");
    assert!(snapshot.orchestration_tui_projection);

    let backend = TestBackend::new(144, 12);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_app_shell(frame, &snapshot))
        .expect("draw live tarpc app shell");

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("VIDA Operator Console"));
    assert!(rendered.contains("Service: ready | Session: active"));
    assert!(rendered
        .contains("Projects: count=2 | active=vida-stack | Worktree: worktree-vida-stack-main"));
    assert!(rendered.contains(
        "Wizard: step=inspect | validation_findings=1 | diff_changes=4 | apply_supported=false"
    ));
    assert!(rendered.contains("Jobs/Events/Receipts: job_status=completed | events=1 | receipts=3"));
    assert!(
        rendered.contains("Lifecycle: state=ready | binary_fingerprint=fixture-binary-fingerprint")
    );
}

fn service_envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("tui-live-service-session".to_string()),
        request_id: VidaRequestId(format!("tui-live-request-{operation}")),
        client_kind: VidaClientKind::Tui,
        project_ref: None,
        claim_kind: Some(VidaClaimKind::Observe),
        payload: serde_json::json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("tui-live-idem-{operation}"))),
        apply_token: None,
    }
}

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    let mut rendered = String::new();
    for y in buffer.area.y..buffer.area.y + buffer.area.height {
        for x in buffer.area.x..buffer.area.x + buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }
    rendered
}
