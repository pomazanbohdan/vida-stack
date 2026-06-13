#[derive(Debug, thiserror::Error)]
pub enum TaskflowAuthorityError {
    #[error("{0}")]
    InvalidAuthority(String),
}
