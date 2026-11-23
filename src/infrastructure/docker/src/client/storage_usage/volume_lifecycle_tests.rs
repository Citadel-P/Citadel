use super::*;
use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, Uri},
    routing::{get, post},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Mutex;
use tokio::{net::UnixListener, sync::Notify};

#[derive(Default)]
struct Daemon {
    same_second: bool,
    state: Mutex<(usize, bool)>,
    captured: Notify,
    release: Notify,
}

impl Daemon {
    fn volume(&self) -> Value {
        let (creation, exists) = *self.state.lock().unwrap();
        assert!(exists);
        let day = if self.same_second { 2 } else { creation };
        json!({"Name":"data", "Driver":"local", "CreatedAt":format!("2026-01-0{day}T00:00:00Z")})
    }
}

async fn scan(State(daemon): State<Arc<Daemon>>, uri: Uri) -> impl axum::response::IntoResponse {
    let mut volume = daemon.volume();
    let creation = daemon.state.lock().unwrap().0;
    volume["UsageData"] = json!({"Size":creation * 100, "RefCount":creation});
    if uri.query().unwrap().contains("type=image") {
        daemon.captured.notify_one();
        daemon.release.notified().await;
    }
    (
        [("date", "Fri, 02 Jan 2026 00:00:00 GMT")],
        Json(json!({"Volumes":[volume]})),
    )
}

#[derive(Clone, Copy)]
enum Mutation {
    Local,
    External { event: Option<&'static str> },
}

async fn verify_recreation(mutation: Mutation, same_second: bool) {
    let external_event = match mutation {
        Mutation::Local => None,
        Mutation::External { event } => event,
    };
    let daemon = Arc::new(Daemon {
        same_second,
        ..Default::default()
    });
    let routes = Router::new()
        .route(
            "/version",
            get(|| async { Json(json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})) }),
        )
        .route(
            "/v1.49/volumes/create",
            post(
                |State(d): State<Arc<Daemon>>, Json(body): Json<Value>| async move {
                    assert_eq!(body["Name"], "data");
                    {
                        let mut state = d.state.lock().unwrap();
                        assert!(!state.1);
                        *state = (state.0 + 1, true);
                    }
                    (StatusCode::CREATED, Json(d.volume()))
                },
            ),
        )
        .route(
            "/v1.49/volumes/data",
            get(|State(d): State<Arc<Daemon>>| async move { Json(d.volume()) }).delete(
                |State(d): State<Arc<Daemon>>| async move {
                    let mut state = d.state.lock().unwrap();
                    assert!(state.1);
                    state.1 = false;
                    StatusCode::NO_CONTENT
                },
            ),
        )
        .route(
            "/v1.49/volumes",
            get(
                |State(d): State<Arc<Daemon>>| async move { Json(json!({"Volumes":[d.volume()]})) },
            ),
        )
        .route("/v1.49/containers/json", get(|| async { Json(json!([])) }))
        .route("/v1.49/system/df", get(scan))
        .route(
            "/v1.49/events",
            get({
                let action = external_event.unwrap_or_default().to_owned();
                move || async move {
                    format!(
                        "{}\n",
                        json!({"Type":"volume","Action":action,"Actor":{"ID":"data"}})
                    )
                }
            }),
        )
        .with_state(daemon.clone());
    let path = std::env::temp_dir().join(format!("volume-{}.sock", uuid::Uuid::now_v7()));
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() });
    let client = DockerClient::new(&path, Duration::from_secs(5)).unwrap();
    let writer = if matches!(mutation, Mutation::External { .. }) {
        DockerClient::new(&path, Duration::from_secs(5)).unwrap()
    } else {
        client.clone()
    };
    let request = super::super::projection::VolumeCreateOptions {
        name: "data".into(),
        ..Default::default()
    };
    writer.create_volume(&request).await.unwrap();
    let (old_usage, ()) = tokio::join!(client.storage_totals(), async {
        daemon.captured.notified().await;
        let mut original = client.volume_metadata("data").await.unwrap();
        let mut compatible =
            Box::pin(client.fill_volume_usage(std::iter::once(&mut original), false));
        if !same_second {
            // This creation predates the scan. Its reader can share the held response.
            assert!(futures_util::poll!(&mut compatible).is_pending());
        }
        writer.delete_volume("data", false).await.unwrap();
        writer.create_volume(&request).await.unwrap();
        if let Some(action) = external_event {
            // Consume the raw event without starting the asynchronous enrichment.
            let mut events = client.events(None, None).await.unwrap();
            assert_eq!(events.next().await.unwrap().unwrap().action, action);
        }
        if matches!(mutation, Mutation::External { event: None }) {
            // No event has arrived. Prove the metadata read really joins the old
            // flight, then let its incompatible creation identity come back.
            let mut metadata = client.volume_metadata("data").await.unwrap();
            {
                let mut read =
                    Box::pin(client.fill_volume_usage(std::iter::once(&mut metadata), false));
                assert!(futures_util::poll!(&mut read).is_pending());
                daemon.release.notify_one();
                read.await.unwrap();
            }
            assert_eq!(metadata.usage_data, Some(json!({"Size":200,"RefCount":2})));
        }
        let (details, list) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::try_join!(client.volume_details("data"), client.list_volumes())
        })
        .await
        .expect("ordinary reads joined the held pre-recreation scan")
        .unwrap();
        for volume in [&details.0, &list[0]] {
            assert_eq!(volume.created_at, "2026-01-02T00:00:00Z");
            assert_eq!(volume.usage_data, Some(json!({"Size":200,"RefCount":2})));
        }
        daemon.release.notify_one();
        if !same_second {
            compatible.await.unwrap();
            assert_eq!(original.usage_data, Some(json!({"Size":100,"RefCount":1})));
        }
    });
    assert_eq!(old_usage.volumes, Some(100));
    server.abort();
    let _ = server.await;
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn details_and_list_read_recreated_volume_before_event_enrichment() {
    verify_recreation(Mutation::Local, true).await;
}

#[tokio::test]
async fn external_create_and_destroy_events_detach_old_scans_at_ingress() {
    for action in ["destroy", "create"] {
        verify_recreation(
            Mutation::External {
                event: Some(action),
            },
            true,
        )
        .await;
    }
}

#[tokio::test]
async fn metadata_creation_identity_rejects_shared_usage_before_an_external_event_arrives() {
    for same_second in [false, true] {
        verify_recreation(Mutation::External { event: None }, same_second).await;
    }
}
