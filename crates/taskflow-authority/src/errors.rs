#[derive(Debug, thiserror::Error)]
pub enum TaskflowAuthorityError {
    #[error("{0}")]
    InvalidAuthority(String),
}

#[cfg(test)]
mod tests {
    use super::TaskflowAuthorityError;

    #[test]
    fn invalid_authority_error_preserves_empty_and_non_empty_messages() {
        let non_empty = TaskflowAuthorityError::InvalidAuthority("missing receipt".to_string());
        assert_eq!(non_empty.to_string(), "missing receipt");
        assert!(matches!(
            &non_empty,
            TaskflowAuthorityError::InvalidAuthority(message) if message == "missing receipt"
        ));

        let empty = TaskflowAuthorityError::InvalidAuthority(String::new());
        assert_eq!(empty.to_string(), "");
        assert!(matches!(
            &empty,
            TaskflowAuthorityError::InvalidAuthority(message) if message.is_empty()
        ));
    }
}
