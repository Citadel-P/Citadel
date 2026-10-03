#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("{0}")]
    Validation(String),
    #[error("storage failed: {0}")]
    Storage(String),
}
