pub const MODULE: &str = "continuation_transition";

pub const PAUSE_GATE_NON_BLOCKING_ONLY: &str = "non_blocking_only";
pub const PAUSE_GATE_ALLOWED_IF_NO_FURTHER_BOUND_WORK: &str =
    "allowed_if_no_further_bound_work_is_evidenced";
pub const POSTURE_SEQUENTIAL_ONLY_OPEN_CYCLE: &str = "sequential_only_open_cycle";
pub const POSTURE_SEQUENTIAL_ONLY: &str = "sequential_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuationGateInput {
    pub delegated_cycle_open: bool,
    pub active_exception_takeover_not_resumable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationGateDecision {
    pub continuation_required_now: bool,
    pub pause_boundary_gate: &'static str,
    pub sequential_vs_parallel_posture: &'static str,
}

#[must_use]
pub fn decide_continuation_gate(input: ContinuationGateInput) -> ContinuationGateDecision {
    ContinuationGateDecision {
        continuation_required_now: input.delegated_cycle_open
            && !input.active_exception_takeover_not_resumable,
        pause_boundary_gate: if input.delegated_cycle_open {
            PAUSE_GATE_NON_BLOCKING_ONLY
        } else {
            PAUSE_GATE_ALLOWED_IF_NO_FURTHER_BOUND_WORK
        },
        sequential_vs_parallel_posture: if input.delegated_cycle_open {
            POSTURE_SEQUENTIAL_ONLY_OPEN_CYCLE
        } else {
            POSTURE_SEQUENTIAL_ONLY
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContinuationGateInput, PAUSE_GATE_ALLOWED_IF_NO_FURTHER_BOUND_WORK,
        PAUSE_GATE_NON_BLOCKING_ONLY, POSTURE_SEQUENTIAL_ONLY, POSTURE_SEQUENTIAL_ONLY_OPEN_CYCLE,
        decide_continuation_gate,
    };

    #[test]
    fn continuation_gate_requires_progress_for_open_delegated_cycle() {
        let decision = decide_continuation_gate(ContinuationGateInput {
            delegated_cycle_open: true,
            active_exception_takeover_not_resumable: false,
        });

        assert!(decision.continuation_required_now);
        assert_eq!(decision.pause_boundary_gate, PAUSE_GATE_NON_BLOCKING_ONLY);
        assert_eq!(
            decision.sequential_vs_parallel_posture,
            POSTURE_SEQUENTIAL_ONLY_OPEN_CYCLE
        );
    }

    #[test]
    fn non_resumable_exception_takeover_suppresses_required_continuation() {
        let decision = decide_continuation_gate(ContinuationGateInput {
            delegated_cycle_open: true,
            active_exception_takeover_not_resumable: true,
        });

        assert!(!decision.continuation_required_now);
        assert_eq!(decision.pause_boundary_gate, PAUSE_GATE_NON_BLOCKING_ONLY);
        assert_eq!(
            decision.sequential_vs_parallel_posture,
            POSTURE_SEQUENTIAL_ONLY_OPEN_CYCLE
        );
    }

    #[test]
    fn no_open_cycle_allows_idle_pause_boundary() {
        let decision = decide_continuation_gate(ContinuationGateInput {
            delegated_cycle_open: false,
            active_exception_takeover_not_resumable: false,
        });

        assert!(!decision.continuation_required_now);
        assert_eq!(
            decision.pause_boundary_gate,
            PAUSE_GATE_ALLOWED_IF_NO_FURTHER_BOUND_WORK
        );
        assert_eq!(
            decision.sequential_vs_parallel_posture,
            POSTURE_SEQUENTIAL_ONLY
        );
    }
}
