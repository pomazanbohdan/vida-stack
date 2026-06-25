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
