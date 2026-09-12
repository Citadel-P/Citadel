use std::error::Error;

use axum::extract::rejection::JsonRejection;
use citadel_identity::IdentityError;

/// JSON body extraction for handlers that previously let Axum return plain-text
/// rejections. Successful bodies have exactly the same decoding semantics.
pub(crate) struct ValidatedJson<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = axum::response::Response;

    async fn from_request(
        request: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = request.headers().clone();
        axum::Json::<T>::from_request(request, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .map_err(|error| {
                crate::identity_http::identity_error_response(invalid_json(error), &headers)
            })
    }
}

impl<S, T> axum::extract::OptionalFromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = axum::response::Response;

    async fn from_request(
        request: axum::extract::Request,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let headers = request.headers().clone();
        <axum::Json<T> as axum::extract::OptionalFromRequest<S>>::from_request(request, state)
            .await
            .map(|value| value.map(|axum::Json(value)| Self(value)))
            .map_err(|error| {
                crate::identity_http::identity_error_response(invalid_json(error), &headers)
            })
    }
}

/// Keep Axum's JSON field path and parser explanation in the existing
/// validation Problem Details contract instead of discarding the rejection.
pub(crate) fn invalid_json(rejection: JsonRejection) -> IdentityError {
    let mut source: Option<&(dyn Error + 'static)> = Some(&rejection);
    while let Some(error) = source {
        if let Some(error) = error.downcast_ref::<serde_path_to_error::Error<serde_json::Error>>() {
            let path = error.path().to_string();
            let path = match path.as_str() {
                "" | "." => "$".to_owned(),
                path if path.starts_with('[') => format!("${path}"),
                path => format!("$.{path}"),
            };
            return field_error(path, error.inner().to_string());
        }
        source = error.source();
    }
    field_error("$".to_owned(), rejection.body_text())
}

fn field_error(path: String, message: String) -> IdentityError {
    IdentityError::FieldValidation([(path, vec![message])].into_iter().collect())
}

pub(crate) fn validation_error(message: String) -> IdentityError {
    field_error("$".to_owned(), message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json,
        body::to_bytes,
        http::{HeaderMap, StatusCode},
    };
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    async fn problem<T: DeserializeOwned>(input: &[u8]) -> Value {
        let rejection = match Json::<T>::from_bytes(input) {
            Ok(_) => panic!("input should fail JSON extraction"),
            Err(rejection) => rejection,
        };
        let response = crate::identity_http::identity_error_response(
            invalid_json(rejection),
            &HeaderMap::new(),
        );
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn deployment_patch_reports_invalid_field_type() {
        let body =
            problem::<citadel_deployments::PatchDeploymentInput>(br#"{"platformId":42}"#).await;
        assert!(
            body["errors"]["$.platformId"][0]
                .as_str()
                .unwrap()
                .contains("invalid type")
        );
    }

    #[tokio::test]
    async fn stack_patch_reports_unknown_field() {
        let body = problem::<citadel_stacks::PatchStackInput>(br#"{"unexpected":true}"#).await;
        assert!(
            body["errors"]
                .to_string()
                .contains("unknown field `unexpected`")
        );
    }

    #[tokio::test]
    async fn platform_input_reports_field_and_expected_type() {
        let body = problem::<citadel_platforms::CreatePlatformInput>(br#"{"name":42}"#).await;
        assert!(
            body["errors"]["$.name"][0]
                .as_str()
                .unwrap()
                .contains("string")
        );
    }

    #[tokio::test]
    async fn malformed_json_reports_parser_location() {
        let body = problem::<citadel_stacks::PatchStackInput>(br#"{"platformId":"#).await;
        let errors = body["errors"].to_string();
        assert!(errors.contains("EOF"), "{body}");
        assert!(errors.contains("line 1"), "{body}");
    }

    #[tokio::test]
    async fn required_json_extractor_returns_problem_details_for_invalid_syntax() {
        let request = axum::extract::Request::builder()
            .header("content-type", "application/json")
            .header("x-request-id", "validation-test")
            .body(axum::body::Body::from("{"))
            .unwrap();
        let response =
            <ValidatedJson<Value> as axum::extract::FromRequest<()>>::from_request(request, &())
                .await
                .err()
                .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
        assert!(body["errors"].to_string().contains("EOF"));
        assert_eq!(body["traceId"], "validation-test");
    }

    #[tokio::test]
    async fn optional_json_allows_an_absent_body_but_rejects_invalid_json() {
        let request = axum::extract::Request::new(axum::body::Body::empty());
        let result =
            <ValidatedJson<Value> as axum::extract::OptionalFromRequest<()>>::from_request(
                request,
                &(),
            )
            .await
            .unwrap();
        assert!(result.is_none());
        let request = axum::extract::Request::builder()
            .header("content-type", "application/json")
            .body(axum::body::Body::from("{"))
            .unwrap();
        let response =
            <ValidatedJson<Value> as axum::extract::OptionalFromRequest<()>>::from_request(
                request,
                &(),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
    }
}
