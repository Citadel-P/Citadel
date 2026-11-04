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
            resume: false
        }),
        async {
            while !runtime.update_gate.is_closed() {
                tokio::task::yield_now().await;
            }
            for command in [
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
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    cancel.cancel();
    server.await.unwrap();
}
