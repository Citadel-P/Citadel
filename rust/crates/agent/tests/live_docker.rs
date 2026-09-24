//! Opt-in tests create only uniquely named, disposable Docker resources.
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_adapters::connectors::agent::client::{AgentClient, AgentRequestSigner};
use citadel_agent::{app::Agent, config::AgentConfig};
use citadel_contracts::citadel::{
    containers::v1::*, deployments::v1::*, images::v1::*, networks::v1::*, volumes::v1::*,
};
use citadel_platforms::PlatformRuntimePort;
use ed25519_dalek::{Signer, SigningKey};
use futures_util::StreamExt;
use prost::Message;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tonic::{Request, metadata::MetadataValue, transport::Channel};
struct Cleanup {
    prefix: String,
    stop: CancellationToken,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.stop.cancel();
        for kind in ["container", "network", "volume"] {
            let names = if kind == "container" {
                vec![self.prefix.clone(), format!("{}-deployment", self.prefix)]
            } else {
                vec![self.prefix.clone()]
            };
            for name in names {
                let mut command = std::process::Command::new("docker");
                command.args([kind, "rm"]);
                if kind == "container" {
                    command.arg("-f");
                }
                let _ = command.arg(name).output();
            }
        }
    }
}
fn sign<T: Message>(v: T, path: &str) -> Request<T> {
    let time = chrono::Utc::now().timestamp().to_le_bytes();
    let nonce = *uuid::Uuid::now_v7().as_bytes();
    let hash = Sha256::digest(v.encode_to_vec());
    let signature = SigningKey::from_bytes(&[42; 32])
        .sign(&[&time[..], &nonce, path.as_bytes(), &hash].concat())
        .to_bytes();
    let mut r = Request::new(v);
    for (k, v) in [
        ("x-timestamp-bin", &time[..]),
        ("x-nonce-bin", &nonce),
        ("x-content-sha256-bin", &hash),
        ("x-signature-bin", &signature),
    ] {
        r.metadata_mut().insert_bin(k, MetadataValue::from_bytes(v));
    }
    r
}
#[tokio::test]
#[ignore = "requires Docker and a locally available redis:latest image"]
async fn core_and_direct_clients_manage_disposable_resources_on_docker() {
    let prefix = format!("citadel-agent-test-{}", uuid::Uuid::now_v7().simple());
    let stop = CancellationToken::new();
    let _cleanup = Cleanup {
        prefix: prefix.clone(),
        stop: stop.clone(),
    };
    let mut config = AgentConfig::from_lookup(|key| match key {
        "HUB_PUBLIC_KEY" => {
            Some(STANDARD.encode(SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes()))
        }
        "DOCKER_HOST" => std::env::var("DOCKER_HOST").ok(),
        _ => None,
    })
    .unwrap();
    config.port = 0;
    let agent = Agent::bind(config).await.unwrap();
    let address = format!("http://127.0.0.1:{}", agent.local_addr().unwrap().port());
    tokio::spawn(agent.serve(stop));
    let core = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(30),
        true,
    )
    .await
    .unwrap();
    let info = core.get_info(&CancellationToken::new()).await.unwrap();
    assert!(!info.daemon_id.is_empty());
    let channel = Channel::from_shared(address)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut volumes = volume_service_client::VolumeServiceClient::new(channel.clone());
    let volume = volumes
        .create(sign(
            CreateVolumeRequest {
                name: prefix.clone(),
                driver: "local".into(),
                ..Default::default()
            },
            "/citadel.volumes.v1.VolumeService/Create",
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(volume.name, prefix);
    volumes
        .inspect(sign(
            InspectVolumeRequest {
                name: prefix.clone(),
            },
            "/citadel.volumes.v1.VolumeService/Inspect",
        ))
        .await
        .unwrap();
    let mut networks = network_service_client::NetworkServiceClient::new(channel.clone());
    let network = networks
        .create(sign(
            CreateNetworkRequest {
                name: prefix.clone(),
                driver: Some("bridge".into()),
                ..Default::default()
            },
            "/citadel.networks.v1.NetworkService/Create",
        ))
        .await
        .unwrap()
        .into_inner();
    networks
        .inspect(sign(
            InspectNetworkRequest {
                id: network.id.clone(),
            },
            "/citadel.networks.v1.NetworkService/Inspect",
        ))
        .await
        .unwrap();
    let mut containers = container_service_client::ContainerServiceClient::new(channel.clone());
    let created = containers
        .create(sign(
            CreateContainerRequest {
                name: prefix.clone(),
                image_id: "redis:latest".into(),
                command: vec![
                    "sh".into(),
                    "-c".into(),
                    "printf 'agent fixture\n'; exec sleep 300".into(),
                ],
                ..Default::default()
            },
            "/citadel.containers.v1.ContainerService/Create",
        ))
        .await
        .unwrap()
        .into_inner();
    let id = created.container_id;
    containers
        .start(sign(
            ContainerIds {
                ids: vec![id.clone()],
            },
            "/citadel.containers.v1.ContainerService/Start",
        ))
        .await
        .unwrap();
    let inspect = containers
        .inspect(sign(
            InspectContainerRequest {
                container_id: id.clone(),
            },
            "/citadel.containers.v1.ContainerService/Inspect",
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(inspect.state.unwrap().running, Some(true));
    let mut exec = containers
        .exec_binary(sign(
            ExecBinaryRequest {
                container_id: id.clone(),
                cmd: vec![
                    "sh".into(),
                    "-c".into(),
                    "printf '\\000\\377A'; printf 'err' >&2; exit 3".into(),
                ],
                ..Default::default()
            },
            "/citadel.containers.v1.ContainerService/ExecBinary",
        ))
        .await
        .unwrap()
        .into_inner();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exits = 0;
    while let Some(v) = exec.message().await.unwrap() {
        match v.msg.unwrap() {
            exec_server_message::Msg::Output(v) => {
                if v.stream == 0 {
                    stdout.extend(v.data)
                } else {
                    stderr.extend(v.data)
                }
            }
            exec_server_message::Msg::Exit(v) => {
                assert_eq!(v.exit_code, 3);
                exits += 1;
            }
            exec_server_message::Msg::Error(e) => panic!("{e:?}"),
        }
    }
    assert_eq!(stdout, vec![0, 255, 65]);
    assert_eq!(stderr, b"err");
    assert_eq!(exits, 1);
    let open = ExecClientMessage {
        msg: Some(exec_client_message::Msg::Open(ExecOpen {
            container_id: id.clone(),
            cmd: vec![
                "sh".into(),
                "-c".into(),
                "read line; printf 'reply:%s' \"$line\"; exit 4".into(),
            ],
            tty: true,
        })),
    };
    let metadata = sign(open.clone(), "/citadel.containers.v1.ContainerService/Exec")
        .metadata()
        .clone();
    let mut input = Request::new(futures_util::stream::iter([
        open,
        ExecClientMessage {
            msg: Some(exec_client_message::Msg::Stdin(ExecStdin {
                data: b"hello\n".to_vec(),
            })),
        },
    ]));
    *input.metadata_mut() = metadata;
    let mut output = containers.exec(input).await.unwrap().into_inner();
    let mut data = Vec::new();
    let mut exits = 0;
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(v) = output.message().await.unwrap() {
            match v.msg.unwrap() {
                exec_server_message::Msg::Output(v) => data.extend(v.data),
                exec_server_message::Msg::Exit(v) => {
                    assert_eq!(v.exit_code, 4);
                    exits += 1;
                }
                exec_server_message::Msg::Error(e) => panic!("{e:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert!(
        String::from_utf8_lossy(&data).contains("reply:hello"),
        "terminal output: {:?}",
        String::from_utf8_lossy(&data)
    );
    assert_eq!(exits, 1);
    let cancel = CancellationToken::new();
    let listed = core.list_containers(&cancel).await.unwrap();
    assert!(listed.iter().any(|v| v.id == id));
    let mut stats = core
        .stream_stats(Duration::from_secs(1), &cancel)
        .await
        .unwrap();
    let sample = tokio::time::timeout(Duration::from_secs(10), stats.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(sample.mem_total > 0);
    drop(stats);
    let mut events = core.stream_daemon_events(&cancel).await.unwrap();
    containers
        .pause(sign(
            ContainerIds {
                ids: vec![id.clone()],
            },
            "/citadel.containers.v1.ContainerService/Pause",
        ))
        .await
        .unwrap();
    containers
        .unpause(sign(
            ContainerIds {
                ids: vec![id.clone()],
            },
            "/citadel.containers.v1.ContainerService/Unpause",
        ))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events.next().await.unwrap().unwrap();
            if event.container_id.as_deref() == Some(&id) && event.action == "pause" {
                break;
            }
        }
    })
    .await
    .unwrap();
    drop(events);
    let mut logs = containers
        .stream_container_logs(sign(
            ContainerLogRequest {
                container_id: id,
                follow: Some(false),
                tail: 20,
            },
            "/citadel.containers.v1.ContainerService/StreamContainerLogs",
        ))
        .await
        .unwrap()
        .into_inner();
    let mut text = Vec::new();
    while let Some(v) = logs.message().await.unwrap() {
        text.extend(v.log);
    }
    assert!(String::from_utf8_lossy(&text).contains("agent fixture"));
    let mut images = image_service_client::ImageServiceClient::new(channel.clone());
    let inspect = images
        .inspect(sign(
            InspectImageRequest {
                id: "redis:latest".into(),
            },
            "/citadel.images.v1.ImageService/Inspect",
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(!inspect.id.is_empty());
    assert!(!inspect.layers.is_empty());
    let mut deployment = deployment_service_client::DeploymentServiceClient::new(channel);
    let result = deployment
        .apply(sign(
            ApplyDeploymentRequest {
                image_id: "redis:latest".into(),
                name: format!("{prefix}-deployment"),
                spec: Some(DeploymentSpec {
                    command: vec!["sleep".into(), "300".into()],
                    ..Default::default()
                }),
            },
            "/citadel.deployments.v1.DeploymentService/Apply",
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        result.deployed_container_state,
        DeployedContainerState::Running as i32
    );
}
