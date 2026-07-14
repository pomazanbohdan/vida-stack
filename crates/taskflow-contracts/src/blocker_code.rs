use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerCode {
    AgentInitOrchestratorRoleForbidden,
    AgentInitRoleUnresolved,
    TaskflowConsumeBundleTimeout,
    HostBridgeRequestMissingFields,
    HostBridgeRequestWrongTransport,
    HostBridgeRequestNotPending,
    HostBridgeReceiptModeMismatch,
    HostBridgeRequestIdentityMismatch,
    HostToolCapabilityMissing,
    HostAgentCapacityUnavailable,
    HostAgentIdMissing,
    HostBridgeCompletionArgsInvalid,
    HostBridgeRequestUnreadable,
    HostBridgeStateRootMissing,
    HostBridgeRequestUntrustedPath,
    HostBridgeRequestPathMissing,
    HostBridgeRequestPathMismatch,
    HostBridgePacketPathUnbounded,
    HostBridgeResultPathUnbounded,
    HostBridgeReceiptPathUnbounded,
    AuthoritativeStateStoreLocked,
    AuthoritativeStateStoreOpenFailed,
    HostBridgeDispatchReceiptMissing,
    HostBridgeDispatchReceiptInactive,
    HostBridgeDispatchReceiptMismatch,
    ImplementationArtifactsMissing,
    ImplementationArtifactAuthorityMissing,
    ImplementationArtifactChangedFilesMissing,
    ImplementationArtifactAuthorityInvalid,
    ImplementationArtifactContractInvalid,
    ImplementationArtifactReceiptMissing,
    ImplementationArtifactReceiptUnverified,
    ImplementationAttemptScopeGuardViolation,
    TimeoutWithoutTakeoverAuthority,
    AgentInitExecuteDispatchMissingPacket,
    InternalDispatchTimeoutWithoutReceipt,
    InternalCodexCarrierUnavailable,
    SelectedLaneAssignmentGuardRequired,
    SelectedLaneRuntimeAssignmentTruthRequired,
    SelectedModelProfileOverBudget,
    SelectedExternalBackendNotReady,
    ActiveCarrierPolicyMismatch,
    CarrierPolicyReselectionRequired,
    BlockedDispatch,
    AutoDispatchPacketActiveUnitMissing,
    AutoDispatchPacketActiveUnitMismatch,
    AutoDispatchPacketActiveUnitAmbiguous,
    AutoDispatchPacketActiveUnitUnavailable,
    AutoDispatchPacketActiveUnitPacketMissing,
}

