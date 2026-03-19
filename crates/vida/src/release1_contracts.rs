use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneStatus {
    PacketReady,
    LaneOpen,
    LaneRunning,
    LaneBlocked,
    LaneCompleted,
    LaneSuperseded,
    LaneExceptionTakeover,
}

impl LaneStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PacketReady => "packet_ready",
            Self::LaneOpen => "lane_open",
            Self::LaneRunning => "lane_running",
            Self::LaneBlocked => "lane_blocked",
            Self::LaneCompleted => "lane_completed",
            Self::LaneSuperseded => "lane_superseded",
            Self::LaneExceptionTakeover => "lane_exception_takeover",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "packet_ready" => Some(Self::PacketReady),
            "lane_open" => Some(Self::LaneOpen),
            "lane_running" => Some(Self::LaneRunning),
            "lane_blocked" => Some(Self::LaneBlocked),
            "lane_completed" => Some(Self::LaneCompleted),
            "lane_superseded" => Some(Self::LaneSuperseded),
            "lane_exception_takeover" => Some(Self::LaneExceptionTakeover),
            _ => None,
        }
    }
}

pub(crate) fn canonical_lane_status_str(value: &str) -> Option<&'static str> {
    LaneStatus::from_str(value).map(LaneStatus::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatibilityClass {
    Compatible,
    Incompatible,
    Degraded,
    Blocking,
}

impl CompatibilityClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Degraded => "degraded",
            Self::Blocking => "blocking",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "compatible" => Some(Self::Compatible),
            "incompatible" => Some(Self::Incompatible),
            "degraded" => Some(Self::Degraded),
            "blocking" => Some(Self::Blocking),
            _ => None,
        }
    }
}

