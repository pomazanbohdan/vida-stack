//! Scheduler dispatch policy extracted from the VIDA shell adapter.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyTaskSelection<'a> {
    pub task_id: &'a str,
    pub active_critical_path: bool,
}

pub const PARALLEL_BLOCKER_NO_CURRENT_TASK_REFERENCE: &str = "no_current_task_reference";
pub const PARALLEL_BLOCKER_CURRENT_TASK_REFERENCE: &str = "current_task_reference";
pub const PARALLEL_BLOCKER_EXECUTION_MODE_NOT_PARALLEL_SAFE: &str =
    "execution_mode_not_parallel_safe";
pub const PARALLEL_BLOCKER_CURRENT_EXECUTION_MODE_NOT_PARALLEL_SAFE: &str =
    "current_execution_mode_not_parallel_safe";
pub const PARALLEL_BLOCKER_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION: &str =
    "missing_owned_paths_for_parallel_execution";
pub const PARALLEL_BLOCKER_CURRENT_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION: &str =
    "current_missing_owned_paths_for_parallel_execution";
pub const PARALLEL_BLOCKER_ORDER_BUCKET_MISMATCH_OR_MISSING: &str =
    "order_bucket_mismatch_or_missing";
pub const PARALLEL_BLOCKER_CONFLICT_DOMAIN_COLLISION: &str = "conflict_domain_collision";
pub const PARALLEL_BLOCKER_MISSING_CONFLICT_DOMAIN: &str = "missing_conflict_domain";
pub const PARALLEL_BLOCKER_PARALLEL_GROUP_MISMATCH: &str = "parallel_group_mismatch";
pub const PARALLEL_BLOCKER_OWNED_PATH_COLLISION: &str = "owned_path_collision";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelSafetyInput<'a> {
    pub task_id: &'a str,
    pub execution_mode: Option<&'a str>,
    pub order_bucket: Option<&'a str>,
    pub parallel_group: Option<&'a str>,
    pub conflict_domain: Option<&'a str>,
    pub owned_paths: Vec<&'a str>,
}

pub fn parallel_blockers_against_current(
    candidate: ParallelSafetyInput<'_>,
    current: Option<ParallelSafetyInput<'_>>,
) -> Vec<String> {
    let Some(current) = current else {
        return vec![PARALLEL_BLOCKER_NO_CURRENT_TASK_REFERENCE.to_string()];
    };
    if candidate.task_id == current.task_id {
        return vec![PARALLEL_BLOCKER_CURRENT_TASK_REFERENCE.to_string()];
    }

    let mut blockers = Vec::new();
    if candidate.execution_mode != Some("parallel_safe") {
        blockers.push(PARALLEL_BLOCKER_EXECUTION_MODE_NOT_PARALLEL_SAFE.to_string());
    }
    if current.execution_mode != Some("parallel_safe") {
        blockers.push(PARALLEL_BLOCKER_CURRENT_EXECUTION_MODE_NOT_PARALLEL_SAFE.to_string());
    }
    let candidate_owned_paths = normalized_owned_paths(&candidate.owned_paths);
    let current_owned_paths = normalized_owned_paths(&current.owned_paths);
    if candidate.execution_mode == Some("parallel_safe") && candidate_owned_paths.is_empty() {
        blockers.push(PARALLEL_BLOCKER_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION.to_string());
    }
    if current.execution_mode == Some("parallel_safe") && current_owned_paths.is_empty() {
        blockers
            .push(PARALLEL_BLOCKER_CURRENT_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION.to_string());
    }
    if owned_paths_overlap(&candidate_owned_paths, &current_owned_paths) {
        blockers.push(PARALLEL_BLOCKER_OWNED_PATH_COLLISION.to_string());
    }

    match (candidate.order_bucket, current.order_bucket) {
        (Some(left), Some(right)) if left == right => {}
        _ => blockers.push(PARALLEL_BLOCKER_ORDER_BUCKET_MISMATCH_OR_MISSING.to_string()),
    }

    match (candidate.conflict_domain, current.conflict_domain) {
        (Some(left), Some(right)) if left != right => {}
        (Some(_), Some(_)) => blockers.push(PARALLEL_BLOCKER_CONFLICT_DOMAIN_COLLISION.to_string()),
        _ => blockers.push(PARALLEL_BLOCKER_MISSING_CONFLICT_DOMAIN.to_string()),
    }

    match (candidate.parallel_group, current.parallel_group) {
        (None, None) => {}
        (Some(left), Some(right)) if left == right => {}
        _ => blockers.push(PARALLEL_BLOCKER_PARALLEL_GROUP_MISMATCH.to_string()),
    }

    blockers
}

fn normalized_owned_paths(owned_paths: &[&str]) -> Vec<String> {
    owned_paths
        .iter()
        .filter_map(|path| normalize_owned_path(path))
        .collect()
}

