use super::*;
use axum::{Json, Router, response::IntoResponse};
use citadel_adapters::connectors::{
    agent::client::decode_daemon_event,
    docker::{DockerClient, projection::DockerEvent},
};
use platform_service_server::PlatformService;
use prost::Message;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

/// Exercise the actual Direct stream also dispatched by Edge, then decode its
/// protobuf bytes through Core's Edge boundary. The Local classifier is the oracle.
#[tokio::test]
async fn daemon_stream_matches_local_semantics_and_ignored_events_do_no_io() {
    let fixtures = [
        ("container", "exec_create: sh", "local"),
        ("container", "attach", "local"),
        ("container", "top", "local"),
        ("container", "kill", "local"),
        ("container", "die", "local"),
        ("container", "stop", "local"),
        ("network", "disconnect", "local"),
        ("network", "connect", "local"),
        ("container", "create", "local"),
        ("container", "pause", "local"),
        ("container", "unpause", "local"),
        ("container", "start", "local"),
        ("container", "restart", "local"),
        ("container", "destroy", "local"),
        ("image", "delete", "local"),
        ("volume", "destroy", "local"),
        ("network", "destroy", "local"),
        ("network", "update", "swarm"),
        ("service", "update", "swarm"),
        ("task", "update", "swarm"),
        ("node", "update", "swarm"),
    ]
    .map(|(resource, action, scope)| {
        serde_json::from_value::<DockerEvent>(serde_json::json!({
            "Type": resource, "Action": action, "scope": scope,
            "Actor": { "ID": "fixture", "Attributes": {"com.docker.swarm.task.id": "task"} }
        }))
        .unwrap()
    });
    let body = fixtures
        .iter()
        .map(|e| format!("{}\n", serde_json::to_string(e).unwrap()))
        .collect::<String>();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(2),
        "/host",
    )
    .unwrap();
    let router = Router::new().fallback({
        let calls = calls.clone();
        move |request: axum::extract::Request| {
            let body = body.clone();
            let calls = calls.clone();
            async move {
                calls.lock().unwrap().push(request.uri().to_string());
                match request.uri().path() {
                    "/version" => Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                    "/v1.49/events" => body.into_response(),
                    "/v1.49/containers/json" => Json(serde_json::json!([{
                        "Id":"fixture", "Names":["/web"], "Image":"alpine", "ImageID":"sha256:image",
                        "State":"running", "Created":123, "Labels":{}, "Ports":[]
                    }])).into_response(),
                    _ => panic!("unexpected observation: {}", request.uri()),
                }
            }
        }
    });
    let stop = CancellationToken::new();
    let server_stop = stop.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(server_stop.cancelled_owned())
            .await
            .unwrap();
    });
    let runtime = Runtime::new(docker, stop.clone(), None);
    let mut stream = runtime
        .stream_daemon_event(Request::new(()))
        .await
        .unwrap()
        .into_inner();
    for raw in &fixtures {
        let Some(expected) = events::normalize(raw) else {
            continue;
        };
        let response = tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(response.scope, if raw.scope == "swarm" { 2 } else { 1 });
        let decoded = decode_daemon_event(&response.encode_to_vec())
            .unwrap()
            .unwrap();
        assert_eq!(decoded.kind, expected, "{}", raw.action);
        assert_eq!(decoded.action, raw.action);
        if let RuntimeEventKind::Container(change) = expected {
            assert_eq!(decoded.container_id.as_deref(), Some("fixture"));
            assert_eq!(decoded.container_state.as_deref(), change.state());
            if change == ContainerChange::Tombstone {
                assert!(decoded.container.is_none());
            } else {
                let container = decoded.container.unwrap();
                assert_eq!(container.state, change.state().unwrap());
                assert_eq!(container.name, "web");
                assert_eq!(container.image_id, "sha256:image");
                assert!(container.is_swarm_task);
            }
        }
    }
    assert!(stream.next().await.is_none());
    let calls = calls.lock().unwrap().clone();
    let observations: Vec<_> = calls
        .iter()
        .filter(|url| url.starts_with("/v1.49/containers/json"))
        .collect();
    assert_eq!(observations.len(), 5, "{calls:?}");
    for request in observations {
        let url = reqwest::Url::parse(&format!("http://fixture{request}")).unwrap();
        let filters = url
            .query_pairs()
            .find(|(key, _)| key == "filters")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&filters).unwrap(),
            serde_json::json!({"id":["fixture"]})
        );
    }
    assert_eq!(
        calls.len(),
        7,
        "ignored events and tombstones must not observe Docker: {calls:?}"
    );
    stop.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn platform_metadata_rpc_does_not_sample_stats_or_enumerate_resources() {
    use citadel_contracts::citadel::platforms::v1::platform_service_server::PlatformService;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let path = std::env::temp_dir().join(format!(
        "citadel-agent-metadata-{}.sock",
        uuid::Uuid::now_v7()
    ));
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 2048];
            while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
            }
            let text = String::from_utf8(request).unwrap();
            let path = text.split_whitespace().nth(1).unwrap();
            let body = match path {
                "/version" => r#"{"Version":"28","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                "/v1.49/info" => {
                    r#"{"ID":"fixture","NCPU":2,"MemTotal":1024,"Swarm":{"ControlAvailable":true,"NodeID":"manager","LocalNodeState":"active","Cluster":{"ID":"cluster"}}}"#
                }
                unexpected => {
                    panic!("metadata must not call resource/statistics APIs: {unexpected}")
                }
            };
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let cancel = tokio_util::sync::CancellationToken::new();
    let docker = citadel_adapters::connectors::docker::DockerClient::new(
        &path,
        std::time::Duration::from_secs(2),
    )
    .unwrap();
    let runtime = super::super::Runtime::new(docker, cancel, None);
    let result = runtime
        .get_platform_info(tonic::Request::new(()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result.id, "fixture");
    assert_eq!(result.cpu_count, 2);
    assert!(result.platform_stat.is_none());
    assert!(result.swarm_info.unwrap().service_count.is_none());
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn failed_container_observations_are_absent_and_missing_image_ids_are_inspected() {
    for mode in ["error", "missing", "fallback"] {
        let inspected = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(2),
            "/host",
        )
        .unwrap();
        let router=Router::new().fallback({let inspected=inspected.clone();move |req:axum::extract::Request| {let inspected=inspected.clone();async move {
            match req.uri().path() {
                "/version"=>Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                "/v1.49/events"=>format!("{}\n",serde_json::json!({"Type":"container","Action":"start","Actor":{"ID":"fixture"}})).into_response(),
                "/v1.49/containers/json"=>match mode {
                    "error"=>(axum::http::StatusCode::INTERNAL_SERVER_ERROR,"failed").into_response(),
                    "missing"=>Json(serde_json::json!([])).into_response(),
                    _=>Json(serde_json::json!([{"Id":"fixture","Names":["/preserved"],"ImageID":"","State":"running"}])).into_response(),
                },
                "/v1.49/containers/fixture/json"=>{inspected.fetch_add(1,std::sync::atomic::Ordering::SeqCst);Json(serde_json::json!({"Id":"fixture","Image":"sha256:recovered"})).into_response()},
                _=>panic!("unexpected {}",req.uri()),
            }
        }}});
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let runtime = Runtime::new(docker, cancel.clone(), None);
        let mut stream = runtime
            .stream_daemon_event(Request::new(()))
            .await
            .unwrap()
            .into_inner();
        let response = stream.next().await.unwrap().unwrap();
        let event = decode_daemon_event(&response.encode_to_vec())
            .unwrap()
            .unwrap();
        if mode == "fallback" {
            let container = event.container.unwrap();
            assert_eq!(container.image_id, "sha256:recovered");
            assert_eq!(container.name, "preserved");
            assert_eq!(inspected.load(std::sync::atomic::Ordering::SeqCst), 1);
        } else {
            assert!(event.container.is_none());
            assert_eq!(inspected.load(std::sync::atomic::Ordering::SeqCst), 0);
        }
        drop(stream);
        cancel.cancel();
        server.await.unwrap();
    }
}
