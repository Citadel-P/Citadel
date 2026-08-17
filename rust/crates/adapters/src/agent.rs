use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_stream::stream;
use base64::Engine;
use citadel_contracts::citadel::containers::v1::{
    ListContainersRequest, StreamContainersStatsRequest,
    container_service_client::ContainerServiceClient,
};
use citadel_contracts::citadel::images::v1::{
    ListImagesRequest, image_service_client::ImageServiceClient,
};
use citadel_contracts::citadel::networks::v1::{
    InspectNetworkRequest, InspectNetworkResponse, ListNetworksRequest,
    network_service_client::NetworkServiceClient,
};
use citadel_contracts::citadel::platforms::v1::{
    PlatformStatsRequest, daemon_event_response, platform_service_client::PlatformServiceClient,
};
use citadel_contracts::citadel::shared_models::v1::{ContainerMessage, PlatformInfoResponse};
use citadel_contracts::citadel::swarm::v1::{
    ListSwarmConfigsRequest, ListSwarmNodesRequest, ListSwarmSecretsRequest,
    ListSwarmServicesRequest, ListSwarmTasksRequest, swarm_service_client::SwarmServiceClient,
};
use citadel_contracts::citadel::volumes::v1::{
    InspectVolumeRequest, ListVolumesRequest, volume_service_client::VolumeServiceClient,
};
use citadel_platforms::{
    PlatformInventoryPort, PlatformRuntimePort, RuntimeCapabilityError, RuntimeContainerStat,
    RuntimeContainerStatsStream, RuntimeErrorKind, RuntimeImageSummary, RuntimeNetworkSummary,
    RuntimeStatsStream, RuntimeSwarmConfig, RuntimeSwarmNode, RuntimeSwarmSecret,
    RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use citadel_platforms::{RuntimeContainerSummary, RuntimePlatformInfo, RuntimePlatformStats};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{FutureExt, StreamExt, future::BoxFuture};
use prost::Message;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request, Status};
use url::Url;

const MAX_GRPC_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_UNARY_ATTEMPTS: usize = 3;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(200);
const PLATFORM_INFO_METHOD: &str = "/citadel.platforms.v1.PlatformService/GetPlatformInfo";
const LIST_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/List";
const STREAM_CONTAINERS_STATS_METHOD: &str =
    "/citadel.containers.v1.ContainerService/StreamContainersStats";
const LIST_IMAGES_METHOD: &str = "/citadel.images.v1.ImageService/List";
const LIST_NETWORKS_METHOD: &str = "/citadel.networks.v1.NetworkService/List";
const INSPECT_NETWORK_METHOD: &str = "/citadel.networks.v1.NetworkService/Inspect";
const LIST_VOLUMES_METHOD: &str = "/citadel.volumes.v1.VolumeService/List";
const INSPECT_VOLUME_METHOD: &str = "/citadel.volumes.v1.VolumeService/Inspect";
const LIST_SWARM_NODES_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListNodes";
const LIST_SWARM_SERVICES_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListServices";
const LIST_SWARM_TASKS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListTasks";
const LIST_SWARM_CONFIGS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListConfigs";
const LIST_SWARM_SECRETS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListSecrets";
const STREAM_PLATFORM_STATS_METHOD: &str =
    "/citadel.platforms.v1.PlatformService/StreamPlatformStats";
const STREAM_DAEMON_EVENTS_METHOD: &str = "/citadel.platforms.v1.PlatformService/StreamDaemonEvent";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDaemonEvent {
    pub resource_type: &'static str,
    pub action: String,
}

pub type AgentDaemonEventStream = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = Result<AgentDaemonEvent, RuntimeCapabilityError>> + Send>,
>;

#[derive(Clone)]
pub struct AgentRequestSigner {
    key: SigningKey,
}

