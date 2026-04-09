#[allow(dead_code)]
pub(crate) fn blocker_code(code: crate::contract_profile_adapter::BlockerCode) -> Option<String> {
    crate::contract_profile_adapter::blocker_code(code)
}

#[allow(dead_code)]
pub(crate) fn blocker_code_str(code: crate::contract_profile_adapter::BlockerCode) -> &'static str {
    crate::contract_profile_adapter::blocker_code_str(code)
}

#[allow(dead_code)]
pub(crate) fn canonical_blocker_codes(entries: &[String]) -> Vec<String> {
    crate::contract_profile_adapter::canonical_blocker_codes(entries)
}

#[allow(dead_code)]
pub(crate) fn release_contract_status(ready: bool) -> &'static str {
    crate::contract_profile_adapter::release_contract_status(ready)
}

#[allow(dead_code)]
pub(crate) fn boot_compatibility_is_backward_compatible(classification: &str) -> bool {
    crate::contract_profile_adapter::boot_compatibility_is_backward_compatible(classification)
}
