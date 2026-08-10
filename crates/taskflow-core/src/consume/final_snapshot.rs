//! Consume final-snapshot placeholders for future TaskFlow core extraction.

#[cfg(test)]
mod tests {
    #[test]
    fn final_snapshot_placeholder_module_identity_is_stable() {
        assert!(module_path!().ends_with("::consume::final_snapshot::tests"));
    }
}
