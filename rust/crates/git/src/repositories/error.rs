use super::*;

#[derive(Debug, thiserror::Error)]
pub enum GitRepositoryExecutionError {
    #[error("{0}")]
    Validation(String),
    #[error("Git repository was not found")]
    NotFound,
    #[error("Git repository has not been synchronized yet")]
    NotSynchronized,
    #[error("Git repository operation is already queued or running")]
    Conflict,
    #[error("Git repository execution failed: {0}")]
    Git(#[from] GitError),
    #[error("Git credential configuration is invalid")]
    Credential,
    #[error("Webhook authentication failed")]
    Authentication,
    #[error("Git repository storage failed: {0}")]
    Storage(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GitRepositoryError {
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
