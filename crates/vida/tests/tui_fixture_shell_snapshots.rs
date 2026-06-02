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
fn tui_fixture_shell_snapshots_render_from_fixture_client_without_live_daemon() {
    let client = FixtureVidaClient::new_ready();
    let snapshot = VidaTuiShellSnapshot::from_client(&client);
    assert_eq!(snapshot.service_status, "ready");
    assert_eq!(snapshot.project_id, "vida-stack");
    assert_eq!(snapshot.wizard_step, "inspect");
    assert!(!snapshot.wizard_apply_supported);

    let backend = TestBackend::new(96, 8);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_app_shell(frame, &snapshot))
        .expect("draw fixture app shell");

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("VIDA Operator Console"));
    assert!(rendered.contains("Service: ready | Session: active"));
    assert!(rendered.contains("Project: vida-stack | Worktree: worktree-vida-stack-main"));
    assert!(rendered.contains("Wizard: step=inspect | apply_supported=false"));
    assert!(rendered.contains("Materialization: safe_updates=1 | manual_conflicts=1"));
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
