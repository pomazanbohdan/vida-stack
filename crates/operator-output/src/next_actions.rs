use crate::command_text::human_command;

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn consume_continue_command(run_id: Option<&str>) -> String {
    match run_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some(run_id) => human_command(&format!(
            "vida taskflow consume continue --run-id {} --json",
            shell_quote(run_id)
        )),
        None => human_command("vida taskflow consume continue"),
    }
}

pub fn recovery_latest_command() -> String {
    human_command("vida taskflow recovery latest")
}

pub fn status_command() -> String {
    human_command("vida status")
}

pub fn human_recovery_status_command(run_id: &str) -> String {
    human_command(&format!(
        "vida taskflow recovery status {}",
        shell_quote(run_id)
    ))
}

pub fn human_lane_show_command(run_id: &str) -> String {
    human_command(&format!("vida lane show {}", shell_quote(run_id)))
}

pub fn human_run_graph_status_command(run_id: &str) -> String {
    human_command(&format!(
        "vida taskflow run-graph status {}",
        shell_quote(run_id)
    ))
}

pub fn human_task_next_lawful_command() -> String {
    human_command("vida task next-lawful")
}

pub fn human_taskflow_graph_summary_command() -> String {
    human_command("vida task validate-graph")
}

pub fn human_protocol_binding_repair_command() -> String {
    human_command("vida protocol binding repair")
}

pub fn human_closed_run_reconcile_command() -> String {
    human_command("vida task reconcile-closed-runs --limit 25")
}

pub fn human_dependency_graph_repair_command() -> String {
    human_command("vida task validate-graph")
}

pub fn human_taskflow_protocol_binding_check_command() -> String {
    human_command("vida taskflow protocol-binding check")
}

pub fn human_taskflow_protocol_binding_sync_command() -> String {
    human_command("vida taskflow protocol-binding sync")
}

pub fn human_project_activator_command() -> String {
    human_command("vida project-activator")
}

pub fn human_bundle_check_command() -> String {
    human_command("vida taskflow consume bundle check")
}

pub fn human_lane_retire_command(run_id: &str) -> String {
    human_command(&format!(
        "vida lane retire {} --receipt-id <concrete-receipt-id> --reason <reason> --json",
        shell_quote(run_id)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_commands_are_human_readable() {
        assert_eq!(
            human_run_graph_status_command("run-1"),
            "vida taskflow run-graph status run-1"
        );
        assert_eq!(
            human_taskflow_graph_summary_command(),
            "vida task validate-graph"
        );
        assert_eq!(
            consume_continue_command(Some("run with space")),
            "vida taskflow consume continue --run-id 'run with space'"
        );
        assert_eq!(
            human_run_graph_status_command("run>pwned"),
            "vida taskflow run-graph status 'run>pwned'"
        );
        assert_eq!(
            human_recovery_status_command("run<secret"),
            "vida taskflow recovery status 'run<secret'"
        );
        assert_eq!(
            consume_continue_command(Some("run>pwned")),
            "vida taskflow consume continue --run-id 'run>pwned'"
        );
        assert_eq!(
            human_closed_run_reconcile_command(),
            "vida task reconcile-closed-runs --limit 25"
        );
    }
}
