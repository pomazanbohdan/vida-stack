#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContractProfileId {
    OperatorContracts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContractProfile {
    pub(crate) id: ContractProfileId,
    pub(crate) name: &'static str,
}

pub(crate) fn selected_contract_profile() -> ContractProfile {
    ContractProfile {
        id: ContractProfileId::OperatorContracts,
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
    fn selected_contract_profile_defaults_to_operator_contracts_profile() {
        let profile = selected_contract_profile();
        assert_eq!(profile.id, ContractProfileId::OperatorContracts);
        assert_eq!(profile.name, "release-1");
    }

    #[test]
    fn selected_contract_profile_id_matches_selected_profile() {
        assert_eq!(
            selected_contract_profile_id(),
            selected_contract_profile().id
        );
    }
}
