use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::{
    ACCEPT, AUTHORIZATION, CONTENT_TYPE, HOST, HeaderName, HeaderValue, STRICT_TRANSPORT_SECURITY,
};
use axum::http::uri::Authority;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::Readiness;
use crate::config::{TransportConfig, TransportMode};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
struct SecurityState {
    transport: Arc<TransportConfig>,
    readiness: Arc<Readiness>,
    rate_limit: Arc<FixedWindowRateLimit>,
}

struct FixedWindowRateLimit {
    state: AtomicU64,
    limit: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    detail: &'static str,
    request_id: String,
}

pub fn secure_router(
    router: Router,
    transport: &TransportConfig,
    readiness: Arc<Readiness>,
) -> Result<Router, Box<dyn std::error::Error>> {
    secure_router_with_grpc(router, transport, readiness, None)
}

pub fn secure_router_with_grpc(
    mut router: Router,
    transport: &TransportConfig,
    readiness: Arc<Readiness>,
    grpc: Option<Router>,
) -> Result<Router, Box<dyn std::error::Error>> {
    router = router.route("/api/{*path}", any(api_not_found));

    if let Some(root) = &transport.static_root {
        if !root.is_dir() {
            return Err(format!("SPA root {} is not a directory", root.display()).into());
        }
        router = router.fallback_service(
            ServeDir::new(root).fallback(ServeFile::new(root.join("index.html"))),
        );
    }

    let cors = if transport.cors_origins.is_empty() {
        CorsLayer::new()
    } else {
        let origins = transport
            .cors_origins
            .iter()
            .map(|origin| HeaderValue::from_str(origin))
            .collect::<Result<Vec<_>, _>>()?;
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                ACCEPT,
                AUTHORIZATION,
                CONTENT_TYPE,
                HeaderName::from_static("x-csrf-token"),
            ])
    };
    let security = SecurityState {
        transport: Arc::new(transport.clone()),
        readiness,
        rate_limit: Arc::new(FixedWindowRateLimit::new(transport.requests_per_minute)),
    };
    router = router.layer(RequestBodyLimitLayer::new(transport.body_limit_bytes));
    // gRPC bounds each decoded message, not the aggregate bytes transferred
    // over a long-lived authenticated bidirectional connection.
    if let Some(grpc) = grpc {
        router = router.merge(grpc);
    }
    let mut router = router
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            security,
            security_middleware,
        ))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID.clone()))
        .layer(SetRequestIdLayer::new(REQUEST_ID.clone(), MakeRequestUuid));
    if transport.mode == TransportMode::Direct {
        router = router.layer(SetResponseHeaderLayer::if_not_present(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        ));
    }
    Ok(router)
}

async fn api_not_found(headers: HeaderMap) -> Response {
    let request_id = headers
        .get(&REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    problem(
        StatusCode::NOT_FOUND,
        "API endpoint not found",
        "The requested Citadel API endpoint does not exist.",
        request_id,
    )
}

pub fn panic_response(_error: Box<dyn std::any::Any + Send + 'static>) -> Response {
    problem(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error",
        "An unexpected error occurred.",
        "unknown".to_owned(),
    )
}

pub async fn serve(
    address: SocketAddr,
    transport: &TransportConfig,
    app: Router,
    cancellation: CancellationToken,
    shutdown_timeout: Duration,
) -> std::io::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let handle = Handle::new();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    match transport.mode {
        TransportMode::Direct => {
            let tls = RustlsConfig::from_pem_file(
                transport
                    .certificate_path
                    .as_ref()
                    .expect("Direct mode was validated with a certificate"),
                transport
                    .certificate_private_key_path
                    .as_ref()
                    .expect("Direct mode was validated with a private key"),
            )
            .await?;
            let server = axum_server::bind_rustls(address, tls)
                .handle(handle.clone())
                .serve(service);
            tokio::pin!(server);
            tokio::select! {
                biased;
                result = &mut server => result,
                () = cancellation.cancelled() => {
                    handle.graceful_shutdown(Some(shutdown_timeout));
                    server.await
                },
            }
        }
        TransportMode::ReverseProxy | TransportMode::Disabled => {
            let server = axum_server::bind(address)
                .handle(handle.clone())
                .serve(service);
            tokio::pin!(server);
            tokio::select! {
                biased;
                result = &mut server => result,
                () = cancellation.cancelled() => {
                    handle.graceful_shutdown(Some(shutdown_timeout));
                    server.await
                },
            }
        }
    }
}