fn normalize_owned_path(path: &str) -> Option<String> {
    let path = path.trim().replace('\\', "/");
    if path.is_empty() {
        return None;
    }

    let absolute = path.starts_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." if segments.last().is_some_and(|segment| *segment != "..") => {
                segments.pop();
            }
            ".." if absolute => {}
            ".." => segments.push(segment),
            _ => segments.push(segment),
        }
    }

    let mut normalized = if segments.is_empty() {
        if absolute { "/" } else { "." }.to_string()
    } else {
        format!("{}{}", if absolute { "/" } else { "" }, segments.join("/"))
    };
    if cfg!(windows) {
        normalized = normalized.to_lowercase();
    }
    Some(normalized)
}

fn owned_paths_overlap(left: &[String], right: &[String]) -> bool {
    left.iter().any(|left_path| {
        right.iter().any(|right_path| {
            path_is_same_or_ancestor(left_path, right_path)
                || path_is_same_or_ancestor(right_path, left_path)
        })
    })
}

fn path_is_same_or_ancestor(ancestor: &str, path: &str) -> bool {
    ancestor == path
        || (ancestor == "." && !path.starts_with('/'))
        || (ancestor == "/" && path.starts_with('/'))
        || path
            .strip_prefix(ancestor)
            .is_some_and(|suffix| suffix.starts_with('/'))
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

    fn parallel_input<'a>(
        task_id: &'a str,
        execution_mode: Option<&'a str>,
        order_bucket: Option<&'a str>,
        parallel_group: Option<&'a str>,
        conflict_domain: Option<&'a str>,
        owned_paths: &'a [&'a str],
    ) -> ParallelSafetyInput<'a> {
        ParallelSafetyInput {
            task_id,
            execution_mode,
            order_bucket,
            parallel_group,
            conflict_domain,
            owned_paths: owned_paths.to_vec(),
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
    fn parallel_blockers_against_current_reports_canonical_codes() {
        let blockers = parallel_blockers_against_current(
            parallel_input(
                "candidate",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("shared-domain"),
                &[],
            ),
            Some(parallel_input(
                "current",
                Some("parallel_safe"),
                Some("wave-b"),
                Some("readers"),
                Some("shared-domain"),
                &[],
            )),
        );

        assert_eq!(
            blockers,
            vec![
                PARALLEL_BLOCKER_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION,
                PARALLEL_BLOCKER_CURRENT_MISSING_OWNED_PATHS_FOR_PARALLEL_EXECUTION,
                PARALLEL_BLOCKER_ORDER_BUCKET_MISMATCH_OR_MISSING,
                PARALLEL_BLOCKER_CONFLICT_DOMAIN_COLLISION,
                PARALLEL_BLOCKER_PARALLEL_GROUP_MISMATCH,
            ]
        );
    }

    #[test]
    fn parallel_blockers_against_current_rejects_overlapping_owned_paths() {
        let blockers = parallel_blockers_against_current(
            parallel_input(
                "candidate",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("candidate-domain"),
                &["crates/shared/src/lib.rs"],
            ),
            Some(parallel_input(
                "current",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("current-domain"),
                &[" crates/shared/src/lib.rs "],
            )),
        );

        assert_eq!(blockers, vec![PARALLEL_BLOCKER_OWNED_PATH_COLLISION]);
    }

    #[test]
    fn parallel_blockers_against_current_allows_disjoint_paths() {
        let blockers = parallel_blockers_against_current(
            parallel_input(
                "candidate",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("candidate-domain"),
                &["crates/frontend/src"],
            ),
            Some(parallel_input(
                "current",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("current-domain"),
                &["crates/backend/src"],
            )),
        );

        assert!(blockers.is_empty());
    }

    #[test]
    fn parallel_blockers_against_current_normalizes_aliases_and_prefixes() {
        for current_path in [
            "crates/shared/src",
            "crates/shared/./src/",
            "crates\\shared\\generated\\..\\src",
        ] {
            let blockers = parallel_blockers_against_current(
                parallel_input(
                    "candidate",
                    Some("parallel_safe"),
                    Some("wave-a"),
                    Some("writers"),
                    Some("candidate-domain"),
                    &["crates/shared/src/lib.rs"],
                ),
                Some(parallel_input(
                    "current",
                    Some("parallel_safe"),
                    Some("wave-a"),
                    Some("writers"),
                    Some("current-domain"),
                    &[current_path],
                )),
            );

            assert_eq!(
                blockers,
                vec![PARALLEL_BLOCKER_OWNED_PATH_COLLISION],
                "alias {current_path} should overlap its descendant"
            );
        }
    }

    #[test]
    fn parallel_blockers_against_current_uses_platform_case_semantics() {
        let blockers = parallel_blockers_against_current(
            parallel_input(
                "candidate",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("candidate-domain"),
                &["Crates/Shared/src"],
            ),
            Some(parallel_input(
                "current",
                Some("parallel_safe"),
                Some("wave-a"),
                Some("writers"),
                Some("current-domain"),
                &["crates/shared/src/lib.rs"],
            )),
        );

        if cfg!(windows) {
            assert_eq!(blockers, vec![PARALLEL_BLOCKER_OWNED_PATH_COLLISION]);
        } else {
            assert!(blockers.is_empty());
        }
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
