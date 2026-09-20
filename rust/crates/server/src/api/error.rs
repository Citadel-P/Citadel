//! Server-only error adaptation and the shared ProblemDetails response contract.
//! Feature errors are mapped by their resource handlers; Identity is one input owner.
use axum::{
    Json,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, PRAGMA, WWW_AUTHENTICATE},
    },
    response::{IntoResponse, Response},
};
use serde::Serialize;
const REQUEST_ID: &str = "x-request-id";

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
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
    #[error("storage failed: {0}")]
    Storage(String),
    #[error("internal operation failed: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("credential processing failed")]
    Credential,
    #[error("external operation failed: {0}")]
    External(String),
}

impl From<citadel_identity::IdentityError> for ApiError {
    fn from(error: citadel_identity::IdentityError) -> Self {
        use citadel_identity::IdentityError;
        match error {
            IdentityError::InvalidCredentials => Self::InvalidCredentials,
            IdentityError::Unauthenticated => Self::Unauthenticated,
            IdentityError::Forbidden => Self::Forbidden,
            IdentityError::SetupRequired => Self::SetupRequired,
            IdentityError::SetupAlreadyComplete => Self::SetupAlreadyComplete,
            IdentityError::NotFound => Self::NotFound,
            IdentityError::Credential => Self::Credential,
            IdentityError::Validation(value) => Self::Validation(value),
            IdentityError::FieldValidation(value) => Self::FieldValidation(value),
            IdentityError::Conflict(value) => Self::Conflict(value),
            IdentityError::ResourceNotFound(value) => Self::ResourceNotFound(value),
            IdentityError::LicenseRequired(value) => Self::LicenseRequired(value),
            source @ IdentityError::Storage(_) => Self::internal(source),
            IdentityError::External(value) => Self::External(value),
            IdentityError::TypedConflict {
                problem_type,
                message,
            } => Self::TypedConflict {
                problem_type,
                message,
            },
        }
    }
}

pub(crate) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

pub(crate) fn error_response(error: impl Into<ApiError>, headers: &HeaderMap) -> Response {
    render_error(error.into(), request_id(headers))
}

fn render_error(error: ApiError, request_id: String) -> Response {
    if let ApiError::Storage(_) | ApiError::Internal(_) | ApiError::Credential = &error {
        tracing::error!(error = %error, source_chain = ?error, %request_id, "API operation failed");
    }
    let mut errors = None;
    let mut resource_problem = false;
    let (status, problem_type, title, detail) = match error {
        ApiError::FieldValidation(fields) => {
            errors = Some(fields);
            resource_problem = true;
            (
                StatusCode::BAD_REQUEST,
                "https://tools.ietf.org/html/rfc9110#section-15.5.1",
                "One or more validation errors occurred.",
                String::new(),
            )
        }
        ApiError::ResourceNotFound(message) => {
            resource_problem = true;
            (
                StatusCode::NOT_FOUND,
                "https://tools.ietf.org/html/rfc9110#section-15.5.5",
                "Not Found",
                message.to_owned(),
            )
        }
        ApiError::InvalidCredentials | ApiError::Unauthenticated => (
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "Authentication required",
            "Valid authentication is required.".to_owned(),
        ),
        ApiError::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "The authenticated principal is not allowed to perform this operation.".to_owned(),
        ),
        ApiError::SetupRequired => (
            StatusCode::CONFLICT,
            "setup_required",
            "Setup required",
            "Citadel must be initialized before this endpoint can be used.".to_owned(),
        ),
        ApiError::SetupAlreadyComplete => (
            StatusCode::CONFLICT,
            "setup_already_complete",
            "Setup already complete",
            "Citadel setup is already complete.".to_owned(),
        ),
        ApiError::Validation(message) => (
            StatusCode::BAD_REQUEST,
            "validation_error",
            "Validation failed",
            message,
        ),
        ApiError::Conflict(message) => (StatusCode::CONFLICT, "conflict", "Conflict", message),
        ApiError::TypedConflict {
            problem_type,
            message,
        } => (StatusCode::CONFLICT, problem_type, "Conflict", message),
        ApiError::NotFound => (
            StatusCode::NOT_FOUND,
            "not_found",
            "Not found",
            "The requested resource was not found.".to_owned(),
        ),
        ApiError::LicenseRequired(capability) => {
            return license_required_response(capability, request_id);
        }
        ApiError::Storage(_) | ApiError::Internal(_) | ApiError::Credential => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            "An unexpected error occurred.".to_owned(),
        ),
        ApiError::External(message) => (
            StatusCode::BAD_GATEWAY,
            "bad_gateway",
            "Bad gateway",
            message,
        ),
    };
    let detail = if errors.is_some() { None } else { Some(detail) };
    let (request_id, trace_id) = if resource_problem {
        (None, Some(request_id))
    } else {
        (Some(request_id), None)
    };
    render_problem(
        status,
        ProblemDetails {
            r#type: problem_type,
            title,
            status: status.as_u16(),
            detail,
            request_id,
            trace_id,
            errors,
            capability: None,
            license_status: None,
            effective_edition: None,
        },
    )
}

pub(crate) type HttpResult<T = Response> = Result<T, HttpError>;

pub(crate) struct HttpError {
    error: ApiError,
    request_id: String,
}

impl HttpError {
    pub(crate) fn from_parts(error: impl Into<ApiError>, headers: &HeaderMap) -> Self {
        Self {
            error: error.into(),
            request_id: request_id(headers),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        render_error(self.error, self.request_id)
    }
}

pub(crate) fn api_result<T>(
    result: Result<T, impl Into<ApiError>>,
    headers: &HeaderMap,
) -> HttpResult<T> {
    result.map_err(|error| HttpError::from_parts(error, headers))
}

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned()
}

fn license_required_response(capability_key: &'static str, request_id: String) -> Response {
    let (capability, display_name) = match capability_key {
        "custom-access-control" => ("CustomAccessControl", "Custom access control"),
        "advanced-alerting" => ("AdvancedAlerting", "Advanced alerting"),
        _ => (capability_key, "This capability"),
    };
    let status = StatusCode::FORBIDDEN;
    render_problem(
        status,
        ProblemDetails {
            r#type: "https://citadel.local/problems/license-capability-required",
            title: "License capability required",
            status: status.as_u16(),
            detail: Some(format!("{display_name} requires a Team license.")),
            request_id: Some(request_id),
            trace_id: None,
            errors: None,
            capability: Some(capability),
            license_status: Some("Community"),
            effective_edition: Some("Community"),
        },
    )
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = ProblemDetails)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<std::collections::BTreeMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_status: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_edition: Option<&'static str>,
}

impl ApiError {
    pub(crate) fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal(Box::new(source))
    }
}

fn render_problem(status: StatusCode, details: ProblemDetails) -> Response {
    let mut response = (status, Json(details)).into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    no_store(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[tokio::test]
    async fn internal_sources_survive_mapping_but_never_reach_the_response() {
        let problem = ApiError::internal(std::io::Error::other("private database credentials"));
        assert!(
            problem
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .is_some()
        );
        let response = error_response(problem, &HeaderMap::new());
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["detail"], "An unexpected error occurred.");
        assert!(!String::from_utf8_lossy(&body).contains("private database"));
    }
}
