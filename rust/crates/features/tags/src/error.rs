#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("{0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
}
