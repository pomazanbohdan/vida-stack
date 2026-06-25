use vida_runtime_local::engine::{FakeRuntimeEngine, LocalRuntimeEngine};
use vida_test_support::engine_conformance::verify_runtime_engine_conformance;

#[test]
fn local_runtime_engine_passes_conformance_scenarios() {
    let report = verify_runtime_engine_conformance("local", &LocalRuntimeEngine).unwrap();

    assert_eq!(report.failed_count(), 0);
    assert!(report.scenario_count() >= 10);
    assert!(
        report
            .supported_capabilities()
            .contains(&"jobs".to_string())
    );
    assert!(
        report
            .supported_capabilities()
            .contains(&"event_export".to_string())
    );
}

#[test]
fn fake_runtime_engine_passes_conformance_scenarios() {
    let report = verify_runtime_engine_conformance("fake", &FakeRuntimeEngine).unwrap();

    assert_eq!(report.failed_count(), 0);
    assert!(
        report
            .supported_capabilities()
            .contains(&"jobs".to_string())
    );
}

#[test]
fn conformance_reports_are_byte_identical_on_repeated_runs() {
    let first = verify_runtime_engine_conformance("local", &LocalRuntimeEngine).unwrap();
    let second = verify_runtime_engine_conformance("local", &LocalRuntimeEngine).unwrap();

    let first_bytes = serde_json::to_vec(&first).unwrap();
    let second_bytes = serde_json::to_vec(&second).unwrap();

    assert_eq!(first_bytes, second_bytes);
    assert_eq!(
        first.deterministic_signature,
        second.deterministic_signature
    );
}
