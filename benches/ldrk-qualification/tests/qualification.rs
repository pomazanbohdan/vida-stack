use vida_test_support::failure_injection::{
    benchmark_review_artifact, failure_matrix_review_artifact, run_ldrk_qualification,
};

#[test]
fn failure_matrix_has_no_lost_commands_or_duplicate_effects() {
    let report = run_ldrk_qualification();

    assert_eq!(report.lost_command_count, 0);
    assert_eq!(report.duplicate_semantic_effect_count, 0);
    assert_eq!(report.concurrency_violation_count, 0);
    assert!(report.repair_needed_count >= 1);
    insta::assert_json_snapshot!("failure_matrix_report", failure_matrix_review_artifact());
}

#[test]
fn recovery_receipts_cover_healthy_and_repair_needed_states() {
    let report = run_ldrk_qualification();
    let outcomes = report
        .recovery_receipts
        .iter()
        .map(|receipt| receipt.outcome)
        .collect::<Vec<_>>();

    assert!(outcomes.contains(&"healthy_projection"));
    assert!(outcomes.contains(&"repair_needed"));
}

#[test]
fn benchmark_comparison_stays_within_frozen_thresholds() {
    let report = run_ldrk_qualification();

    assert!(report.benchmark_comparison.within_threshold);
    insta::assert_json_snapshot!("benchmark_comparison", benchmark_review_artifact());
}
