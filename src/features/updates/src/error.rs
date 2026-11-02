#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Administrator access is required.")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Conflict(String),
    #[error("The update operation was not found.")]
    NotFound,
    #[error("The update preparation was cancelled.")]
    Cancelled,
    #[error("Update persistence failed: {0}")]
    Storage(String),
}
