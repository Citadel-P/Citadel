#[derive(Debug, Clone, thiserror::Error)]
pub enum BuildError {
    #[error("This Build operation requires license capability {0:?}.")]
    LicenseRequired(citadel_domain::LicenseCapability),
    #[error("{0}")]
    Validation(String),
    #[error("Build resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
}
