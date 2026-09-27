use super::*;
use axum::{Json, response::IntoResponse};
use citadel_contracts::citadel::{
    containers::v1::{
        StreamContainerStatsRequest, StreamContainersStatsRequest,
        container_service_server::ContainerService,
    },
    platforms::v1::{PlatformStatsRequest, platform_service_server::PlatformService},
};
use futures_util::StreamExt;
use std::{sync::Mutex, time::Duration};
use tonic::Request;

async fn fixture(
    fail: Arc<std::sync::atomic::AtomicBool>,
) -> (
    Runtime,
    Arc<Mutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    fixture_with_metadata_delay(fail, Duration::ZERO).await
}

async fn fixture_with_metadata_delay(
    fail: Arc<std::sync::atomic::AtomicBool>,
    metadata_delay: Duration,
) -> (
    Runtime,
    Arc<Mutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let calls = Arc::new(Mutex::new(vec![]));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(3),
        "/host",
    )
    .unwrap();
    let router=Router::new().fallback({ let calls=calls.clone(); move |req:axum::extract::Request| { let calls=calls.clone(); let fail=fail.clone(); async move {
        calls.lock().unwrap().push(req.uri().to_string());
        if req.uri().path().ends_with("/system/df") { tokio::time::sleep(metadata_delay).await; }
        if req.uri().path().ends_with("/stats") && fail.load(std::sync::atomic::Ordering::SeqCst) { return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,"unavailable").into_response(); }
        if req.uri().path() == "/_ping" { return "OK".into_response(); }
        if req.method()==axum::http::Method::POST {return axum::http::StatusCode::NO_CONTENT.into_response();}
        let body=match req.uri().path() {
            "/version"=>serde_json::json!({"Version":"27","ApiVersion":"1.49","MinAPIVersion":"1.41"}),
            "/v1.49/info"=>serde_json::json!({"ID":"daemon","NCPU":2,"MemTotal":1024,"Driver":"overlay2","OperatingSystem":"fixture"}),
            "/v1.49/containers/json"=>serde_json::Value::Array((0..6).map(|i|serde_json::json!({"Id":format!("c{i}"),"State":"running","ImageID":"sha256:image"})).collect()),
            "/v1.49/volumes"=>serde_json::json!({"Volumes":[]}),
            "/v1.49/system/df"=>serde_json::json!({"LayersSize":0,"Volumes":[]}),
            "/v1.49/images/json"|"/v1.49/networks"=>serde_json::json!([]),
            p if p.ends_with("/stats")=>serde_json::json!({"read":"2026-09-27T00:00:00Z"}),
            _=>panic!("unexpected request {}",req.uri()),
        };
        Json(body).into_response()
    }}});
    let stop = CancellationToken::new();
    let server_stop = stop.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(server_stop.cancelled_owned())
            .await
            .unwrap();
    });
    (Runtime::new(docker, stop, None), calls, server)
}
#[tokio::test]
async fn both_stats_rpcs_share_one_six_container_cycle_and_cancel_independently() {
    let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (runtime, calls, server) = fixture(fail.clone()).await;
    let mut containers = runtime
        .stream_containers_stats(Request::new(StreamContainersStatsRequest {
            fetch_interval_ms: 10_000,
        }))
        .await
        .unwrap()
        .into_inner();
    let mut platform = runtime
        .stream_platform_stats(Request::new(PlatformStatsRequest {
            fetch_interval_ms: 10_000,
        }))
        .await
        .unwrap()
        .into_inner();
    let (a, b) = tokio::join!(containers.next(), platform.next());
    assert_eq!(a.unwrap().unwrap().containers.len(), 6);
    b.unwrap().unwrap();
    let count = |part: &str| {
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.contains(part))
            .count()
    };
    assert_eq!(count("/containers/json"), 1);
    assert_eq!(count("/stats?"), 6);
    drop(containers);
    let cached = runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(cached.containers.stats.len(), 6);
    assert_eq!(count("/containers/json"), 1);
    runtime.samples.invalidate();
    runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(count("/containers/json"), 2);
    runtime.docker.invalidate_daemon().await;
    runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(count("/containers/json"), 3);
    assert_eq!(count("/stats?"), 18);
    runtime.samples.invalidate();
    fail.store(true, std::sync::atomic::Ordering::SeqCst);
    let failed = runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(failed.containers.failed_samples, 6);
    fail.store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        runtime
            .samples
            .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
            .await
            .unwrap()
            .containers
            .stats
            .len(),
        6
    );
    assert_eq!(
        count("/containers/json"),
        5,
        "failed samples must not be cached as zeroes"
    );
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(7)).await;
    tokio::time::resume();
    runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(count("/containers/json"), 6);
    drop(platform);
    runtime.shutdown.cancel();
    server.await.unwrap();
}
#[tokio::test]
async fn metadata_reads_info_once_and_single_stats_does_not_request_sizes() {
    let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (runtime, calls, server) = fixture(fail.clone()).await;
    let info = runtime
        .get_platform_info(Request::new(()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(info.operating_system, "fixture");
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.ends_with("/info"))
            .count(),
        1
    );
    let mut stream = runtime
        .stream_container_stats(Request::new(StreamContainerStatsRequest {
            container_id: "c5".into(),
            fetch_interval_ms: 10_000,
        }))
        .await
        .unwrap()
        .into_inner();
    stream.next().await.unwrap().unwrap();
    for uri in calls
        .lock()
        .unwrap()
        .iter()
        .filter(|p| p.contains("/containers/json"))
    {
        assert!(!uri.contains("size=true"), "{uri}");
    }
    drop(stream);
    runtime.shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn agent_command_defaults_match_local_and_invalidate_shared_samples() {
    use citadel_contracts::citadel::containers::v1::ContainerIds;
    let (runtime, calls, server) =
        fixture(Arc::new(std::sync::atomic::AtomicBool::new(false))).await;
    runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    runtime
        .stop(Request::new(ContainerIds {
            ids: vec!["c0".into()],
        }))
        .await
        .unwrap();
    runtime
        .restart(Request::new(ContainerIds {
            ids: vec!["c0".into()],
        }))
        .await
        .unwrap();
    runtime
        .samples
        .sample(std::time::Duration::from_secs(6), &runtime.shutdown)
        .await
        .unwrap();
    {
        let calls = calls.lock().unwrap();
        assert!(
            calls
                .iter()
                .any(|uri| uri == "/v1.49/containers/c0/stop?signal=SIGTERM&t=10")
        );
        assert!(
            calls
                .iter()
                .any(|uri| uri == "/v1.49/containers/c0/restart?signal=SIGINT&t=5")
        );
        assert_eq!(
            calls
                .iter()
                .filter(|uri| uri.contains("/containers/json"))
                .count(),
            2
        );
    }
    runtime.shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn inventory_metadata_mode_avoids_stats_and_legacy_default_keeps_them() {
    use citadel_contracts::citadel::containers::v1::ListContainersRequest;
    let (runtime, calls, server) =
        fixture(Arc::new(std::sync::atomic::AtomicBool::new(false))).await;
    let response = runtime
        .list(Request::new(ListContainersRequest {
            all: Some(true),
            metadata_only: true,
            ..Default::default()
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.containers.len(), 6);
    assert!(
        response
            .containers
            .values()
            .all(|c| c.container_stat_message.is_none())
    );
    assert!(!calls.lock().unwrap().iter().any(|p| p.contains("/stats?")));
    runtime
        .list(Request::new(ListContainersRequest {
            all: Some(true),
            ..Default::default()
        }))
        .await
        .unwrap();
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.contains("/stats?"))
            .count(),
        6
    );
    runtime.shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn requested_freshness_expires_shared_sample_before_six_seconds() {
    let (runtime, calls, server) =
        fixture(Arc::new(std::sync::atomic::AtomicBool::new(false))).await;
    runtime
        .samples
        .sample(Duration::from_secs(1), &runtime.shutdown)
        .await
        .unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(2)).await;
    tokio::time::resume();
    runtime
        .samples
        .sample(Duration::from_secs(1), &runtime.shutdown)
        .await
        .unwrap();
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.contains("/containers/json"))
            .count(),
        2
    );
    runtime.shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn slow_metadata_never_blocks_fast_container_cycles() {
    let (runtime, calls, server) = fixture_with_metadata_delay(
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        Duration::from_secs(5),
    )
    .await;
    let first = tokio::time::timeout(
        Duration::from_secs(2),
        runtime
            .samples
            .sample(Duration::from_secs(1), &runtime.shutdown),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(first.containers.stats.len(), 6);
    tokio::time::timeout(Duration::from_secs(2), async {
        while !calls
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.contains("/system/df"))
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    runtime.samples.invalidate();
    let second = tokio::time::timeout(
        Duration::from_secs(2),
        runtime
            .samples
            .sample(Duration::from_secs(1), &runtime.shutdown),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(second.containers.stats.len(), 6);
    assert!(second.platform.is_none());
    runtime.shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn agent_mutations_share_daemon_permits_and_do_not_block_health() {
    use citadel_contracts::citadel::containers::v1::ContainerIds;
    let (runtime, calls, server) =
        fixture(Arc::new(std::sync::atomic::AtomicBool::new(false))).await;
    let permits = runtime.mutations.acquire_many(8).await.unwrap();
    let task = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            runtime
                .start(Request::new(ContainerIds {
                    ids: vec!["c0".into()],
                }))
                .await
        }
    });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    runtime.check_health(Request::new(())).await.unwrap();
    assert!(!calls.lock().unwrap().iter().any(|p| p.contains("/start")));
    drop(permits);
    task.await.unwrap().unwrap();
    assert_eq!(runtime.mutations.available_permits(), 8);
    runtime.shutdown.cancel();
    server.await.unwrap();
}
