use super::*;
use citadel_contracts::citadel::stacks::v1::{StackCommand, StackSecretFile};
use citadel_docker::client::DockerClient;
use futures_util::StreamExt;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;

struct Fixture {
    runtime: Runtime,
    root: PathBuf,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.runtime.shutdown.cancel();
        self.server.abort();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    async fn new(health: &'static str) -> Self {
        use axum::{Json, Router};
        use serde_json::json;
        let root =
            std::env::temp_dir().join(format!("citadel-stack-apply-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root).unwrap();
        let cli = root.join("docker");
        std::fs::write(&cli, "#!/bin/sh\ncase \" $* \" in\n*' up -d '*) echo 'Container counter-1 Started'; exit \"${FIXTURE_EXIT:-0}\" ;;\n*) exit 99 ;;\nesac\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(3),
            "/host",
        )
        .unwrap();
        let router = Router::new().fallback(move |request: axum::extract::Request| async move {
            Json(match request.uri().path() {
                "/version" => json!({"Version":"28", "ApiVersion":"1.49", "MinAPIVersion":"1.41"}),
                "/v1.49/containers/json" => json!([{"Id":"counter", "State":"running"}]),
                "/v1.49/containers/counter/json" => json!({"Id":"counter", "State":{"Status":"running", "Running":true, "Health":{"Status":health}}}),
                path => panic!("Unexpected Docker request {path}"),
            })
        });
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut runtime = Runtime::new(docker, CancellationToken::new(), None)
            .with_docker_cli(cli.display().to_string());
        runtime.stack_root = root.join("stacks");
        Self {
            runtime,
            root,
            server,
        }
    }

    fn request(&self) -> StackApplyRequest {
        StackApplyRequest {
            stack_name: "counter".into(),
            compose_file_content: Some(
                "services:\n  counter:\n    image: python:3.13-alpine\n".into(),
            ),
            generated_files_directory: Some(self.root.join("mounted").display().to_string()),
            secret_files: vec![StackSecretFile {
                name: "token".into(),
                target_path: "/run/secrets/token".into(),
                content: "fixture-secret".into(),
            }],
            secret_target_service_names: vec!["counter".into()],
            ..Default::default()
        }
    }

    async fn apply(&self, request: StackApplyRequest) -> Vec<Result<StackApplyResponse, Status>> {
        tokio::time::timeout(Duration::from_secs(10), async {
            self.runtime
                .apply(Request::new(request))
                .await
                .unwrap()
                .into_inner()
                .collect()
                .await
        })
        .await
        .expect("Stack apply must terminate")
    }
}

#[tokio::test]
async fn compose_completion_is_independent_of_startup_health_and_retains_mounts() {
    for health in ["starting", "unhealthy"] {
        let fixture = Fixture::new(health).await;
        let responses = fixture.apply(fixture.request()).await;
        assert!(responses.iter().all(Result::is_ok));
        let completed = responses.last().unwrap().as_ref().unwrap();
        assert_eq!(completed.exit_code, Some(0));
        assert_eq!(
            completed.stack_status.as_deref(),
            Some("Healthy"),
            "runtime health: {health}"
        );
        let mounted = fixture.root.join("mounted");
        let version = std::fs::read_to_string(mounted.join(".citadel-mounted-secrets")).unwrap();
        assert!(!version.is_empty());
        assert!(mounted.join("secrets").join(version).is_dir());
    }
}

#[tokio::test]
async fn compose_and_post_deploy_failures_remain_failed() {
    for post_deploy in [false, true] {
        let fixture = Fixture::new("healthy").await;
        let mut request = fixture.request();
        if post_deploy {
            request.post_deploy = Some(StackCommand {
                commands: vec!["exit 7".into()],
                path: ".".into(),
            });
        } else {
            request.environment_variables.push("FIXTURE_EXIT=7".into());
        }
        let responses = fixture.apply(request).await;
        assert!(responses.iter().all(Result::is_ok));
        assert!(
            responses
                .iter()
                .any(|r| r.as_ref().unwrap().exit_code == Some(7))
        );
        let completed = responses.last().unwrap().as_ref().unwrap();
        assert_eq!(completed.exit_code, Some(1));
        assert_eq!(completed.stack_status.as_deref(), Some("Failed"));
        assert!(
            !fixture
                .root
                .join("mounted/.citadel-mounted-secrets")
                .exists()
        );
    }
}

#[tokio::test]
async fn cancelled_apply_does_not_report_completion() {
    let fixture = Fixture::new("healthy").await;
    let mut request = fixture.request();
    request.pre_deploy = Some(StackCommand {
        commands: vec!["echo hook-ready; sleep 60".into()],
        path: ".".into(),
    });
    let mut stream = fixture
        .runtime
        .apply(Request::new(request))
        .await
        .unwrap()
        .into_inner();
    let responses: Vec<_> = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let frame = stream.next().await.unwrap().unwrap();
            if frame.message.as_deref() == Some("hook-ready") {
                break;
            }
        }
        fixture.runtime.shutdown.cancel();
        stream.collect().await
    })
    .await
    .expect("Cancellation must interrupt the running hook");
    assert_eq!(
        responses.last().unwrap().as_ref().unwrap_err().code(),
        tonic::Code::Cancelled
    );
    assert!(!responses.iter().any(|r| {
        r.as_ref()
            .is_ok_and(|r| r.stack_status.as_deref() == Some("Healthy"))
    }));
}