impl AgentRequestSigner {
    pub fn from_file(path: &std::path::Path) -> Result<Self, RuntimeCapabilityError> {
        let bytes = fs::read(path).map_err(|error| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Authentication,
                format!("failed to read the Agent signing key: {error}"),
                false,
            )
        })?;
        let mut key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Authentication,
                "the Agent signing key must contain exactly 32 bytes",
                false,
            )
        })?;
        let key = SigningKey::from_bytes(&key_bytes);
        key_bytes.fill(0);
        Ok(Self { key })
    }

    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(bytes),
        }
    }

    #[must_use]
    pub fn public_key_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.key.verifying_key().as_bytes())
    }

    fn sign<T: Message>(
        &self,
        message: T,
        method: &'static str,
        timeout: Option<Duration>,
    ) -> Result<Request<T>, RuntimeCapabilityError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Authentication,
                    format!("system clock is before the Unix epoch: {error}"),
                    false,
                )
            })?
            .as_secs() as i64;
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|error| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Authentication,
                format!("failed to create the Agent request nonce: {error}"),
                false,
            )
        })?;
        self.sign_with(message, method, timeout, timestamp, nonce)
    }

    fn sign_with<T: Message>(
        &self,
        message: T,
        method: &'static str,
        timeout: Option<Duration>,
        timestamp: i64,
        nonce: [u8; 16],
    ) -> Result<Request<T>, RuntimeCapabilityError> {
        let body_hash = Sha256::digest(message.encode_to_vec());
        let mut payload = Vec::with_capacity(8 + nonce.len() + method.len() + body_hash.len());
        payload.extend_from_slice(&timestamp.to_le_bytes());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(method.as_bytes());
        payload.extend_from_slice(&body_hash);
        let signature = self.key.sign(&payload).to_bytes();

        let mut request = Request::new(message);
        if let Some(timeout) = timeout {
            request.set_timeout(timeout);
        }
        insert_binary(&mut request, "x-timestamp-bin", &timestamp.to_le_bytes())?;
        insert_binary(&mut request, "x-nonce-bin", &nonce)?;
        insert_binary(&mut request, "x-content-sha256-bin", &body_hash)?;
        insert_binary(&mut request, "x-signature-bin", &signature)?;
        Ok(request)
    }
}

fn insert_binary<T>(
    request: &mut Request<T>,
    name: &'static str,
    value: &[u8],
) -> Result<(), RuntimeCapabilityError> {
    let metadata = MetadataValue::from_bytes(value);
    request.metadata_mut().insert_bin(name, metadata);
    Ok(())
}

#[derive(Clone)]
pub struct AgentClient {
    channel: Channel,
    signer: AgentRequestSigner,
    operation_timeout: Duration,
    address: String,
}

