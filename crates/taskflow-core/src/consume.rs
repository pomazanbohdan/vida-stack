//! Consume/resume module skeleton for future TaskFlow core extraction.

pub mod continue_use_case;
pub mod final_snapshot;
pub mod resume_input;
pub mod resume_projection;
pub mod resume_receipt;
pub mod resume_reconciliation;
pub mod resume_state_machine;

#[cfg(test)]
mod tests {
    #[test]
    fn consume_module_identity_is_stable() {
        assert!(module_path!().ends_with("::consume::tests"));
    }
}