async fn security_middleware(
    axum::extract::State(state): axum::extract::State<SecurityState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let request_id = request
        .headers()
        .get(&REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    if !allowed_host(&state.transport, &request) {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid host",
            "The request host is not allowed.",
            request_id,
        );
    }
    if !trusted_forwarding(&state.transport, &request) {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid forwarded headers",
            "Forwarded headers were supplied by an untrusted peer or exceeded the configured limit.",
            request_id,
        );
    }
    if !state.rate_limit.admit() {
        let mut response = problem(
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded",
            "The global foundation request limit was exceeded.",
            request_id,
        );
        response.headers_mut().insert(
            axum::http::header::RETRY_AFTER,
            HeaderValue::from_static("60"),
        );
        return response;
    }
    if request.method() != Method::OPTIONS
        && !state.readiness.is_setup()
        && setup_gated(request.uri().path())
    {
        if state.readiness.is_database_ready() {
            return crate::identity_http::identity_error_response(
                citadel_identity::IdentityError::SetupRequired,
                request.headers(),
            );
        }
        return problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Setup unavailable",
            "Citadel setup state is unavailable.",
            request_id,
        );
    }
    let response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Request body too large",
            "The request body exceeded this route's configured limit.",
            request_id,
        );
    }
    response
}

fn allowed_host(transport: &TransportConfig, request: &Request<Body>) -> bool {
    if transport.mode == TransportMode::Disabled {
        return true;
    }
    let Some(host) = request
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Ok(authority) = Authority::from_str(host) else {
        return false;
    };
    transport
        .allowed_hosts
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(authority.host()))
}

fn trusted_forwarding(transport: &TransportConfig, request: &Request<Body>) -> bool {
    let forwarded_for = request.headers().get("x-forwarded-for");
    let forwarded_proto = request.headers().get("x-forwarded-proto");
    if transport.mode != TransportMode::ReverseProxy {
        return forwarded_for.is_none() && forwarded_proto.is_none();
    }
    if forwarded_for.is_none() && forwarded_proto.is_none() {
        return true;
    }
    let Some(peer) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip())
    else {
        return false;
    };
    if !is_trusted_peer(transport, peer) {
        return false;
    }
    let Some(forwarded_for) = forwarded_for.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let hops = forwarded_for.split(',').map(str::trim).collect::<Vec<_>>();
    if hops.is_empty()
        || hops.len() > transport.forward_limit
        || hops
            .iter()
            .any(|address| address.parse::<IpAddr>().is_err())
    {
        return false;
    }
    forwarded_proto
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("https"))
}

fn is_trusted_peer(transport: &TransportConfig, peer: IpAddr) -> bool {
    transport.known_proxies.contains(&peer)
        || transport
            .known_networks
            .iter()
            .any(|network| network.contains(&peer))
}

fn setup_gated(path: &str) -> bool {
    let explicitly_exempt = crate::openapi::setup_exempt(path);
    !explicitly_exempt
        && !path.starts_with("/api/v1/setup")
        && (path.starts_with("/api/v1")
            || path.starts_with("/hubs")
            || path.starts_with("/listener")
            || path == "/phase0/realtime")
}

