#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContractProfileId {
    Release1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContractProfile {
    pub(crate) id: ContractProfileId,
    pub(crate) name: &'static str,
}

pub(crate) fn selected_contract_profile() -> ContractProfile {
    ContractProfile {
        id: ContractProfileId::Release1,
        name: "release-1",
    }
}

pub(crate) fn selected_contract_profile_id() -> ContractProfileId {
    selected_contract_profile().id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_contract_profile_defaults_to_release1() {
        let profile = selected_contract_profile();
        assert_eq!(profile.id, ContractProfileId::Release1);
        assert_eq!(profile.name, "release-1");
    }
}
