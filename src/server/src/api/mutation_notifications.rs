use crate::realtime::RealtimeHub;
use axum::{Router, extract::State, response::Response};
use uuid::Uuid;

/// Coarse invalidations for resource routers whose writes finish inside the
/// handler. Jobs publish their claim/completion separately; no payload or IDs
/// are broadcast here, and every follow-up read applies its normal ACL.
pub(crate) fn notify_mutations(router: Router, resource_type: &'static str) -> Router {
    router.layer(axum::middleware::from_fn_with_state(
        resource_type,
        mutation_notification,
    ))
}

async fn mutation_notification(
    State(resource_type): State<&'static str>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let hub = (!request.method().is_safe())
        .then(|| request.extensions().get::<RealtimeHub>().cloned())
        .flatten();
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(hub) = hub
    {
        hub.publish_resource_change(resource_type, Uuid::nil(), "resourceChanged");
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metrics;
    use std::sync::Arc;
    #[tokio::test]
    async fn successful_mutations_invalidate_but_reads_and_failures_do_not() {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode},
        };
        use tower::ServiceExt;

        let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
        let _subscriber = hub.subscribe();
        for (method, status, revision) in [
            (Method::GET, StatusCode::OK, 0),
            (Method::POST, StatusCode::BAD_REQUEST, 0),
            (Method::POST, StatusCode::INTERNAL_SERVER_ERROR, 0),
            (Method::POST, StatusCode::NO_CONTENT, 1),
            (Method::DELETE, StatusCode::OK, 2),
        ] {
            let app = notify_mutations(
                axum::Router::new().route(
                    "/resource",
                    axum::routing::any(move || async move { status }),
                ),
                "User",
            )
            .layer(axum::Extension(hub.clone()));
            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/resource")
                        .method(method)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(hub.current_revision(), revision);
        }
    }
}
