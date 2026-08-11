pub const MODULE: &str = "run_graph_evidence";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphReworkEvidence {
    pub allowed_next_node: String,
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphBlockedSourceLane {
    pub dispatch_target: String,
    pub blocker_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphDownstreamPacketEvidence {
    pub source_dispatch_target: String,
    pub source_dispatch_status: String,
    pub source_blocker_code: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGraphCompletionEvidence {
    pub dispatch_target: String,
    pub dispatch_status: String,
    pub blocker_code: Option<String>,
    pub rework: Option<RunGraphReworkEvidence>,
    pub source_lane: Option<RunGraphDownstreamPacketEvidence>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_blockers: Vec<String>,
}

#[must_use]
pub fn normalize_run_graph_node(value: &str) -> String {
    value.trim().replace('-', "_")
}

#[must_use]
pub fn blocked_source_lane_from_packet_evidence(
    receipt_dispatch_target: &str,
    receipt_dispatch_status: &str,
    packet: RunGraphDownstreamPacketEvidence,
) -> Option<RunGraphBlockedSourceLane> {
    if !matches!(
        receipt_dispatch_status,
        "bridge_request_pending" | "blocked"
    ) {
        return None;
    }
    let source_dispatch_target = packet.source_dispatch_target.trim();
    if source_dispatch_target.is_empty() || source_dispatch_target == receipt_dispatch_target.trim()
    {
        return None;
    }
    let source_terminal =
        packet.source_dispatch_status == "executed" && packet.source_blocker_code.is_none();
    if source_terminal
        && packet.downstream_dispatch_ready
        && packet.downstream_dispatch_blockers.is_empty()
    {
        return None;
    }
    Some(RunGraphBlockedSourceLane {
        dispatch_target: source_dispatch_target.to_string(),
        blocker_code: packet.source_blocker_code,
    })
}

#[must_use]
pub fn rework_route_from_completion_evidence(
    evidence: &RunGraphCompletionEvidence,
) -> Option<RunGraphReworkEvidence> {
    evidence.rework.clone()
}

#[must_use]
pub fn downstream_handoff_ready_from_completion_evidence(
    evidence: &RunGraphCompletionEvidence,
) -> bool {
    evidence.dispatch_status == "executed"
        && evidence.blocker_code.as_deref().is_none_or(str::is_empty)
        && evidence.downstream_dispatch_ready
        && evidence.downstream_dispatch_blockers.is_empty()
        && evidence
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        RunGraphCompletionEvidence, RunGraphDownstreamPacketEvidence, RunGraphReworkEvidence,
        blocked_source_lane_from_packet_evidence,
        downstream_handoff_ready_from_completion_evidence, normalize_run_graph_node,
        rework_route_from_completion_evidence,
    };

    #[test]
    fn blocked_source_lane_uses_packet_facts_without_runtime_paths() {
        let blocked = blocked_source_lane_from_packet_evidence(
            "developer",
            "blocked",
            RunGraphDownstreamPacketEvidence {
                source_dispatch_target: "analysis".to_string(),
                source_dispatch_status: "blocked".to_string(),
                source_blocker_code: Some("missing_owned_write_scope".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["missing_owned_write_scope".to_string()],
            },
        )
        .expect("blocked source lane should be detected");

        assert_eq!(blocked.dispatch_target, "analysis");
        assert_eq!(
            blocked.blocker_code,
            Some("missing_owned_write_scope".to_string())
        );
    }

    #[test]
    fn terminal_source_lane_with_ready_downstream_is_not_blocked() {
        assert!(
            blocked_source_lane_from_packet_evidence(
                "developer",
                "blocked",
                RunGraphDownstreamPacketEvidence {
                    source_dispatch_target: "analysis".to_string(),
                    source_dispatch_status: "executed".to_string(),
                    source_blocker_code: None,
                    downstream_dispatch_ready: true,
                    downstream_dispatch_blockers: Vec::new(),
                },
            )
            .is_none()
        );
    }

    #[test]
    fn completion_evidence_carries_rework_and_downstream_facts() {
        let evidence = RunGraphCompletionEvidence {
            dispatch_target: "developer".to_string(),
            dispatch_status: "executed".to_string(),
            blocker_code: None,
            rework: Some(RunGraphReworkEvidence {
                allowed_next_node: "analysis".to_string(),
                blocker_code: Some("review_rework_required".to_string()),
            }),
            source_lane: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_target: Some("tester".to_string()),
            downstream_dispatch_blockers: Vec::new(),
        };

        assert_eq!(
            rework_route_from_completion_evidence(&evidence)
                .expect("rework evidence should be carried")
                .allowed_next_node,
            "analysis"
        );
        assert!(downstream_handoff_ready_from_completion_evidence(&evidence));
        assert_eq!(normalize_run_graph_node("work-pool"), "work_pool");
    }

    #[test]
    fn blocked_source_lane_requires_distinct_source_and_preserves_bridge_pending() {
        let packet = RunGraphDownstreamPacketEvidence {
            source_dispatch_target: "analysis".to_string(),
            source_dispatch_status: "executed".to_string(),
            source_blocker_code: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review".to_string()],
        };

        let pending = blocked_source_lane_from_packet_evidence(
            "developer",
            "bridge_request_pending",
            packet.clone(),
        )
        .expect("pending bridge request should preserve blocked source evidence");
        assert_eq!(pending.dispatch_target, "analysis");
        assert_eq!(pending.blocker_code, None);

        assert!(
            blocked_source_lane_from_packet_evidence("analysis", "blocked", packet.clone())
                .is_none(),
            "same source and receipt target must not self-report as blocked"
        );

        let mut empty_source = packet;
        empty_source.source_dispatch_target = "  ".to_string();
        assert!(
            blocked_source_lane_from_packet_evidence("developer", "blocked", empty_source)
                .is_none(),
            "empty source target must fail closed"
        );
    }

    #[test]
    fn downstream_handoff_requires_executed_clean_ready_target() {
        let mut evidence = RunGraphCompletionEvidence {
            dispatch_target: "developer".to_string(),
            dispatch_status: "executed".to_string(),
            blocker_code: None,
            rework: None,
            source_lane: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_target: Some("tester".to_string()),
            downstream_dispatch_blockers: Vec::new(),
        };
        assert!(downstream_handoff_ready_from_completion_evidence(&evidence));

        evidence.dispatch_status = "blocked".to_string();
        assert!(!downstream_handoff_ready_from_completion_evidence(
            &evidence
        ));
        evidence.dispatch_status = "executed".to_string();
        evidence.blocker_code = Some("review_required".to_string());
        assert!(!downstream_handoff_ready_from_completion_evidence(
            &evidence
        ));
        evidence.blocker_code = None;
        evidence.downstream_dispatch_ready = false;
        assert!(!downstream_handoff_ready_from_completion_evidence(
            &evidence
        ));
        evidence.downstream_dispatch_ready = true;
        evidence.downstream_dispatch_target = Some("  ".to_string());
        assert!(!downstream_handoff_ready_from_completion_evidence(
            &evidence
        ));
        evidence.downstream_dispatch_target = Some("tester".to_string());
        evidence.downstream_dispatch_blockers = vec!["pending_review".to_string()];
        assert!(!downstream_handoff_ready_from_completion_evidence(
            &evidence
        ));
    }
}