impl AgentClient {
    pub async fn connect(
        address: &str,
        signer: AgentRequestSigner,
        operation_timeout: Duration,
        allow_insecure: bool,
    ) -> Result<Self, RuntimeCapabilityError> {
        if operation_timeout.is_zero() {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Agent operation timeout must be greater than zero",
                false,
            ));
        }
        let address = validate_address(address, allow_insecure)?;
        let endpoint = Endpoint::from_shared(address.clone())
            .map_err(|error| invalid_address(error.to_string()))?
            .connect_timeout(operation_timeout);
        let channel = tokio::time::timeout(operation_timeout, endpoint.connect())
            .await
            .map_err(|_| timeout_error("connecting to the Agent"))?
            .map_err(|error| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    format!("failed to connect to the Agent: {error}"),
                    true,
                )
            })?;
        Ok(Self {
            channel,
            signer,
            operation_timeout,
            address,
        })
    }

    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    pub async fn handshake(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<RuntimePlatformInfo, RuntimeCapabilityError> {
        let response = self
            .retry_unary(cancellation, || async {
                let request =
                    self.signer
                        .sign((), PLATFORM_INFO_METHOD, Some(self.operation_timeout))?;
                let mut client = self.platform_client();
                client
                    .get_platform_info(request)
                    .await
                    .map(|value| value.into_inner())
                    .map_err(normalize_status)
            })
            .await?;
        Ok(map_platform_info(response))
    }

    async fn retry_unary<T, F, Fut>(
        &self,
        cancellation: &CancellationToken,
        mut operation: F,
    ) -> Result<T, RuntimeCapabilityError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, RuntimeCapabilityError>>,
    {
        let mut delay = INITIAL_RETRY_DELAY;
        for attempt in 1..=MAX_UNARY_ATTEMPTS {
            let result = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                // The gRPC request carries the actual deadline. This slightly wider guard
                // only prevents a broken transport from ignoring that deadline.
                result = tokio::time::timeout(
                    self.operation_timeout.saturating_add(Duration::from_secs(1)),
                    operation(),
                ) => {
                    result.map_err(|_| timeout_error("calling the Agent"))?
                }
            };
            match result {
                Ok(value) => return Ok(value),
                Err(error) if error.retryable && attempt < MAX_UNARY_ATTEMPTS => {
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => return Err(cancelled_error()),
                        () = tokio::time::sleep(delay) => {}
                    }
                    delay = delay.saturating_mul(2);
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the bounded retry loop always returns")
    }

    fn platform_client(&self) -> PlatformServiceClient<Channel> {
        PlatformServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn container_client(&self) -> ContainerServiceClient<Channel> {
        ContainerServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn image_client(&self) -> ImageServiceClient<Channel> {
        ImageServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn network_client(&self) -> NetworkServiceClient<Channel> {
        NetworkServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn volume_client(&self) -> VolumeServiceClient<Channel> {
        VolumeServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn swarm_client(&self) -> SwarmServiceClient<Channel> {
        SwarmServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    pub async fn stream_container_stats(
        &self,
        fetch_interval: Duration,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeContainerStatsStream, RuntimeCapabilityError> {
        let interval_ms = i32::try_from(fetch_interval.as_millis()).map_err(|_| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Agent container stats interval exceeds the protobuf range",
                false,
            )
        })?;
        if interval_ms <= 0 {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Agent container stats interval must be at least one millisecond",
                false,
            ));
        }
        let request = self.signer.sign(
            StreamContainersStatsRequest {
                fetch_interval_ms: interval_ms,
            },
            STREAM_CONTAINERS_STATS_METHOD,
            None,
        )?;
        let mut client = self.container_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(
                self.operation_timeout,
                client.stream_containers_stats(request),
            ) => result.map_err(|_| timeout_error("opening the Agent container stats stream"))?
                .map_err(normalize_status)?,
        };
        let mut input = response.into_inner();
        let cancellation = cancellation.clone();
        Ok(Box::pin(stream! {
            loop {
                let next = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => break,
                    item = input.next() => item,
                };
                match next {
                    Some(Ok(value)) => {
                        let created = chrono::Utc::now().timestamp();
                        yield Ok(value.containers.into_iter().map(|(id, stat)| RuntimeContainerStat {
                            docker_container_id: id,
                            memory_active: stat.memory_active,
                            memory_cache: stat.memory_cache,
                            cpu_usage: stat.cpu_usage,
                            memory_limit: stat.memory_limit,
                            rx_bytes: stat.rx_bytes,
                            tx_bytes: stat.tx_bytes,
                            created,
                        }).collect());
                    }
                    Some(Err(error)) => {
                        yield Err(normalize_status(error));
                        break;
                    }
                    None => break,
                }
            }
        }) as RuntimeContainerStatsStream)
    }

    pub async fn stream_daemon_events(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<AgentDaemonEventStream, RuntimeCapabilityError> {
        let request = self.signer.sign((), STREAM_DAEMON_EVENTS_METHOD, None)?;
        let mut client = self.platform_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(
                self.operation_timeout,
                client.stream_daemon_event(request),
            ) => result.map_err(|_| timeout_error("opening the Agent daemon event stream"))?
                .map_err(normalize_status)?,
        };
        let mut input = response.into_inner();
        let cancellation = cancellation.clone();
        Ok(Box::pin(stream! {
            loop {
                let next = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => break,
                    item = input.next() => item,
                };
                match next {
                    Some(Ok(value)) => {
                        if let Some(event) = map_daemon_event(value.kind) {
                            yield Ok(event);
                        }
                    }
                    Some(Err(error)) => {
                        yield Err(normalize_status(error));
                        break;
                    }
                    None => break,
                }
            }
        }) as AgentDaemonEventStream)
    }
}

fn map_daemon_event(kind: Option<daemon_event_response::Kind>) -> Option<AgentDaemonEvent> {
    let event = match kind? {
        daemon_event_response::Kind::DaemonContainerEventResponse(value) => AgentDaemonEvent {
            resource_type: "container",
            action: value.action,
        },
        daemon_event_response::Kind::DaemonImageEventResponse(value) => AgentDaemonEvent {
            resource_type: "image",
            action: value.action,
        },
        daemon_event_response::Kind::DaemonVolumeEventResponse(value) => AgentDaemonEvent {
            resource_type: "volume",
            action: value.action,
        },
        daemon_event_response::Kind::DaemonNetworkEventResponse(value) => AgentDaemonEvent {
            resource_type: "network",
            action: value.action,
        },
        daemon_event_response::Kind::DaemonResourceEventResponse(value) => AgentDaemonEvent {
            resource_type: daemon_resource_type(value.r#type),
            action: value.action,
        },
    };
    (!event.action.is_empty()).then_some(event)
}

const fn daemon_resource_type(value: i32) -> &'static str {
    match value {
        0 => "builder",
        1 => "config",
        2 => "container",
        3 => "daemon",
        4 => "image",
        5 => "network",
        6 => "node",
        7 => "plugin",
        8 => "secret",
        9 => "service",
        10 => "volume",
        _ => "unknown",
    }
}

impl PlatformRuntimePort for AgentClient {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        self.handshake(cancellation).boxed()
    }

    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListContainersRequest {
                            all: Some(true),
                            limit: None,
                            size: Some(false),
                            filters: Default::default(),
                        },
                        LIST_CONTAINERS_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    let mut client = self.container_client();
                    client
                        .list(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            let mut containers = response
                .containers
                .into_values()
                .map(map_container)
                .collect::<Vec<_>>();
            containers.sort_unstable_by(|left, right| left.id.cmp(&right.id));
            Ok(containers)
        }
        .boxed()
    }

    fn stream_stats<'a>(
        &'a self,
        fetch_interval: Duration,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
        async move {
            let interval_ms = i32::try_from(fetch_interval.as_millis()).map_err(|_| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "the Agent stats interval exceeds the protobuf range",
                    false,
                )
            })?;
            if interval_ms <= 0 {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "the Agent stats interval must be at least one millisecond",
                    false,
                ));
            }
            let request = self.signer.sign(
                PlatformStatsRequest {
                    fetch_interval_ms: interval_ms,
                },
                STREAM_PLATFORM_STATS_METHOD,
                None,
            )?;
            let mut client = self.platform_client();
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.stream_platform_stats(request)) => {
                    result.map_err(|_| timeout_error("opening the Agent stats stream"))?
                        .map_err(normalize_status)?
                }
            };
            let mut input = response.into_inner();
            let cancellation = cancellation.clone();
            Ok(Box::pin(stream! {
                loop {
                    let next = tokio::select! {
                        biased;
                        () = cancellation.cancelled() => break,
                        item = input.next() => item,
                    };
                    match next {
                        Some(Ok(value)) => yield Ok(map_platform_stats(value)),
                        Some(Err(error)) => {
                            yield Err(normalize_status(error));
                            break;
                        }
                        None => break,
                    }
                }
            }) as RuntimeStatsStream)
        }
        .boxed()
    }
}

