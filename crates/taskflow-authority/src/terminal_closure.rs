pub const MODULE: &str = "terminal_closure";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalClosureStatus<'a> {
    pub run_id: &'a str,
    pub active_node: &'a str,
    pub next_node: Option<&'a str>,
    pub status: &'a str,
    pub lifecycle_stage: &'a str,
    pub resume_target: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalClosureReceipt<'a> {
    pub run_id: &'a str,
    pub dispatch_target: &'a str,
    pub dispatch_status: &'a str,
    pub lane_status: &'a str,
    pub exception_path_receipt_id: Option<&'a str>,
}

pub fn terminal_missing_task_closure_has_clean_dispatch_receipt(
    status: &TerminalClosureStatus<'_>,
    receipt: Option<&TerminalClosureReceipt<'_>>,
) -> bool {
    if !status_is_terminal_closure_without_next_unit(status) {
        return false;
    }
    let Some(receipt) = receipt else {
        return false;
    };
    if receipt.run_id != status.run_id {
        return false;
    }
    if receipt.dispatch_status != "executed" || receipt.lane_status != "lane_completed" {
        return false;
    }
    if receipt.exception_path_receipt_id.is_some() {
        return false;
    }
    status.active_node.trim().is_empty()
        || status.active_node == "closure"
        || receipt.dispatch_target == status.active_node
}

pub fn status_is_terminal_closure_without_next_unit(status: &TerminalClosureStatus<'_>) -> bool {
    status.lifecycle_stage == "closure_complete"
        && matches!(status.status, "completed" | "pass")
        && status.next_node.is_none()
        && status
            .resume_target
            .map(str::trim)
            .is_none_or(|value| value.is_empty() || value == "none")
}

#[cfg(test)]
mod tests {
    use super::{
        TerminalClosureReceipt, TerminalClosureStatus,
        terminal_missing_task_closure_has_clean_dispatch_receipt,
    };

    fn terminal_status() -> TerminalClosureStatus<'static> {
        TerminalClosureStatus {
            run_id: "run-1",
            active_node: "closure",
            next_node: None,
            status: "completed",
            lifecycle_stage: "closure_complete",
            resume_target: Some("none"),
        }
    }

    fn clean_receipt() -> TerminalClosureReceipt<'static> {
        TerminalClosureReceipt {
            run_id: "run-1",
            dispatch_target: "closure",
            dispatch_status: "executed",
            lane_status: "lane_completed",
            exception_path_receipt_id: None,
        }
    }

    #[test]
    fn terminal_missing_task_closure_accepts_clean_receipt() {
        assert!(terminal_missing_task_closure_has_clean_dispatch_receipt(
            &terminal_status(),
            Some(&clean_receipt())
        ));
    }

    #[test]
    fn terminal_missing_task_closure_rejects_missing_receipt() {
        assert!(!terminal_missing_task_closure_has_clean_dispatch_receipt(
            &terminal_status(),
            None
        ));
    }

    #[test]
    fn terminal_missing_task_closure_rejects_exception_receipt() {
        let mut receipt = clean_receipt();
        receipt.exception_path_receipt_id = Some("exception-1");

        assert!(!terminal_missing_task_closure_has_clean_dispatch_receipt(
            &terminal_status(),
            Some(&receipt)
        ));
    }

    #[test]
    fn terminal_missing_task_closure_rejects_run_mismatch() {
        let mut receipt = clean_receipt();
        receipt.run_id = "other-run";

        assert!(!terminal_missing_task_closure_has_clean_dispatch_receipt(
            &terminal_status(),
            Some(&receipt)
        ));
    }

    #[test]
    fn terminal_missing_task_closure_rejects_non_terminal_status() {
        let mut status = terminal_status();
        status.lifecycle_stage = "closure_active";

        assert!(!terminal_missing_task_closure_has_clean_dispatch_receipt(
            &status,
            Some(&clean_receipt())
        ));
    }

    #[test]
    fn terminal_closure_rejects_pending_resume_target() {
        let mut status = terminal_status();
        status.resume_target = Some("dispatch.developer");

        assert!(!terminal_missing_task_closure_has_clean_dispatch_receipt(
            &status,
            Some(&clean_receipt())
        ));
    }
}
