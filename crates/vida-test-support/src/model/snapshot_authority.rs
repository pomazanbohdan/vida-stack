use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const JUNE_2026_INVARIANT: &str = "june_2026_live_snapshot_divergence_recovery";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAuthorityInput {
    pub live_ids: BTreeSet<String>,
    pub snapshot_ids: BTreeSet<String>,
    pub snapshot_fresh: bool,
}

impl SnapshotAuthorityInput {
    pub fn from_ids(
        live_ids: impl IntoIterator<Item = impl Into<String>>,
        snapshot_ids: impl IntoIterator<Item = impl Into<String>>,
        snapshot_fresh: bool,
    ) -> Self {
        Self {
            live_ids: live_ids.into_iter().map(Into::into).collect(),
            snapshot_ids: snapshot_ids.into_iter().map(Into::into).collect(),
            snapshot_fresh,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotReadAuthority {
    LiveStore,
    FreshSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAuthorityDecision {
    pub authority: SnapshotReadAuthority,
    pub invariant: String,
    pub reason: String,
    pub recovered_snapshot_only_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAuthorityCounterexample {
    pub input: SnapshotAuthorityInput,
    pub decision: SnapshotAuthorityDecision,
    pub expected_authority: SnapshotReadAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAuthorityModelReport {
    pub invariant: String,
    pub bounded_state_count: usize,
    pub counterexamples: Vec<SnapshotAuthorityCounterexample>,
}

pub fn decide_snapshot_read_authority(input: &SnapshotAuthorityInput) -> SnapshotAuthorityDecision {
    let recovered_snapshot_only_ids = input
        .snapshot_ids
        .difference(&input.live_ids)
        .cloned()
        .collect::<Vec<_>>();
    let snapshot_recovers_live_loss = input.snapshot_fresh
        && input.snapshot_ids.is_superset(&input.live_ids)
        && !recovered_snapshot_only_ids.is_empty();

    if snapshot_recovers_live_loss {
        SnapshotAuthorityDecision {
            authority: SnapshotReadAuthority::FreshSnapshot,
            invariant: JUNE_2026_INVARIANT.to_string(),
            reason: "fresh_snapshot_contains_live_rows_and_recovers_missing_live_rows".to_string(),
            recovered_snapshot_only_ids,
        }
    } else {
        SnapshotAuthorityDecision {
            authority: SnapshotReadAuthority::LiveStore,
            invariant: JUNE_2026_INVARIANT.to_string(),
            reason: "live_store_remains_authoritative_without_fresh_richer_snapshot".to_string(),
            recovered_snapshot_only_ids,
        }
    }
}

pub fn bounded_snapshot_authority_state_space() -> Vec<SnapshotAuthorityInput> {
    let universe = ["epic-root", "task-a", "task-b"];
    let mut states = Vec::new();

    for live_mask in 0u8..8 {
        for snapshot_mask in 0u8..8 {
            for snapshot_fresh in [false, true] {
                states.push(masked_state(
                    &universe,
                    live_mask,
                    snapshot_mask,
                    snapshot_fresh,
                ));
            }
        }
    }

    states
}

pub fn masked_snapshot_authority_state(
    live_mask: u8,
    snapshot_mask: u8,
    snapshot_fresh: bool,
) -> SnapshotAuthorityInput {
    masked_state(
        &["epic-root", "task-a", "task-b"],
        live_mask,
        snapshot_mask,
        snapshot_fresh,
    )
}

pub fn model_check_snapshot_authority() -> SnapshotAuthorityModelReport {
    let states = bounded_snapshot_authority_state_space();
    let counterexamples = states
        .iter()
        .filter_map(|input| {
            let decision = decide_snapshot_read_authority(input);
            let expected_authority = expected_authority(input);
            (decision.authority != expected_authority).then(|| SnapshotAuthorityCounterexample {
                input: input.clone(),
                decision,
                expected_authority,
            })
        })
        .collect::<Vec<_>>();

    SnapshotAuthorityModelReport {
        invariant: JUNE_2026_INVARIANT.to_string(),
        bounded_state_count: states.len(),
        counterexamples,
    }
}

pub fn snapshot_authority_review_artifact() -> serde_json::Value {
    let report = model_check_snapshot_authority();
    serde_json::json!({
        "invariant": report.invariant,
        "bounded_state_count": report.bounded_state_count,
        "counterexample_count": report.counterexamples.len(),
        "required_authority_switch": {
            "when": "snapshot_fresh && snapshot_ids is a strict superset of live_ids",
            "authority": "fresh_snapshot"
        },
        "june_2026_regression_seed": {
            "live_ids": ["epic-root", "task-a"],
            "snapshot_ids": ["epic-root", "task-a", "task-b"],
            "snapshot_fresh": true,
            "expected_authority": "fresh_snapshot"
        }
    })
}

fn expected_authority(input: &SnapshotAuthorityInput) -> SnapshotReadAuthority {
    let snapshot_has_extra = input
        .snapshot_ids
        .difference(&input.live_ids)
        .next()
        .is_some();
    if input.snapshot_fresh && input.snapshot_ids.is_superset(&input.live_ids) && snapshot_has_extra
    {
        SnapshotReadAuthority::FreshSnapshot
    } else {
        SnapshotReadAuthority::LiveStore
    }
}

fn masked_state(
    universe: &[&str],
    live_mask: u8,
    snapshot_mask: u8,
    snapshot_fresh: bool,
) -> SnapshotAuthorityInput {
    let live_ids = ids_for_mask(universe, live_mask);
    let snapshot_ids = ids_for_mask(universe, snapshot_mask);
    SnapshotAuthorityInput::from_ids(live_ids, snapshot_ids, snapshot_fresh)
}

fn ids_for_mask(universe: &[&str], mask: u8) -> Vec<String> {
    universe
        .iter()
        .enumerate()
        .filter_map(|(index, id)| ((mask & (1 << index)) != 0).then(|| (*id).to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        JUNE_2026_INVARIANT, SnapshotAuthorityInput, SnapshotReadAuthority,
        bounded_snapshot_authority_state_space, decide_snapshot_read_authority,
        masked_snapshot_authority_state, model_check_snapshot_authority,
        snapshot_authority_review_artifact,
    };

    #[test]
    fn fresh_richer_snapshot_becomes_authoritative_and_reports_recovered_ids() {
        let input = SnapshotAuthorityInput::from_ids(
            ["epic-root", "task-a"],
            ["epic-root", "task-a", "task-b"],
            true,
        );

        let decision = decide_snapshot_read_authority(&input);

        assert_eq!(decision.authority, SnapshotReadAuthority::FreshSnapshot);
        assert_eq!(decision.invariant, JUNE_2026_INVARIANT);
        assert_eq!(
            decision.reason,
            "fresh_snapshot_contains_live_rows_and_recovers_missing_live_rows"
        );
        assert_eq!(decision.recovered_snapshot_only_ids, vec!["task-b"]);
    }

    #[test]
    fn live_store_remains_authoritative_for_stale_or_non_richer_snapshots() {
        for input in [
            masked_snapshot_authority_state(0b011, 0b011, true),
            masked_snapshot_authority_state(0b011, 0b001, true),
            masked_snapshot_authority_state(0b011, 0b111, false),
        ] {
            let decision = decide_snapshot_read_authority(&input);
            assert_eq!(decision.authority, SnapshotReadAuthority::LiveStore);
            assert_eq!(
                decision.reason,
                "live_store_remains_authoritative_without_fresh_richer_snapshot"
            );
        }
    }

    #[test]
    fn bounded_model_check_covers_all_masks_without_counterexamples() {
        assert_eq!(bounded_snapshot_authority_state_space().len(), 128);

        let report = model_check_snapshot_authority();

        assert_eq!(report.invariant, JUNE_2026_INVARIANT);
        assert_eq!(report.bounded_state_count, 128);
        assert!(report.counterexamples.is_empty());
    }

    #[test]
    fn review_artifact_exposes_switch_rule_and_regression_seed() {
        let artifact = snapshot_authority_review_artifact();

        assert_eq!(artifact["invariant"], JUNE_2026_INVARIANT);
        assert_eq!(artifact["bounded_state_count"], 128);
        assert_eq!(artifact["counterexample_count"], 0);
        assert_eq!(
            artifact["required_authority_switch"]["authority"],
            "fresh_snapshot"
        );
        assert_eq!(
            artifact["june_2026_regression_seed"]["expected_authority"],
            "fresh_snapshot"
        );
    }
}
