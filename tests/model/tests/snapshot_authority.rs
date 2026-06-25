use proptest::prelude::*;
use vida_test_support::model::snapshot_authority::{
    SnapshotAuthorityInput, SnapshotReadAuthority, decide_snapshot_read_authority,
    masked_snapshot_authority_state, model_check_snapshot_authority,
    snapshot_authority_review_artifact,
};

#[test]
fn model_check_finds_no_snapshot_authority_counterexample() {
    let report = model_check_snapshot_authority();

    assert_eq!(report.bounded_state_count, 128);
    assert!(report.counterexamples.is_empty());
}

#[test]
fn snapshot_authority_changes_require_insta_review() {
    insta::assert_json_snapshot!(
        "snapshot_authority_review",
        snapshot_authority_review_artifact()
    );
}

#[test]
fn june_2026_live_snapshot_divergence_seed_uses_fresh_snapshot() {
    let seed: SnapshotAuthorityInput = serde_json::from_str(include_str!(
        "../proptest-seeds/june_2026_live_snapshot_divergence.json"
    ))
    .expect("seed fixture should parse");

    let decision = decide_snapshot_read_authority(&seed);

    assert_eq!(decision.authority, SnapshotReadAuthority::FreshSnapshot);
    assert_eq!(
        decision.reason,
        "fresh_snapshot_contains_live_rows_and_recovers_missing_live_rows"
    );
    assert_eq!(decision.recovered_snapshot_only_ids, vec!["task-b"]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn richer_fresh_snapshot_is_authoritative_for_all_bounded_masks(
        live_mask in 0u8..8,
        snapshot_mask in 0u8..8,
        snapshot_fresh in any::<bool>(),
    ) {
        let input = masked_snapshot_authority_state(live_mask, snapshot_mask, snapshot_fresh);
        let decision = decide_snapshot_read_authority(&input);
        let snapshot_has_extra = input.snapshot_ids.difference(&input.live_ids).next().is_some();
        let expected = if snapshot_fresh
            && input.snapshot_ids.is_superset(&input.live_ids)
            && snapshot_has_extra
        {
            SnapshotReadAuthority::FreshSnapshot
        } else {
            SnapshotReadAuthority::LiveStore
        };

        prop_assert_eq!(decision.authority, expected);
    }
}
