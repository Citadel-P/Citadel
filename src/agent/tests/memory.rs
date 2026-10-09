//! Isolated Agent process soak. Run inside the Agent runtime image, with the
//! current release binary mounted at CITADEL_MEMORY_AGENT_BINARY. No Docker
//! socket is used: the daemon and build CLI are deterministic local fixtures.
#![cfg(all(target_os = "linux", target_env = "gnu"))]
#[path = "memory/edge.rs"]
mod edge;

use axum::{Json, Router, body::Body, response::IntoResponse};
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_contracts::citadel::{containers::v1::*, images::v1::*};
use ed25519_dalek::{Signer, SigningKey};
use prost::Message;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tonic::{Request, metadata::MetadataValue, transport::Channel};
use uuid::Uuid;

const MIB: usize = 1024 * 1024;
const LIMIT: usize = 16 * MIB;

struct LiveStream(Arc<AtomicUsize>);
impl Drop for LiveStream {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
struct Fixture {
    root: PathBuf,
    docker: String,
    streams: Arc<AtomicUsize>,
    stop: CancellationToken,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.cancel();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = std::env::temp_dir().join(format!("citadel-agent-memory-{}", Uuid::now_v7()));
        std::fs::create_dir(&root).unwrap();
        let cli = root.join("docker");
        std::fs::write(&cli, "#!/bin/sh\nset -eu\ntest \"$1\" = build\nn=$(wc -c)\ncase $n in 1048576|4194304|12582912) ;; *) exit 2;; esac\nhead -c 262144 /dev/zero | tr '\\000' x\nprintf '\\n'\n").unwrap();
        std::fs::set_permissions(cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let streams = Arc::new(AtomicUsize::new(0));
        let live = streams.clone();
        let router = Router::new().fallback(move |uri: axum::http::Uri| {
            let live = live.clone();
            async move {
                let path = uri.path();
                if path == "/version" { return Json(json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(); }
                if path == "/v1.49/info" { return Json(json!({"ID":"memory-daemon","Name":"memory-fixture","ServerVersion":"29"})).into_response(); }
                if path.ends_with("/json") {
                    return Json(json!({"Id":"fixture","Config":{"Tty":path.contains("/tty/")}})).into_response();
                }
                if path.ends_with("/logs") {
                    let tty = path.contains("/tty/");
                    live.fetch_add(1, Ordering::SeqCst);
                    let guard = LiveStream(live);
                    return Body::from_stream(async_stream::stream! {
                        let _guard = guard;
                        for size in [MIB, 64] {
                            let mut data = Vec::new();
                            if !tty { data.extend_from_slice(&[1,0,0,0]); data.extend_from_slice(&(size as u32).to_be_bytes()); }
                            data.extend(std::iter::repeat_n(b'x', size));
                            yield Ok::<_, std::io::Error>(bytes::Bytes::from(data));
                        }
                        std::future::pending::<()>().await;
                    }).into_response();
                }
                if path == "/v1.49/images/create" {
                    return Body::from_stream(async_stream::stream! {
                        for _ in 0..4 {
                            let line = json!({"status":"x".repeat(256*1024)}).to_string()+"\n";
                            yield Ok::<_, std::io::Error>(bytes::Bytes::from(line));
                        }
                    }).into_response();
                }
                (axum::http::StatusCode::NOT_FOUND, path.to_owned()).into_response()
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = format!("tcp://{}", listener.local_addr().unwrap());
        let stop = CancellationToken::new();
        let stopped = stop.clone();
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(stopped.cancelled_owned())
                .await
                .unwrap();
        });
        Self {
            root,
            docker,
            streams,
            stop,
        }
    }
    async fn drained(&self) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.streams.load(Ordering::SeqCst) != 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("Agent cancellation must close the upstream Docker stream");
    }
}

struct Process {
    child: Child,
    address: String,
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Process {
    async fn start(f: &Fixture, core: Option<&str>) -> Self {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let binary = std::env::var_os("CITADEL_MEMORY_AGENT_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_citadel-agent").into());
        let mut command = Command::new(binary);
        command
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", f.root.display()))
            .env("HOME", &f.root)
            .env("TMPDIR", &f.root)
            .env("TOKIO_WORKER_THREADS", "2")
            .env("RUST_LOG", "warn")
            .env("CITADEL_AGENT_PORT", port.to_string())
            .env("DOCKER_HOST", &f.docker)
            .env(
                "HUB_PUBLIC_KEY",
                STANDARD.encode(SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes()),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        if let Ok(value) = std::env::var("CITADEL_MEMORY_MMAP_THRESHOLD") {
            command.env("MALLOC_MMAP_THRESHOLD_", value);
        }
        if let Some(core) = core {
            command
                .env("CITADEL_AGENT_MODE", "edge")
                .env("CITADEL_CORE_URL", core)
                .env("CITADEL_EDGE_ENROLLMENT_TOKEN", "memory-fixture")
                .env("CITADEL_EDGE_AGENT_KEY_PATH", f.root.join("agent.key"))
                .env("CITADEL_EDGE_IDENTITY_PATH", f.root.join("identity.json"));
        }
        let mut process = Self {
            child: command.spawn().unwrap(),
            address: format!("http://127.0.0.1:{port}"),
        };
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                assert!(
                    process.child.try_wait().unwrap().is_none(),
                    "Agent exited before becoming healthy"
                );
                if client
                    .get(format!("{}/health", process.address))
                    .send()
                    .await
                    .is_ok_and(|r| r.status().is_success())
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();
        process
    }
    fn sample(&self, mode: &str, stage: &str) -> serde_json::Value {
        let status = std::fs::read_to_string(format!("/proc/{}/status", self.child.id())).unwrap();
        let field = |name: &str| {
            status
                .lines()
                .find(|s| s.starts_with(name))
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        let value = json!({"mode":mode,"stage":stage,"rss_kib":field("VmRSS:"),"swap_kib":field("VmSwap:"),"threads":field("Threads:"),"fds":std::fs::read_dir(format!("/proc/{}/fd", self.child.id())).unwrap().count()});
        println!("MEMORY_SAMPLE {value}");
        value
    }
}

fn signed<T: Message>(value: T, method: &str) -> Request<T> {
    let time = chrono::Utc::now().timestamp().to_le_bytes();
    let nonce = *Uuid::now_v7().as_bytes();
    let hash = Sha256::digest(value.encode_to_vec());
    let signature = SigningKey::from_bytes(&[42; 32])
        .sign(&[&time[..], &nonce, method.as_bytes(), &hash].concat())
        .to_bytes();
    let mut request = Request::new(value);
    for (key, value) in [
        ("x-timestamp-bin", &time[..]),
        ("x-nonce-bin", &nonce[..]),
        ("x-content-sha256-bin", &hash[..]),
        ("x-signature-bin", &signature[..]),
    ] {
        request
            .metadata_mut()
            .insert_bin(key, MetadataValue::from_bytes(value));
    }
    request
}
fn log_request(tty: bool) -> ContainerLogRequest {
    ContainerLogRequest {
        container_id: if tty { "tty" } else { "plain" }.into(),
        follow: Some(true),
        tail: 1,
    }
}
fn build_request(round: usize) -> BuildImageRequest {
    BuildImageRequest {
        context_archive: vec![0; [1, 4, 12][round % 3] * MIB],
        dockerfile_archive_path: Some("Dockerfile".into()),
        tags: vec!["fixture:memory".into()],
        timeout_seconds: 10,
        ..Default::default()
    }
}
async fn direct_cycle(channel: Channel, round: usize) {
    let mut containers = container_service_client::ContainerServiceClient::new(channel.clone())
        .max_decoding_message_size(LIMIT);
    for tty in [false, true] {
        let mut stream = containers
            .stream_container_logs(signed(
                log_request(tty),
                "/citadel.containers.v1.ContainerService/StreamContainerLogs",
            ))
            .await
            .unwrap()
            .into_inner();
        let mut bytes = 0;
        while bytes < MIB + 64 {
            bytes += stream.message().await.unwrap().unwrap().log.len();
        }
        assert_eq!(bytes, MIB + 64);
        // Keep the subscription open after its large frame, then cancel it.
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(stream);
    }
    let mut images = image_service_client::ImageServiceClient::new(channel)
        .max_encoding_message_size(LIMIT)
        .max_decoding_message_size(LIMIT);
    let mut pull = images
        .pull(signed(
            PullImageRequest {
                from_image: "fixture".into(),
                ..Default::default()
            },
            "/citadel.images.v1.ImageService/Pull",
        ))
        .await
        .unwrap()
        .into_inner();
    let mut frames = 0;
    while let Some(value) = pull.message().await.unwrap() {
        assert_eq!(value.status.unwrap().len(), 256 * 1024);
        frames += 1;
    }
    assert_eq!(frames, 4);
    let mut build = images
        .build(signed(
            build_request(round),
            "/citadel.images.v1.ImageService/Build",
        ))
        .await
        .unwrap()
        .into_inner();
    let mut completed = false;
    while let Some(value) = build.message().await.unwrap() {
        assert!(value.error_message.is_none(), "{value:?}");
        completed |= value.status.as_deref() == Some("completed");
    }
    assert!(completed);
}
fn footprint(value: &serde_json::Value) -> u64 {
    value["rss_kib"].as_u64().unwrap() + value["swap_kib"].as_u64().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated process RSS soak; run with --ignored --nocapture --test-threads=1"]
async fn direct_and_edge_release_streams_and_bound_idle_memory() {
    for mode in ["direct", "edge"] {
        let f = Fixture::new().await;
        let mut core = edge::Core::start(f.stop.clone()).await;
        let process = Process::start(&f, (mode == "edge").then_some(core.address.as_str())).await;
        let channel = if mode == "direct" {
            Some(
                Channel::from_shared(process.address.clone())
                    .unwrap()
                    .connect()
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let mut session = if mode == "edge" {
            Some(core.next().await)
        } else {
            None
        };
        process.sample(mode, "cold");
        let mut baseline = None;
        for round in 0..36 {
            tokio::time::timeout(Duration::from_secs(30), async {
                if let Some(channel) = &channel {
                    direct_cycle(channel.clone(), round).await;
                } else {
                    session.as_mut().unwrap().cycle(round).await;
                }
                f.drained().await;
            })
            .await
            .expect("workload must complete and release its Docker streams");
            if mode == "edge" && round % 6 == 5 {
                session.take().unwrap().disconnect().await;
                session = Some(core.next().await);
                f.drained().await;
            }
            if round % 6 == 5 {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let sample = process.sample(mode, &format!("round-{}", round + 1));
                baseline.get_or_insert(sample);
            }
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
        let idle = process.sample(mode, "final-idle");
        let baseline = baseline.unwrap();
        assert!(
            footprint(&idle) <= footprint(&baseline) + 32 * 1024,
            "{mode}: idle footprint grew by more than 32 MiB after warm-up"
        );
        assert!(
            idle["fds"].as_u64().unwrap() <= baseline["fds"].as_u64().unwrap() + 4,
            "{mode}: descriptors accumulated"
        );
        drop(process);
        f.drained().await;
    }
}
