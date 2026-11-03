use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::Response,
};
use citadel_runtime::admission::{AdmissionGate, AdmissionPermit};
use http_body::{Body as HttpBody, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub async fn admit(State(gate): State<AdmissionGate>, request: Request, next: Next) -> Response {
    // These reads are required to observe/reconcile an update. This is not an
    // authorization bypass: normal authentication and permissions still run.
    let path = request.uri().path().trim_end_matches('/');
    let observation = request.method() == Method::GET
        && matches!(
            path,
            "/health"
                | "/ready"
                | "/api/v1/updates"
                | "/api/v1/activities"
                | "/api/v1/application/info"
                | "/api/v1/realtime"
        );
    if observation && !(path == "/ready" && gate.is_closed()) {
        return next.run(request).await;
    }
    let Some(permit) = gate.try_enter("HTTP requests") else {
        let mut response = crate::api::error::error_response(
            crate::api::error::ApiError::Maintenance,
            request.headers(),
        );
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "5".parse().expect("static header"));
        return response;
    };
    let response = next.run(request).await;
    response.map(|body| {
        Body::new(AdmittedBody {
            body,
            permit: Some(permit),
        })
    })
}

// A streaming operation can continue after its response headers. Retain admission
// through its body (including trailers), not just until the handler returns.
struct AdmittedBody {
    body: Body,
    permit: Option<AdmissionPermit>,
}
impl HttpBody for AdmittedBody {
    type Data = Bytes;
    type Error = axum::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, axum::Error>>> {
        let result = Pin::new(&mut self.body).poll_frame(cx);
        if matches!(result, Poll::Ready(None | Some(Err(_)))) || self.body.is_end_stream() {
            self.permit.take();
        }
        result
    }
    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
