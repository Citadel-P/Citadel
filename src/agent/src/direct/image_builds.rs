use super::{Runtime, build_admission::AdmissionRefusal};
use base64::Engine;
use citadel_builds::{BuildArgSpec, BuildError, BuildLogSink, BuildRegistryCredentials};
use citadel_contracts::citadel::images::v1::{
    BuildAdmissionResult, BuildImageRequest, ImageBuildResponse, PushImageRequest,
};
use citadel_docker::builds::runtime::BuildRuntimeError;
use citadel_docker::builds::runtime::DockerBuildOptions;
use citadel_docker::builds::runtime::DockerBuildSession;
use citadel_docker::builds::runtime::DockerBuildSource;
use futures_util::{future::BoxFuture, stream::BoxStream};
use std::{path::Path, time::Duration};
use tokio::sync::mpsc;
use tonic::{Response, Status};
use zeroize::Zeroizing;
type Output = Response<BoxStream<'static, Result<ImageBuildResponse, Status>>>;
struct Progress(mpsc::Sender<ImageBuildResponse>, usize);
impl BuildLogSink for Progress {
    fn append<'a>(&'a self, _: &'a str, message: &'a str) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            self.0
                .send(ImageBuildResponse {
                    stream: Some(
                        message[..message.floor_char_boundary(self.1.min(message.len()))].into(),
                    ),
                    ..Default::default()
                })
                .await
                .map_err(|_| BuildError::Conflict("Build stream closed".into()))
        })
    }
}
type RegistryAuth = (String, BuildRegistryCredentials, Option<Zeroizing<String>>);
fn credentials(value: Option<&str>) -> Result<Option<RegistryAuth>, Status> {
    let Some(value) = value.filter(|v| !v.trim().is_empty()) else {
        return Ok(None);
    };
    let bytes = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(value)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(value))
            .map_err(|_| Status::invalid_argument("Invalid registry authentication encoding"))?,
    );
    #[derive(serde::Deserialize)]
    struct Auth {
        #[serde(default)]
        username: String,
        #[serde(default)]
        password: String,
        #[serde(default)]
        serveraddress: String,
        #[serde(default)]
        identitytoken: String,
    }
    let v: Auth = serde_json::from_slice(&bytes)
        .map_err(|_| Status::invalid_argument("Invalid registry authentication"))?;
    Ok(Some((
        v.serveraddress,
        BuildRegistryCredentials {
            username: v.username,
            password: Zeroizing::new(v.password),
        },
        (!v.identitytoken.is_empty()).then(|| Zeroizing::new(v.identitytoken)),
    )))
}
fn failure(e: BuildRuntimeError) -> ImageBuildResponse {
    ImageBuildResponse {
        error_message: Some(e.to_string()),
        status: Some("error".into()),
        ..Default::default()
    }
}
pub(super) async fn build(runtime: &Runtime, r: BuildImageRequest) -> Result<Output, Status> {
    let registry = credentials(r.registry_auth.as_deref())?;
    let pushes = if r.push_to_registry {
        r.tags.clone()
    } else {
        vec![]
    };
    let mut permit = match runtime.builds.build(
        &r.run_id,
        &r.attempt_id,
        pushes,
        &r.admission_authority_id,
        r.admission_generation,
    )? {
        Ok(permit) => permit,
        Err(refusal) => {
            return Ok(Response::new(Box::pin(futures_util::stream::once(
                async move {
                    Ok(ImageBuildResponse {
                        admission: match refusal {
                            AdmissionRefusal::Fenced => {
                                BuildAdmissionResult::BuildAdmissionFencedNotStarted
                            }
                            AdmissionRefusal::CapacityBusy => {
                                BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted
                            }
                        } as i32,
                        ..Default::default()
                    })
                },
            ))));
        }
    };
    let endpoint = runtime.docker.endpoint().clone();
    let docker_cli = runtime.docker_cli.clone();
    let cancel = runtime.shutdown.child_token();
    let guard = cancel.clone().drop_guard();
    let (tx, mut rx) = mpsc::channel(8);
    Ok(Response::new(Box::pin(async_stream::try_stream! {
        let _guard=guard;
        yield ImageBuildResponse { admission: BuildAdmissionResult::BuildAdmissionAccepted as i32, ..Default::default() };
        permit.start();
        let operation=async {
            let progress=Progress(tx, if r.max_line_bytes<=0 {16384} else {r.max_line_bytes.min(1024*1024) as usize});
            let host=r.registry_host.as_deref().filter(|v|!v.is_empty()).or_else(||registry.as_ref().map(|v|v.0.as_str()).filter(|v|!v.is_empty())).unwrap_or("docker.io");
            let mut session=DockerBuildSession::open(docker_cli,endpoint,registry.as_ref().filter(|v|v.2.is_none()).map(|v|(host,&v.1)),Duration::from_secs(r.timeout_seconds.max(1) as u64),4*1024*1024,&cancel).await?;
            if let Some(token)=registry.as_ref().and_then(|v|v.2.as_ref())
                && let Err(error) = session.set_identity_token(host,token.clone()) {
                session.cleanup()?;
                return Err(error.not_started());
            }
            let args:Vec<_>=r.build_args.into_iter().map(|(name,value)|BuildArgSpec{name,value:Some(value)}).collect();
            let secrets:Vec<_>=r.build_secrets.into_iter().map(|v|(v.id,Zeroizing::new(v.value))).collect();
            let source=if r.context_archive.is_empty(){DockerBuildSource::Directory{context:Path::new(&r.context_directory),dockerfile:Path::new(&r.dockerfile_path)}}else{DockerBuildSource::Archive{data:r.context_archive,dockerfile:r.dockerfile_archive_path.as_deref().unwrap_or("Dockerfile")}};
            let result=session.build(DockerBuildOptions{source,tags:&r.tags,build_args:&args,target:r.target.as_deref(),secrets:&secrets},&progress,&cancel).await;
            session.cleanup()?;
            result
        };
        tokio::pin!(operation);
        let outcome=loop{tokio::select!{result=&mut operation=>break result,Some(item)=rx.recv()=>yield item}};
        if outcome.is_ok() || outcome.as_ref().is_err_and(|error| error.execution_finished()) { permit.complete(outcome.is_ok()); }
        while let Ok(item)=rx.try_recv(){yield item;}
        match outcome{Ok(())=>yield ImageBuildResponse{status:Some("completed".into()),..Default::default()},Err(error)=>yield failure(error)}
    })))
}
pub(super) async fn push(runtime: &Runtime, r: PushImageRequest) -> Result<Output, Status> {
    let registry = credentials(r.registry_auth.as_deref())?;
    let mut permit = runtime.builds.push(
        &r.run_id,
        &r.attempt_id,
        &r.image_reference,
        &r.admission_authority_id,
    )?;
    let endpoint = runtime.docker.endpoint().clone();
    let docker_cli = runtime.docker_cli.clone();
    let cancel = runtime.shutdown.child_token();
    let guard = cancel.clone().drop_guard();
    let (tx, mut rx) = mpsc::channel(8);
    Ok(Response::new(Box::pin(async_stream::try_stream! {
        let _guard=guard;
        permit.start();
        let operation=async {
            let progress=Progress(tx,16384);
            let first=r.image_reference.split('/').next().unwrap_or("docker.io");
            let host=registry.as_ref().map(|v|v.0.as_str()).filter(|v|!v.is_empty()).unwrap_or(if first.contains('.')||first.contains(':')||first=="localhost"{first}else{"docker.io"});
            let mut session=DockerBuildSession::open(docker_cli,endpoint,registry.as_ref().filter(|v|v.2.is_none()).map(|v|(host,&v.1)),Duration::from_secs(3600),4*1024*1024,&cancel).await?;
            if let Some(token)=registry.as_ref().and_then(|v|v.2.as_ref())
                && let Err(error) = session.set_identity_token(host,token.clone()) {
                session.cleanup()?;
                return Err(error.not_started());
            }
            let result=session.push(&r.image_reference,&progress,&cancel).await;
            session.cleanup()?;
            result
        };
        tokio::pin!(operation);
        let outcome=loop{tokio::select!{result=&mut operation=>break result,Some(item)=rx.recv()=>yield item}};
        if outcome.is_ok() || outcome.as_ref().is_err_and(|error| error.execution_finished()) { permit.complete(outcome.is_ok()); }
        while let Ok(item)=rx.try_recv(){yield item;}
        match outcome{Ok(digest)=>yield ImageBuildResponse{status:Some("completed".into()),id:digest,..Default::default()},Err(error)=>yield failure(error)}
    })))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(script: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("citadel-agent-build-test-{}", uuid::Uuid::now_v7()));
            std::fs::create_dir(&root).unwrap();
            let path = root.join("docker");
            std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{script}")).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(root)
        }
        fn runtime(&self) -> Runtime {
            let mut runtime = Runtime::new(
                citadel_docker::client::DockerClient::with_endpoint(
                    "tcp://fixture:2375".parse().unwrap(),
                    Duration::from_secs(3),
                    "/host",
                )
                .unwrap(),
                tokio_util::sync::CancellationToken::new(),
                None,
            );
            runtime.docker_cli = self.0.join("docker").display().to_string();
            runtime
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn request(runtime: &Runtime) -> BuildImageRequest {
        BuildImageRequest {
            admission_authority_id: runtime.builds.authority().unwrap(),
            admission_generation: runtime.builds.generation().unwrap(),
            run_id: uuid::Uuid::now_v7().to_string(),
            attempt_id: uuid::Uuid::now_v7().to_string(),
            context_archive: vec![0, 255, 1],
            dockerfile_archive_path: Some("Dockerfile".into()),
            tags: vec!["fixture:latest".into()],
            timeout_seconds: 5,
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn command_contexts_share_a_slot_through_every_push() {
        let f = Fixture::new(
            r#"root=$(dirname "$0")
printf '%s\n' "$1" >> "$root/calls"
if [ "$1" = build ]; then cat >/dev/null; fi"#,
        );
        let mut runtime = f.runtime().with_build_capacity(1);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stopped = tokio_util::sync::CancellationToken::new();
        let server_cancel = stopped.clone();
        let calls = f.0.join("calls");
        let server = tokio::spawn(async move {
            axum::serve(listener,axum::Router::new().fallback(move |uri:axum::http::Uri| {
                let calls=calls.clone();
                async move {
                    if uri.path()=="/version" { return axum::Json(serde_json::json!({"ApiVersion":"1.45","MinAPIVersion":"1.24"})); }
                    assert!(uri.path().ends_with("/push"));
                    use std::io::Write;
                    writeln!(std::fs::OpenOptions::new().append(true).open(calls).unwrap(),"push").unwrap();
                    axum::Json(serde_json::json!({"aux":{"Digest":format!("sha256:{}","a".repeat(64))}}))
                }
            })).with_graceful_shutdown(server_cancel.cancelled_owned()).await.unwrap();
        });
        runtime.docker = citadel_docker::client::DockerClient::with_endpoint(
            format!("http://{address}").parse().unwrap(),
            Duration::from_secs(3),
            "/host",
        )
        .unwrap();
        let next = runtime.for_command(tokio_util::sync::CancellationToken::new());
        let mut r = request(&runtime);
        r.tags = vec!["fixture:one".into(), "fixture:two".into()];
        r.push_to_registry = true;
        let frames: Vec<_> = build(&runtime, r.clone())
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
        assert_eq!(
            frames.last().unwrap().as_ref().unwrap().status.as_deref(),
            Some("completed")
        );
        assert!(
            build(&next, r.clone()).await.is_err(),
            "an existing attempt cannot execute twice"
        );
        let frames: Vec<_> = build(&next, request(&runtime))
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].as_ref().unwrap().admission,
            BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted as i32
        );
        assert_eq!(
            std::fs::read_to_string(f.0.join("calls")).unwrap(),
            "build\n"
        );
        for reference in r.tags {
            let frames: Vec<_> = push(
                &next,
                PushImageRequest {
                    admission_authority_id: r.admission_authority_id.clone(),
                    run_id: r.run_id.clone(),
                    attempt_id: r.attempt_id.clone(),
                    image_reference: reference,
                    registry_auth: None,
                },
            )
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
            assert_eq!(
                frames.last().unwrap().as_ref().unwrap().status.as_deref(),
                Some("completed")
            );
        }
        assert_eq!(
            std::fs::read_to_string(f.0.join("calls")).unwrap(),
            "build\npush\npush\n"
        );
        assert_eq!(runtime.builds.capacity().unwrap(), (1, 0, 0));
        stopped.cancel();
        server.await.unwrap();
    }
    #[tokio::test]
    async fn preparation_failures_release_slots_after_cleanup() {
        for scenario in ["login", "context", "secret", "token"] {
            let f = Fixture::new(
                r#"root=$(dirname "$0")
printf '%s' "$DOCKER_CONFIG" > "$root/config"
printf '%s\n' "$1" >> "$root/calls"
if [ "$1" = login ]; then cat >/dev/null; exit 1; fi
exit 99"#,
            );
            let runtime = f.runtime().with_build_capacity(1);
            for _ in 0..3 {
                let mut r = request(&runtime);
                match scenario {
                    "login" => {
                        r.registry_auth = Some(
                            base64::engine::general_purpose::STANDARD
                                .encode(br#"{"username":"builder","password":"private-password"}"#),
                        )
                    }
                    "context" => r.dockerfile_archive_path = Some("../Dockerfile".into()),
                    "secret" => {
                        r.build_secrets =
                            vec![citadel_contracts::citadel::images::v1::BuildImageSecret {
                                id: "invalid/id".into(),
                                value: "secret".into(),
                            }]
                    }
                    "token" => {
                        r.registry_auth = Some(base64::engine::general_purpose::STANDARD.encode(
                            br#"{"serveraddress":"invalid/host","identitytoken":"private-token"}"#,
                        ))
                    }
                    _ => unreachable!(),
                }
                let frames: Vec<_> = build(&runtime, r)
                    .await
                    .unwrap()
                    .into_inner()
                    .collect()
                    .await;
                assert_eq!(
                    frames.last().unwrap().as_ref().unwrap().status.as_deref(),
                    Some("error"),
                    "{scenario}"
                );
                assert_eq!(runtime.builds.capacity().unwrap(), (1, 0, 0), "{scenario}");
            }
            if scenario == "login" {
                assert_eq!(
                    std::fs::read_to_string(f.0.join("calls")).unwrap(),
                    "login\nlogin\nlogin\n"
                );
                assert!(!Path::new(&std::fs::read_to_string(f.0.join("config")).unwrap()).exists());
            } else {
                assert!(
                    !f.0.join("calls").exists(),
                    "validation must precede Docker execution"
                );
            }
        }
    }

    #[tokio::test]
    async fn failed_login_with_unconfirmed_cleanup_keeps_capacity() {
        let f = Fixture::new(
            r#"root=$(dirname "$0")
printf '%s' "$DOCKER_CONFIG" > "$root/config"
cat >/dev/null
rmdir "$DOCKER_CONFIG"
printf 'fixture' > "$DOCKER_CONFIG"
exit 1"#,
        );
        let runtime = f.runtime().with_build_capacity(1);
        let mut r = request(&runtime);
        r.registry_auth = Some(
            base64::engine::general_purpose::STANDARD
                .encode(br#"{"username":"builder","password":"private-password"}"#),
        );
        let frames: Vec<_> = build(&runtime, r)
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
        assert!(
            frames
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .error_message
                .as_deref()
                .unwrap()
                .contains("Could not clean")
        );
        assert_eq!(runtime.builds.capacity().unwrap(), (1, 1, 1));
        std::fs::remove_file(std::fs::read_to_string(f.0.join("config")).unwrap()).unwrap();
    }

    #[tokio::test]
    async fn successful_command_with_failed_cleanup_retains_capacity() {
        let f = Fixture::new(
            r#"root=$(dirname "$0")
printf '%s' "$DOCKER_CONFIG" > "$root/config"
cat >/dev/null
rmdir "$DOCKER_CONFIG"
printf 'fixture' > "$DOCKER_CONFIG""#,
        );
        let runtime = f.runtime();
        let frames: Vec<_> = build(&runtime, request(&runtime))
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
        let config = std::fs::read_to_string(f.0.join("config")).unwrap();
        std::fs::remove_file(config).unwrap();
        assert_eq!(runtime.builds.capacity().unwrap(), (2, 1, 1));
        let last = frames.last().unwrap().as_ref().unwrap();
        assert_eq!(last.status.as_deref(), Some("error"));
        assert!(
            last.error_message
                .as_deref()
                .unwrap()
                .contains("Could not clean")
        );
    }

    #[tokio::test]
    async fn registry_identity_tokens_use_private_config_and_are_redacted() {
        for (host, key) in [
            ("registry.example", "registry.example"),
            ("https://index.docker.io/v1/", "https://index.docker.io/v1/"),
        ] {
            let script = format!(
                r#"root=$(dirname "$0")
printf '%s' "$DOCKER_CONFIG" > "$root/config"
test "$1" = build
test "$(stat -c %a "$DOCKER_CONFIG/config.json")" = 600
test "$(cat "$DOCKER_CONFIG/config.json")" = '{{"auths":{{"{key}":{{"identitytoken":"private-token"}}}}}}'
cat >/dev/null
printf 'progress private-token\n' >&2"#
            );
            let f = Fixture::new(&script);
            let runtime = f.runtime();
            let mut r = request(&runtime);
            r.registry_auth = Some(
                base64::engine::general_purpose::STANDARD.encode(
                    serde_json::to_vec(
                        &serde_json::json!({"serveraddress":host,"identitytoken":"private-token"}),
                    )
                    .unwrap(),
                ),
            );
            let frames: Vec<_> = build(&runtime, r)
                .await
                .unwrap()
                .into_inner()
                .collect()
                .await;
            let frames: Vec<_> = frames.into_iter().map(Result::unwrap).collect();
            assert_eq!(frames.last().unwrap().status.as_deref(), Some("completed"));
            assert!(!format!("{frames:?}").contains("private-token"));
            assert!(
                !Path::new(std::fs::read_to_string(f.0.join("config")).unwrap().trim()).exists()
            );
        }
    }

    #[tokio::test]
    async fn failed_build_streams_redacted_progress_and_never_emits_completion() {
        let f = Fixture::new(
            r#"root=$(dirname "$0")
printf '%s' "$DOCKER_CONFIG" > "$root/config"
case "$1" in
login) cat >/dev/null;;
build) cat >/dev/null; printf 'progress private-password and private-secret\n' >&2; exit 7;;
esac"#,
        );
        let runtime = f.runtime();
        let mut r = request(&runtime);
        r.registry_host = Some("registry.example".into());
        r.registry_auth = Some(
            base64::engine::general_purpose::STANDARD
                .encode(br#"{"username":"builder","password":"private-password"}"#),
        );
        r.build_secrets = vec![citadel_contracts::citadel::images::v1::BuildImageSecret {
            id: "token".into(),
            value: "private-secret".into(),
        }];
        let stream = build(&runtime, r).await.unwrap().into_inner();
        let frames: Vec<_> = stream.collect().await;
        let frames: Vec<_> = frames.into_iter().map(Result::unwrap).collect();
        assert!(
            frames
                .iter()
                .any(|v| v.stream.as_ref().is_some_and(|s| s.contains("progress")))
        );
        assert_eq!(runtime.builds.capacity().unwrap(), (2, 1, 1));
        assert_eq!(
            frames.last().unwrap().error_message.as_deref(),
            Some("Docker build failed with exit code 7. See the build output for details.")
        );
        let transcript: String = frames
            .iter()
            .filter_map(|frame| frame.stream.as_deref())
            .collect();
        assert_eq!(transcript.matches("progress").count(), 1);
        assert!(
            !frames
                .iter()
                .any(|v| v.status.as_deref() == Some("completed"))
        );
        for frame in frames {
            let text = format!("{frame:?}");
            assert!(!text.contains("private-password"));
            assert!(!text.contains("private-secret"));
        }
        assert!(!Path::new(std::fs::read_to_string(f.0.join("config")).unwrap().trim()).exists());
    }
    #[tokio::test]
    async fn dropping_build_stream_kills_the_process_and_removes_credentials() {
        let f = Fixture::new(
            r#"root=$(dirname "$0")
printf '%s' "$$" > "$root/pid"
printf '%s' "$DOCKER_CONFIG" > "$root/config"
cat >/dev/null
while :; do printf 'still building\n'; done"#,
        );
        let runtime = f.runtime();
        let mut stream = build(&runtime, request(&runtime))
            .await
            .unwrap()
            .into_inner();
        tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(frame) = stream.next().await {
                if frame.unwrap().stream.is_some() {
                    break;
                }
            }
        })
        .await
        .unwrap();
        let pid = std::fs::read_to_string(f.0.join("pid")).unwrap();
        let config = std::fs::read_to_string(f.0.join("config")).unwrap();
        drop(stream);
        assert_eq!(runtime.builds.capacity().unwrap(), (2, 1, 1));
        tokio::time::timeout(Duration::from_secs(2), async {
            while Path::new(&format!("/proc/{pid}")).exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!Path::new(&config).exists());
    }
}
