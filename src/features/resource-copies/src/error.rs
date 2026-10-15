#[derive(Debug, thiserror::Error)]
pub enum CopyError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Resource copy was not found or access was denied.")]
    NotFound,
    #[error("{0}")]
    Storage(String),
}
