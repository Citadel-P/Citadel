//! JSON-array progress responses with idle keepalives.
use axum::{
    body::Body,
    http::{HeaderValue, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use std::time::Duration;

/// Keep transport liveness separate from operation progress and cancellation.
pub(crate) fn json_response<T, S>(items: S) -> Response
where
    T: Serialize + Send + 'static,
    S: Stream<Item = T> + Send + 'static,
{
    let stream = async_stream::stream! {
        let mut items = std::pin::pin!(items);
        yield Ok::<_, serde_json::Error>(Bytes::from_static(b"["));
        let mut first = true;
        loop {
            let item = match tokio::time::timeout(Duration::from_secs(15), items.next()).await {
                Ok(Some(item)) => item,
                Ok(None) => break,
                Err(_) => {
                    // Whitespace is legal between complete JSON items and is ignored
                    // by the frontend. Never insert it inside a serialized item.
                    yield Ok(Bytes::from_static(b"\n"));
                    continue;
                }
            };
            let value = match serde_json::to_vec(&item) {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(%error, "failed to serialize operation progress");
                    yield Err(error);
                    return;
                }
            };
            if !first {
                yield Ok(Bytes::from_static(b","));
            }
            first = false;
            yield Ok(Bytes::from(value));
        }
        yield Ok(Bytes::from_static(b"]"));
    };
    let mut response = super::error::no_store(Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    response
}