impl PlatformInventoryPort for AgentClient {
    fn list_images<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListImagesRequest {},
                        LIST_IMAGES_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.image_client()
                        .list(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.images.into_iter().map(map_image).collect())
        }
        .boxed()
    }

    fn list_networks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListNetworksRequest {
                            dangling: None,
                            driver: None,
                            id: None,
                            name: None,
                        },
                        LIST_NETWORKS_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.network_client()
                        .list(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.networks.into_iter().map(map_network).collect())
        }
        .boxed()
    }

    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        InspectNetworkRequest { id: id.to_owned() },
                        INSPECT_NETWORK_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.network_client()
                        .inspect(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(map_network_inspect(response))
        }
        .boxed()
    }

    fn list_volumes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListVolumesRequest {
                            dangling: None,
                            driver: None,
                            name: None,
                        },
                        LIST_VOLUMES_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.volume_client()
                        .list(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.volumes.into_iter().map(map_volume).collect())
        }
        .boxed()
    }

    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        InspectVolumeRequest {
                            name: name.to_owned(),
                        },
                        INSPECT_VOLUME_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.volume_client()
                        .inspect(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(map_volume(response))
        }
        .boxed()
    }

    fn list_swarm_nodes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmNode>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListSwarmNodesRequest {
                            max_items: 10_000,
                            include_task_counts: true,
                        },
                        LIST_SWARM_NODES_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .list_nodes(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.nodes.into_iter().map(map_swarm_node).collect())
        }
        .boxed()
    }

    fn list_swarm_services<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmService>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListSwarmServicesRequest { max_items: 10_000 },
                        LIST_SWARM_SERVICES_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .list_services(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response
                .services
                .into_iter()
                .map(map_swarm_service)
                .collect())
        }
        .boxed()
    }

    fn list_swarm_tasks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListSwarmTasksRequest { max_items: 10_000 },
                        LIST_SWARM_TASKS_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .list_tasks(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.tasks.into_iter().map(map_swarm_task).collect())
        }
        .boxed()
    }

    fn list_swarm_configs<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListSwarmConfigsRequest { max_items: 10_000 },
                        LIST_SWARM_CONFIGS_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .list_configs(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.configs.into_iter().map(map_swarm_config).collect())
        }
        .boxed()
    }

    fn list_swarm_secrets<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListSwarmSecretsRequest { max_items: 10_000 },
                        LIST_SWARM_SECRETS_METHOD,
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .list_secrets(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(response.secrets.into_iter().map(map_swarm_secret).collect())
        }
        .boxed()
    }
}

fn map_image(
    image: citadel_contracts::citadel::shared_models::v1::ImageReply,
) -> RuntimeImageSummary {
    RuntimeImageSummary {
        id: image.id,
        repo_tags: image.repo_tags,
        repo_digests: image.repo_digests,
        created: image.created,
        size: image.size.clamp(i64::MIN as f64, i64::MAX as f64) as i64,
        containers: i64::from(image.containers),
    }
}

fn map_network(
    network: citadel_contracts::citadel::shared_models::v1::Network,
) -> RuntimeNetworkSummary {
    let ipam = network.ipam.map(|ipam| {
        serde_json::json!({
            "Driver": ipam.driver,
            "Config": ipam.config.into_iter().map(|config| serde_json::json!({
                "Subnet": config.subnet,
                "IPRange": config.ip_range,
                "Gateway": config.gateway,
            })).collect::<Vec<_>>(),
            "Options": ipam.options,
        })
    });
    RuntimeNetworkSummary {
        id: network.id,
        name: network.name,
        created: network.created,
        driver: network.driver,
        scope: network.scope,
        enable_ipv4: network.enable_i_pv4,
        enable_ipv6: network.enable_i_pv6,
        internal: network.internal,
        attachable: network.attachable,
        ingress: network.ingress,
        config_only: network.config_only,
        config_from: network.config_from,
        ipam,
        options: network.options.into_iter().collect(),
        labels: network.labels.into_iter().collect(),
        container_count: usize::from(network.in_use),
        containers: Default::default(),
        peers: Vec::new(),
    }
}

