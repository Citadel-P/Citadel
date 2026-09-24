use super::Runtime;
use base64::Engine;
use citadel_adapters::external::builds::runtime::{
    BuildRuntimeError, DockerBuildOptions, DockerBuildSession, DockerBuildSource,
};
use citadel_builds::{BuildArgSpec, BuildError, BuildLogSink, BuildRegistryCredentials};
use citadel_contracts::citadel::images::v1::{
    BuildImageRequest, ImageBuildResponse, PushImageRequest,
};
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
    let endpoint = runtime.docker.endpoint().clone();
    let docker_cli = runtime.docker_cli.clone();
    let cancel = runtime.shutdown.child_token();
    let guard = cancel.clone().drop_guard();
    let (tx, mut rx) = mpsc::channel(8);
    Ok(Response::new(Box::pin(async_stream::try_stream! {
        let _guard=guard;
        let operation=async {
            let progress=Progress(tx, if r.max_line_bytes<=0 {16384} else {r.max_line_bytes.min(1024*1024) as usize});
            let host=r.registry_host.as_deref().filter(|v|!v.is_empty()).or_else(||registry.as_ref().map(|v|v.0.as_str()).filter(|v|!v.is_empty())).unwrap_or("docker.io");
            let mut session=DockerBuildSession::open(docker_cli,endpoint,registry.as_ref().filter(|v|v.2.is_none()).map(|v|(host,&v.1)),Duration::from_secs(r.timeout_seconds.max(1) as u64),4*1024*1024,&cancel).await?;
            if let Some(token)=registry.as_ref().and_then(|v|v.2.as_ref()){session.set_identity_token(host,token.clone())?;}
            let args:Vec<_>=r.build_args.into_iter().map(|(name,value)|BuildArgSpec{name,value:Some(value)}).collect();
            let secrets:Vec<_>=r.build_secrets.into_iter().map(|v|(v.id,Zeroizing::new(v.value))).collect();
            let source=if r.context_archive.is_empty(){DockerBuildSource::Directory{context:Path::new(&r.context_directory),dockerfile:Path::new(&r.dockerfile_path)}}else{DockerBuildSource::Archive{data:&r.context_archive,dockerfile:r.dockerfile_archive_path.as_deref().unwrap_or("Dockerfile")}};
            session.build(DockerBuildOptions{source,tags:&r.tags,build_args:&args,target:r.target.as_deref(),secrets:&secrets},&progress,&cancel).await
        };
        tokio::pin!(operation);
        let outcome=loop{tokio::select!{result=&mut operation=>break result,Some(item)=rx.recv()=>yield item}};
        while let Ok(item)=rx.try_recv(){yield item;}
        match outcome{Ok(())=>yield ImageBuildResponse{status:Some("completed".into()),..Default::default()},Err(error)=>yield failure(error)}
    })))
}
pub(super) async fn push(runtime: &Runtime, r: PushImageRequest) -> Result<Output, Status> {
    let registry = credentials(r.registry_auth.as_deref())?;
    let endpoint = runtime.docker.endpoint().clone();
    let docker_cli = runtime.docker_cli.clone();
    let cancel = runtime.shutdown.child_token();
    let guard = cancel.clone().drop_guard();
    let (tx, mut rx) = mpsc::channel(8);
    Ok(Response::new(Box::pin(async_stream::try_stream! {
        let _guard=guard;
        let operation=async {
            let progress=Progress(tx,16384);
            let first=r.image_reference.split('/').next().unwrap_or("docker.io");
            let host=registry.as_ref().map(|v|v.0.as_str()).filter(|v|!v.is_empty()).unwrap_or(if first.contains('.')||first.contains(':')||first=="localhost"{first}else{"docker.io"});
            let mut session=DockerBuildSession::open(docker_cli,endpoint,registry.as_ref().filter(|v|v.2.is_none()).map(|v|(host,&v.1)),Duration::from_secs(3600),4*1024*1024,&cancel).await?;
            if let Some(token)=registry.as_ref().and_then(|v|v.2.as_ref()){session.set_identity_token(host,token.clone())?;}
            session.push(&r.image_reference,&progress,&cancel).await
        };
        tokio::pin!(operation);
        let outcome=loop{tokio::select!{result=&mut operation=>break result,Some(item)=rx.recv()=>yield item}};
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
            Runtime {
                docker: citadel_adapters::connectors::docker::DockerClient::with_endpoint(
                    "tcp://fixture:2375".parse().unwrap(),
                    Duration::from_secs(3),
                    "/host",
                )
                .unwrap(),
                docker_cli: self.0.join("docker").display().to_string(),
                shutdown: tokio_util::sync::CancellationToken::new(),
                runtime_container: None,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn request() -> BuildImageRequest {
        BuildImageRequest {
            context_archive: vec![0, 255, 1],
            dockerfile_archive_path: Some("Dockerfile".into()),
            tags: vec!["fixture:latest".into()],
            timeout_seconds: 5,
            ..Default::default()
        }
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
            let mut r = request();
            r.registry_auth = Some(
                base64::engine::general_purpose::STANDARD.encode(
                    serde_json::to_vec(
                        &serde_json::json!({"serveraddress":host,"identitytoken":"private-token"}),
                    )
                    .unwrap(),
                ),
            );
            let frames: Vec<_> = build(&f.runtime(), r)
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
        let mut r = request();
        r.registry_host = Some("registry.example".into());
        r.registry_auth = Some(
            base64::engine::general_purpose::STANDARD
                .encode(br#"{"username":"builder","password":"private-password"}"#),
        );
        r.build_secrets = vec![citadel_contracts::citadel::images::v1::BuildImageSecret {
            id: "token".into(),
            value: "private-secret".into(),
        }];
        let stream = build(&f.runtime(), r).await.unwrap().into_inner();
        let frames: Vec<_> = stream.collect().await;
        let frames: Vec<_> = frames.into_iter().map(Result::unwrap).collect();
        assert!(
            frames
                .iter()
                .any(|v| v.stream.as_ref().is_some_and(|s| s.contains("progress")))
        );
        assert!(frames.last().unwrap().error_message.is_some());
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
        let mut stream = build(&f.runtime(), request()).await.unwrap().into_inner();
        tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let pid = std::fs::read_to_string(f.0.join("pid")).unwrap();
        let config = std::fs::read_to_string(f.0.join("config")).unwrap();
        drop(stream);
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
