#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("authentication is required")]
    Unauthenticated,
    #[error("the authenticated principal is not allowed to perform this operation")]
    Forbidden,
    #[error("Citadel setup is required")]
    SetupRequired,
    #[error("Citadel setup is already complete")]
    SetupAlreadyComplete,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("one or more validation errors occurred")]
    FieldValidation(std::collections::BTreeMap<String, Vec<String>>),
    #[error("resource conflict: {0}")]
    Conflict(String),
    #[error("resource conflict: {message}")]
    TypedConflict {
        problem_type: &'static str,
        message: String,
    },
    #[error("resource was not found")]
    NotFound,
    #[error("{0}")]
    ResourceNotFound(&'static str),
    #[error("license capability '{0}' is unavailable")]
    LicenseRequired(&'static str),
    #[error("identity storage failed: {0}")]
    Storage(String),
    #[error("identity credential processing failed")]
    Credential,
    #[error("external operation failed: {0}")]
    External(String),
}