fn map_network_inspect(network: InspectNetworkResponse) -> RuntimeNetworkSummary {
    let container_count = network.containers.len();
    RuntimeNetworkSummary {
        id: network.id,
        name: network.name,
        created: network.created,
        driver: network.driver,
        scope: network.scope,
        enable_ipv4: network.enable_i_pv4,
        enable_ipv6: network.enable_i_pv6,
        internal: network.internal,
        attachable: network.attachable,
        ingress: network.ingress,
        config_only: network.config_only,
        config_from: network.config_from,
        ipam: network.ipam.map(|ipam| {
            serde_json::json!({
                "Driver": ipam.driver,
                "Config": ipam.config.into_iter().map(|config| serde_json::json!({
                    "Subnet": config.subnet,
                    "IPRange": config.ip_range,
                    "Gateway": config.gateway,
                })).collect::<Vec<_>>(),
                "Options": ipam.options,
            })
        }),
        options: network.options.into_iter().collect(),
        labels: network.labels.into_iter().collect(),
        container_count,
        containers: network
            .containers
            .into_iter()
            .map(|(id, container)| {
                (
                    id,
                    serde_json::json!({
                        "Name": container.name,
                        "EndpointID": container.endpoint_id,
                        "MacAddress": container.mac_address,
                        "IPv4Address": container.ip_pv4_address,
                        "IPv6Address": container.ipv6_address,
                    }),
                )
            })
            .collect(),
        peers: network
            .peers
            .into_iter()
            .map(|peer| serde_json::json!({ "Name": peer.name, "IP": peer.ip }))
            .collect(),
    }
}

fn map_volume(
    volume: citadel_contracts::citadel::shared_models::v1::VolumeResponse,
) -> RuntimeVolumeSummary {
    RuntimeVolumeSummary {
        name: volume.name,
        in_use: volume.in_use,
        scope: volume.scope,
        driver: volume.driver,
        mountpoint: volume.mountpoint,
        created_at: volume.created_at,
        // Cluster Volume details are not used by Phase 4 reads and the current
        // protobuf model is richer than the Docker JSON subset. Preserve absence
        // instead of fabricating a transport-specific shape.
        cluster_volume: None,
        usage_data: volume.usage_data.map(|usage| {
            serde_json::json!({
                "Size": usage.size,
                "RefCount": usage.ref_count,
            })
        }),
        status: volume
            .status
            .into_iter()
            .map(|(key, value)| (key, serde_json::Value::String(value)))
            .collect(),
        labels: volume.labels.into_iter().collect(),
        options: volume.options.into_iter().collect(),
        containers: volume
            .containers
            .into_iter()
            .map(|container| {
                serde_json::json!({
                    "id": container.id,
                    "name": container.name,
                    "image": container.image,
                    "imageId": container.image_id,
                    "state": container.state,
                    "networks": container.networks,
                })
            })
            .collect(),
    }
}

