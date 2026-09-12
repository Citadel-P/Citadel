//! Optional documentation routes. Assets are embedded, so Swagger UI works offline.
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use std::sync::{Arc, LazyLock};
use utoipa::openapi::Server;
use utoipa_swagger_ui::Config;

pub fn router(default_scheme: &'static str) -> Router {
    Router::new()
        .route("/openapi/v1.json", get(full))
        .route("/openapi/public/v1.json", get(public))
        .route(
            "/swagger",
            get(|| async { Redirect::temporary("/swagger/") }),
        )
        .route("/swagger/", get(index))
        .route("/swagger/index.html", get(index))
        .route("/swagger/{*path}", get(asset))
        .with_state(default_scheme)
}

fn config() -> Config<'static> {
    Config::new(["/openapi/v1.json", "/openapi/public/v1.json"])
        .deep_linking(true)
        .display_operation_id(true)
        .display_request_duration(true)
        .try_it_out_enabled(true)
        .persist_authorization(true)
        .validator_url("")
}

async fn index() -> Html<&'static str> {
    Html(include_str!("swagger.html"))
}

async fn asset(Path(path): Path<String>) -> Response {
    if path == "swagger-initializer.js" {
        let script = include_str!("swagger-initializer.js").replace(
            "/* CITADEL_CONFIG */",
            &serde_json::to_string(&config()).expect("Swagger configuration serializes"),
        );
        return ([(header::CONTENT_TYPE, "text/javascript")], script).into_response();
    }
    if path == "custom.css" {
        return (
            [(header::CONTENT_TYPE, "text/css")],
            include_str!("swagger.css"),
        )
            .into_response();
    }
    match utoipa_swagger_ui::serve(&path, Arc::new(config())) {
        Ok(Some(file)) => (
            [(header::CONTENT_TYPE, file.content_type)],
            file.bytes.to_vec(),
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(%error, "failed to serve Swagger UI asset");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn full(State(scheme): State<&'static str>, headers: HeaderMap, uri: Uri) -> Response {
    response(false, scheme, &headers, &uri)
}

async fn public(State(scheme): State<&'static str>, headers: HeaderMap, uri: Uri) -> Response {
    response(true, scheme, &headers, &uri)
}

fn response(public: bool, default_scheme: &str, headers: &HeaderMap, uri: &Uri) -> Response {
    // The outer transport middleware validates Host and proxy headers first.
    // Never cache a request's origin in the shared/static document.
    let host = uri
        .authority()
        .map(|v| v.as_str())
        .or_else(|| headers.get(header::HOST).and_then(|v| v.to_str().ok()));
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .filter(|v| *v == "https")
        .or(uri.scheme_str())
        .unwrap_or(default_scheme);
    static DOCUMENTS: LazyLock<[utoipa::openapi::OpenApi; 2]> =
        LazyLock::new(|| [super::document(false), super::document(true)]);
    let mut document = DOCUMENTS[usize::from(public)].clone();
    document.servers = Some(vec![Server::new(
        host.map(|host| format!("{scheme}://{host}"))
            .unwrap_or_else(|| "/".into()),
    )]);
    ([(header::CACHE_CONTROL, "no-store")], Json(document)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn serves_request_origin_without_polluting_static_exports() {
        for (path, host, scheme) in [
            ("/openapi/v1.json", "localhost:8000", "http"),
            ("/openapi/public/v1.json", "citadel.example", "https"),
        ] {
            let response = router(scheme)
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("host", host)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let doc: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            assert_eq!(doc["servers"][0]["url"], format!("{scheme}://{host}"));
        }
        assert!(super::super::document(false).servers.is_none());
    }

    #[tokio::test]
    async fn serves_embedded_ui_assets_and_dotnet_options() {
        for path in [
            "/swagger/",
            "/swagger/index.html",
            "/swagger/swagger-ui.css",
            "/swagger/swagger-ui-bundle.js",
            "/swagger/swagger-ui-standalone-preset.js",
            "/swagger/swagger-initializer.js",
            "/swagger/custom.css",
        ] {
            let response = router("http")
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert!(!body.is_empty());
            if path.ends_with("swagger-initializer.js") {
                let body = String::from_utf8(body.to_vec()).unwrap();
                let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
                for option in [
                    "deepLinking",
                    "displayOperationId",
                    "displayRequestDuration",
                    "tryItOutEnabled",
                    "persistAuthorization",
                ] {
                    assert!(
                        body.contains(&format!("\"{option}\":true")),
                        "{option}: {body}"
                    );
                }
            }
        }
    }
}