pub(crate) fn canonical_compatibility_class_str(value: &str) -> Option<&'static str> {
    CompatibilityClass::from_str(value).map(CompatibilityClass::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatibilityBoundary {
    Compatible,
    BlockingSupported,
    Unsupported,
}

pub(crate) fn classify_compatibility_boundary(value: &str) -> CompatibilityBoundary {
    match canonical_compatibility_class_str(value) {
        Some("compatible") => CompatibilityBoundary::Compatible,
        Some(_) => CompatibilityBoundary::BlockingSupported,
        None => CompatibilityBoundary::Unsupported,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1ContractType {
    OperatorContracts,
    SharedFields,
}

impl Release1ContractType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::OperatorContracts => "release-1-operator-contracts",
            Self::SharedFields => "release-1-shared-fields",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "release-1-operator-contracts" => Some(Self::OperatorContracts),
            "release-1-shared-fields" => Some(Self::SharedFields),
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_contract_type_str(value: &str) -> Option<&'static str> {
    Release1ContractType::from_str(value).map(Release1ContractType::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1SchemaVersion {
    V1,
}

impl Release1SchemaVersion {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "release-1-v1",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "release-1-v1" => Some(Self::V1),
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_schema_version_str(value: &str) -> Option<&'static str> {
    Release1SchemaVersion::from_str(value).map(Release1SchemaVersion::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release1ContractStatus {
    Pass,
    Blocked,
}

impl Release1ContractStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Blocked => "blocked",
        }
    }

    pub(crate) const fn from_bool(ok: bool) -> Self {
        if ok {
            Self::Pass
        } else {
            Self::Blocked
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            value if value.eq_ignore_ascii_case("pass") || value.eq_ignore_ascii_case("ok") => {
                Some(Self::Pass)
            }
            value
                if value.eq_ignore_ascii_case("blocked") || value.eq_ignore_ascii_case("block") =>
            {
                Some(Self::Blocked)
            }
            _ => None,
        }
    }
}

pub(crate) fn canonical_release1_contract_status_str(value: &str) -> Option<&'static str> {
    Release1ContractStatus::from_str(value).map(Release1ContractStatus::as_str)
}

pub(crate) fn release1_contract_status_str(ok: bool) -> &'static str {
    Release1ContractStatus::from_bool(ok).as_str()
}

pub(crate) fn has_evidence_id(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub(crate) fn derive_lane_status(
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> LaneStatus {
    if has_evidence_id(exception_path_receipt_id) {
        return LaneStatus::LaneExceptionTakeover;
    }
    if has_evidence_id(supersedes_receipt_id) {
        return LaneStatus::LaneSuperseded;
    }
    match dispatch_status {
        "packet_ready" => LaneStatus::PacketReady,
        "routed" => LaneStatus::LaneOpen,
        "executed" => LaneStatus::LaneRunning,
        "blocked" => LaneStatus::LaneBlocked,
        _ => LaneStatus::LaneOpen,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockerCode {
    MissingPacket,
    MissingLaneReceipt,
    OpenDelegatedCycle,
    ExceptionPathMissing,
    MissingProtocolBindingReceipt,
    ProtocolBindingNotRuntimeReady,
    Unsupported,
}

impl BlockerCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::MissingPacket => "missing_packet",
            Self::MissingLaneReceipt => "missing_lane_receipt",
            Self::OpenDelegatedCycle => "open_delegated_cycle",
            Self::ExceptionPathMissing => "exception_path_missing",
            Self::MissingProtocolBindingReceipt => "missing_protocol_binding_receipt",
            Self::ProtocolBindingNotRuntimeReady => "protocol_binding_not_runtime_ready",
            Self::Unsupported => "unsupported_blocker_code",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "missing_packet" => Some(Self::MissingPacket),
            "missing_lane_receipt" => Some(Self::MissingLaneReceipt),
            "open_delegated_cycle" => Some(Self::OpenDelegatedCycle),
            "exception_path_missing" => Some(Self::ExceptionPathMissing),
            "missing_protocol_binding_receipt" => Some(Self::MissingProtocolBindingReceipt),
            "protocol_binding_not_runtime_ready" => Some(Self::ProtocolBindingNotRuntimeReady),
            "unsupported_blocker_code" => Some(Self::Unsupported),
            _ => None,
        }
    }
}

pub(crate) fn canonical_blocker_code_str(value: &str) -> Option<&'static str> {
    BlockerCode::from_str(value).map(BlockerCode::as_str)
}

pub(crate) fn canonical_blocker_code_list<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .filter_map(|value| canonical_blocker_code_str(value.as_ref()))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn blocker_code_str(code: BlockerCode) -> &'static str {
    canonical_blocker_code_str(code.as_str()).unwrap_or(code.as_str())
}

pub(crate) fn blocker_code_value(code: BlockerCode) -> Option<String> {
    canonical_blocker_code_list([code.as_str()])
        .into_iter()
        .next()
}

struct DecisionGateRule {
    gate_id: &'static str,
    missing_receipt_blocker: BlockerCode,
    not_ready_blocker: BlockerCode,
}

const DECISION_GATE_TABLE: &[DecisionGateRule] = &[DecisionGateRule {
    gate_id: "retrieval_evidence",
    missing_receipt_blocker: BlockerCode::MissingProtocolBindingReceipt,
    not_ready_blocker: BlockerCode::ProtocolBindingNotRuntimeReady,
}];

pub(crate) fn evaluate_policy_gate_protocol_binding(
    gate_id: &str,
    protocol_binding_receipt_id: Option<&str>,
    runtime_ready: bool,
) -> Option<BlockerCode> {
    let Some(rule) = DECISION_GATE_TABLE
        .iter()
        .find(|rule| rule.gate_id == gate_id.trim())
    else {
        return Some(BlockerCode::Unsupported);
    };

    if !has_evidence_id(protocol_binding_receipt_id) {
        return Some(rule.missing_receipt_blocker);
    }
    if !runtime_ready {
        return Some(rule.not_ready_blocker);
    }
    None
}