fn map_swarm_node(
    value: citadel_contracts::citadel::swarm::v1::SwarmNodeMessage,
) -> RuntimeSwarmNode {
    RuntimeSwarmNode {
        id: value.id,
        version_index: bounded_u64(value.version_index),
        hostname: value.hostname,
        role: value.role,
        is_leader: value.is_leader,
        reachability: value.reachability,
        status: value.status,
        status_message: nonempty(value.status_message),
        availability: value.availability,
        engine_version: value.engine_version,
        operating_system: value.operating_system,
        architecture: value.architecture,
        address: value.address,
        labels: value.labels.into_iter().collect(),
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
}

fn map_swarm_service(
    value: citadel_contracts::citadel::swarm::v1::SwarmServiceMessage,
) -> RuntimeSwarmService {
    RuntimeSwarmService {
        id: value.id,
        version_index: bounded_u64(value.version_index),
        name: value.name,
        mode: value.mode,
        image: value.image,
        running_task_count: value.running_task_count,
        desired_task_count: value.desired_task_count,
        update_state: value.update_state,
        update_message: nonempty(value.update_message),
        ports: value.ports,
        network_ids: value.network_ids,
        secret_ids: value.secret_ids,
        config_ids: value.config_ids,
        stack_namespace: value.labels.get("com.docker.stack.namespace").cloned(),
        labels: value.labels.into_iter().collect(),
        ownership: Default::default(),
        ownership_diagnostic: None,
        swarm_service_id: None,
        stack_id: None,
        force_update: value.force_update,
        runtime_hash: value.runtime_hash,
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
    .normalize_ownership()
}

fn map_swarm_task(
    value: citadel_contracts::citadel::swarm::v1::SwarmTaskMessage,
) -> RuntimeSwarmTask {
    RuntimeSwarmTask {
        id: value.id,
        version_index: bounded_u64(value.version_index),
        name: value.name,
        service_id: value.service_id,
        slot: value.slot,
        node_id: value.node_id,
        desired_state: value.desired_state,
        state: value.state,
        status_message: nonempty(value.status_message),
        error: nonempty(value.error),
        image: value.image,
        ports: value.ports,
        container_id: nonempty(value.container_id),
        status_timestamp: protobuf_timestamp(value.status_timestamp),
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
}

fn map_swarm_config(
    value: citadel_contracts::citadel::swarm::v1::SwarmConfigMessage,
) -> RuntimeSwarmConfig {
    RuntimeSwarmConfig {
        id: value.id,
        version_index: bounded_u64(value.version_index),
        name: value.name,
        templating_driver: nonempty(value.templating_driver),
        labels: value.labels.into_iter().collect(),
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
}

fn map_swarm_secret(
    value: citadel_contracts::citadel::swarm::v1::SwarmSecretMessage,
) -> RuntimeSwarmSecret {
    RuntimeSwarmSecret {
        id: value.id,
        version_index: bounded_u64(value.version_index),
        name: value.name,
        driver: nonempty(value.driver),
        labels: value.labels.into_iter().collect(),
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
}

fn protobuf_timestamp(
    value: Option<prost_types::Timestamp>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let value = value?;
    chrono::DateTime::from_timestamp(value.seconds, u32::try_from(value.nanos).ok()?)
}

fn bounded_u64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn map_platform_info(value: PlatformInfoResponse) -> RuntimePlatformInfo {
    let stats = value.platform_stat.unwrap_or_default();
    RuntimePlatformInfo {
        daemon_id: value.id,
        server_version: value.server_version,
        operating_system: value.operating_system,
        os_type: value.os_type,
        architecture: value.architecture,
        cpu_count: value.cpu_count,
        memory_total: value.mem_total,
        container_count: i64::from(stats.container_count),
        containers_running: i64::from(stats.containers_running),
        containers_paused: i64::from(stats.containers_paused),
        containers_stopped: i64::from(stats.containers_stopped),
        api_version: value.api_version,
        minimum_api_version: value.minimum_api_version,
        agent_version: Some(value.agent_version),
    }
}

fn map_container(value: ContainerMessage) -> RuntimeContainerSummary {
    let state = value.state().as_str_name().to_ascii_lowercase();
    let ports = serde_json::Value::Object(
        value
            .ports
            .into_iter()
            .map(|(port, bindings)| {
                let bindings = bindings
                    .host_port_binding
                    .into_iter()
                    .map(|binding| {
                        serde_json::json!({
                            "hostIp": binding.host_ip,
                            "hostPort": binding.host_port,
                        })
                    })
                    .collect();
                (port, serde_json::Value::Array(bindings))
            })
            .collect(),
    );
    RuntimeContainerSummary {
        id: value.id,
        name: value.name.trim_start_matches('/').to_owned(),
        image: value.image,
        image_id: value.image_id,
        created: value.created,
        state,
        status: value.status,
        labels: Default::default(),
        ports,
        stack: value.stack,
        is_system: value.is_system,
        system_role: value.system_role,
        has_citadel_ownership_labels: value.has_citadel_ownership_labels,
        is_swarm_task: value.is_swarm_task,
    }
}

fn map_platform_stats(
    value: citadel_contracts::citadel::platforms::v1::PlatformStatsResponse,
) -> RuntimePlatformStats {
    let stat = value.stat.unwrap_or_default();
    RuntimePlatformStats {
        memory_usage: stat.memory_usage,
        cpu_usage: stat.cpu_usage,
        receive_bytes: stat.rx_bytes,
        transmit_bytes: stat.tx_bytes,
        container_count: i64::from(stat.container_count),
        containers_running: i64::from(stat.containers_running),
        containers_paused: i64::from(stat.containers_paused),
        containers_stopped: i64::from(stat.containers_stopped),
    }
}

fn validate_address(address: &str, allow_insecure: bool) -> Result<String, RuntimeCapabilityError> {
    let url = Url::parse(address).map_err(|error| invalid_address(error.to_string()))?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err(invalid_address(
            "the Agent address must be an origin without credentials, path, query, or fragment",
        ));
    }
    match url.scheme() {
        "http" if !allow_insecure => Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Authentication,
            "unencrypted Agent transport is disabled",
            false,
        )),
        "http" => Ok(url.to_string().trim_end_matches('/').to_owned()),
        "https" => Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "direct Agent TLS is deferred to the transport-security phase; use the current h2c Agent only when AgentTransport__AllowInsecure=true",
            false,
        )),
        _ => Err(invalid_address(
            "the Agent address scheme must be http or https",
        )),
    }
}

