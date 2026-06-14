//! Scheduler dispatch policy extracted from the VIDA shell adapter.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyTaskSelection<'a> {
    pub task_id: &'a str,
    pub active_critical_path: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryReadySelection {
    pub index: Option<usize>,
    pub source: PrimaryReadySelectionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryReadySelectionSource {
    RequestedCurrentTask,
    RequestedCurrentTaskNotReady,
    ExplicitRunGraphContinuationBinding,
    ExplicitRunGraphContinuationBindingNotReady,
    CriticalPathReadyHead,
    ReadyHeadFallback,
    NoReadyPrimary,
}

impl PrimaryReadySelectionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequestedCurrentTask => "requested_current_task",
            Self::RequestedCurrentTaskNotReady => "requested_current_task_not_ready",
            Self::ExplicitRunGraphContinuationBinding => "explicit_run_graph_continuation_binding",
            Self::ExplicitRunGraphContinuationBindingNotReady => {
                "explicit_run_graph_continuation_binding_not_ready"
            }
            Self::CriticalPathReadyHead => "critical_path_ready_head",
            Self::ReadyHeadFallback => "ready_head_fallback",
            Self::NoReadyPrimary => "no_ready_primary",
        }
    }
}

pub fn effective_parallel_limit(configured: u64, requested: Option<u64>) -> u64 {
    let configured = configured.max(1);
    requested
        .filter(|value| *value > 0)
        .map(|value| configured.min(value))
        .unwrap_or(configured)
        .max(1)
}

pub fn select_primary_ready_task(
    ready: &[ReadyTaskSelection<'_>],
    requested_current_task_id: Option<&str>,
    explicit_bound_current_task_id: Option<&str>,
) -> PrimaryReadySelection {
    if let Some(task_id) = requested_current_task_id {
        let index = ready
            .iter()
            .position(|candidate| candidate.task_id == task_id);
        return PrimaryReadySelection {
            index,
            source: if index.is_some() {
                PrimaryReadySelectionSource::RequestedCurrentTask
            } else {
                PrimaryReadySelectionSource::RequestedCurrentTaskNotReady
            },
        };
    }

    if let Some(task_id) = explicit_bound_current_task_id {
        let index = ready
            .iter()
            .position(|candidate| candidate.task_id == task_id);
        return PrimaryReadySelection {
            index,
            source: if index.is_some() {
                PrimaryReadySelectionSource::ExplicitRunGraphContinuationBinding
            } else {
                PrimaryReadySelectionSource::ExplicitRunGraphContinuationBindingNotReady
            },
        };
    }

    if let Some(index) = ready
        .iter()
        .position(|candidate| candidate.active_critical_path)
    {
        return PrimaryReadySelection {
            index: Some(index),
            source: PrimaryReadySelectionSource::CriticalPathReadyHead,
        };
    }

    PrimaryReadySelection {
        index: (!ready.is_empty()).then_some(0),
        source: if ready.is_empty() {
            PrimaryReadySelectionSource::NoReadyPrimary
        } else {
            PrimaryReadySelectionSource::ReadyHeadFallback
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanoutRejectedCandidate<'a> {
    pub ready_now: bool,
    pub parallel_blockers: &'a [String],
    pub reasons: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanoutGuardSummary {
    pub status: &'static str,
    pub ready_parallel_safe_count: usize,
    pub lanes_selected: usize,
    pub cap_limited_rejected_count: usize,
    pub conflict_rejected_count: usize,
    pub unsafe_ready_rejected_count: usize,
    pub rejected_candidate_count: usize,
}

pub fn fanout_guard_summary(
    ready_parallel_flags: &[bool],
    selected_task_count: usize,
    rejected_candidates: &[FanoutRejectedCandidate<'_>],
    blocker_codes: &[String],
) -> FanoutGuardSummary {
    FanoutGuardSummary {
        status: if blocker_codes.is_empty() {
            "pass"
        } else {
            "blocked"
        },
        ready_parallel_safe_count: ready_parallel_flags
            .iter()
            .filter(|ready_parallel_safe| **ready_parallel_safe)
            .count(),
        lanes_selected: selected_task_count,
        cap_limited_rejected_count: rejected_candidates
            .iter()
            .filter(|candidate| {
                candidate.reasons.iter().any(|reason| {
                    reason == "max_parallel_agents_cap_reached"
                        || reason == "effective_max_parallel_agents_cap_reached"
                })
            })
            .count(),
        conflict_rejected_count: rejected_candidates
            .iter()
            .filter(|candidate| {
                candidate.reasons.iter().any(|reason| {
                    reason.starts_with("conflict_domain_already_selected:")
                        || reason.starts_with("owned_path_already_selected:")
                        || reason.starts_with("active_scheduler_reservation_")
                        || reason.starts_with("active_orchestrator_claim_")
                })
            })
            .count(),
        unsafe_ready_rejected_count: rejected_candidates
            .iter()
            .filter(|candidate| candidate.ready_now && !candidate.parallel_blockers.is_empty())
            .count(),
        rejected_candidate_count: rejected_candidates.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(task_id: &'static str, active_critical_path: bool) -> ReadyTaskSelection<'static> {
        ReadyTaskSelection {
            task_id,
            active_critical_path,
        }
    }

    #[test]
    fn effective_parallel_limit_clamps_to_configured_positive_limit() {
        assert_eq!(effective_parallel_limit(0, None), 1);
        assert_eq!(effective_parallel_limit(4, Some(2)), 2);
        assert_eq!(effective_parallel_limit(4, Some(0)), 4);
        assert_eq!(effective_parallel_limit(2, Some(8)), 2);
    }

    #[test]
    fn select_primary_ready_task_prefers_explicit_inputs_then_critical_path() {
        let candidates = [ready("a", false), ready("b", true), ready("c", false)];

        let requested = select_primary_ready_task(&candidates, Some("c"), None);
        assert_eq!(requested.index, Some(2));
        assert_eq!(
            requested.source,
            PrimaryReadySelectionSource::RequestedCurrentTask
        );

        let explicit = select_primary_ready_task(&candidates, None, Some("missing"));
        assert_eq!(explicit.index, None);
        assert_eq!(
            explicit.source,
            PrimaryReadySelectionSource::ExplicitRunGraphContinuationBindingNotReady
        );

        let critical = select_primary_ready_task(&candidates, None, None);
        assert_eq!(critical.index, Some(1));
        assert_eq!(
            critical.source,
            PrimaryReadySelectionSource::CriticalPathReadyHead
        );
    }

    #[test]
    fn fanout_guard_summary_classifies_rejection_groups() {
        let cap = vec!["max_parallel_agents_cap_reached".to_string()];
        let conflict = vec!["owned_path_already_selected:crates/vida".to_string()];
        let blockers = vec!["unsafe_parallel".to_string()];
        let rejected = [
            FanoutRejectedCandidate {
                ready_now: true,
                parallel_blockers: &[],
                reasons: &cap,
            },
            FanoutRejectedCandidate {
                ready_now: true,
                parallel_blockers: &blockers,
                reasons: &conflict,
            },
        ];

        let summary = fanout_guard_summary(&[true, false, true], 2, &rejected, &[]);

        assert_eq!(summary.status, "pass");
        assert_eq!(summary.ready_parallel_safe_count, 2);
        assert_eq!(summary.lanes_selected, 2);
        assert_eq!(summary.cap_limited_rejected_count, 1);
        assert_eq!(summary.conflict_rejected_count, 1);
        assert_eq!(summary.unsafe_ready_rejected_count, 1);
        assert_eq!(summary.rejected_candidate_count, 2);
    }
}
