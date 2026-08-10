//! Scheduling route-explanation placeholders for future TaskFlow core extraction.

#[cfg(test)]
mod tests {
    #[test]
    fn scheduling_route_explain_placeholder_module_identity_contract() {
        assert!(module_path!().ends_with("::scheduling::route_explain::tests"));
    }
}