fn invalid_address(message: impl Into<String>) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::InvalidRequest, message, false)
}

fn cancelled_error() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Agent call cancelled", false)
}

fn timeout_error(operation: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Timeout,
        format!("timed out while {operation}"),
        false,
    )
}

fn normalize_status(status: Status) -> RuntimeCapabilityError {
    let (kind, retryable) = match status.code() {
        // Local cancellation is handled by the biased CancellationToken branch. Tonic
        // reports an expired client-side grpc-timeout as Cancelled on this transport.
        Code::Cancelled | Code::DeadlineExceeded => (RuntimeErrorKind::Timeout, false),
        Code::Unavailable => (RuntimeErrorKind::Unavailable, true),
        Code::Unauthenticated => (RuntimeErrorKind::Authentication, false),
        Code::PermissionDenied => (RuntimeErrorKind::PermissionDenied, false),
        Code::InvalidArgument | Code::FailedPrecondition | Code::OutOfRange => {
            (RuntimeErrorKind::InvalidRequest, false)
        }
        Code::NotFound => (RuntimeErrorKind::NotFound, false),
        Code::AlreadyExists | Code::Aborted => (RuntimeErrorKind::Conflict, false),
        Code::ResourceExhausted => (RuntimeErrorKind::ResourceExhausted, false),
        _ => (RuntimeErrorKind::Remote, false),
    };
    RuntimeCapabilityError::new(kind, status.message(), retryable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_contracts::citadel::networks::v1::{NetworkContainerMessage, PeerInfoMessage};
    use citadel_contracts::citadel::platforms::v1::{
        DaemonContainerEventResponse, DaemonResourceEventResponse,
    };
    use citadel_contracts::citadel::shared_models::v1::{
        ContainerStateType, ContainerVolumeResult, HostPortBinding, HostPortBindingList,
        IpamConfigMessage, IpamMessage, Network, UsageDataMessage, VolumeResponse,
    };
    use citadel_contracts::citadel::swarm::v1::SwarmServiceMessage;
    use ed25519_dalek::{Signature, Verifier};
    use uuid::Uuid;

    #[test]
    fn signs_the_exact_dotnet_payload_shape() {
        let signer = AgentRequestSigner::from_bytes(&[7; 32]);
        let nonce = [9; 16];
        let timestamp = 1_725_000_000_i64;
        let request = signer
            .sign_with((), PLATFORM_INFO_METHOD, None, timestamp, nonce)
            .unwrap();
        let body_hash = Sha256::digest([]);
        let mut expected = Vec::new();
        expected.extend_from_slice(&timestamp.to_le_bytes());
        expected.extend_from_slice(&nonce);
        expected.extend_from_slice(PLATFORM_INFO_METHOD.as_bytes());
        expected.extend_from_slice(&body_hash);
        let signature = request
            .metadata()
            .get_bin("x-signature-bin")
            .unwrap()
            .to_bytes()
            .unwrap();
        signer
            .key
            .verifying_key()
            .verify(&expected, &Signature::from_slice(&signature).unwrap())
            .unwrap();
        assert_eq!(
            request
                .metadata()
                .get_bin("x-content-sha256-bin")
                .unwrap()
                .to_bytes()
                .unwrap(),
            body_hash.as_slice()
        );
    }

    #[test]
    fn enforces_explicit_insecure_transport_opt_in() {
        assert!(validate_address("http://agent:9000", true).is_ok());
        assert!(matches!(
            validate_address("http://agent:9000", false),
            Err(RuntimeCapabilityError {
                kind: RuntimeErrorKind::Authentication,
                ..
            })
        ));
        assert!(validate_address("http://user@agent:9000/path", true).is_err());
    }

    #[test]
    fn normalizes_only_transient_agent_failures_as_retryable() {
        assert!(normalize_status(Status::unavailable("offline")).retryable);
        assert!(!normalize_status(Status::deadline_exceeded("slow")).retryable);
        assert!(!normalize_status(Status::unauthenticated("bad signature")).retryable);
        assert!(!normalize_status(Status::invalid_argument("bad input")).retryable);
    }

    #[test]
    fn agent_daemon_events_normalize_to_the_shared_inventory_trigger_shape() {
        let container = map_daemon_event(Some(
            daemon_event_response::Kind::DaemonContainerEventResponse(
                DaemonContainerEventResponse {
                    action: "start".to_owned(),
                    ..Default::default()
                },
            ),
        ))
        .unwrap();
        assert_eq!(container.resource_type, "container");
        assert_eq!(container.action, "start");

        let service = map_daemon_event(Some(
            daemon_event_response::Kind::DaemonResourceEventResponse(DaemonResourceEventResponse {
                r#type: 9,
                action: "update".to_owned(),
                ..Default::default()
            }),
        ))
        .unwrap();
        assert_eq!(service.resource_type, "service");
        assert_eq!(service.action, "update");
        assert!(map_daemon_event(None).is_none());
    }

    #[test]
    fn reads_stream_container_counts_from_the_platform_stat_message() {
        let mapped = map_platform_stats(
            citadel_contracts::citadel::platforms::v1::PlatformStatsResponse {
                container_count: 0,
                containers_running: 0,
                stat: Some(
                    citadel_contracts::citadel::shared_models::v1::PlatformStatMessage {
                        container_count: 7,
                        containers_running: 5,
                        containers_paused: 1,
                        containers_stopped: 1,
                        ..Default::default()
                    },
                ),
                ..Default::default()
            },
        );

        assert_eq!(mapped.container_count, 7);
        assert_eq!(mapped.containers_running, 5);
        assert_eq!(mapped.containers_paused, 1);
        assert_eq!(mapped.containers_stopped, 1);
    }

    #[test]
    fn container_mapping_preserves_agent_port_bindings() {
        let mapped = map_container(ContainerMessage {
            id: "container-1".to_owned(),
            name: "/web".to_owned(),
            state: ContainerStateType::Running as i32,
            ports: [(
                "80/tcp".to_owned(),
                HostPortBindingList {
                    host_port_binding: vec![HostPortBinding {
                        host_ip: Some("0.0.0.0".to_owned()),
                        host_port: Some("8080".to_owned()),
                    }],
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        });

        assert_eq!(mapped.name, "web");
        assert_eq!(mapped.state, "running");
        assert_eq!(mapped.ports["80/tcp"][0]["hostIp"], "0.0.0.0");
        assert_eq!(mapped.ports["80/tcp"][0]["hostPort"], "8080");
    }

    #[test]
    fn network_mapping_preserves_ipam_across_agent_transport() {
        let mapped = map_network(Network {
            id: "network-1".to_owned(),
            name: "frontend".to_owned(),
            in_use: true,
            ipam: Some(IpamMessage {
                driver: Some("default".to_owned()),
                config: vec![IpamConfigMessage {
                    subnet: Some("10.0.0.0/24".to_owned()),
                    ip_range: None,
                    gateway: Some("10.0.0.1".to_owned()),
                }],
                ..Default::default()
            }),
            ..Default::default()
        });

        assert_eq!(mapped.container_count, 1);
        assert_eq!(mapped.ipam.as_ref().unwrap()["Driver"], "default");
        assert_eq!(
            mapped.ipam.as_ref().unwrap()["Config"][0]["Subnet"],
            "10.0.0.0/24"
        );
    }

    #[test]
    fn network_inspect_preserves_connected_containers_and_peers() {
        let mapped = map_network_inspect(InspectNetworkResponse {
            id: "network-1".to_owned(),
            name: "frontend".to_owned(),
            containers: [(
                "container-1".to_owned(),
                NetworkContainerMessage {
                    name: "web".to_owned(),
                    endpoint_id: "endpoint-1".to_owned(),
                    ip_pv4_address: "10.0.0.2/24".to_owned(),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            peers: vec![PeerInfoMessage {
                name: "worker-1".to_owned(),
                ip: "10.0.0.3".to_owned(),
            }],
            ..Default::default()
        });

        assert_eq!(mapped.container_count, 1);
        assert_eq!(mapped.containers["container-1"]["Name"], "web");
        assert_eq!(mapped.peers[0]["Name"], "worker-1");
    }

    #[test]
    fn volume_mapping_preserves_usage_and_status() {
        let mapped = map_volume(VolumeResponse {
            name: "data".to_owned(),
            in_use: true,
            usage_data: Some(UsageDataMessage {
                size: Some(1024),
                ref_count: Some(2),
            }),
            status: [("state".to_owned(), "ready".to_owned())]
                .into_iter()
                .collect(),
            containers: vec![ContainerVolumeResult {
                id: "container-1".to_owned(),
                name: "database".to_owned(),
                image: "postgres".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });

        assert!(mapped.in_use);
        assert_eq!(mapped.usage_data.as_ref().unwrap()["Size"], 1024);
        assert_eq!(mapped.status["state"], "ready");
        assert_eq!(mapped.containers[0]["name"], "database");
    }

    #[test]
    fn swarm_service_mapping_preserves_ownership_and_bounded_version() {
        let service_id = Uuid::now_v7();
        let mapped = map_swarm_service(SwarmServiceMessage {
            id: "docker-service-1".to_owned(),
            version_index: u64::MAX,
            name: "web".to_owned(),
            labels: [
                ("com.citadel.managed".to_owned(), "true".to_owned()),
                ("com.citadel.service-id".to_owned(), service_id.to_string()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        });

        assert_eq!(mapped.version_index, i64::MAX);
        assert_eq!(mapped.swarm_service_id, Some(service_id));
        assert_eq!(
            mapped.ownership,
            citadel_domain::SwarmServiceOwnership::CitadelService
        );
    }
}
