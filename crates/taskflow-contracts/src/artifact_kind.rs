use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    DispatchPacket,
    DispatchResult,
    DownstreamDispatchPacket,
    DownstreamDispatchResult,
    HostToolBridgeRequest,
    HostToolBridgeReceipt,
    ExceptionPathMetadata,
    ExecutionEvidence,
    ClosureReceipt,
}

impl ArtifactKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DispatchPacket => "dispatch_packet",
            Self::DispatchResult => "dispatch_result",
            Self::DownstreamDispatchPacket => "downstream_dispatch_packet",
            Self::DownstreamDispatchResult => "downstream_dispatch_result",
            Self::HostToolBridgeRequest => "host_tool_bridge_request",
            Self::HostToolBridgeReceipt => "host_tool_bridge_receipt",
            Self::ExceptionPathMetadata => "exception_path_metadata",
            Self::ExecutionEvidence => "execution_evidence",
            Self::ClosureReceipt => "closure_receipt",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::DispatchPacket,
            Self::DispatchResult,
            Self::DownstreamDispatchPacket,
            Self::DownstreamDispatchResult,
            Self::HostToolBridgeRequest,
            Self::HostToolBridgeReceipt,
            Self::ExceptionPathMetadata,
            Self::ExecutionEvidence,
            Self::ClosureReceipt,
        ]
    }
}

impl TryFrom<&str> for ArtifactKind {
    type Error = UnknownArtifactKind;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "dispatch_packet" => Ok(Self::DispatchPacket),
            "dispatch_result" => Ok(Self::DispatchResult),
            "downstream_dispatch_packet" => Ok(Self::DownstreamDispatchPacket),
            "downstream_dispatch_result" => Ok(Self::DownstreamDispatchResult),
            "host_tool_bridge_request" => Ok(Self::HostToolBridgeRequest),
            "host_tool_bridge_receipt" => Ok(Self::HostToolBridgeReceipt),
            "exception_path_metadata" => Ok(Self::ExceptionPathMetadata),
            "execution_evidence" => Ok(Self::ExecutionEvidence),
            "closure_receipt" => Ok(Self::ClosureReceipt),
            _ => Err(UnknownArtifactKind {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownArtifactKind {
    pub value: String,
}

impl std::fmt::Display for UnknownArtifactKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown artifact kind `{}`", self.value)
    }
}

impl std::error::Error for UnknownArtifactKind {}

#[cfg(test)]
mod tests {
    use super::ArtifactKind;

    #[test]
    fn artifact_kind_round_trips_canonical_strings() {
        for kind in ArtifactKind::all() {
            assert_eq!(ArtifactKind::try_from(kind.as_str()), Ok(*kind));
        }
    }

    #[test]
    fn artifact_kind_rejects_unknown_strings() {
        let error = ArtifactKind::try_from("packet").expect_err("unknown kind should fail");
        assert_eq!(error.value, "packet");
    }
}
