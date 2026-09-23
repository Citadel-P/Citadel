#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("resource was not found")]
    NotFound,
    #[error("activity storage failed: {0}")]
    Storage(String),
}
