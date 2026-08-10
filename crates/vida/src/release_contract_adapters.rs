pub(crate) fn blocker_code(code: crate::contract_profile_adapter::BlockerCode) -> Option<String> {
    crate::contract_profile_adapter::blocker_code(code)
}

pub(crate) fn canonical_blocker_codes(entries: &[String]) -> Vec<String> {
    crate::contract_profile_adapter::canonical_blocker_codes(entries)
}

pub(crate) fn release_contract_status(ready: bool) -> &'static str {
    crate::contract_profile_adapter::release_contract_status(ready)
}

#[cfg(test)]
mod tests {
    use super::{blocker_code, canonical_blocker_codes, release_contract_status};
    use crate::contract_profile_adapter::BlockerCode;

    #[test]
    fn release_contract_adapters_preserve_status_and_blocker_contracts() {
        assert_eq!(release_contract_status(true), "pass");
        assert_eq!(release_contract_status(false), "blocked");
        assert_eq!(
            blocker_code(BlockerCode::MissingPacket),
            Some("missing_packet".to_string())
        );
        assert_eq!(
            canonical_blocker_codes(&[
                "missing_packet".to_string(),
                "unknown".to_string(),
                "missing_packet".to_string(),
            ]),
            vec!["missing_packet".to_string()]
        );
        assert!(canonical_blocker_codes(&[]).is_empty());
    }
}
