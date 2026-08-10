//! Consume resume-projection placeholders for future TaskFlow core extraction.

#[cfg(test)]
mod tests {
    #[test]
    fn resume_projection_placeholder_module_identity_is_stable() {
        assert!(module_path!().ends_with("::consume::resume_projection::tests"));
    }
}
