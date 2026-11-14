use crate::direct::Runtime;
use axum::{
    Json, Router,
    http::{Method, StatusCode, Uri},
    routing::any,
};
use citadel_contracts::citadel::platforms::v1::{
    AgentUpdateDrainRequest, platform_service_server::PlatformService,
};
use citadel_docker::client::DockerClient;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

struct Documents {
    current: Value,
    source: Option<Value>,
    helper: Option<Value>,
    deleted: Vec<String>,
    lose_reply: Option<String>,
}

struct Fixture {
    documents: Arc<Mutex<Documents>>,
    docker: DockerClient,
    token: CancellationToken,
    server: tokio::task::JoinHandle<()>,
    request: AgentUpdateDrainRequest,
}
impl Fixture {
    async fn new() -> Self {
        let source = "a".repeat(64);
        let current = "b".repeat(64);
        let helper = "c".repeat(64);
        let operation = uuid::Uuid::now_v7().to_string();
        let image = format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "d".repeat(64));
        let helper_name = crate::updates::helper_name(&operation, &source);
        let documents = Arc::new(Mutex::new(Documents {
            current: json!({"Id": current, "Config": {"Image": image, "Labels": {
                "com.citadel.agent-update": operation, "com.citadel.system": "true", "com.citadel.system-role": "edge-agent"
            }}, "State": {"Running": true}}),
            source: Some(
                json!({"Id": source, "Image": "sha256:old", "Name": format!("/citadel-agent-previous-{operation}-{}", &source[..12]),
                "State": {"Running": false, "Status": "exited"}, "HostConfig": {"RestartPolicy": {"Name": "no"}}}),
            ),
            helper: Some(
                json!({"Id": helper, "Name": format!("/{helper_name}"), "Config": {"Image": image,
                "Entrypoint": ["/app/citadel-agent"], "Cmd": ["replace-agent", source, "sha256:old", image, operation],
                "Labels": {"com.citadel.system-role": "agent-updater", "com.citadel.agent-update": operation}},
                "State": {"Running": false, "Status": "exited", "ExitCode": 0}}),
            ),
            deleted: vec![],
            lose_reply: None,
        }));
        let state = documents.clone();
        let app = Router::new().route("/{*path}", any(move |method: Method, uri: Uri| {
            let state = state.clone();
            let helper_name = helper_name.clone();
            async move {
                let mut state = state.lock().unwrap();
                let path = uri.path();
                if path.ends_with("/version") {
                    return (StatusCode::OK, Json(json!({"Version": "28", "ApiVersion": "1.49", "MinAPIVersion": "1.41"})));
                }
                let resource = path.split("/containers/").nth(1).expect("container request");
                if method == Method::DELETE {
                    // Mutations must use full IDs and must neither force removal nor delete volumes.
                    let query: std::collections::HashMap<_, _> = url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).into_owned().collect();
                    for key in ["force", "v", "link"] {
                        assert_eq!(query.get(key).map(String::as_str), Some("false"));
                    }
                    assert!(resource == "a".repeat(64) || resource == "c".repeat(64));
                    state.deleted.push(resource.into());
                    if resource == "a".repeat(64) { state.source = None; } else { state.helper = None; }
                    if state.lose_reply.as_deref() == Some(resource) {
                        state.lose_reply = None;
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Deletion completed but response was lost"})));
                    }
                    return (StatusCode::NO_CONTENT, Json(Value::Null));
                }
                assert_eq!(method, Method::GET);
                let id = resource.strip_suffix("/json").unwrap();
                let document = if id == "b".repeat(64) { Some(state.current.clone()) }
                    else if id == "a".repeat(64) { state.source.clone() }
                    else if id == helper_name { state.helper.clone() }
                    else { panic!("unexpected inspect {id}") };
                match document {
                    Some(document) => (StatusCode::OK, Json(document)),
                    None => (StatusCode::NOT_FOUND, Json(json!({"message": "No such container"}))),
                }
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(5),
            "/unused-host",
        )
        .unwrap();
        let token = CancellationToken::new();
        let stop = token.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        Self {
            documents,
            docker,
            token,
            server,
            request: AgentUpdateDrainRequest {
                operation_id: operation,
                resume: true,
                source_container_id: source,
                source_image: "sha256:old".into(),
                cleanup: true,
            },
        }
    }
    fn runtime(&self) -> Runtime {
        Runtime::new(
            self.docker.clone(),
            self.token.clone(),
            Some("b".repeat(64)),
        )
    }
    async fn close(self) {
        self.token.cancel();
        self.server.await.unwrap();
    }
}

#[tokio::test]
async fn cleanup_removes_source_then_helper_and_retries_lost_replies_after_restart() {
    for lose_reply in [None, Some("a".repeat(64)), Some("c".repeat(64))] {
        let fixture = Fixture::new().await;
        fixture.documents.lock().unwrap().lose_reply = lose_reply.clone();
        let result = fixture
            .runtime()
            .drain_agent_update(fixture.request.clone())
            .await;
        assert_eq!(result.is_err(), lose_reply.is_some());
        if lose_reply.as_deref() == Some("a".repeat(64).as_str()) {
            assert!(
                fixture.documents.lock().unwrap().helper.is_some(),
                "helper evidence must survive partial deletion"
            );
        }
        // A new Runtime has no in-memory receipt. Core's durable success authorizes retries.
        assert!(
            fixture
                .runtime()
                .set_agent_update_drain(tonic::Request::new(fixture.request.clone()))
                .await
                .unwrap()
                .into_inner()
                .cleanup_completed
        );
        assert!(
            fixture
                .runtime()
                .set_agent_update_drain(tonic::Request::new(fixture.request.clone()))
                .await
                .unwrap()
                .into_inner()
                .cleanup_completed
        );
        {
            let documents = fixture.documents.lock().unwrap();
            assert!(documents.source.is_none() && documents.helper.is_none());
            assert_eq!(documents.deleted, ["a".repeat(64), "c".repeat(64)]);
        }
        let mut ordinary_settlement = fixture.request.clone();
        ordinary_settlement.cleanup = false;
        assert!(
            fixture
                .runtime()
                .drain_agent_update(ordinary_settlement)
                .await
                .is_err(),
            "absence must never prove replacement on the pre-commit path"
        );
        fixture.close().await;
    }
}

#[tokio::test]
async fn cleanup_rejects_changed_identity_running_containers_and_unverified_helpers() {
    for case in 0..16 {
        let fixture = Fixture::new().await;
        {
            let mut docs = fixture.documents.lock().unwrap();
            match case {
                0 => {
                    docs.current["Config"]["Labels"]["com.citadel.agent-update"] =
                        json!("another-operation")
                }
                1 => docs.current["State"]["Running"] = json!(false),
                2 => {
                    docs.current["Config"]["Labels"]["com.docker.swarm.service.id"] =
                        json!("service")
                }
                3 => docs.source.as_mut().unwrap()["Image"] = json!("sha256:foreign"),
                4 => docs.source.as_mut().unwrap()["Name"] = json!("/foreign"),
                5 => docs.source.as_mut().unwrap()["State"]["Running"] = json!(true),
                6 => {
                    docs.source.as_mut().unwrap()["HostConfig"]["RestartPolicy"]["Name"] =
                        json!("always")
                }
                7 => {
                    docs.helper.as_mut().unwrap()["Config"]["Labels"]["com.citadel.agent-update"] =
                        json!("another-operation")
                }
                8 => docs.helper.as_mut().unwrap()["State"]["ExitCode"] = json!(1),
                9 => docs.helper.as_mut().unwrap()["State"]["Running"] = json!(true),
                10 => docs.helper.as_mut().unwrap()["Config"]["Cmd"][1] = json!("foreign-source"),
                11 => docs.helper.as_mut().unwrap()["Id"] = json!("b".repeat(64)),
                12 => docs.helper = None,
                13 => docs.current["Id"] = json!("a".repeat(64)),
                14 => docs.current["Id"] = Value::Null,
                15 => docs.source.as_mut().unwrap()["Id"] = json!("e".repeat(64)),
                _ => unreachable!(),
            }
        }
        assert!(
            fixture
                .runtime()
                .drain_agent_update(fixture.request.clone())
                .await
                .is_err(),
            "case {case}"
        );
        assert!(
            fixture.documents.lock().unwrap().deleted.is_empty(),
            "case {case} deleted containers"
        );
        fixture.close().await;
    }
}

#[tokio::test]
async fn cleanup_requires_explicit_standalone_resume_identity() {
    let fixture = Fixture::new().await;
    for case in 0..3 {
        let mut request = fixture.request.clone();
        match case {
            0 => request.resume = false,
            1 => request.source_container_id.clear(),
            2 => request.source_container_id = "short".into(),
            _ => unreachable!(),
        }
        assert_eq!(
            fixture
                .runtime()
                .drain_agent_update(request)
                .await
                .unwrap_err()
                .code(),
            tonic::Code::InvalidArgument
        );
    }
    assert!(fixture.documents.lock().unwrap().deleted.is_empty());
    fixture.close().await;
}
