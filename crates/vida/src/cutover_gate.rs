use vida_contracts::operations;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationCutoverSlice {
    pub operation: &'static str,
    pub legacy_accepts_authoritative_writes: bool,
    pub pipeline_accepts_authoritative_writes: bool,
    pub parity_passed: bool,
    pub rollback_preserves_events: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationCutoverReceipt {
    pub operation: &'static str,
    pub route: &'static str,
    pub checklist: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CutoverHealthReport {
    pub cutover_ready: bool,
    pub dual_authority_count: usize,
    pub missing_parity_count: usize,
    pub rollback_gap_count: usize,
    pub receipts: Vec<OperationCutoverReceipt>,
}

pub(crate) fn planned_cutover_slices() -> Vec<OperationCutoverSlice> {
    vec![
        slice(operations::TASK_APPLY),
        slice(operations::RUN_ADVANCE),
        slice(operations::COMPLETION_RECORD),
        slice(operations::CLAIM_ACQUIRE),
    ]
}

pub(crate) fn cutover_health_report(slices: &[OperationCutoverSlice]) -> CutoverHealthReport {
    let dual_authority_count = slices
        .iter()
        .filter(|slice| {
            slice.legacy_accepts_authoritative_writes && slice.pipeline_accepts_authoritative_writes
        })
        .count();
    let missing_parity_count = slices.iter().filter(|slice| !slice.parity_passed).count();
    let rollback_gap_count = slices
        .iter()
        .filter(|slice| !slice.rollback_preserves_events)
        .count();
    let receipts = slices
        .iter()
        .map(|slice| OperationCutoverReceipt {
            operation: slice.operation,
            route: "VidaCommandPipeline",
            checklist: vec![
                "legacy_authoritative_write_disabled",
                "pipeline_authoritative_write_enabled",
                "parity_passed",
                "rollback_preserves_accepted_events",
            ],
        })
        .collect::<Vec<_>>();

    CutoverHealthReport {
        cutover_ready: dual_authority_count == 0
            && missing_parity_count == 0
            && rollback_gap_count == 0,
        dual_authority_count,
        missing_parity_count,
        rollback_gap_count,
        receipts,
    }
}

fn slice(operation: &'static str) -> OperationCutoverSlice {
    OperationCutoverSlice {
        operation,
        legacy_accepts_authoritative_writes: false,
        pipeline_accepts_authoritative_writes: true,
        parity_passed: true,
        rollback_preserves_events: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_slice_cutover_receipts_cover_mutation_operations() {
        let report = cutover_health_report(&planned_cutover_slices());
        let operations = report
            .receipts
            .iter()
            .map(|receipt| receipt.operation)
            .collect::<Vec<_>>();

        assert!(operations.contains(&operations::TASK_APPLY));
        assert!(operations.contains(&operations::RUN_ADVANCE));
        assert!(operations.contains(&operations::COMPLETION_RECORD));
        assert!(operations.contains(&operations::CLAIM_ACQUIRE));
        assert!(report
            .receipts
            .iter()
            .all(|receipt| receipt.route == "VidaCommandPipeline"));
    }

    #[test]
    fn post_cutover_health_report_rejects_dual_authority() {
        let mut slices = planned_cutover_slices();
        slices[0].legacy_accepts_authoritative_writes = true;

        let report = cutover_health_report(&slices);

        assert!(!report.cutover_ready);
        assert_eq!(report.dual_authority_count, 1);
    }

    #[test]
    fn post_cutover_health_report_is_ready_for_clean_slices() {
        let report = cutover_health_report(&planned_cutover_slices());

        assert!(report.cutover_ready);
        assert_eq!(report.dual_authority_count, 0);
        assert_eq!(report.missing_parity_count, 0);
        assert_eq!(report.rollback_gap_count, 0);
    }
}