impl BlockerCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AgentInitOrchestratorRoleForbidden => "agent_init_orchestrator_role_forbidden",
            Self::AgentInitRoleUnresolved => "agent_init_role_unresolved",
            Self::TaskflowConsumeBundleTimeout => "taskflow_consume_bundle_timeout",
            Self::HostBridgeRequestMissingFields => "host_bridge_request_missing_fields",
            Self::HostBridgeRequestWrongTransport => "host_bridge_request_wrong_transport",
            Self::HostBridgeRequestNotPending => "host_bridge_request_not_pending",
            Self::HostBridgeReceiptModeMismatch => "host_bridge_receipt_mode_mismatch",
            Self::HostBridgeRequestIdentityMismatch => "host_bridge_request_identity_mismatch",
            Self::HostToolCapabilityMissing => "host_tool_capability_missing",
            Self::HostAgentCapacityUnavailable => "host_agent_capacity_unavailable",
            Self::HostAgentIdMissing => "host_agent_id_missing",
            Self::HostBridgeCompletionArgsInvalid => "host_bridge_completion_args_invalid",
            Self::HostBridgeRequestUnreadable => "host_bridge_request_unreadable",
            Self::HostBridgeStateRootMissing => "host_bridge_state_root_missing",
            Self::HostBridgeRequestUntrustedPath => "host_bridge_request_untrusted_path",
            Self::HostBridgeRequestPathMissing => "host_bridge_request_path_missing",
            Self::HostBridgeRequestPathMismatch => "host_bridge_request_path_mismatch",
            Self::HostBridgePacketPathUnbounded => "host_bridge_packet_path_unbounded",
            Self::HostBridgeResultPathUnbounded => "host_bridge_result_path_unbounded",
            Self::HostBridgeReceiptPathUnbounded => "host_bridge_receipt_path_unbounded",
            Self::AuthoritativeStateStoreLocked => "authoritative_state_store_locked",
            Self::AuthoritativeStateStoreOpenFailed => "authoritative_state_store_open_failed",
            Self::HostBridgeDispatchReceiptMissing => "host_bridge_dispatch_receipt_missing",
            Self::HostBridgeDispatchReceiptInactive => "host_bridge_dispatch_receipt_inactive",
            Self::HostBridgeDispatchReceiptMismatch => "host_bridge_dispatch_receipt_mismatch",
            Self::ImplementationArtifactsMissing => "implementation_artifacts_missing",
            Self::ImplementationArtifactAuthorityMissing => {
                "implementation_artifact_authority_missing"
            }
            Self::ImplementationArtifactChangedFilesMissing => {
                "implementation_artifact_changed_files_missing"
            }
            Self::ImplementationArtifactAuthorityInvalid => {
                "implementation_artifact_authority_invalid"
            }
            Self::ImplementationArtifactContractInvalid => {
                "implementation_artifact_contract_invalid"
            }
            Self::ImplementationArtifactReceiptMissing => "implementation_artifact_receipt_missing",
            Self::ImplementationArtifactReceiptUnverified => {
                "implementation_artifact_receipt_unverified"
            }
            Self::ImplementationAttemptScopeGuardViolation => {
                "implementation_attempt_scope_guard_violation"
            }
            Self::TimeoutWithoutTakeoverAuthority => "timeout_without_takeover_authority",
            Self::AgentInitExecuteDispatchMissingPacket => {
                "agent_init_execute_dispatch_missing_packet"
            }
            Self::InternalDispatchTimeoutWithoutReceipt => {
                "internal_dispatch_timeout_without_receipt"
            }
            Self::InternalCodexCarrierUnavailable => "internal_codex_carrier_unavailable",
            Self::SelectedLaneAssignmentGuardRequired => "selected_lane_assignment_guard_required",
            Self::SelectedLaneRuntimeAssignmentTruthRequired => {
                "selected_lane_runtime_assignment_truth_required"
            }
            Self::SelectedModelProfileOverBudget => "selected_model_profile_over_budget",
            Self::SelectedExternalBackendNotReady => "selected_external_backend_not_ready",
            Self::ActiveCarrierPolicyMismatch => "active_carrier_policy_mismatch",
            Self::CarrierPolicyReselectionRequired => "carrier_policy_reselection_required",
            Self::BlockedDispatch => "blocked_dispatch",
            Self::AutoDispatchPacketActiveUnitMissing => "auto_dispatch_packet_active_unit_missing",
            Self::AutoDispatchPacketActiveUnitMismatch => {
                "auto_dispatch_packet_active_unit_mismatch"
            }
            Self::AutoDispatchPacketActiveUnitAmbiguous => {
                "auto_dispatch_packet_active_unit_ambiguous"
            }
            Self::AutoDispatchPacketActiveUnitUnavailable => {
                "auto_dispatch_packet_active_unit_unavailable"
            }
            Self::AutoDispatchPacketActiveUnitPacketMissing => {
                "auto_dispatch_packet_active_unit_packet_missing"
            }
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::AgentInitOrchestratorRoleForbidden,
            Self::AgentInitRoleUnresolved,
            Self::TaskflowConsumeBundleTimeout,
            Self::HostBridgeRequestMissingFields,
            Self::HostBridgeRequestWrongTransport,
            Self::HostBridgeRequestNotPending,
            Self::HostBridgeReceiptModeMismatch,
            Self::HostBridgeRequestIdentityMismatch,
            Self::HostToolCapabilityMissing,
            Self::HostAgentCapacityUnavailable,
            Self::HostAgentIdMissing,
            Self::HostBridgeCompletionArgsInvalid,
            Self::HostBridgeRequestUnreadable,
            Self::HostBridgeStateRootMissing,
            Self::HostBridgeRequestUntrustedPath,
            Self::HostBridgeRequestPathMissing,
            Self::HostBridgeRequestPathMismatch,
            Self::HostBridgePacketPathUnbounded,
            Self::HostBridgeResultPathUnbounded,
            Self::HostBridgeReceiptPathUnbounded,
            Self::AuthoritativeStateStoreLocked,
            Self::AuthoritativeStateStoreOpenFailed,
            Self::HostBridgeDispatchReceiptMissing,
            Self::HostBridgeDispatchReceiptInactive,
            Self::HostBridgeDispatchReceiptMismatch,
            Self::ImplementationArtifactsMissing,
            Self::ImplementationArtifactAuthorityMissing,
            Self::ImplementationArtifactChangedFilesMissing,
            Self::ImplementationArtifactAuthorityInvalid,
            Self::ImplementationArtifactContractInvalid,
            Self::ImplementationArtifactReceiptMissing,
            Self::ImplementationArtifactReceiptUnverified,
            Self::ImplementationAttemptScopeGuardViolation,
            Self::TimeoutWithoutTakeoverAuthority,
            Self::AgentInitExecuteDispatchMissingPacket,
            Self::InternalDispatchTimeoutWithoutReceipt,
            Self::InternalCodexCarrierUnavailable,
            Self::SelectedLaneAssignmentGuardRequired,
            Self::SelectedLaneRuntimeAssignmentTruthRequired,
            Self::SelectedModelProfileOverBudget,
            Self::SelectedExternalBackendNotReady,
            Self::ActiveCarrierPolicyMismatch,
            Self::CarrierPolicyReselectionRequired,
            Self::BlockedDispatch,
            Self::AutoDispatchPacketActiveUnitMissing,
            Self::AutoDispatchPacketActiveUnitMismatch,
            Self::AutoDispatchPacketActiveUnitAmbiguous,
            Self::AutoDispatchPacketActiveUnitUnavailable,
            Self::AutoDispatchPacketActiveUnitPacketMissing,
        ]
    }
}

