use super::*;
use crate::direct::Runtime;
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind, platforms::v1::AgentUpdateDrainRequest,
};
use citadel_docker::client::DockerClient;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn edge_drain_allows_cleanup_to_release_existing_work_but_denies_new_admission() {
    let runtime = Runtime::new(
        DockerClient::new("/unused-agent-update-test.sock", Duration::from_secs(1)).unwrap(),
        CancellationToken::new(),
        None,
    );
    let work = runtime.heavy.clone().acquire_owned().await.unwrap();
    let operation = uuid::Uuid::now_v7();
    let (result, ()) = tokio::join!(
        runtime.drain_agent_update(AgentUpdateDrainRequest {
            operation_id: operation.to_string(),
            resume: false,
            ..Default::default()
        }),
        async {
            while !runtime.update_gate.is_closed() {
                tokio::task::yield_now().await;
            }
            for command in [
                EdgeCommandKind::ContainerLogsStream,
                EdgeCommandKind::ContainerStatsStream,
                EdgeCommandKind::ImageReconcileData,
                EdgeCommandKind::ImageReconcileBuild,
                EdgeCommandKind::ScriptReconcile,
            ] {
                assert!(runtime.update_admission(command).unwrap().is_none());
            }
            for command in [
                EdgeCommandKind::ImageReserveData,
                EdgeCommandKind::ImageBuildStream,
                EdgeCommandKind::ScriptRun,
            ] {
                assert!(runtime.update_admission(command).is_err());
            }
            drop(work);
        }
    );
    result.unwrap();
    assert!(runtime.update_gate.is_closed());
    runtime
        .drain_agent_update(AgentUpdateDrainRequest {
            operation_id: operation.to_string(),
            resume: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        runtime
            .update_admission(EdgeCommandKind::ImageReserveData)
            .is_ok()
    );
}

#[tokio::test]
async fn direct_cleanup_reaches_handlers_while_new_work_is_rejected() {
    use axum::{Router, middleware, routing::post};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let gate = Gate::default();
    let mut drain = gate.drain(uuid::Uuid::now_v7()).await.unwrap();
    drain.retain().unwrap();
    drop(drain);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let app = Router::new()
        .route(
            "/{*method}",
            post(move || {
                let counted = counted.clone();
                async move {
                    counted.fetch_add(1, Ordering::SeqCst);
                    axum::http::StatusCode::NO_CONTENT
                }
            }),
        )
        .layer(middleware::from_fn_with_state(gate, admission));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
            .unwrap()
    });
    let client = reqwest::Client::new();
    for method in [
        "citadel.containers.v1.ContainerService/StreamContainerLogs",
        "citadel.containers.v1.ContainerService/StreamContainerStats",
        "citadel.images.v1.ImageService/ReconcileData",
        "citadel.images.v1.ImageService/ReconcileBuild",
        "citadel.scripts.v1.ScriptService/Reconcile",
    ] {
        let response = client
            .post(format!("http://{address}/{method}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    }
    for method in [
        "citadel.images.v1.ImageService/ReserveData",
        "citadel.images.v1.ImageService/Build",
        "citadel.scripts.v1.ScriptService/Run",
    ] {
        let response = client
            .post(format!("http://{address}/{method}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.headers()["grpc-status"], "14");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 5);
    cancel.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn standalone_preparation_failure_settles_but_helper_dispatch_stays_owned() {
    use axum::{Json, Router, http::StatusCode, routing::any};
    use citadel_contracts::citadel::platforms::v1::UpdateAgentRequest;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // A fake daemon only: no real containers, images or host work are touched.
    for fail_pull in [true, false] {
        let source = "a".repeat(64);
        let document = json!({
            "Id": source, "Name": "/agent", "Image": "sha256:old",
            "State": {"Running": true},
            "Config": {"Image": "agent:old", "Env": [], "Labels": {
                "com.citadel.system": "true", "com.citadel.system-role": "agent"
            }},
            "HostConfig": {"NetworkMode": "bridge"},
            "Mounts": [{"Type": "bind", "Source": "/var/run/docker.sock",
                "Destination": "/var/run/docker.sock", "RW": true}]
        });
        let creates = Arc::new(AtomicUsize::new(0));
        let calls = creates.clone();
        let app = Router::new().route(
            "/{*path}",
            any(move |uri: axum::http::Uri| {
                let document = document.clone();
                let calls = calls.clone();
                async move {
                    let path = uri.path();
                    let body = if path.ends_with("/version") {
                        json!({"Version":"28", "ApiVersion":"1.49", "MinAPIVersion":"1.41"})
                    } else if path.ends_with("/info") {
                        json!({"ID":"daemon", "NCPU":2, "MemTotal":1024})
                    } else if path.ends_with(&format!("/containers/{}/json", "a".repeat(64))) {
                        document
                    } else if path.ends_with("/images/create") {
                        if fail_pull {
                            json!({"error":"pull failed"})
                        } else {
                            json!({"status":"complete"})
                        }
                    } else if path.ends_with("/containers/create") {
                        calls.fetch_add(1, Ordering::SeqCst);
                        // A daemon error is ambiguous once create was dispatched.
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"message":"lost create reply"})),
                        );
                    } else {
                        panic!("unexpected Docker request: {uri}");
                    };
                    (StatusCode::OK, Json(body))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(5),
            "/unused-host",
        )
        .unwrap();
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let runtime = Runtime::new(docker.clone(), cancel.clone(), Some(source.clone()));
        let operation = uuid::Uuid::now_v7();
        let request = UpdateAgentRequest {
            operation_id: operation.to_string(),
            installation_id: "daemon/agent".into(),
            source_image: "sha256:old".into(),
            target_image: format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "b".repeat(64)),
        };
        assert!(runtime.start_agent_update(request.clone()).await.is_err());
        let settle = AgentUpdateDrainRequest {
            operation_id: operation.to_string(),
            resume: true,
            source_container_id: source.clone(),
            source_image: "sha256:old".into(),
            cleanup: false,
        };
        assert_eq!(creates.load(Ordering::SeqCst), usize::from(!fail_pull));
        assert_eq!(
            runtime.drain_agent_update(settle.clone()).await.is_ok(),
            fail_pull
        );
        assert_eq!(
            runtime.drain_agent_update(settle.clone()).await.is_ok(),
            fail_pull
        );
        assert_eq!(runtime.update_gate.enter().is_ok(), fail_pull);
        assert!(
            runtime.start_agent_update(request).await.is_err(),
            "settled requests cannot replay"
        );
        assert_eq!(creates.load(Ordering::SeqCst), usize::from(!fail_pull));
        let mut wrong = settle.clone();
        wrong.operation_id = uuid::Uuid::now_v7().to_string();
        assert!(runtime.drain_agent_update(wrong).await.is_err());
        let mut wrong = settle.clone();
        wrong.source_image = "sha256:changed".into();
        assert!(runtime.drain_agent_update(wrong).await.is_err());
        // Process loss must not turn helper absence into proof of cancellation.
        let restarted = Runtime::new(docker, cancel.clone(), Some(source));
        assert!(restarted.drain_agent_update(settle).await.is_err());
        cancel.cancel();
        server.await.unwrap();
    }
}
