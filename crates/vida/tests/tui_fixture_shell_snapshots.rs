#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_fixture.rs"]
mod vida_client_fixture;
#[path = "../src/vida_tui_shell.rs"]
mod vida_tui_shell;

use ratatui::{backend::TestBackend, Terminal};
use vida_client_fixture::FixtureVidaClient;
use vida_tui_shell::{render_app_shell, VidaTuiShellSnapshot};

#[test]
fn ratatui_snapshots_render_fixture_operator_console_mvp_without_live_daemon() {
    let client = FixtureVidaClient::new_ready();
    let snapshot = VidaTuiShellSnapshot::from_client(&client);
    assert_eq!(snapshot.service_status, "ready");
    assert_eq!(snapshot.project_count, 2);
    assert_eq!(snapshot.project_id, "vida-stack");
    assert_eq!(snapshot.wizard_step, "inspect");
    assert!(!snapshot.wizard_apply_supported);
    assert_eq!(snapshot.wizard_form_fields.len(), 2);
    assert_eq!(snapshot.wizard_form_fields[0].field_id, "project");
    assert!(snapshot.wizard_form_fields[0].required);
    assert_eq!(snapshot.wizard_form_fields[0].control, "text_input");
    assert_eq!(snapshot.wizard_form_fields[1].field_id, "wizard_kind");
    assert_eq!(snapshot.wizard_form_fields[1].control, "select");
    assert_eq!(snapshot.wizard_validation_findings, 1);
    assert_eq!(snapshot.wizard_diff_change_count, 4);
    assert_eq!(
        snapshot.wizard_disabled_apply_reason,
        "apply-token and claim-proof execution are not implemented"
    );
    assert_eq!(snapshot.drift_report_only, 1);
    assert_eq!(snapshot.job_status, "completed");
    assert_eq!(snapshot.event_count, 1);
    assert_eq!(snapshot.receipt_count, 3);
    assert_eq!(snapshot.lifecycle_state, "ready");
    assert_eq!(
        snapshot.lifecycle_binary_fingerprint,
        "fixture-binary-fingerprint"
    );
    assert_eq!(
        snapshot.orchestration_workspace_owner,
        "task_worktree_assignment"
    );
    assert_eq!(
        snapshot.orchestration_parallelism_source,
        "taskflow_execution_semantics"
    );
    assert!(snapshot.orchestration_tui_projection);

    let backend = TestBackend::new(144, 12);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_app_shell(frame, &snapshot))
        .expect("draw fixture app shell");

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("VIDA Operator Console"));
    assert!(rendered.contains("Service: ready | Session: active"));
    assert!(rendered
        .contains("Projects: count=2 | active=vida-stack | Worktree: worktree-vida-stack-main"));
    assert!(rendered.contains(
        "Wizard: step=inspect | validation_findings=1 | diff_changes=4 | apply_supported=false"
    ));
    assert!(rendered.contains(
        "Wizard form fields[2]{field_id,label,required,control}: project:Project:true:text_input | wizard_kind:Wizard kind:false:select"
    ));
    assert!(rendered
        .contains("Disabled action: apply-token and claim-proof execution are not implemented"));
    assert!(rendered
        .contains("Materialization: safe_updates=1 | manual_conflicts=1 | drift_report_only=1"));
    assert!(rendered.contains("Jobs/Events/Receipts: job_status=completed | events=1 | receipts=3"));
    assert!(
        rendered.contains("Lifecycle: state=ready | binary_fingerprint=fixture-binary-fingerprint")
    );
    assert!(rendered.contains(
        "Orchestration: workspace_owner=task_worktree_assignment | parallelism_source=taskflow_execution_semantics | tui_projection=true"
    ));
}

#[test]
fn tui_shell_does_not_import_direct_project_state_writers() {
    let source = include_str!("../src/vida_tui_shell.rs");
    for forbidden in [
        "StateStore",
        "std::fs::write",
        "write_all",
        "remove_file",
        "remove_dir",
        "create_dir",
        "taskflow_state",
        ".vida/data/state",
    ] {
        assert!(
            !source.contains(forbidden),
            "TUI shell should not directly mutate project state through `{forbidden}`"
        );
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
