//! HTTP diagnostics deliberately exclude URI values, headers and request bodies.
use axum::{body::Body, extract::MatchedPath, http::Request};

pub fn request_span(request: &Request<Body>) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("<unmatched>", MatchedPath::as_str);
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    tracing::debug_span!("http.request", method = %request.method(), route, request_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::fmt::format::FmtSpan;

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn debug_traces_exclude_credentials_in_text_and_json() {
        for json in [false, true] {
            let buffer = Capture(Arc::new(Mutex::new(Vec::new())));
            let writer = buffer.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_ansi(false)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_writer(move || writer.clone());
            let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = if json {
                Box::new(subscriber.json().finish())
            } else {
                Box::new(subscriber.finish())
            };
            async {
                let app = axum::Router::new()
                    .route("/oidc/callback", axum::routing::get(|| async { "ok" }))
                    .layer(
                        tower_http::trace::TraceLayer::new_for_http().make_span_with(request_span),
                    );
                for uri in [
                    "/oidc/callback?code=SECRET_CODE&state=SECRET_STATE",
                    "/SECRET_PATH?code=SECRET_CODE",
                ] {
                    app.clone()
                        .oneshot(
                            Request::builder()
                                .uri(uri)
                                .header("x-request-id", "trace-test")
                                .header("authorization", "Bearer SECRET_TOKEN")
                                .header("cookie", "session=SECRET_COOKIE")
                                .body(Body::from("SECRET_PASSWORD"))
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                }
            }
            .with_subscriber(subscriber)
            .await;
            let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
            assert!(!output.contains("SECRET_"), "{output}");
            for safe in ["GET", "/oidc/callback", "trace-test", "status", "latency"] {
                assert!(output.contains(safe), "missing {safe}: {output}");
            }
        }
    }
}
