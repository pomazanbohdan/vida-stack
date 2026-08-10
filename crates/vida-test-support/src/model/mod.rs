pub mod snapshot_authority;

#[cfg(test)]
mod tests {
    use super::snapshot_authority::{
        JUNE_2026_INVARIANT, SnapshotReadAuthority, bounded_snapshot_authority_state_space,
        decide_snapshot_read_authority, masked_snapshot_authority_state,
        model_check_snapshot_authority,
    };

    #[test]
    fn snapshot_authority_model_contract_is_reachable_from_model_module() {
        let empty = masked_snapshot_authority_state(0, 0, false);
        let empty_decision = decide_snapshot_read_authority(&empty);
        assert_eq!(empty_decision.authority, SnapshotReadAuthority::LiveStore);
        assert_eq!(empty_decision.invariant, JUNE_2026_INVARIANT);

        let richer_fresh_snapshot = masked_snapshot_authority_state(0b001, 0b011, true);
        let richer_decision = decide_snapshot_read_authority(&richer_fresh_snapshot);
        assert_eq!(
            richer_decision.authority,
            SnapshotReadAuthority::FreshSnapshot
        );

        let states = bounded_snapshot_authority_state_space();
        assert_eq!(states.len(), 128);
        let report = model_check_snapshot_authority();
        assert_eq!(report.invariant, JUNE_2026_INVARIANT);
        assert_eq!(report.bounded_state_count, states.len());
        assert!(report.counterexamples.is_empty());
    }
}
