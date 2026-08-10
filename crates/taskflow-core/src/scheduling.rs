//! Scheduling module skeleton for future TaskFlow core extraction.

pub mod actualize;
pub mod graph_summary;
pub mod next_lawful;
pub mod route_explain;
pub mod scheduler_dispatch;

#[cfg(test)]
mod tests {
    #[test]
    fn scheduling_module_identity_contract() {
        assert!(module_path!().ends_with("::scheduling::tests"));
    }
}
