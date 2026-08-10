pub use taskflow_authority::domain_conformance::{
    DOMAIN_CONFORMANCE_SCHEMA_VERSION, DomainConformanceReport, DomainConformanceScenarioResult,
    verify_domain_conformance,
};

#[must_use]
pub fn assert_domain_conformance() -> DomainConformanceReport {
    let report = verify_domain_conformance();
    assert!(report.clean(), "{report:#?}");
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assertion_wrapper_returns_clean_versioned_report() {
        let report = assert_domain_conformance();

        assert_eq!(report.schema_version, DOMAIN_CONFORMANCE_SCHEMA_VERSION);
        assert_eq!(report.scenario_count(), 9);
        assert_eq!(report.error_count(), 0);
        assert!(report.clean());
        assert!(!report.covered_semantic_areas().is_empty());
    }
}
