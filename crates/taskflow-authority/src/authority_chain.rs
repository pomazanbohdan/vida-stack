pub const MODULE: &str = "authority_chain";

#[cfg(test)]
mod tests {
    use super::MODULE;

    #[test]
    fn authority_chain_module_is_registered() {
        assert_eq!(MODULE, "authority_chain");
    }
}
