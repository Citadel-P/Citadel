#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error("Administrator access is required.")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("instance settings storage failed: {0}")]
    Storage(String),
}
