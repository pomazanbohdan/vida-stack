//! Run-graph closure placeholders for future TaskFlow core extraction.

#[cfg(test)]
mod tests {
    #[test]
    fn closure_placeholder_module_identity_is_stable() {
        assert!(module_path!().ends_with("::run_graph::closure::tests"));
    }
}
