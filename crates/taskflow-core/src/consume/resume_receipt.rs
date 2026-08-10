//! Consume resume-receipt placeholders for future TaskFlow core extraction.

#[cfg(test)]
mod tests {
    #[test]
    fn resume_receipt_placeholder_module_identity_is_stable() {
        assert!(module_path!().ends_with("::consume::resume_receipt::tests"));
    }
}
