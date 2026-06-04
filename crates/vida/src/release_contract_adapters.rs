pub(crate) fn blocker_code(code: crate::contract_profile_adapter::BlockerCode) -> Option<String> {
    crate::contract_profile_adapter::blocker_code(code)
}

pub(crate) fn canonical_blocker_codes(entries: &[String]) -> Vec<String> {
    crate::contract_profile_adapter::canonical_blocker_codes(entries)
}

pub(crate) fn release_contract_status(ready: bool) -> &'static str {
    crate::contract_profile_adapter::release_contract_status(ready)
}