impl TryFrom<&str> for BlockerCode {
    type Error = UnknownBlockerCode;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::all()
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
            .ok_or_else(|| UnknownBlockerCode {
                value: value.to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBlockerCode {
    pub value: String,
}

impl std::fmt::Display for UnknownBlockerCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown blocker code `{}`", self.value)
    }
}

impl std::error::Error for UnknownBlockerCode {}

#[must_use]
pub fn canonical_blocker_code_str(value: &str) -> Option<&'static str> {
    BlockerCode::try_from(value).map(BlockerCode::as_str).ok()
}

#[must_use]
pub fn canonical_blocker_code_value_from_str(value: &str) -> Option<String> {
    canonical_blocker_code_str(value)
        .map(str::to_string)
        .or_else(|| canonical_parametric_blocker_code_value(value))
}

#[must_use]
pub fn canonical_blocker_code_list<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .filter_map(|value| canonical_blocker_code_value_from_str(value.as_ref()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[must_use]
pub fn blocker_code_list_preserving_legacy<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .filter_map(|value| {
            let trimmed = value.as_ref().trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(
                    canonical_blocker_code_value_from_str(trimmed)
                        .unwrap_or_else(|| trimmed.to_string()),
                )
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_parametric_blocker_code_value(value: &str) -> Option<String> {
    if is_selected_lane_runtime_assignment_truth_missing(value)
        || is_selected_lane_assignment_guard_blocked(value)
    {
        Some(value.to_string())
    } else {
        None
    }
}

#[must_use]
pub fn selected_lane_runtime_assignment_truth_missing(task_id: &str, reason: &str) -> String {
    format!("selected_lane_runtime_assignment_truth_missing:task={task_id}:{reason}")
}

#[must_use]
pub fn selected_lane_assignment_guard_blocked(task_id: &str, blocker: &str) -> String {
    format!("selected_lane_assignment_guard_blocked:task={task_id}:{blocker}")
}

#[must_use]
pub fn is_selected_lane_runtime_assignment_truth_missing(value: &str) -> bool {
    value.starts_with("selected_lane_runtime_assignment_truth_missing:task=")
}

#[must_use]
pub fn is_selected_lane_assignment_guard_blocked(value: &str) -> bool {
    value.starts_with("selected_lane_assignment_guard_blocked:task=")
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_blocker_code_list, canonical_blocker_code_str,
        canonical_blocker_code_value_from_str, is_selected_lane_assignment_guard_blocked,
        is_selected_lane_runtime_assignment_truth_missing, selected_lane_assignment_guard_blocked,
        selected_lane_runtime_assignment_truth_missing, BlockerCode,
    };
    use std::collections::BTreeSet;

    #[test]
    fn blocker_code_round_trips_canonical_strings() {
        for code in BlockerCode::all() {
            assert_eq!(
                canonical_blocker_code_str(code.as_str()),
                Some(code.as_str())
            );
            assert_eq!(
                BlockerCode::try_from(code.as_str()).map(BlockerCode::as_str),
                Ok(code.as_str())
            );
        }
    }

    #[test]
    fn carrier_policy_blocker_codes_are_registry_backed() {
        let codes = [
            BlockerCode::ActiveCarrierPolicyMismatch,
            BlockerCode::CarrierPolicyReselectionRequired,
        ];

        for code in codes {
            assert_eq!(
                canonical_blocker_code_str(code.as_str()),
                Some(code.as_str())
            );
            assert_eq!(BlockerCode::try_from(code.as_str()), Ok(code));
        }

        assert_eq!(
            canonical_blocker_code_list(codes.iter().map(|code| code.as_str())),
            vec![
                "active_carrier_policy_mismatch".to_string(),
                "carrier_policy_reselection_required".to_string(),
            ]
        );
    }

    #[test]
    fn blocker_code_list_dedupes_and_sorts() {
        let codes = canonical_blocker_code_list([
            "host_tool_capability_missing",
            "agent_init_role_unresolved",
            "host_tool_capability_missing",
        ]);
        assert_eq!(
            codes,
            vec![
                "agent_init_role_unresolved".to_string(),
                "host_tool_capability_missing".to_string()
            ]
        );
    }

    #[test]
    fn blocker_code_dynamic_lane_assignment_codes_stay_canonical() {
        let values = [
            selected_lane_runtime_assignment_truth_missing("task-a", "missing"),
            selected_lane_assignment_guard_blocked("task-b", "blocked"),
        ];
        let canonical = canonical_blocker_code_list(values.iter().map(String::as_str))
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            canonical,
            values
                .iter()
                .map(String::as_str)
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            canonical_blocker_code_value_from_str(&values[0]),
            Some(values[0].clone())
        );
        assert!(is_selected_lane_runtime_assignment_truth_missing(
            &values[0]
        ));
        assert!(is_selected_lane_assignment_guard_blocked(&values[1]));
    }

    #[test]
    fn blocker_code_rejects_unknown_strings() {
        assert!(canonical_blocker_code_value_from_str("unknown_blocker").is_none());
    }

    #[test]
    fn blocker_code_rejects_case_and_whitespace_drift() {
        assert!(canonical_blocker_code_str("HOST_TOOL_CAPABILITY_MISSING").is_none());
        assert!(canonical_blocker_code_str(" host_tool_capability_missing ").is_none());
        assert!(
            canonical_blocker_code_value_from_str(
                " selected_lane_runtime_assignment_truth_missing:task=task-a:missing "
            )
            .is_none()
        );
        assert!(BlockerCode::try_from(" host_tool_capability_missing ").is_err());
    }

    #[test]
    fn blocker_code_legacy_preserving_list_keeps_unknown_values() {
        let codes = super::blocker_code_list_preserving_legacy([
            " host_tool_capability_missing ",
            "legacy_runtime_gate",
            "legacy_runtime_gate",
            "",
        ]);
        assert_eq!(
            codes,
            vec![
                "host_tool_capability_missing".to_string(),
                "legacy_runtime_gate".to_string()
            ]
        );
    }
}