pub(crate) fn missing_downstream_lane_evidence_blocker(
    parsed_downstream_lane_status: Option<LaneStatus>,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> Option<BlockerCode> {
    if matches!(
        parsed_downstream_lane_status,
        Some(LaneStatus::LaneExceptionTakeover)
    ) && !has_evidence_id(exception_path_receipt_id)
    {
        return Some(BlockerCode::ExceptionPathMissing);
    }
    if matches!(
        parsed_downstream_lane_status,
        Some(LaneStatus::LaneSuperseded)
    ) && !has_evidence_id(supersedes_receipt_id)
    {
        return Some(BlockerCode::MissingLaneReceipt);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        blocker_code_str, blocker_code_value, canonical_blocker_code_list,
        canonical_compatibility_class_str, canonical_release1_contract_status_str,
        canonical_release1_contract_type_str, canonical_release1_schema_version_str,
        classify_compatibility_boundary, evaluate_policy_gate_protocol_binding,
        missing_downstream_lane_evidence_blocker, release1_contract_status_str, BlockerCode,
        CompatibilityBoundary, CompatibilityClass, LaneStatus, Release1ContractStatus,
        Release1ContractType, Release1SchemaVersion,
    };

    #[test]
    fn blocker_code_normalization_round_trips_to_canonical_values() {
        assert_eq!(
            blocker_code_str(BlockerCode::MissingPacket),
            "missing_packet"
        );
        assert_eq!(
            blocker_code_value(BlockerCode::MissingLaneReceipt),
            Some("missing_lane_receipt".to_string())
        );
    }

    #[test]
    fn lane_exception_takeover_requires_exception_receipt_evidence() {
        let blocker = missing_downstream_lane_evidence_blocker(
            Some(LaneStatus::LaneExceptionTakeover),
            None,
            None,
        );
        assert_eq!(blocker, Some(BlockerCode::ExceptionPathMissing));
    }

    #[test]
    fn lane_superseded_requires_supersedes_receipt_evidence() {
        let blocker =
            missing_downstream_lane_evidence_blocker(Some(LaneStatus::LaneSuperseded), None, None);
        assert_eq!(blocker, Some(BlockerCode::MissingLaneReceipt));
    }

    #[test]
    fn compatibility_class_round_trips_to_canonical_values() {
        assert_eq!(CompatibilityClass::Compatible.as_str(), "compatible");
        assert_eq!(CompatibilityClass::Incompatible.as_str(), "incompatible");
        assert_eq!(
            canonical_compatibility_class_str("degraded"),
            Some("degraded")
        );
        assert_eq!(
            canonical_compatibility_class_str("blocking"),
            Some("blocking")
        );
    }

    #[test]
    fn compatibility_class_rejects_unknown_values() {
        assert_eq!(canonical_compatibility_class_str("COMPATIBLE"), None);
        assert_eq!(canonical_compatibility_class_str("unknown"), None);
    }

    #[test]
    fn lane_status_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            super::canonical_lane_status_str(" lane_running "),
            Some("lane_running")
        );
    }

    #[test]
    fn blocker_code_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            super::canonical_blocker_code_str(" missing_packet "),
            Some("missing_packet")
        );
    }

    #[test]
    fn canonical_blocker_code_list_dedupes_and_sorts() {
        let codes = canonical_blocker_code_list([
            " missing_lane_receipt ",
            "open_delegated_cycle",
            "missing_lane_receipt",
            "exception_path_missing",
            "protocol_binding_not_runtime_ready",
            "open_delegated_cycle",
        ]);
        assert_eq!(
            codes,
            vec![
                "exception_path_missing".to_string(),
                "missing_lane_receipt".to_string(),
                "open_delegated_cycle".to_string(),
                "protocol_binding_not_runtime_ready".to_string()
            ]
        );
    }

    #[test]
    fn canonical_blocker_code_list_ignores_unknown_and_empty_values() {
        let codes = canonical_blocker_code_list([
            "missing_packet",
            "",
            " ",
            "MISSING_PACKET",
            "not_a_real_code",
            " missing_packet ",
        ]);
        assert_eq!(codes, vec!["missing_packet".to_string()]);
    }

    #[test]
    fn blocker_code_normalization_supports_consume_bundle_protocol_binding_codes() {
        let codes = canonical_blocker_code_list([
            " missing_protocol_binding_receipt ",
            "protocol_binding_not_runtime_ready",
            "unsupported_blocker_code",
        ]);
        assert_eq!(
            codes,
            vec![
                "missing_protocol_binding_receipt".to_string(),
                "protocol_binding_not_runtime_ready".to_string(),
                "unsupported_blocker_code".to_string()
            ]
        );
    }

    #[test]
    fn compatibility_class_canonicalization_trims_surrounding_whitespace() {
        assert_eq!(
            canonical_compatibility_class_str(" compatible "),
            Some("compatible")
        );
    }

    #[test]
    fn compatibility_boundary_classifier_fails_closed_for_unknown_values() {
        assert_eq!(
            classify_compatibility_boundary("compatible"),
            CompatibilityBoundary::Compatible
        );
        assert_eq!(
            classify_compatibility_boundary("degraded"),
            CompatibilityBoundary::BlockingSupported
        );
        assert_eq!(
            classify_compatibility_boundary("unsupported_value"),
            CompatibilityBoundary::Unsupported
        );
    }

    #[test]
    fn release1_contract_type_round_trips_to_canonical_values() {
        assert_eq!(
            Release1ContractType::OperatorContracts.as_str(),
            "release-1-operator-contracts"
        );
        assert_eq!(
            Release1ContractType::SharedFields.as_str(),
            "release-1-shared-fields"
        );
        assert_eq!(
            canonical_release1_contract_type_str(" release-1-operator-contracts "),
            Some("release-1-operator-contracts")
        );
        assert_eq!(
            canonical_release1_contract_type_str(" release-1-shared-fields "),
            Some("release-1-shared-fields")
        );
        assert_eq!(
            canonical_release1_contract_type_str("unknown-contract"),
            None
        );
    }

    #[test]
    fn release1_schema_version_round_trips_to_canonical_values() {
        assert_eq!(Release1SchemaVersion::V1.as_str(), "release-1-v1");
        assert_eq!(
            canonical_release1_schema_version_str(" release-1-v1 "),
            Some("release-1-v1")
        );
        assert_eq!(canonical_release1_schema_version_str("release-2-v1"), None);
    }

    #[test]
    fn release1_contract_status_round_trips_to_canonical_values() {
        assert_eq!(Release1ContractStatus::Pass.as_str(), "pass");
        assert_eq!(Release1ContractStatus::Blocked.as_str(), "blocked");
        assert_eq!(
            canonical_release1_contract_status_str(" pass "),
            Some("pass")
        );
        assert_eq!(
            canonical_release1_contract_status_str(" blocked "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str(" ok "), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str(" BLOCK "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("unknown"), None);
    }

    #[test]
    fn release1_contract_status_emission_maps_bool_to_canonical_values() {
        assert_eq!(release1_contract_status_str(true), "pass");
        assert_eq!(release1_contract_status_str(false), "blocked");
    }

    #[test]
    fn retrieval_decision_gate_blocks_when_receipt_or_runtime_readiness_missing() {
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", None, false),
            Some(BlockerCode::MissingProtocolBindingReceipt)
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), false),
            Some(BlockerCode::ProtocolBindingNotRuntimeReady)
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), true),
            None
        );
    }

    #[test]
    fn decision_gate_fails_closed_for_unknown_gate_id() {
        assert_eq!(
            evaluate_policy_gate_protocol_binding("unknown_gate", Some("pb-1"), true),
            Some(BlockerCode::Unsupported)
        );
    }
}
