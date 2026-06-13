use crate::command_text::human_command;

pub fn human_recovery_status_command(run_id: &str) -> String {
    human_command(&format!("vida taskflow recovery status {run_id}"))
}

pub fn human_lane_show_command(run_id: &str) -> String {
    human_command(&format!("vida lane show {run_id}"))
}

pub fn human_run_graph_status_command(run_id: &str) -> String {
    human_command(&format!("vida taskflow run-graph status {run_id}"))
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
    human_command("vida taskflow run-graph reconcile-closed")
}

pub fn human_dependency_graph_repair_command() -> String {
    human_command("vida task repair-graph")
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
    }
}
