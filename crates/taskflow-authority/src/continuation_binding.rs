pub const MODULE: &str = "continuation_binding";

#[cfg(test)]
mod tests {
    use super::MODULE;

    #[test]
    fn continuation_binding_module_is_registered() {
        assert_eq!(MODULE, "continuation_binding");
    }
}
