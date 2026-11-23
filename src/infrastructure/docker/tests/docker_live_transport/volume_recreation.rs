use super::*;
use axum::{
    Router,
    body::Bytes,
    http::{Method, StatusCode, Uri},
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::test]
#[ignore = "requires isolated CITADEL_DOCKER_FIXTURE_TCP daemon and CITADEL_DOCKER_FIXTURE_IMAGE"]
async fn recreated_volume_details_do_not_wait_for_pre_recreation_telemetry() {
    let address = std::env::var("CITADEL_DOCKER_FIXTURE_TCP").unwrap();
    let image = std::env::var("CITADEL_DOCKER_FIXTURE_IMAGE").unwrap();
    let name = format!("recreated-{}", uuid::Uuid::now_v7());
    let path = std::env::temp_dir().join(format!("{name}.sock"));
    let captured = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    // Buffer the real daemon's reply after it has counted the old attachment.
    let proxy = Router::new().fallback({
        let (captured, release, name) = (captured.clone(), release.clone(), name.clone());
        let http = reqwest::Client::new();
        move |method: Method, uri: Uri, body: Bytes| {
            let request = http.request(method, format!("http://{address}{uri}"));
            let (captured, release, name) = (captured.clone(), release.clone(), name.clone());
            async move {
                let reply = request
                    .header("content-type", "application/json")
                    .body(body)
                    .send()
                    .await
                    .unwrap();
                let status = reply.status();
                let body = reply.bytes().await.unwrap();
                if uri.path().ends_with("/system/df")
                    && uri.query().is_some_and(|q| q.contains("type=image"))
                {
                    assert_eq!(status, StatusCode::OK);
                    let snapshot: Value = serde_json::from_slice(&body).unwrap();
                    let old = snapshot["Volumes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|v| v["Name"] == name)
                        .unwrap();
                    assert_eq!(old["UsageData"]["RefCount"], 1);
                    captured.notify_one();
                    release.notified().await;
                }
                (status, [("content-type", "application/json")], body).into_response()
            }
        }
    });
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, proxy).await.unwrap() });
    let client = DockerClient::new(&path, Duration::from_secs(15)).unwrap();
    let options = serde_json::from_value(json!({"Name":name,"Driver":"local"})).unwrap();
    client.create_volume(&options).await.unwrap();
    let container = client
        .create_container(
            &name,
            &json!({"Image":image,"HostConfig":{"Binds":[format!("{name}:/data")]}}),
        )
        .await
        .unwrap();
    let (stats, ()) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(client.platform_stats(), async {
            captured.notified().await;
            client
                .delete_container(&container, true, false)
                .await
                .unwrap();
            client.delete_volume(&name, false).await.unwrap();
            client.create_volume(&options).await.unwrap();
            let (details, volumes) =
                tokio::try_join!(client.volume_details(&name), client.list_volumes()).unwrap();
            let listed = volumes.into_iter().find(|v| v.name == name).unwrap();
            for volume in [details.0, listed] {
                assert_eq!(volume.usage_data.unwrap()["RefCount"], 0);
            }
            release.notify_one();
        })
    })
    .await
    .expect("ordinary reads should finish while old telemetry is held");
    stats.unwrap();
    client.delete_volume(&name, false).await.unwrap();
    server.abort();
    let _ = server.await;
    std::fs::remove_file(path).unwrap();
}
