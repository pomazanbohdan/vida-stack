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
        match value {
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
        match value {
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
}

impl BlockerCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::MissingPacket => "missing_packet",
            Self::MissingLaneReceipt => "missing_lane_receipt",
            Self::OpenDelegatedCycle => "open_delegated_cycle",
            Self::ExceptionPathMissing => "exception_path_missing",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "missing_packet" => Some(Self::MissingPacket),
            "missing_lane_receipt" => Some(Self::MissingLaneReceipt),
            "open_delegated_cycle" => Some(Self::OpenDelegatedCycle),
            "exception_path_missing" => Some(Self::ExceptionPathMissing),
            _ => None,
        }
    }
}

pub(crate) fn canonical_blocker_code_str(value: &str) -> Option<&'static str> {
    BlockerCode::from_str(value).map(BlockerCode::as_str)
}

pub(crate) fn blocker_code_str(code: BlockerCode) -> &'static str {
    canonical_blocker_code_str(code.as_str()).unwrap_or(code.as_str())
}

pub(crate) fn blocker_code_value(code: BlockerCode) -> Option<String> {
    Some(blocker_code_str(code).to_string())
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
        blocker_code_str, blocker_code_value, canonical_compatibility_class_str,
        missing_downstream_lane_evidence_blocker, BlockerCode, CompatibilityClass, LaneStatus,
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
}
