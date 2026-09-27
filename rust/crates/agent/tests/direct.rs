use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_adapters::connectors::agent::client::{AgentClient, AgentRequestSigner};
use citadel_agent::{app::Agent, config::AgentConfig};
use citadel_contracts::citadel::volumes::v1::{
    CreateVolumeRequest, volume_service_client::VolumeServiceClient,
};
use citadel_platforms::CreateRuntimeVolume;
use citadel_platforms::VolumeMutationPort;
use ed25519_dalek::{Signer, SigningKey};
use prost::Message;
use sha2::{Digest, Sha256};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tonic::{metadata::MetadataValue, transport::Channel};

#[derive(Default)]
struct DockerState {
    calls: AtomicUsize,
    created: Mutex<Vec<serde_json::Value>>,
    requests: Mutex<Vec<(String, String, serde_json::Value)>>,
    registry_headers: Mutex<Vec<String>>,
}
struct Harness {
    address: String,
    state: Arc<DockerState>,
    stop: CancellationToken,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl Harness {
    async fn start() -> Self {
        let state = Arc::new(DockerState::default());
        let daemon = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let daemon_address = daemon.local_addr().unwrap();
        let stop = CancellationToken::new();
        let router = Router::new()
            .fallback(mock_docker)
            .with_state(state.clone());
        let daemon_stop = stop.clone();
        tokio::spawn(async move {
            axum::serve(daemon, router)
                .with_graceful_shutdown(daemon_stop.cancelled_owned())
                .await
                .unwrap();
        });
        let mut config = AgentConfig::from_lookup(|name| match name {
            "HUB_PUBLIC_KEY" => {
                Some(STANDARD.encode(SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes()))
            }
            "DOCKER_HOST" => Some(format!("tcp://{daemon_address}")),
            _ => None,
        })
        .unwrap();
        config.port = 0;
        let agent = Agent::bind(config).await.unwrap();
        let address = format!("http://127.0.0.1:{}", agent.local_addr().unwrap().port());
        tokio::spawn(agent.serve(stop.clone()));
        Self {
            address,
            state,
            stop,
        }
    }
    async fn channel(&self) -> Channel {
        Channel::from_shared(self.address.clone())
            .unwrap()
            .connect()
            .await
            .unwrap()
    }
}
async fn mock_docker(
    State(state): State<Arc<DockerState>>,
    request: Request,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use serde_json::json;
    state.calls.fetch_add(1, Ordering::SeqCst);
    if let Some(auth) = request.headers().get("X-Registry-Auth") {
        state
            .registry_headers
            .lock()
            .unwrap()
            .push(auth.to_str().unwrap().to_owned());
    }
    let uri = request.uri().to_string();
    let path = request.uri().path().to_owned();
    let method = request.method().to_string();
    let bytes = to_bytes(request.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    state
        .requests
        .lock()
        .unwrap()
        .push((method.clone(), uri.clone(), value.clone()));
    let result=match path.as_str() {
        "/version"|"/v1.49/version" => json!({"Version":"28.0.0","ApiVersion":"1.49","MinAPIVersion":"1.41","Os":"linux","Arch":"amd64","Components":[{"Name":"BuildKit","Version":"v0.20.0"}]}),
        "/_ping"|"/v1.49/_ping"=>return "OK".into_response(),
        "/v1.49/volumes/create" => {
            state.created.lock().unwrap().push(value.clone());
            json!({"Name":value["Name"],"Driver":"local","Scope":"local","Labels":value["Labels"],"Options":value["DriverOpts"]})
        }
        "/v1.49/containers/create"=>json!({"Id":"created-container","Warnings":[]}),
        "/v1.49/containers/created-container/start"=>return axum::http::StatusCode::NO_CONTENT.into_response(),
        "/v1.49/containers/created-container/json"=>json!({"Id":"created-container","State":{"Running":true,"Status":"running"}}),
        "/v1.49/containers/slow/start"=>{tokio::time::sleep(Duration::from_secs(5)).await;return axum::http::StatusCode::NO_CONTENT.into_response()},
        "/v1.49/containers/fixture/json"=>json!({"Id":"fixture","SizeRw":9007199254740993i64,"State":{"Status":"running","Running":true,"ExitCode":0},"Config":{"Image":"redis","Env":null,"Labels":null,"ExposedPorts":{"6379/tcp":{}}},"HostConfig":{"ReadonlyRootfs":false,"Memory":0,"PortBindings":{"6379/tcp":[{"HostIp":"127.0.0.1","HostPort":"6379"}]}},"NetworkSettings":{"HairpinMode":false,"Networks":{"frontend":{"NetworkID":"net","Aliases":null,"GlobalIPv6Address":"::1"}}}}),
        "/v1.49/containers/json" if uri.contains("volume") => json!([{"Id":"web", "Names":["/web"], "Image":"alpine", "ImageID":"image", "State":"running", "NetworkSettings":{"Networks":{"Mixed.Network":{"NetworkID":"net"}}}, "Ports":[{"PrivatePort":80,"PublicPort":8080,"Type":"tcp","IP":"::"}]}]),
        "/v1.49/containers/json"=>json!([]),
        "/v1.49/volumes/data" => json!({"Name":"data", "Scope":"local", "UsageData":{"Size":42,"RefCount":0}, "ClusterVolume":{"ID":"cluster", "Version":{"Index":9007199254740993_i64}, "Spec":{"Group":"g", "AccessMode":{"Scope":"multi", "Sharing":"readonly", "Availability":"active", "Secrets":[{"Key":"key","Secret":"secret"}], "CapacityRange":{"RequiredBytes":4096}}}, "Info":{"VolumeID":"csi", "CapacityBytes":8192, "VolumeContext":{"Mixed.Key":"v"}, "AccessibleTopology":[{"Zone.Name":"z"}]}, "PublishStatus":[{"NodeID":"node", "State":"published", "PublishContext":{"Mount.Path":"/data"}}]}}),
        "/v1.49/images/json"=>json!([{"Id":"sha256:fixture","ParentId":"parent","Created":123,"RepoTags":["redis:latest"],"RepoDigests":["redis@sha256:fixture"],"Size":9876543210i64,"SharedSize":-1,"VirtualSize":9876543210i64,"Containers":2,"Labels":{"test":"yes"}}]),
        "/v1.49/images/redis/json"=>json!({"Id":"sha256:fixture","Created":"2026-01-01T00:00:00Z","Size":9876543210i64,"Os":"linux","Architecture":"amd64","Config":{"User":"1000","WorkingDir":"/data","Entrypoint":["redis-server"],"StopSignal":"SIGTERM","ExposedPorts":{"6379/tcp":{}}}}),
        "/v1.49/images/redis/history"=>json!([{"Id":"layer","Created":123,"Size":9007199254740993i64,"CreatedBy":"RUN true","Comment":"fixture"}]),
        "/v1.49/images/create"=>return ([("content-type","application/json")],"{\"status\":\"Downloading\",\"progressDetail\":{\"current\":0,\"total\":100}}\n{\"errorDetail\":{\"code\":123,\"message\":\"pull failed\"},\"error\":\"pull failed\"}\n").into_response(),
        "/v1.49/networks/create"=>json!({"Id":"net","Warning":""}),
        "/v1.49/networks/net"=>json!({"Name":"frontend","Id":"net","Driver":"overlay","Scope":"swarm","EnableIPv4":false,"EnableIPv6":true,"Attachable":true,"IPAM":{"Driver":"default","Config":[{"Subnet":"2001:db8::/64","Gateway":"2001:db8::1"}]},"Containers":{},"Peers":[]}),
        "/v1.49/configs/config" if method=="GET"=>json!({"ID":"config","Version":{"Index":9007199254740993u64},"Spec":{"Name":"settings","Data":"AP8BAg==","Labels":{"old":"value"}}}),
        "/v1.49/configs/config/update"=>return axum::http::StatusCode::OK.into_response(),
        "/v1.49/secrets/create"=>json!({"ID":"secret"}),
        "/v1.49/services/create"=>json!({"ID":"service","Warnings":["fixture warning"]}),
        "/v1.49/services/service" if method=="GET"=>json!({"ID":"service","Version":{"Index":9007199254740993u64},"Spec":{"Name":"worker","TaskTemplate":{"ContainerSpec":{"Image":"redis","Init":true},"ForceUpdate":4},"Mode":{"Replicated":{"Replicas":1}},"RollbackConfig":{"Parallelism":2}}}),
        "/v1.49/services/service/update"=>json!({"Warnings":[]}),
        _=>return (axum::http::StatusCode::NOT_FOUND,Json(json!({"message":"fixture resource not found"}))).into_response(),
    };
    Json(result).into_response()
}

fn signed<T: Message>(message: T, method: &str, nonce: [u8; 16]) -> tonic::Request<T> {
    let timestamp = chrono::Utc::now().timestamp().to_le_bytes();
    let hash = Sha256::digest(message.encode_to_vec());
    let payload = [&timestamp[..], &nonce, method.as_bytes(), &hash].concat();
    let signature = SigningKey::from_bytes(&[42; 32]).sign(&payload).to_bytes();
    let mut request = tonic::Request::new(message);
    for (name, bytes) in [
        ("x-timestamp-bin", &timestamp[..]),
        ("x-nonce-bin", &nonce),
        ("x-content-sha256-bin", &hash),
        ("x-signature-bin", &signature),
    ] {
        request
            .metadata_mut()
            .insert_bin(name, MetadataValue::from_bytes(bytes));
    }
    request
}

#[tokio::test]
async fn core_client_can_create_a_volume_with_maps_without_reserialization_failures() {
    let host = Harness::start().await;
    let client = AgentClient::connect(
        &host.address,
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(5),
        true,
    )
    .await
    .unwrap();
    for index in 0..8 {
        let input = CreateRuntimeVolume {
            name: format!("volume-{index}"),
            driver: "local".into(),
            labels: (0..12)
                .map(|i| (format!("label-{i}"), format!("value-{i}")))
                .collect(),
            options: [
                ("type".into(), "tmpfs".into()),
                ("device".into(), "tmpfs".into()),
            ]
            .into(),
        };
        let response = client
            .create_volume(&input, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(response.name, input.name);
        assert_eq!(response.labels, input.labels);
    }
    assert_eq!(host.state.created.lock().unwrap().len(), 8);
}

#[tokio::test]
async fn unsigned_tampered_and_cross_method_requests_never_reach_docker() {
    let host = Harness::start().await;
    let mut client = VolumeServiceClient::new(host.channel().await);
    assert_eq!(
        client
            .create(CreateVolumeRequest::default())
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    let request = signed(
        CreateVolumeRequest::default(),
        "/citadel.volumes.v1.VolumeService/Inspect",
        [1; 16],
    );
    assert_eq!(
        client.create(request).await.unwrap_err().code(),
        tonic::Code::Unauthenticated
    );
    let mut request = signed(
        CreateVolumeRequest::default(),
        "/citadel.volumes.v1.VolumeService/Create",
        [2; 16],
    );
    request.get_mut().name = "tampered".into();
    assert_eq!(
        client.create(request).await.unwrap_err().code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(host.state.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn replay_is_shared_between_requests_and_services() {
    let host = Harness::start().await;
    let mut client = VolumeServiceClient::new(host.channel().await);
    let request = CreateVolumeRequest {
        name: "once".into(),
        ..Default::default()
    };
    let path = "/citadel.volumes.v1.VolumeService/Create";
    client
        .create(signed(request.clone(), path, [5; 16]))
        .await
        .unwrap();
    assert_eq!(
        client
            .create(signed(request, path, [5; 16]))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    let mut network =
        citadel_contracts::citadel::networks::v1::network_service_client::NetworkServiceClient::new(
            host.channel().await,
        );
    let request = signed(
        citadel_contracts::citadel::networks::v1::ListNetworksRequest::default(),
        "/citadel.networks.v1.NetworkService/List",
        [5; 16],
    );
    assert_eq!(
        network.list(request).await.unwrap_err().code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(host.state.created.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn every_direct_rpc_is_registered_and_requires_authentication() {
    let host = Harness::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let protos = [
        include_str!("../../infrastructure/contracts/proto/container_service.proto"),
        include_str!("../../infrastructure/contracts/proto/deployment_service.proto"),
        include_str!("../../infrastructure/contracts/proto/image_service.proto"),
        include_str!("../../infrastructure/contracts/proto/network_service.proto"),
        include_str!("../../infrastructure/contracts/proto/platform_service.proto"),
        include_str!("../../infrastructure/contracts/proto/stack_service.proto"),
        include_str!("../../infrastructure/contracts/proto/swarm_service.proto"),
        include_str!("../../infrastructure/contracts/proto/volume_service.proto"),
    ];
    let mut count = 0;
    for proto in protos {
        let package = proto
            .lines()
            .find_map(|l| l.trim().strip_prefix("package "))
            .unwrap()
            .trim_end_matches(';');
        let service = proto
            .lines()
            .find_map(|l| l.trim().strip_prefix("service "))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        for line in proto.lines().filter_map(|l| l.trim().strip_prefix("rpc ")) {
            let method = line.split([' ', '(']).next().unwrap();
            let path = format!("/{package}.{service}/{method}");
            let response = client
                .post(format!("{}{path}", host.address))
                .header("content-type", "application/grpc")
                .body(vec![0; 5])
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.headers().get("grpc-status").unwrap(),
                "16",
                "{path}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 68);
    assert_eq!(host.state.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn container_inspection_preserves_nulls_false_zero_large_numbers_and_networks() {
    use citadel_contracts::citadel::containers::v1::*;
    let host = Harness::start().await;
    let mut client = container_service_client::ContainerServiceClient::new(host.channel().await);
    let r = client
        .inspect(signed(
            InspectContainerRequest {
                container_id: "fixture".into(),
            },
            "/citadel.containers.v1.ContainerService/Inspect",
            [11; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(r.size_rw, Some(9007199254740993));
    assert_eq!(r.state.unwrap().exit_code, Some(0));
    let config = r.config.unwrap();
    assert!(config.env.is_empty());
    assert!(config.labels.is_empty());
    assert_eq!(config.exposed_ports, vec!["6379/tcp"]);
    let host = r.host_config.unwrap();
    assert_eq!(host.readonly_rootfs, Some(false));
    assert_eq!(host.memory, Some(0));
    assert_eq!(
        host.port_bindings["6379/tcp"].host_port_binding[0]
            .host_ip
            .as_deref(),
        Some("127.0.0.1")
    );
    let network = r.network_settings.unwrap();
    assert_eq!(network.hairpin_mode, Some(false));
    assert_eq!(network.networks[0].key, "frontend");
    assert_eq!(
        network.networks[0]
            .value
            .as_ref()
            .unwrap()
            .global_i_pv6_address
            .as_deref(),
        Some("::1")
    );
}

#[tokio::test]
async fn deployment_apply_passes_wire_resources_and_observes_the_started_container() {
    use citadel_contracts::citadel::deployments::v1::*;
    let host = Harness::start().await;
    let mut client = deployment_service_client::DeploymentServiceClient::new(host.channel().await);
    let r = ApplyDeploymentRequest {
        image_id: "redis".into(),
        name: "cache".into(),
        spec: Some(DeploymentSpec {
            ports: vec!["6379:6379".into()],
            volumes: vec!["cache:/data:ro".into()],
            resource_spec: Some(ResourceSpec {
                nano_cpus: Some(1000000000.0),
                memory_limit: Some(268435456.0),
            }),
            ..Default::default()
        }),
    };
    let result = client
        .apply(signed(
            r,
            "/citadel.deployments.v1.DeploymentService/Apply",
            [12; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result.container_id, "created-container");
    assert_eq!(
        result.deployed_container_state,
        DeployedContainerState::Running as i32
    );
    let requests = host.state.requests.lock().unwrap();
    let (_, _, body) = requests
        .iter()
        .find(|(_, path, _)| path.starts_with("/v1.49/containers/create"))
        .unwrap();
    assert_eq!(body["HostConfig"]["NanoCpus"], 1000000000);
    assert_eq!(body["HostConfig"]["Memory"], 268435456);
    assert_eq!(body["HostConfig"]["Mounts"][0]["ReadOnly"], true);
}

#[tokio::test]
async fn image_inspection_history_build_host_and_pull_failures_round_trip() {
    use citadel_contracts::citadel::images::v1::*;
    let host = Harness::start().await;
    let mut client = image_service_client::ImageServiceClient::new(host.channel().await);
    let list = client
        .list(signed(
            ListImagesRequest {},
            "/citadel.images.v1.ImageService/List",
            [21; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.images[0].parent_id, "parent");
    assert_eq!(list.images[0].shared_size, -1);
    assert_eq!(list.images[0].size, 9876543210.0);
    let result = client
        .inspect(signed(
            InspectImageRequest { id: "redis".into() },
            "/citadel.images.v1.ImageService/Inspect",
            [22; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result.user.as_deref(), Some("1000"));
    assert_eq!(result.entry_point, vec!["redis-server"]);
    assert_eq!(result.stop_signal.as_deref(), Some("SIGTERM"));
    assert_eq!(result.layers[0].size, 9007199254740993);
    assert_eq!(
        host.state
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, uri, _)| uri == "/v1.49/images/redis/json")
            .count(),
        1
    );
    let result = client
        .check_build_host(signed(
            (),
            "/citadel.images.v1.ImageService/CheckBuildHost",
            [23; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(result.available);
    assert_eq!(result.build_kit_version, "v0.20.0");
    let mut stream = client
        .pull(signed(
            PullImageRequest {
                from_image: "redis".into(),
                ..Default::default()
            },
            "/citadel.images.v1.ImageService/Pull",
            [24; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        stream
            .message()
            .await
            .unwrap()
            .unwrap()
            .progress
            .unwrap()
            .current,
        Some(0)
    );
    let error = stream.message().await.unwrap().unwrap();
    assert_eq!(error.error.unwrap().code, Some(123));
    assert_eq!(error.error_message.as_deref(), Some("pull failed"));
    assert!(stream.message().await.unwrap().is_none());
}

#[tokio::test]
async fn swarm_config_bytes_and_64_bit_versions_survive_metadata_updates() {
    use citadel_contracts::citadel::swarm::v1::*;
    let host = Harness::start().await;
    let mut client = swarm_service_client::SwarmServiceClient::new(host.channel().await);
    let result = client
        .get_config_data(signed(
            InspectSwarmConfigRequest {
                config_id: "config".into(),
            },
            "/citadel.swarm.v1.SwarmService/GetConfigData",
            [31; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result.data, vec![0, 255, 1, 2]);
    let r = UpdateSwarmResourceLabelsRequest {
        resource_id: "config".into(),
        version_index: 9007199254740993,
        labels: [("new".into(), "label".into())].into(),
    };
    client
        .update_config_labels(signed(
            r,
            "/citadel.swarm.v1.SwarmService/UpdateConfigLabels",
            [32; 16],
        ))
        .await
        .unwrap();
    let r = UpdateSwarmNodeRequest {
        node_id: "node".into(),
        version_index: u64::MAX,
        availability: "active".into(),
        ..Default::default()
    };
    assert_eq!(
        client
            .update_node(signed(
                r,
                "/citadel.swarm.v1.SwarmService/UpdateNode",
                [33; 16]
            ))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::InvalidArgument
    );
    let calls = host.state.requests.lock().unwrap();
    let (_, path, body) = calls
        .iter()
        .find(|(_, path, _)| path.contains("/configs/config/update"))
        .unwrap();
    assert!(path.contains("version=9007199254740993"));
    assert_eq!(body["Data"], "AP8BAg==");
    assert_eq!(body["Labels"]["new"], "label");
}

#[tokio::test]
async fn request_deadline_interrupts_docker_and_releases_the_call() {
    use citadel_contracts::citadel::containers::v1::*;
    let host = Harness::start().await;
    let mut r = signed(
        ContainerIds {
            ids: vec!["slow".into()],
        },
        "/citadel.containers.v1.ContainerService/Start",
        [41; 16],
    );
    r.metadata_mut()
        .insert("grpc-timeout", "50m".parse().unwrap());
    // Use raw HTTP here so the client library's own timeout cannot win the
    // race against the server and translate it to Cancelled.
    let mut body = vec![0];
    let encoded = r.get_ref().encode_to_vec();
    body.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
    body.extend_from_slice(&encoded);
    let response = reqwest::Client::new()
        .post(format!(
            "{}/citadel.containers.v1.ContainerService/Start",
            host.address
        ))
        .headers(r.metadata().clone().into_headers())
        .header("content-type", "application/grpc")
        .body(body)
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers().get("grpc-status").unwrap(), "4");
}

#[tokio::test]
async fn duplex_open_is_authenticated_and_must_be_the_first_message() {
    use citadel_contracts::citadel::containers::v1::*;
    let host = Harness::start().await;
    let mut client = container_service_client::ContainerServiceClient::new(host.channel().await);
    let first = ExecClientMessage::default();
    let metadata = signed(
        first.clone(),
        "/citadel.containers.v1.ContainerService/Exec",
        [51; 16],
    )
    .metadata()
    .clone();
    let mut request = tonic::Request::new(tokio_stream_for_test(first));
    *request.metadata_mut() = metadata;
    assert_eq!(
        client.exec(request).await.unwrap_err().code(),
        tonic::Code::InvalidArgument
    );
    assert_eq!(host.state.calls.load(Ordering::SeqCst), 0);
}
fn tokio_stream_for_test<T: Send + 'static>(
    value: T,
) -> impl futures_util::Stream<Item = T> + Send {
    futures_util::stream::iter([value])
}

#[tokio::test]
async fn swarm_mutations_forward_registry_auth_and_preserve_unknown_live_fields() {
    use citadel_contracts::citadel::swarm::v1::*;
    let host = Harness::start().await;
    let mut client = swarm_service_client::SwarmServiceClient::new(host.channel().await);
    let spec = SwarmServiceMutationSpecMessage {
        image: "registry.example/worker".into(),
        scheduling_mode: "Replicated".into(),
        replicas: Some(0),
        ..Default::default()
    };
    let r = CreateManagedSwarmServiceRequest {
        docker_name: "worker".into(),
        spec: Some(spec.clone()),
        registry_auth: "fixture-auth".into(),
        ..Default::default()
    };
    let created = client
        .create_service(signed(
            r,
            "/citadel.swarm.v1.SwarmService/CreateService",
            [61; 16],
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(created.service_id, "service");
    assert_eq!(created.warnings, vec!["fixture warning"]);
    let r = UpdateManagedSwarmServiceRequest {
        service_id: "service".into(),
        version_index: 9007199254740993,
        spec: Some(spec),
        registry_auth: "fixture-auth".into(),
        ..Default::default()
    };
    client
        .update_service(signed(
            r,
            "/citadel.swarm.v1.SwarmService/UpdateService",
            [62; 16],
        ))
        .await
        .unwrap();
    assert_eq!(
        *host.state.registry_headers.lock().unwrap(),
        vec!["fixture-auth", "fixture-auth"]
    );
    let calls = host.state.requests.lock().unwrap();
    let (_, path, body) = calls
        .iter()
        .find(|(_, p, _)| p.contains("/services/service/update"))
        .unwrap();
    assert!(path.contains("registryAuthFrom=spec"));
    assert!(!path.contains("rollback="));
    assert_eq!(body["TaskTemplate"]["ContainerSpec"]["Init"], true);
    assert_eq!(body["RollbackConfig"]["Parallelism"], 2);
    assert_eq!(body["Mode"]["Replicated"]["Replicas"], 0);
}

#[tokio::test]
async fn oversized_and_incomplete_openings_do_not_reach_docker() {
    let host = Harness::start().await;
    let client = reqwest::Client::new();
    let path = "/citadel.volumes.v1.VolumeService/Create";
    let headers = signed(CreateVolumeRequest::default(), path, [71; 16])
        .metadata()
        .clone()
        .into_headers();
    let mut body = vec![0];
    body.extend_from_slice(&(16 * 1024 * 1024 + 1u32).to_be_bytes());
    let response = client
        .post(format!("{}{path}", host.address))
        .headers(headers.clone())
        .header("content-type", "application/grpc")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers().get("grpc-status").unwrap(), "8");
    let response = client
        .post(format!("{}{path}", host.address))
        .headers(headers)
        .header("content-type", "application/grpc")
        .body(vec![0, 0, 0, 0, 2, 0])
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers().get("grpc-status").unwrap(), "16");
    assert_eq!(host.state.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn container_batch_attempts_each_id_and_reports_partial_failure() {
    use citadel_contracts::citadel::containers::v1::*;
    let host = Harness::start().await;
    let mut client = container_service_client::ContainerServiceClient::new(host.channel().await);
    let request = ContainerIds {
        ids: vec!["missing".into(), "created-container".into()],
    };
    let error = client
        .start(signed(
            request,
            "/citadel.containers.v1.ContainerService/Start",
            [81; 16],
        ))
        .await
        .unwrap_err();
    assert!(error.message().contains("Failed (1/2)"));
    let calls = host.state.requests.lock().unwrap();
    assert!(
        calls
            .iter()
            .any(|(_, p, _)| p == "/v1.49/containers/created-container/start")
    );
    assert!(
        calls
            .iter()
            .any(|(_, p, _)| p == "/v1.49/containers/missing/start")
    );
}

#[tokio::test]
async fn core_volume_inspection_keeps_cluster_metadata_and_attached_container_ports() {
    use citadel_platforms::VolumeObservationPort;
    let host = Harness::start().await;
    let client = AgentClient::connect(
        &host.address,
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(5),
        true,
    )
    .await
    .unwrap();
    let volume = client
        .inspect_volume("data", &CancellationToken::new())
        .await
        .unwrap();
    assert!(volume.in_use);
    assert_eq!(volume.containers[0]["state"], "Running");
    assert_eq!(volume.containers[0]["networks"]["Mixed.Network"], "net");
    assert_eq!(volume.containers[0]["ports"]["80/tcp"][0]["hostIP"], "::");
    let cluster = volume.cluster_volume.unwrap();
    assert_eq!(cluster["Version"]["Index"], 9007199254740993_i64);
    assert_eq!(cluster["Spec"]["AccessMode"]["Secrets"][0]["Key"], "key");
    assert_eq!(cluster["Info"]["AccessibleTopology"][0]["Zone.Name"], "z");
    assert_eq!(
        cluster["PublishStatus"][0]["PublishContext"]["Mount.Path"],
        "/data"
    );
    let requests = host.state.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|(_, uri, _)| uri.starts_with("/v1.49/containers/json?all=true&filters="))
            .count(),
        1
    );
    assert!(!requests.iter().any(|(_, uri, _)| uri.contains("/stats")));
}

#[tokio::test]
async fn core_image_inspection_keeps_launch_fields_with_one_inspection() {
    use citadel_platforms::images::ImageInspectionPort;
    let host = Harness::start().await;
    let client = AgentClient::connect(
        &host.address,
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(5),
        true,
    )
    .await
    .unwrap();
    let image = client
        .inspect_image("redis", &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(image.user.as_deref(), Some("1000"));
    assert_eq!(image.working_dir.as_deref(), Some("/data"));
    assert_eq!(image.entry_point, ["redis-server"]);
    assert_eq!(image.stop_signal.as_deref(), Some("SIGTERM"));
    assert_eq!(
        host.state
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, uri, _)| uri == "/v1.49/images/redis/json")
            .count(),
        1
    );
}
