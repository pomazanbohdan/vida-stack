use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    ApprovalNotRequired,
    ApprovalRequired,
    WaitingForApproval,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalNotRequired => "approval_not_required",
            Self::ApprovalRequired => "approval_required",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::ApprovalNotRequired,
            Self::ApprovalRequired,
            Self::WaitingForApproval,
            Self::Approved,
            Self::Denied,
            Self::Expired,
        ]
    }
}

impl TryFrom<&str> for ApprovalStatus {
    type Error = UnknownStatusCode;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        Self::all()
            .iter()
            .copied()
            .find(|status| status.as_str() == trimmed)
            .ok_or_else(|| UnknownStatusCode {
                value: value.to_string(),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneStatus {
    PacketReady,
    LaneOpen,
    LaneRunning,
    LaneBlocked,
    LaneCompleted,
    LaneSuperseded,
    LaneExceptionRecorded,
    LaneExceptionTakeover,
}

impl LaneStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PacketReady => "packet_ready",
            Self::LaneOpen => "lane_open",
            Self::LaneRunning => "lane_running",
            Self::LaneBlocked => "lane_blocked",
            Self::LaneCompleted => "lane_completed",
            Self::LaneSuperseded => "lane_superseded",
            Self::LaneExceptionRecorded => "lane_exception_recorded",
            Self::LaneExceptionTakeover => "lane_exception_takeover",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::PacketReady,
            Self::LaneOpen,
            Self::LaneRunning,
            Self::LaneBlocked,
            Self::LaneCompleted,
            Self::LaneSuperseded,
            Self::LaneExceptionRecorded,
            Self::LaneExceptionTakeover,
        ]
    }
}

impl TryFrom<&str> for LaneStatus {
    type Error = UnknownStatusCode;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        Self::all()
            .iter()
            .copied()
            .find(|status| status.as_str() == trimmed)
            .ok_or_else(|| UnknownStatusCode {
                value: value.to_string(),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Release1ContractStatus {
    Pass,
    Blocked,
}

impl Release1ContractStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Blocked => "blocked",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Pass, Self::Blocked]
    }
}

impl TryFrom<&str> for Release1ContractStatus {
    type Error = UnknownStatusCode;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "pass" | "ok" => Ok(Self::Pass),
            "blocked" | "BLOCK" => Ok(Self::Blocked),
            _ => Err(UnknownStatusCode {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStatusCode {
    pub value: String,
}

impl std::fmt::Display for UnknownStatusCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown status code `{}`", self.value)
    }
}

impl std::error::Error for UnknownStatusCode {}

#[must_use]
pub fn canonical_approval_status_str(value: &str) -> Option<&'static str> {
    ApprovalStatus::try_from(value)
        .map(ApprovalStatus::as_str)
        .ok()
}

#[must_use]
pub fn canonical_lane_status_str(value: &str) -> Option<&'static str> {
    LaneStatus::try_from(value).map(LaneStatus::as_str).ok()
}

#[must_use]
pub fn canonical_release1_contract_status_str(value: &str) -> Option<&'static str> {
    Release1ContractStatus::try_from(value)
        .map(Release1ContractStatus::as_str)
        .ok()
}

#[must_use]
pub const fn release1_contract_status_str(ok: bool) -> &'static str {
    if ok {
        Release1ContractStatus::Pass.as_str()
    } else {
        Release1ContractStatus::Blocked.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalStatus, LaneStatus, Release1ContractStatus, canonical_approval_status_str,
        canonical_lane_status_str, canonical_release1_contract_status_str,
        release1_contract_status_str,
    };

    #[test]
    fn approval_status_round_trips() {
        for status in ApprovalStatus::all() {
            assert_eq!(
                canonical_approval_status_str(status.as_str()),
                Some(status.as_str())
            );
        }
        assert_eq!(canonical_approval_status_str("pending"), None);
    }

    #[test]
    fn lane_status_round_trips() {
        for status in LaneStatus::all() {
            assert_eq!(
                canonical_lane_status_str(status.as_str()),
                Some(status.as_str())
            );
        }
        assert_eq!(
            canonical_lane_status_str(" lane_running "),
            Some("lane_running")
        );
        assert_eq!(canonical_lane_status_str("lane_block"), None);
    }

    #[test]
    fn release1_contract_status_round_trips() {
        for status in Release1ContractStatus::all() {
            assert_eq!(
                canonical_release1_contract_status_str(status.as_str()),
                Some(status.as_str())
            );
        }
        assert_eq!(canonical_release1_contract_status_str(" ok "), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str(" BLOCK "),
            Some("blocked")
        );
        assert_eq!(release1_contract_status_str(true), "pass");
        assert_eq!(release1_contract_status_str(false), "blocked");
    }

    #[test]
    fn status_code_errors_preserve_input_and_display() {
        let approval_error = ApprovalStatus::try_from(" pending ").unwrap_err();
        assert_eq!(approval_error.value, " pending ");
        assert_eq!(
            approval_error.to_string(),
            "unknown status code ` pending `"
        );

        let lane_error = LaneStatus::try_from("lane_unknown").unwrap_err();
        assert_eq!(lane_error.value, "lane_unknown");
        assert_eq!(lane_error.to_string(), "unknown status code `lane_unknown`");

        let release_error = Release1ContractStatus::try_from(" reject ").unwrap_err();
        assert_eq!(release_error.value, " reject ");
        assert_eq!(release_error.to_string(), "unknown status code ` reject `");
    }

    #[test]
    fn status_code_serialization_uses_canonical_snake_case() {
        assert_eq!(
            serde_json::to_value(ApprovalStatus::WaitingForApproval).unwrap(),
            serde_json::json!("waiting_for_approval")
        );
        assert_eq!(
            serde_json::to_value(LaneStatus::LaneExceptionTakeover).unwrap(),
            serde_json::json!("lane_exception_takeover")
        );
        assert_eq!(
            serde_json::to_value(Release1ContractStatus::Blocked).unwrap(),
            serde_json::json!("blocked")
        );
    }
}