fn problem(
    status: StatusCode,
    title: &'static str,
    detail: &'static str,
    request_id: String,
) -> Response {
    (
        status,
        [
            (axum::http::header::CONTENT_TYPE, "application/problem+json"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        axum::Json(ProblemDetails {
            r#type: "about:blank",
            title,
            status: status.as_u16(),
            detail,
            request_id,
        }),
    )
        .into_response()
}

impl FixedWindowRateLimit {
    fn new(limit: u64) -> Self {
        Self {
            state: AtomicU64::new(current_minute() << 32),
            limit,
        }
    }

    fn admit(&self) -> bool {
        let minute = current_minute();
        let mut observed = self.state.load(Ordering::Acquire);
        loop {
            let observed_minute = observed >> 32;
            let count = observed & u64::from(u32::MAX);
            if observed_minute == minute && count >= self.limit {
                return false;
            }
            let next = if observed_minute == minute {
                observed + 1
            } else {
                (minute << 32) | 1
            };
            match self.state.compare_exchange_weak(
                observed,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => observed = actual,
            }
        }
    }
}

fn current_minute() -> u64 {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 60)
        & u64::from(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use ipnet::IpNet;
    use tower::ServiceExt;
    use url::Url;

    #[test]
    fn fixed_window_rejects_after_the_bound() {
        let limiter = FixedWindowRateLimit::new(2);
        assert!(limiter.admit());
        assert!(limiter.admit());
        assert!(!limiter.admit());
    }

    #[tokio::test]
    async fn grpc_streams_are_not_subject_to_the_rest_aggregate_body_limit() {
        let readiness = Arc::new(Readiness::default());
        readiness.set(true, true);
        readiness.set_setup(true);
        let transport = fixture_transport();
        let read_body = || async |body: axum::body::Bytes| body;
        let rest = Router::new().route("/api/v1/test", axum::routing::post(read_body()));
        let grpc = Router::new().route(
            "/citadel.edge.v1.EdgeAgentService/Connect",
            axum::routing::post(read_body()),
        );
        let app = secure_router_with_grpc(rest, &transport, readiness, Some(grpc)).unwrap();
        for (path, expected) in [
            ("/api/v1/test", StatusCode::PAYLOAD_TOO_LARGE),
            ("/citadel.edge.v1.EdgeAgentService/Connect", StatusCode::OK),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri(path)
                        .body(Body::from(vec![0; transport.body_limit_bytes + 1]))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{path}");
        }
    }

    #[tokio::test]
    async fn setup_gate_returns_problem_details_and_preserves_health() {
        let readiness = Arc::new(Readiness::default());
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/api/v1/private",
                get(|| async { "private" }).post(|_: axum::body::Bytes| async { "private" }),
            );
        let app = secure_router(app, &fixture_transport(), Arc::clone(&readiness)).unwrap();

        let blocked = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/private")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            blocked
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap(),
            "application/problem+json"
        );
        assert!(blocked.headers().contains_key(&REQUEST_ID));

        readiness.set(true, false);
        let setup_required = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/private")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(setup_required.status(), StatusCode::CONFLICT);
        assert_eq!(
            setup_required.headers()[axum::http::header::CACHE_CONTROL],
            "no-store"
        );
        let correlation = setup_required.headers()[&REQUEST_ID]
            .to_str()
            .unwrap()
            .to_owned();
        let body = axum::body::to_bytes(setup_required.into_body(), 4096)
            .await
            .unwrap();
        let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(problem["type"], "setup_required");
        assert_eq!(problem["requestId"], correlation);

        readiness.set_setup(true);
        let too_large = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/private")
                    .body(Body::from(vec![0_u8; 1_025]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            too_large
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap(),
            "application/problem+json"
        );

        let health = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn spa_deep_links_return_the_index_document_with_success() {
        let root = std::env::temp_dir().join(format!("citadel-spa-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("index.html"), "<html>citadel</html>").unwrap();

        let readiness = Arc::new(Readiness::default());
        readiness.set(true, true);
        readiness.set_setup(true);
        let mut transport = fixture_transport();
        transport.static_root = Some(root.clone());
        let app = secure_router(Router::new(), &transport, readiness).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/profile")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "<html>citadel</html>");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn unknown_api_routes_return_problem_details_instead_of_the_spa() {
        let root = std::env::temp_dir().join(format!("citadel-spa-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("index.html"), "<html>citadel</html>").unwrap();

        let readiness = Arc::new(Readiness::default());
        readiness.set(true, true);
        readiness.set_setup(true);
        let mut transport = fixture_transport();
        transport.static_root = Some(root.clone());
        let app = secure_router(Router::new(), &transport, readiness).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/not-implemented")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/problem+json"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reverse_proxy_headers_require_a_trusted_peer_and_https() {
        let mut transport = fixture_transport();
        transport.mode = TransportMode::ReverseProxy;
        transport.known_networks = vec!["10.0.0.0/24".parse::<IpNet>().unwrap()];
        let request = Request::builder()
            .header("x-forwarded-for", "192.0.2.2")
            .header("x-forwarded-proto", "https")
            .extension(ConnectInfo(SocketAddr::from(([10, 0, 0, 3], 1234))))
            .body(Body::empty())
            .unwrap();
        assert!(trusted_forwarding(&transport, &request));

        let request = Request::builder()
            .header("x-forwarded-for", "192.0.2.2")
            .header("x-forwarded-proto", "http")
            .extension(ConnectInfo(SocketAddr::from(([10, 0, 0, 3], 1234))))
            .body(Body::empty())
            .unwrap();
        assert!(!trusted_forwarding(&transport, &request));
    }

    fn fixture_transport() -> TransportConfig {
        TransportConfig {
            mode: TransportMode::Disabled,
            public_url: Url::parse("http://localhost:8000").unwrap(),
            edge_agent_public_url: Url::parse("http://localhost:8001").unwrap(),
            edge_grpc_port: 8001,
            allowed_hosts: Vec::new(),
            known_proxies: Vec::new(),
            known_networks: Vec::new(),
            forward_limit: 1,
            certificate_path: None,
            certificate_private_key_path: None,
            cors_origins: Vec::new(),
            body_limit_bytes: 1_024,
            requests_per_minute: 100,
            openapi_enabled: false,
            static_root: None,
        }
    }
}
