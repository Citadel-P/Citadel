#[derive(Debug, thiserror::Error)]
pub enum BindingError {
    #[error("{0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("credential protection failed")]
    Credential,
    #[error("storage failed: {0}")]
    Storage(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SecretValueError {
    #[error("{0}")]
    Validation(String),
    #[error("secret storage failed: {0}")]
    Storage(String),
    #[error("external Secret provider failed: {0}")]
    External(String),
}
