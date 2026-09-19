#[derive(Debug, thiserror::Error)]
pub enum GitAccountError {
    #[error("{0}")]
    Validation(String),
    #[error("Git account was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("Git credential protection failed")]
    Credential,
    #[error("Git account storage failed: {0}")]
    Storage(String),
}
