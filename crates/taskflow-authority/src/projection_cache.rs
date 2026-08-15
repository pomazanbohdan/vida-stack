pub const MODULE: &str = "projection_cache";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CachedProjectionShape {
    pub has_current_session: bool,
    pub has_storage_metadata: bool,
    pub has_state_spine: bool,
    pub has_operator_contracts: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedProjectionSession<'a> {
    pub session_id: Option<&'a str>,
    pub worktree_environment_id: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedStatusProjection<'a> {
    pub surface: Option<&'a str>,
    pub has_status: bool,
    pub shape: CachedProjectionShape,
    pub session: CachedProjectionSession<'a>,
    pub cache: ProjectionCacheContract<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectionCacheOperatorMarkers {
    pub task_snapshot_marker: bool,
    pub dispatch_receipts_marker: bool,
    pub continuation_bindings_marker: bool,
    pub run_graph_updates_marker: bool,
    pub runtime_consumption_snapshots_marker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionCacheContract<'a> {
    pub freshness_contract: Option<&'a str>,
    pub operator_markers: ProjectionCacheOperatorMarkers,
}

pub fn cached_status_projection_admissible(
    summary_only: bool,
    payload: &CachedStatusProjection<'_>,
    current_session: &CachedProjectionSession<'_>,
) -> bool {
    payload
        .surface
        .is_none_or(|surface| surface == "vida status")
        && payload.has_status
        && cached_status_projection_has_required_shape(summary_only, &payload.shape)
        && cached_status_projection_matches_current_session(&payload.session, current_session)
}

pub fn cached_status_projection_has_required_shape(
    summary_only: bool,
    shape: &CachedProjectionShape,
) -> bool {
    if summary_only {
        return true;
    }
    shape.has_current_session
        || (shape.has_storage_metadata && shape.has_state_spine && shape.has_operator_contracts)
}

pub fn cached_status_projection_matches_current_session(
    cached: &CachedProjectionSession<'_>,
    current: &CachedProjectionSession<'_>,
) -> bool {
    let cached_worktree_environment_id = nonempty(cached.worktree_environment_id);
    let cached_session_id = nonempty(cached.session_id);

    if let (Some(cached_id), Some(current_id)) = (
        cached_worktree_environment_id,
        nonempty(current.worktree_environment_id),
    ) && cached_id == current_id
    {
        return true;
    }

    if let (Some(cached_id), Some(current_id)) = (cached_session_id, nonempty(current.session_id)) {
        return cached_id == current_id;
    }

    false
}

pub fn cached_projection_is_state_bound_read_only_operator_fallback(
    cache: &ProjectionCacheContract<'_>,
) -> bool {
    let markers = cache.operator_markers;
    matches!(
        cache.freshness_contract.unwrap_or_default(),
        "state_marker_fresh_structural_cache_ok_for_read_only_operator_query"
            | "bounded_state_marker_stale_ok_for_doctor_summary_read_only_operator_query"
            | "recent_bounded_stale_ok_for_read_only_operator_query"
    ) && markers.task_snapshot_marker
        && markers.dispatch_receipts_marker
        && markers.continuation_bindings_marker
        && markers.run_graph_updates_marker
        && markers.runtime_consumption_snapshots_marker
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        CachedProjectionSession, CachedProjectionShape, CachedStatusProjection,
        ProjectionCacheContract, ProjectionCacheOperatorMarkers,
        cached_projection_is_state_bound_read_only_operator_fallback,
        cached_status_projection_admissible, cached_status_projection_has_required_shape,
        cached_status_projection_matches_current_session,
    };

    fn full_shape() -> CachedProjectionShape {
        CachedProjectionShape {
            has_current_session: true,
            has_storage_metadata: false,
            has_state_spine: false,
            has_operator_contracts: false,
        }
    }

    fn current_session() -> CachedProjectionSession<'static> {
        CachedProjectionSession {
            session_id: Some("session-a"),
            worktree_environment_id: Some("worktree-a"),
        }
    }

    fn projection() -> CachedStatusProjection<'static> {
        CachedStatusProjection {
            surface: Some("vida status"),
            has_status: true,
            shape: full_shape(),
            session: current_session(),
            cache: ProjectionCacheContract {
                freshness_contract: None,
                operator_markers: ProjectionCacheOperatorMarkers::default(),
            },
        }
    }

    #[test]
    fn cached_status_projection_shape_accepts_summary_or_full_contract_shape() {
        assert!(cached_status_projection_has_required_shape(
            true,
            &CachedProjectionShape::default()
        ));
        assert!(cached_status_projection_has_required_shape(
            false,
            &full_shape()
        ));
        assert!(cached_status_projection_has_required_shape(
            false,
            &CachedProjectionShape {
                has_current_session: false,
                has_storage_metadata: true,
                has_state_spine: true,
                has_operator_contracts: true,
            }
        ));
        assert!(!cached_status_projection_has_required_shape(
            false,
            &CachedProjectionShape {
                has_current_session: false,
                has_storage_metadata: true,
                has_state_spine: true,
                has_operator_contracts: false,
            }
        ));
        assert!(!cached_status_projection_has_required_shape(
            false,
            &CachedProjectionShape {
                has_current_session: false,
                has_storage_metadata: true,
                has_state_spine: false,
                has_operator_contracts: true,
            }
        ));
        let summary_projection = CachedStatusProjection {
            shape: CachedProjectionShape::default(),
            ..projection()
        };
        assert!(cached_status_projection_admissible(
            true,
            &summary_projection,
            &current_session()
        ));
        assert!(!cached_status_projection_admissible(
            false,
            &summary_projection,
            &current_session()
        ));
    }

    #[test]
    fn cached_status_projection_matches_session_or_worktree() {
        let current = current_session();
        assert!(cached_status_projection_matches_current_session(
            &CachedProjectionSession {
                session_id: Some("session-a"),
                worktree_environment_id: None,
            },
            &current,
        ));
        assert!(cached_status_projection_matches_current_session(
            &CachedProjectionSession {
                session_id: Some("session-b"),
                worktree_environment_id: Some("worktree-a"),
            },
            &current,
        ));
        assert!(!cached_status_projection_matches_current_session(
            &CachedProjectionSession {
                session_id: Some("session-b"),
                worktree_environment_id: Some("worktree-b"),
            },
            &current,
        ));
    }

    #[test]
    fn cached_status_projection_rejects_sessionless_cache_even_with_state_marker() {
        let mut payload = projection();
        payload.session = CachedProjectionSession {
            session_id: None,
            worktree_environment_id: None,
        };
        payload.cache = ProjectionCacheContract {
            freshness_contract: Some(
                "state_marker_fresh_structural_cache_ok_for_read_only_operator_query",
            ),
            operator_markers: ProjectionCacheOperatorMarkers {
                task_snapshot_marker: true,
                dispatch_receipts_marker: true,
                continuation_bindings_marker: true,
                run_graph_updates_marker: true,
                runtime_consumption_snapshots_marker: true,
            },
        };

        assert!(!cached_status_projection_admissible(
            false,
            &payload,
            &current_session()
        ));
    }

    #[test]
    fn cached_status_projection_admissible_requires_status_surface_and_session_match() {
        assert!(cached_status_projection_admissible(
            false,
            &projection(),
            &current_session()
        ));

        let missing_status = CachedStatusProjection {
            has_status: false,
            ..projection()
        };
        assert!(!cached_status_projection_admissible(
            false,
            &missing_status,
            &current_session()
        ));

        let wrong_surface = CachedStatusProjection {
            surface: Some("vida doctor"),
            ..projection()
        };
        assert!(!cached_status_projection_admissible(
            false,
            &wrong_surface,
            &current_session()
        ));
    }

    #[test]
    fn read_only_operator_fallback_requires_all_operator_state_markers() {
        let full = ProjectionCacheContract {
            freshness_contract: Some("recent_bounded_stale_ok_for_read_only_operator_query"),
            operator_markers: ProjectionCacheOperatorMarkers {
                task_snapshot_marker: true,
                dispatch_receipts_marker: true,
                continuation_bindings_marker: true,
                run_graph_updates_marker: true,
                runtime_consumption_snapshots_marker: true,
            },
        };
        assert!(cached_projection_is_state_bound_read_only_operator_fallback(&full));

        let task_only = ProjectionCacheContract {
            operator_markers: ProjectionCacheOperatorMarkers {
                task_snapshot_marker: true,
                ..ProjectionCacheOperatorMarkers::default()
            },
            ..full
        };
        assert!(!cached_projection_is_state_bound_read_only_operator_fallback(&task_only));

        let unknown_freshness = ProjectionCacheContract {
            freshness_contract: Some("unknown"),
            ..full
        };
        assert!(!cached_projection_is_state_bound_read_only_operator_fallback(&unknown_freshness));
    }
}
