use crate::connectors::docker::events::{self, ContainerChange, RuntimeEventKind};
use std::fs;

mod containers;
#[cfg(test)]
mod key_rotation_tests;

pub(crate) mod images;
mod logs;
mod node_agents;
mod swarm_inventory;
mod terminal;
mod volumes;
pub(crate) mod workloads;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_stream::stream;
use base64::Engine;
use citadel_contracts::citadel::containers::v1::{
    ContainerIds, CreateContainerRequest, DeleteContainerRequest, ExecBinaryRequest,
    ExecServerMessage, ListContainersRequest, StreamContainersStatsRequest,
    container_service_client::ContainerServiceClient, exec_server_message,
};
use citadel_contracts::citadel::deployments::v1::{
    ApplyDeploymentRequest, ContainerRestartPolicy as ProtoRestartPolicy, DeployedContainerState,
    DeploymentSpec as ProtoDeploymentSpec, LifeCycleSpec as ProtoLifeCycleSpec,
    ResourceSpec as ProtoResourceSpec, StopSignal as ProtoStopSignal,
    deployment_service_client::DeploymentServiceClient,
};
use citadel_contracts::citadel::images::v1::{
    BuildImageRequest, BuildImageSecret, ImageBuildResponse, ListImagesRequest, PullImageRequest,
    PushImageRequest, image_service_client::ImageServiceClient,
};
use citadel_contracts::citadel::networks::v1::{
    ConfigFromMessage, CreateNetworkRequest, DeleteNetworkRequest, InspectNetworkRequest,
    InspectNetworkResponse, ListNetworksRequest, network_service_client::NetworkServiceClient,
};
use citadel_contracts::citadel::platforms::v1::{
    PlatformStatsRequest, daemon_event_response, platform_service_client::PlatformServiceClient,
};
use citadel_contracts::citadel::shared_models::v1::{
    ContainerMessage, IpamConfigMessage, IpamMessage, PlatformInfoResponse,
};
use citadel_contracts::citadel::stacks::v1::{
    StackApplyEventType as ProtoStackApplyEventType, StackApplyRequest,
    StackCommand as ProtoStackCommand, StackOrchestrationMode as ProtoStackOrchestrationMode,
    StackSourceFile as ProtoStackSourceFile, stack_service_client::StackServiceClient,
};
use citadel_contracts::citadel::swarm::v1::{
    CreateManagedSwarmServiceRequest, DeleteManagedSwarmServiceRequest, InspectSwarmServiceRequest,
    ListSwarmConfigsRequest, ListSwarmNodesRequest, ListSwarmSecretsRequest,
    ListSwarmServicesRequest, ListSwarmTasksRequest, SwarmServiceMessage,
    SwarmServiceMutationResponse, UpdateManagedSwarmServiceRequest,
    swarm_service_client::SwarmServiceClient,
};
use citadel_contracts::citadel::volumes::v1::{
    CreateVolumeRequest, InspectVolumeRequest, ListVolumesRequest, RemoveVolumeRequest,
    volume_service_client::VolumeServiceClient,
};
use citadel_deployments::{
    ContainerRestartPolicy, RuntimeContainerState, RuntimeDeploymentCommand,
    RuntimeDeploymentResult, StopSignal,
};
use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, CreatedRuntimeNetwork, RuntimeCapabilityError,
    RuntimeContainerStat, RuntimeContainerStatsStream, RuntimeErrorKind, RuntimeImageSummary,
    RuntimeNetworkSummary, RuntimeStatsStream, RuntimeSwarmConfig, RuntimeSwarmNode,
    RuntimeSwarmSecret, RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use citadel_platforms::{
    RuntimeContainerSummary, RuntimePlatformInfo, RuntimePlatformStats, RuntimeSwarmInfo,
    RuntimeSwarmPeer,
};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{FutureExt, StreamExt, future::BoxFuture};
use prost::Message;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tonic::metadata::MetadataValue;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint};
use tonic::{Code, Request, Status};
use url::Url;
use zeroize::Zeroizing;

const MAX_GRPC_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_UNARY_ATTEMPTS: usize = 3;

pub(crate) fn map_container_stats(
    value: citadel_contracts::citadel::containers::v1::ContainersStatsResponse,
) -> Vec<RuntimeContainerStat> {
    let created = value
        .captured_at
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    value
        .containers
        .into_iter()
        .map(|(id, stat)| RuntimeContainerStat {
            docker_container_id: id,
            memory_active: stat.memory_active,
            memory_cache: stat.memory_cache,
            cpu_usage: stat.cpu_usage,
            memory_limit: stat.memory_limit,
            rx_bytes: stat.rx_bytes,
            tx_bytes: stat.tx_bytes,
            created,
        })
        .collect()
}

impl citadel_platforms::SwarmTaskRuntimePort for AgentClient {
    fn inspect_task<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeSwarmTask, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        citadel_contracts::citadel::swarm::v1::InspectSwarmTaskRequest {
                            task_id: id.into(),
                        },
                        "/citadel.swarm.v1.SwarmService/InspectTask",
                        Some(self.operation_timeout),
                    )?;
                    self.swarm_client()
                        .inspect_task(request)
                        .await
                        .map(|value| value.into_inner())
                        .map_err(normalize_status)
                })
                .await?;
            Ok(map_swarm_task(response))
        }
        .boxed()
    }
}
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(200);

#[doc(hidden)]
pub struct AgentStackRegistry {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) auth: Zeroizing<String>,
}
const PLATFORM_INFO_METHOD: &str = "/citadel.platforms.v1.PlatformService/GetPlatformInfo";
const LIST_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/List";
const DELETE_CONTAINER_METHOD: &str = "/citadel.containers.v1.ContainerService/Delete";
const CREATE_CONTAINER_METHOD: &str = "/citadel.containers.v1.ContainerService/Create";
const EXEC_BINARY_METHOD: &str = "/citadel.containers.v1.ContainerService/ExecBinary";
const START_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/Start";
const STOP_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/Stop";
const PAUSE_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/Pause";
const UNPAUSE_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/Unpause";
const RESTART_CONTAINERS_METHOD: &str = "/citadel.containers.v1.ContainerService/Restart";
const APPLY_DEPLOYMENT_METHOD: &str = "/citadel.deployments.v1.DeploymentService/Apply";
const APPLY_STACK_METHOD: &str = "/citadel.stacks.v1.StackService/Apply";
const STREAM_CONTAINERS_STATS_METHOD: &str =
    "/citadel.containers.v1.ContainerService/StreamContainersStats";
const LIST_IMAGES_METHOD: &str = "/citadel.images.v1.ImageService/List";
const PULL_IMAGE_METHOD: &str = "/citadel.images.v1.ImageService/Pull";
const BUILD_IMAGE_METHOD: &str = "/citadel.images.v1.ImageService/Build";
const PUSH_IMAGE_METHOD: &str = "/citadel.images.v1.ImageService/Push";
const LIST_NETWORKS_METHOD: &str = "/citadel.networks.v1.NetworkService/List";
const INSPECT_NETWORK_METHOD: &str = "/citadel.networks.v1.NetworkService/Inspect";
const CREATE_NETWORK_METHOD: &str = "/citadel.networks.v1.NetworkService/Create";
const DELETE_NETWORK_METHOD: &str = "/citadel.networks.v1.NetworkService/Delete";
const LIST_VOLUMES_METHOD: &str = "/citadel.volumes.v1.VolumeService/List";
const INSPECT_VOLUME_METHOD: &str = "/citadel.volumes.v1.VolumeService/Inspect";
const CREATE_VOLUME_METHOD: &str = "/citadel.volumes.v1.VolumeService/Create";
const DELETE_VOLUME_METHOD: &str = "/citadel.volumes.v1.VolumeService/Remove";
const LIST_SWARM_NODES_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListNodes";
const LIST_SWARM_SERVICES_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListServices";
const LIST_SWARM_TASKS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListTasks";
const LIST_SWARM_CONFIGS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListConfigs";
const LIST_SWARM_SECRETS_METHOD: &str = "/citadel.swarm.v1.SwarmService/ListSecrets";
const INSPECT_SWARM_SERVICE_METHOD: &str = "/citadel.swarm.v1.SwarmService/InspectService";
const CREATE_SWARM_SERVICE_METHOD: &str = "/citadel.swarm.v1.SwarmService/CreateService";
const UPDATE_SWARM_SERVICE_METHOD: &str = "/citadel.swarm.v1.SwarmService/UpdateService";
const DELETE_SWARM_SERVICE_METHOD: &str = "/citadel.swarm.v1.SwarmService/DeleteService";
const STREAM_PLATFORM_STATS_METHOD: &str =
    "/citadel.platforms.v1.PlatformService/StreamPlatformStats";
const STREAM_DAEMON_EVENTS_METHOD: &str = "/citadel.platforms.v1.PlatformService/StreamDaemonEvent";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDaemonEvent {
    pub resource: Option<citadel_platforms::jobs::ResourceDelta>,
    pub swarm_scope: bool,
    pub kind: RuntimeEventKind,
    pub container_id: Option<String>,
    pub container_state: Option<String>,
    pub container_name: Option<String>,
    pub container: Option<RuntimeContainerSummary>,
    pub resource_type: &'static str,
    pub action: String,
}

pub type AgentDaemonEventStream = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = Result<AgentDaemonEvent, RuntimeCapabilityError>> + Send>,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentContainerAction {
    Start,
    Stop,
    Pause,
    Unpause,
    Restart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBinaryExecOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[derive(Clone)]
pub struct AgentRequestSigner {
    key: std::sync::Arc<std::sync::RwLock<SigningKey>>,
    path: Option<std::sync::Arc<std::path::PathBuf>>,
}

impl AgentRequestSigner {
    /// Initialize the instance identity once, publishing only a fully written key.
    /// Existing or malformed files are never replaced automatically.
    pub fn load_or_create(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        use std::io::Write;
        if path.try_exists()? {
            return Ok(Self::from_file(path)?);
        }
        let parent = path
            .parent()
            .ok_or("Agent key path has no parent directory")?;
        let mut directories = fs::DirBuilder::new();
        directories.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            directories.mode(0o700);
        }
        directories.create(parent)?;
        let temporary = parent.join(format!(".agent-key-{}", uuid::Uuid::now_v7()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
            getrandom::fill(bytes.as_mut()).map_err(|error| error.to_string())?;
            file.write_all(bytes.as_ref())?;
            file.sync_all()?;
            drop(file);
            match fs::hard_link(&temporary, path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
                Err(error) => Err(error.into()),
            }
        })();
        fs::remove_file(&temporary)?;
        result?;
        Ok(Self::from_file(path)?)
    }

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
        Ok(Self {
            key: std::sync::Arc::new(std::sync::RwLock::new(key)),
            path: Some(std::sync::Arc::new(path.to_owned())),
        })
    }

    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            key: std::sync::Arc::new(std::sync::RwLock::new(SigningKey::from_bytes(bytes))),
            path: None,
        }
    }

    #[must_use]
    pub fn public_key_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(
            self.key
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .verifying_key()
                .as_bytes(),
        )
    }

    /// Publish the new key only after its atomic replacement succeeds. Cloned clients
    /// share this lock, so subsequent requests immediately use the rotated identity.
    pub fn rotate(&self) -> Result<String, std::io::Error> {
        use std::io::Write;
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| std::io::Error::other("No persistent Agent key is configured."))?;
        let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::fill(bytes.as_mut()).map_err(std::io::Error::other)?;
        let next = SigningKey::from_bytes(&bytes);
        let temporary = path.with_extension(format!("rotate-{}", uuid::Uuid::now_v7()));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(bytes.as_ref())?;
            file.sync_all()?;
            drop(file);
            // Preparing and flushing the file must not block ordinary signing on
            // a synchronous lock while the filesystem is slow. Serialize only
            // publication so concurrent rotations cannot diverge on disk/in memory.
            let mut key = self
                .key
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            fs::rename(&temporary, path.as_ref())?;
            *key = next;
            Ok::<_, std::io::Error>(
                base64::engine::general_purpose::STANDARD.encode(key.verifying_key().as_bytes()),
            )
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
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
        let signature = self
            .key
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sign(&payload)
            .to_bytes();

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
    allow_insecure: bool,
    ca_certificate: Option<std::sync::Arc<[u8]>>,
}

pub(crate) struct AgentBuildCommand {
    pub output: Option<tokio::sync::mpsc::Sender<citadel_execution::ProcessChunk>>,
    pub context_archive: Vec<u8>,
    pub dockerfile_path: String,
    pub tags: Vec<String>,
    pub build_args: std::collections::HashMap<String, String>,
    pub target: Option<String>,
    pub registry_auth: Option<String>,
    pub registry_host: Option<String>,
    pub timeout_seconds: i32,
    pub maximum_log_bytes: usize,
    pub secrets: Vec<(String, String)>,
}

impl AgentClient {
    /// Construct a reusable signing/transport context without requiring any
    /// remote platform to be online during Core startup.
    pub fn lazy(
        address: &str,
        signer: AgentRequestSigner,
        operation_timeout: Duration,
        allow_insecure: bool,
    ) -> Result<Self, RuntimeCapabilityError> {
        if operation_timeout.is_zero() {
            return Err(invalid_address("Agent timeout must be positive"));
        }
        let address = validate_address(address, allow_insecure)?;
        let endpoint = agent_endpoint(&address, operation_timeout, None)?;
        citadel_runtime::runtime_metrics::RuntimeWork::AgentChannelCreated.units(1);
        Ok(Self {
            channel: endpoint.connect_lazy(),
            signer,
            operation_timeout,
            address,
            allow_insecure,
            ca_certificate: None,
        })
    }

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
        let endpoint = agent_endpoint(&address, operation_timeout, None)?;
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
            allow_insecure,
            ca_certificate: None,
        })
    }

    /// Reuse the configured signing identity and transport policy for an
    /// explicitly selected pool. Do not cache an unbounded set of endpoints.
    pub async fn for_address(
        &self,
        address: &str,
        cancellation: &CancellationToken,
    ) -> Result<Self, RuntimeCapabilityError> {
        if cancellation.is_cancelled() {
            return Err(cancelled_error());
        }
        let address = validate_address(address, self.allow_insecure)?;
        if self.address == address {
            return Ok(self.clone());
        }
        let mut client = self.at_address(&address)?;
        let endpoint = agent_endpoint(
            &address,
            self.operation_timeout,
            self.ca_certificate.as_deref(),
        )?;
        client.channel = tokio::select! {
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, endpoint.connect()) => {
                result.map_err(|_| timeout_error("connecting to the Agent"))?
                    .map_err(|error| RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, format!("failed to connect to the Agent: {error}"), true))?
            }
        };
        Ok(client)
    }

    /// Apply additional private CA trust and retain it when selecting another Platform.
    pub fn with_ca_certificate(
        mut self,
        ca: Option<std::sync::Arc<[u8]>>,
    ) -> Result<Self, RuntimeCapabilityError> {
        self.channel =
            agent_endpoint(&self.address, self.operation_timeout, ca.as_deref())?.connect_lazy();
        self.ca_certificate = ca;
        Ok(self)
    }

    /// Resolve a persisted Platform address without opening a second eager connection.
    /// RPC deadlines still bound the lazy connection; unchanged addresses reuse their channel.
    pub fn at_address(&self, address: &str) -> Result<Self, RuntimeCapabilityError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::AgentConnect.start();
        let address = validate_address(address, self.allow_insecure)?;
        if self.address == address {
            return Ok(self.clone());
        }
        let endpoint = agent_endpoint(
            &address,
            self.operation_timeout,
            self.ca_certificate.as_deref(),
        )?;
        citadel_runtime::runtime_metrics::RuntimeWork::AgentChannelCreated.units(1);
        Ok(Self {
            channel: endpoint.connect_lazy(),
            signer: self.signer.clone(),
            operation_timeout: self.operation_timeout,
            address,
            allow_insecure: self.allow_insecure,
            ca_certificate: self.ca_certificate.clone(),
        })
    }

    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    pub async fn check_build_host(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<
        citadel_contracts::citadel::images::v1::CheckBuildHostResponse,
        RuntimeCapabilityError,
    > {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                (),
                "/citadel.images.v1.ImageService/CheckBuildHost",
                Some(self.operation_timeout),
            )?;
            self.image_client()
                .check_build_host(request)
                .await
                .map(|value| value.into_inner())
                .map_err(normalize_status)
        })
        .await
    }

    pub async fn handshake(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<RuntimePlatformInfo, RuntimeCapabilityError> {
        Ok(map_platform_info(self.platform_info(cancellation).await?))
    }

    pub(crate) async fn platform_info(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<PlatformInfoResponse, RuntimeCapabilityError> {
        self.retry_unary(cancellation, || async {
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
        .await
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

    pub async fn delete_container(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        self.delete_container_with_options(
            id,
            citadel_platforms::containers::DeleteContainerOptions {
                v: true,
                force: true,
                link: false,
            },
            cancellation,
        )
        .await
    }

    pub async fn delete_container_with_options(
        &self,
        id: &str,
        options: citadel_platforms::containers::DeleteContainerOptions,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        self.delete_containers_with_options(&[id.to_owned()], options, cancellation)
            .await
    }

    pub async fn delete_containers_with_options(
        &self,
        ids: &[String],
        options: citadel_platforms::containers::DeleteContainerOptions,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if ids.is_empty()
            || ids.len() > citadel_platforms::containers::MAX_CONTAINER_BATCH
            || ids.iter().any(|id| id.trim().is_empty() || id.len() > 256)
        {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Container identifier is invalid",
                false,
            ));
        }
        let request = self.signer.sign(
            DeleteContainerRequest {
                ids: ids.to_vec(),
                v: Some(options.v),
                force: Some(options.force),
                link: Some(options.link),
            },
            DELETE_CONTAINER_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.container_client();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.delete(request)) => {
                result.map_err(|_| timeout_error("deleting a Container through the Agent"))?
                    .map_err(normalize_status)?;
            }
        }
        Ok(())
    }

    pub async fn create_container(
        &self,
        request: CreateContainerRequest,
        cancellation: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        if request.name.trim().is_empty() || request.name.len() > 255 {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Container name is invalid",
                false,
            ));
        }
        let request = self.signer.sign(
            request,
            CREATE_CONTAINER_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.container_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.create(request)) => {
                result.map_err(|_| timeout_error("creating a Container through the Agent"))?
                    .map_err(normalize_status)?
            }
        };
        let id = response.into_inner().container_id;
        if id.trim().is_empty() || id.len() > 256 {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                "the Agent returned an invalid Container identifier",
                false,
            ));
        }
        Ok(id)
    }

    pub(crate) async fn exec_binary_stream(
        &self,
        request: ExecBinaryRequest,
        cancellation: &CancellationToken,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<ExecServerMessage, tonic::Status>>,
        RuntimeCapabilityError,
    > {
        use futures_util::StreamExt;
        let signed =
            self.signer
                .sign(request, EXEC_BINARY_METHOD, Some(Duration::from_secs(1800)))?;
        let mut client = self.container_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.exec_binary(signed)) => {
                result.map_err(|_| timeout_error("opening Agent binary execution"))?.map_err(normalize_status)?
            }
        };
        let mut stream = response.into_inner();
        let cancellation = cancellation.clone();
        Ok(Box::pin(async_stream::stream! {
            loop {
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => { yield Err(tonic::Status::cancelled("Binary execution canceled.")); break; }
                    item = stream.next() => match item { Some(item) => yield item, None => break }
                }
            }
        }))
    }

    pub async fn exec_binary(
        &self,
        request: ExecBinaryRequest,
        timeout: Duration,
        maximum_output_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<AgentBinaryExecOutput, RuntimeCapabilityError> {
        self.exec_binary_observed(request, timeout, maximum_output_bytes, cancellation, None)
            .await
    }
    pub(crate) async fn exec_binary_observed(
        &self,
        request: ExecBinaryRequest,
        timeout: Duration,
        maximum_output_bytes: usize,
        cancellation: &CancellationToken,
        output_sender: Option<tokio::sync::mpsc::Sender<citadel_execution::ProcessChunk>>,
    ) -> Result<AgentBinaryExecOutput, RuntimeCapabilityError> {
        if request.container_id.trim().is_empty()
            || request.container_id.len() > 256
            || request.cmd.is_empty()
            || request.cmd.len() > 256
            || maximum_output_bytes == 0
        {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Agent binary execution request is invalid",
                false,
            ));
        }
        let signed = self
            .signer
            .sign(request, EXEC_BINARY_METHOD, Some(timeout))?;
        let mut client = self.container_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.exec_binary(signed)) => {
                result.map_err(|_| timeout_error("opening Agent binary execution"))?
                    .map_err(normalize_status)?
            }
        };
        let consume = async {
            let mut stream = response.into_inner();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = None;
            while let Some(message) = stream.message().await.map_err(normalize_status)? {
                match message.msg {
                    Some(exec_server_message::Msg::Output(output)) => {
                        if stdout
                            .len()
                            .saturating_add(stderr.len())
                            .saturating_add(output.data.len())
                            > maximum_output_bytes
                        {
                            return Err(RuntimeCapabilityError::new(
                                RuntimeErrorKind::Remote,
                                "Agent binary execution exceeded the output limit",
                                false,
                            ));
                        }
                        let target = if output.stream == 1 {
                            &mut stderr
                        } else {
                            &mut stdout
                        };
                        target.extend_from_slice(&output.data);
                        if let Some(sender) = &output_sender {
                            let _ = sender
                                .send(citadel_execution::ProcessChunk {
                                    stream: if output.stream == 1 {
                                        "stderr"
                                    } else {
                                        "stdout"
                                    },
                                    bytes: output.data,
                                })
                                .await;
                        }
                    }
                    Some(exec_server_message::Msg::Exit(exit)) => exit_code = Some(exit.exit_code),
                    Some(exec_server_message::Msg::Error(error)) => {
                        return Err(RuntimeCapabilityError::new(
                            RuntimeErrorKind::Remote,
                            error.message,
                            false,
                        ));
                    }
                    None => {}
                }
            }
            Ok(AgentBinaryExecOutput {
                stdout,
                stderr,
                exit_code: exit_code.ok_or_else(|| {
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Agent binary execution ended without an exit code",
                        false,
                    )
                })?,
            })
        };
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled_error()),
            result = tokio::time::timeout(timeout, consume) => {
                result.map_err(|_| timeout_error("running Agent binary execution"))?
            }
        }
    }

    pub async fn change_containers_state(
        &self,
        ids: &[String],
        action: AgentContainerAction,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if ids.is_empty()
            || ids.len() > 1_000
            || ids.iter().any(|id| id.trim().is_empty() || id.len() > 256)
        {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "the Container identifiers are invalid",
                false,
            ));
        }
        let method = match action {
            AgentContainerAction::Start => START_CONTAINERS_METHOD,
            AgentContainerAction::Stop => STOP_CONTAINERS_METHOD,
            AgentContainerAction::Pause => PAUSE_CONTAINERS_METHOD,
            AgentContainerAction::Unpause => UNPAUSE_CONTAINERS_METHOD,
            AgentContainerAction::Restart => RESTART_CONTAINERS_METHOD,
        };
        // Shutdown may legitimately consume the full Docker grace period per
        // container. Keep that separate from the configured transport budget,
        // including when an Agent processes the batch sequentially.
        let grace_seconds = match action {
            AgentContainerAction::Stop => 10,
            AgentContainerAction::Restart => 5,
            _ => 0,
        };
        let timeout = self
            .operation_timeout
            .saturating_add(Duration::from_secs(grace_seconds * ids.len() as u64));
        let request =
            self.signer
                .sign(ContainerIds { ids: ids.to_vec() }, method, Some(timeout))?;
        let mut client = self.container_client();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(timeout, async {
                match action {
                    AgentContainerAction::Start => client.start(request).await,
                    AgentContainerAction::Stop => client.stop(request).await,
                    AgentContainerAction::Pause => client.pause(request).await,
                    AgentContainerAction::Unpause => client.unpause(request).await,
                    AgentContainerAction::Restart => client.restart(request).await,
                }
            }) => {
                result.map_err(|_| timeout_error("changing Container state through the Agent"))?
                    .map_err(normalize_status)?;
            }
        }
        Ok(())
    }

    fn image_client(&self) -> ImageServiceClient<Channel> {
        ImageServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    pub(crate) async fn build_image(
        &self,
        command: AgentBuildCommand,
        cancellation: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        let maximum_log_bytes = command.maximum_log_bytes.max(1024);
        let request = self.signer.sign(
            BuildImageRequest {
                context_directory: ".".to_owned(),
                dockerfile_path: command.dockerfile_path.clone(),
                tags: command.tags.clone(),
                build_args: command.build_args,
                target: command.target,
                registry_auth: command.registry_auth.clone(),
                registry_host: command.registry_host,
                timeout_seconds: command.timeout_seconds,
                max_line_bytes: 16 * 1024,
                context_archive: command.context_archive,
                dockerfile_archive_path: Some(command.dockerfile_path),
                build_secrets: command
                    .secrets
                    .into_iter()
                    .map(|(id, value)| BuildImageSecret { id, value })
                    .collect(),
            },
            BUILD_IMAGE_METHOD,
            None,
        )?;
        let mut client = self.image_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.build(request)) => {
                result.map_err(|_| timeout_error("opening the Agent image Build stream"))?
                    .map_err(normalize_status)?
            }
        };
        let mut output = consume_image_build_stream(
            response.into_inner(),
            maximum_log_bytes,
            command.output.as_ref(),
            cancellation,
        )
        .await?;
        for reference in command.tags {
            let request = self.signer.sign(
                PushImageRequest {
                    image_reference: reference,
                    registry_auth: command.registry_auth.clone(),
                },
                PUSH_IMAGE_METHOD,
                None,
            )?;
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.push(request)) => {
                    result.map_err(|_| timeout_error("opening the Agent image Push stream"))?
                        .map_err(normalize_status)?
                }
            };
            let pushed = consume_image_build_stream(
                response.into_inner(),
                maximum_log_bytes.saturating_sub(output.len()),
                command.output.as_ref(),
                cancellation,
            )
            .await?;
            if !pushed.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                append_bounded(&mut output, &pushed, maximum_log_bytes);
            }
        }
        Ok(output)
    }

    fn deployment_client(&self) -> DeploymentServiceClient<Channel> {
        DeploymentServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    fn stack_client(&self) -> StackServiceClient<Channel> {
        StackServiceClient::new(self.channel.clone())
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES)
    }

    /// Stack Apply is a mutation and is deliberately dispatched at most once.
    pub async fn apply_stack(
        &self,
        claim: &citadel_stacks::StackOperationClaim,
        source: &citadel_stacks::StackApplySource,
        environment: &[String],
        registry: Option<&AgentStackRegistry>,
        cancellation: &CancellationToken,
        progress: Option<&citadel_stacks::StackProgress>,
    ) -> Result<citadel_stacks::StackRuntimeResult, RuntimeCapabilityError> {
        let request = self.signer.sign(
            workloads::stack_request(claim, source, environment, registry),
            APPLY_STACK_METHOD,
            None,
        )?;
        let mut client = self.stack_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.apply(request)) => {
                result.map_err(|_| timeout_error("opening the Agent Stack Apply stream"))?
                    .map_err(normalize_status)?
            }
        };
        workloads::consume_stack_stream(response.into_inner(), cancellation, progress).await
    }

    /// Pull is deliberately not retried: once an Agent accepts this mutation,
    /// retrying after an ambiguous transport failure could duplicate work.
    pub async fn pull_deployment_image(
        &self,
        image: &str,
        registry_auth: Option<String>,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let request = self.signer.sign(
            PullImageRequest {
                from_image: image.to_owned(),
                from_src: None,
                repo: None,
                tag: None,
                auth: registry_auth,
                changes: Vec::new(),
            },
            PULL_IMAGE_METHOD,
            None,
        )?;
        let mut client = self.image_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(
                self.operation_timeout,
                client.pull(request),
            ) => result
                .map_err(|_| timeout_error("opening the Agent image-pull stream"))?
                .map_err(normalize_status)?,
        };
        let mut stream = response.into_inner();
        loop {
            let item = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                item = stream.next() => item,
            };
            let Some(item) = item else { break };
            let item = item.map_err(normalize_status)?;
            let message = item
                .error_message
                .or_else(|| item.error.and_then(|error| error.message));
            if let Some(message) = message.filter(|message| !message.trim().is_empty()) {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    message,
                    false,
                ));
            }
        }
        Ok(())
    }

    pub async fn open_image_pull(
        &self,
        input: PullImageRequest,
        cancel: &CancellationToken,
    ) -> Result<
        tonic::Streaming<citadel_contracts::citadel::images::v1::PullImageResponse>,
        RuntimeCapabilityError,
    > {
        let request = self.signer.sign(input, PULL_IMAGE_METHOD, None)?;
        let mut client = self.image_client();
        tokio::select! {
            biased;
            ()=cancel.cancelled()=>Err(cancelled_error()),
            result=tokio::time::timeout(self.operation_timeout,client.pull(request))=>result.map_err(|_|timeout_error("opening image pull"))?.map(|v|v.into_inner()).map_err(normalize_status),
        }
    }

    pub async fn prune_platform(
        &self,
        input: citadel_contracts::citadel::platforms::v1::PruneRequest,
        cancel: &CancellationToken,
    ) -> Result<citadel_contracts::citadel::platforms::v1::PruneResponse, RuntimeCapabilityError>
    {
        let request = self.signer.sign(
            input,
            "/citadel.platforms.v1.PlatformService/Prune",
            Some(self.operation_timeout),
        )?;
        let mut client = self.platform_client();
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout,client.prune(request)) => result.map_err(|_| timeout_error("pruning the Agent platform"))?.map(|r| r.into_inner()).map_err(normalize_status),
        }
    }

    /// Apply is deliberately not retried for the same mutation-safety reason as Pull.
    pub async fn apply_deployment(
        &self,
        command: &RuntimeDeploymentCommand,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeDeploymentResult, RuntimeCapabilityError> {
        let request = self.signer.sign(
            workloads::deployment_request(command),
            APPLY_DEPLOYMENT_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.deployment_client();
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = tokio::time::timeout(
                self.operation_timeout,
                client.apply(request),
            ) => result
                .map_err(|_| timeout_error("applying a Deployment through the Agent"))?
                .map_err(normalize_status)?,
        }
        .into_inner();
        Ok(workloads::deployment_result(command, response))
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

    pub async fn inspect_managed_swarm_service(
        &self,
        service_id: &str,
        cancellation: &CancellationToken,
    ) -> Result<SwarmServiceMessage, RuntimeCapabilityError> {
        let service_id = service_id.to_owned();
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                InspectSwarmServiceRequest {
                    service_id: service_id.clone(),
                },
                INSPECT_SWARM_SERVICE_METHOD,
                Some(self.operation_timeout),
            )?;
            self.swarm_client()
                .inspect_service(request)
                .await
                .map(|value| value.into_inner())
                .map_err(normalize_status)
        })
        .await
    }

    /// Mutations are deliberately dispatched once. A lost response is ambiguous and
    /// the durable Citadel operation is reconciled by its operation label.
    pub async fn create_managed_swarm_service(
        &self,
        request: CreateManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<SwarmServiceMutationResponse, RuntimeCapabilityError> {
        let request = self.signer.sign(
            request,
            CREATE_SWARM_SERVICE_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.swarm_client();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.create_service(request)) => {
                result.map_err(|_| timeout_error("creating a Swarm Service through the Agent"))?
                    .map(|value| value.into_inner())
                    .map_err(normalize_status)
            }
        }
    }

    pub async fn update_managed_swarm_service(
        &self,
        request: UpdateManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<SwarmServiceMutationResponse, RuntimeCapabilityError> {
        let request = self.signer.sign(
            request,
            UPDATE_SWARM_SERVICE_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.swarm_client();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.update_service(request)) => {
                result.map_err(|_| timeout_error("updating a Swarm Service through the Agent"))?
                    .map(|value| value.into_inner())
                    .map_err(normalize_status)
            }
        }
    }

    pub async fn delete_managed_swarm_service(
        &self,
        request: DeleteManagedSwarmServiceRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let request = self.signer.sign(
            request,
            DELETE_SWARM_SERVICE_METHOD,
            Some(self.operation_timeout),
        )?;
        let mut client = self.swarm_client();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled_error()),
            result = tokio::time::timeout(self.operation_timeout, client.delete_service(request)) => {
                match result.map_err(|_| timeout_error("deleting a Swarm Service through the Agent"))? {
                    Ok(_) => Ok(()),
                    Err(status) if status.code() == Code::NotFound => Ok(()),
                    Err(status) => Err(normalize_status(status)),
                }
            }
        }
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
                        yield Ok(map_container_stats(value));
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
                        if let Some(event) = map_scoped_daemon_event(value.kind, value.scope) {
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

pub fn decode_daemon_event(bytes: &[u8]) -> Result<Option<AgentDaemonEvent>, prost::DecodeError> {
    let response = citadel_contracts::citadel::platforms::v1::DaemonEventResponse::decode(bytes)?;
    Ok(map_scoped_daemon_event(response.kind, response.scope))
}

pub fn map_daemon_event(kind: Option<daemon_event_response::Kind>) -> Option<AgentDaemonEvent> {
    map_scoped_daemon_event(kind, 0)
}

fn map_scoped_daemon_event(
    kind: Option<daemon_event_response::Kind>,
    scope: i32,
) -> Option<AgentDaemonEvent> {
    let mut event = match kind? {
        daemon_event_response::Kind::DaemonContainerEventResponse(value) => AgentDaemonEvent {
            resource: None,
            swarm_scope: scope == 2,
            kind: RuntimeEventKind::Unknown,
            resource_type: "container",
            container_id: Some(value.container_id),
            container_state: value
                .container
                .as_ref()
                .map(|c| c.state().as_str_name().to_ascii_lowercase()),
            container_name: value.container.as_ref().map(|c| c.name.clone()),
            container: value.container.map(map_container),
            action: value.action,
        },
        daemon_event_response::Kind::DaemonImageEventResponse(value) => AgentDaemonEvent {
            resource: Some(citadel_platforms::jobs::ResourceDelta::Image {
                id: value.image_id,
                value: value.image.map(map_image),
            }),
            swarm_scope: scope == 2,
            kind: RuntimeEventKind::Unknown,
            resource_type: "image",
            action: value.action,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
        },
        daemon_event_response::Kind::DaemonVolumeEventResponse(value) => AgentDaemonEvent {
            resource: Some(citadel_platforms::jobs::ResourceDelta::Volume {
                id: value.volume_id,
                value: value.volume.map(map_volume),
            }),
            swarm_scope: scope == 2,
            kind: RuntimeEventKind::Unknown,
            resource_type: "volume",
            action: value.action,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
        },
        daemon_event_response::Kind::DaemonNetworkEventResponse(value) => AgentDaemonEvent {
            resource: Some(citadel_platforms::jobs::ResourceDelta::Network {
                id: value.network_id,
                value: value.network.map(map_network),
            }),
            swarm_scope: scope == 2,
            kind: RuntimeEventKind::Unknown,
            resource_type: "network",
            action: value.action,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
        },
        daemon_event_response::Kind::DaemonResourceEventResponse(value) => AgentDaemonEvent {
            resource: None,
            swarm_scope: scope == 2,
            kind: RuntimeEventKind::Unknown,
            resource_type: daemon_resource_type(value.r#type),
            action: value.action,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
        },
    };
    event.kind = events::classify(
        event.resource_type,
        &event.action,
        if scope == 2 { "swarm" } else { "local" },
    )?;
    if let RuntimeEventKind::Container(change) = event.kind {
        if change == ContainerChange::Tombstone {
            event.container = None;
            event.container_state = None;
            event.container_name = None;
        } else if let Some(state) = change.state() {
            event.container_state = Some(state.to_owned());
            if let Some(container) = &mut event.container {
                container.state = state.to_owned();
            }
        }
    }
    if event
        .resource
        .as_ref()
        .is_some_and(|delta| !delta.valid_for(event.kind))
    {
        event.resource = None;
    }
    Some(event)
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

pub(crate) fn network_request(input: &CreateRuntimeNetwork) -> CreateNetworkRequest {
    CreateNetworkRequest {
        name: input.name.clone(),
        driver: Some(input.driver.clone()),
        scope: Some(input.scope.clone()),
        internal: input.internal,
        attachable: input.attachable,
        ingress: input.ingress,
        config_only: input.config_only,
        config_from: input.config_from.as_ref().map(|value| ConfigFromMessage {
            network: value.network.clone(),
        }),
        ipam: input.ipam.as_ref().map(|value| IpamMessage {
            driver: Some(value.driver.clone()),
            config: value
                .config
                .iter()
                .map(|item| IpamConfigMessage {
                    subnet: item.subnet.clone(),
                    ip_range: item.ip_range.clone(),
                    gateway: item.gateway.clone(),
                })
                .collect(),
            options: value.options.clone().into_iter().collect(),
        }),
        enable_i_pv6: input.enable_ipv6,
        enable_i_pv4: input.enable_ipv4,
        options: input.options.clone().into_iter().collect(),
        labels: input.labels.clone().into_iter().collect(),
    }
}

impl citadel_platforms::NetworkMutationPort for AgentClient {
    fn create_network<'a>(
        &'a self,
        input: &'a CreateRuntimeNetwork,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>> {
        async move {
            let request = self.signer.sign(
                network_request(input),
                CREATE_NETWORK_METHOD,
                Some(self.operation_timeout),
            )?;
            let mut client = self.network_client();
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.create(request)) => {
                    result.map_err(|_| timeout_error("creating a Network through the Agent"))?
                        .map_err(normalize_status)?
                }
            };
            Ok(CreatedRuntimeNetwork {
                id: response.into_inner().id,
            })
        }
        .boxed()
    }

    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move {
            let request = self.signer.sign(
                DeleteNetworkRequest {
                    ids: vec![id.to_owned()],
                },
                DELETE_NETWORK_METHOD,
                Some(self.operation_timeout),
            )?;
            let mut client = self.network_client();
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.delete(request)) => {
                    result.map_err(|_| timeout_error("deleting a Network through the Agent"))?
                        .map_err(normalize_status)?;
                }
            }
            Ok(())
        }
        .boxed()
    }
}

impl citadel_platforms::VolumeMutationPort for AgentClient {
    fn create_volume<'a>(
        &'a self,
        input: &'a CreateRuntimeVolume,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        async move {
            let request = self.signer.sign(
                CreateVolumeRequest {
                    name: input.name.clone(),
                    driver: input.driver.clone(),
                    labels: input.labels.clone().into_iter().collect(),
                    options: input.options.clone().into_iter().collect(),
                },
                CREATE_VOLUME_METHOD,
                Some(self.operation_timeout),
            )?;
            let mut client = self.volume_client();
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.create(request)) => {
                    result.map_err(|_| timeout_error("creating a Volume through the Agent"))?
                        .map_err(normalize_status)?
                }
            };
            Ok(map_volume(response.into_inner()))
        }
        .boxed()
    }

    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move {
            let request = self.signer.sign(
                RemoveVolumeRequest {
                    names: vec![name.to_owned()],
                    force,
                },
                DELETE_VOLUME_METHOD,
                Some(self.operation_timeout),
            )?;
            let mut client = self.volume_client();
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.remove(request)) => {
                    result.map_err(|_| timeout_error("deleting a Volume through the Agent"))?
                        .map_err(normalize_status)?;
                }
            }
            Ok(())
        }
        .boxed()
    }
}

impl citadel_platforms::PlatformHealthPort for AgentClient {
    fn probe<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let request = self.signer.sign(
                (),
                "/citadel.platforms.v1.PlatformService/CheckHealth",
                Some(self.operation_timeout),
            )?;
            let mut client = self.platform_client();
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.check_health(request)) => {
                    result.map_err(|_| timeout_error("checking Agent health"))?.map_err(normalize_status)?
                }
            };
            if response.into_inner().healthy {
                Ok(())
            } else {
                Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    "Agent health check failed",
                    true,
                ))
            }
        })
    }
}

impl citadel_platforms::PlatformInfoPort for AgentClient {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        self.handshake(cancellation).boxed()
    }
}

impl citadel_platforms::ContainerInventoryPort for AgentClient {
    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        async move {
            let response = self
                .retry_unary(cancellation, || async {
                    let request = self.signer.sign(
                        ListContainersRequest {
                            metadata_only: true,
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
}

impl citadel_platforms::PlatformStatsPort for AgentClient {
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

impl citadel_platforms::ImageInventoryPort for AgentClient {
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
}

impl citadel_platforms::NetworkInventoryPort for AgentClient {
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
}

impl citadel_platforms::NetworkObservationPort for AgentClient {
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
}

impl citadel_platforms::VolumeInventoryPort for AgentClient {
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
}

impl citadel_platforms::VolumeObservationPort for AgentClient {
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
}

impl citadel_platforms::SwarmInventoryPort for AgentClient {
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
                        ListSwarmTasksRequest {
                            max_items: i32::MAX,
                        },
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

pub(crate) fn map_image(
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

pub(crate) fn map_network(
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

pub(crate) fn map_network_inspect(network: InspectNetworkResponse) -> RuntimeNetworkSummary {
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

pub(crate) fn map_volume(
    volume: citadel_contracts::citadel::shared_models::v1::VolumeResponse,
) -> RuntimeVolumeSummary {
    RuntimeVolumeSummary {
        name: volume.name,
        in_use: volume.in_use,
        scope: volume.scope,
        driver: volume.driver,
        mountpoint: volume.mountpoint,
        created_at: volume.created_at,
        cluster_volume: volume.cluster_volume.map(volumes::cluster),
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
                let state = container.state().as_str_name();
                serde_json::json!({
                    "id": container.id,
                    "name": container.name,
                    "image": container.image,
                    "imageId": container.image_id,
                    "state": state,
                    "networks": container.networks,
                    "ports": container.ports.into_iter().map(|(port, bindings)| (port,
                        serde_json::Value::Array(bindings.host_port_binding.into_iter().map(|binding|
                            serde_json::json!({"hostIP": binding.host_ip, "hostPort": binding.host_port})
                        ).collect())
                    )).collect::<serde_json::Map<String, serde_json::Value>>(),
                })
            })
            .collect(),
    }
}

pub(crate) fn map_swarm_node(
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

pub(crate) fn map_swarm_service(
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
        secret_mounts: value
            .definition
            .as_ref()
            .into_iter()
            .flat_map(|d| &d.secrets)
            .map(|r| citadel_platforms::RuntimeSwarmResourceMount {
                resource_id: r.id.clone(),
                target_name: r.target_name.clone(),
            })
            .collect(),
        config_mounts: value
            .definition
            .as_ref()
            .into_iter()
            .flat_map(|d| &d.configs)
            .map(|r| citadel_platforms::RuntimeSwarmResourceMount {
                resource_id: r.id.clone(),
                target_name: r.target_name.clone(),
            })
            .collect(),
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
        legacy_runtime_hash: None,
        created_at: protobuf_timestamp(value.created_at),
        updated_at: protobuf_timestamp(value.updated_at),
    }
    .normalize_ownership()
}

pub(crate) fn map_swarm_task(
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

pub(crate) fn map_swarm_config(
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

pub(crate) fn map_swarm_secret(
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

pub(crate) fn map_platform_info(value: PlatformInfoResponse) -> RuntimePlatformInfo {
    let stats = value.platform_stat.unwrap_or_default();
    let swarm = value.swarm_info.map(|swarm| RuntimeSwarmInfo {
        node_id: swarm.node_id,
        node_addr: swarm.node_addr,
        local_node_state: swarm.local_node_state,
        control_available: swarm.control_available,
        error: nonempty(swarm.error),
        remote_managers: swarm
            .remote_managers
            .into_iter()
            .map(|manager| RuntimeSwarmPeer {
                node_id: manager.node_id,
                address: manager.addr,
            })
            .collect(),
        nodes: swarm.nodes,
        managers: swarm.managers,
        cluster_id: nonempty(swarm.cluster_id),
        cluster_created_at: protobuf_timestamp(swarm.cluster_created_at),
    });
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
        swarm,
    }
}

pub(crate) fn map_container(value: ContainerMessage) -> RuntimeContainerSummary {
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
                            "hostIP": binding.host_ip,
                            "hostPort": binding.host_port,
                        })
                    })
                    .collect();
                (port, serde_json::Value::Array(bindings))
            })
            .collect(),
    );
    let mut labels: std::collections::BTreeMap<_, _> = value.labels.into_iter().collect();
    if let Ok(id) = uuid::Uuid::parse_str(&value.stack_id)
        && !id.is_nil()
    {
        labels.insert("com.citadel.managed".into(), "true".into());
        labels.insert("com.citadel.stack-id".into(), id.to_string());
    }
    let image_id = if value.image_id.is_empty() {
        value.image.clone()
    } else {
        value.image_id.clone()
    };
    RuntimeContainerSummary {
        id: value.id,
        name: value.name.trim_start_matches('/').to_owned(),
        image: value.image,
        image_id,
        created: value.created,
        state,
        status: value.status,
        labels,
        ports,
        stack: value.stack,
        is_system: value.is_system,
        system_role: value.system_role,
        has_citadel_ownership_labels: value.has_citadel_ownership_labels,
        is_swarm_task: value.is_swarm_task,
    }
}

pub(crate) fn map_platform_stats(
    value: citadel_contracts::citadel::platforms::v1::PlatformStatsResponse,
) -> RuntimePlatformStats {
    // Existing Agents put container counts in the embedded sample. Use the
    // envelope counts when the optional sample is absent (descriptor-only frame).
    let has_stat = value.stat.is_some();
    let stat = value.stat.unwrap_or_default();
    RuntimePlatformStats {
        image_used_bytes: value.image_used_bytes,
        volume_used_bytes: value.volume_used_bytes,
        image_count: value.image_count,
        volume_count: value.volume_count,
        network_count: value.network_count,
        mem_total: value.mem_total,
        disk_used_bytes: stat.disk_used_bytes,
        disk_total_bytes: stat.disk_total_bytes,
        disk_usage: stat.disk_usage,
        memory_usage: stat.memory_usage,
        cpu_usage: stat.cpu_usage,
        receive_bytes: stat.rx_bytes,
        transmit_bytes: stat.tx_bytes,
        container_count: if has_stat {
            i64::from(stat.container_count)
        } else {
            value.container_count
        },
        containers_running: if has_stat {
            i64::from(stat.containers_running)
        } else {
            value.containers_running
        },
        containers_paused: if has_stat {
            i64::from(stat.containers_paused)
        } else {
            value.containers_paused
        },
        containers_stopped: if has_stat {
            i64::from(stat.containers_stopped)
        } else {
            value.containers_stopped
        },
    }
}

fn agent_endpoint(
    address: &str,
    timeout: Duration,
    ca: Option<&[u8]>,
) -> Result<Endpoint, RuntimeCapabilityError> {
    let mut endpoint = Endpoint::from_shared(address.to_owned())
        .map_err(|error| invalid_address(error.to_string()))?
        .connect_timeout(timeout);
    if address.starts_with("https://") {
        // Keep the same provider as Core's TLS listener when dependencies enable both.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut tls = ClientTlsConfig::new().with_native_roots();
        if let Some(ca) = ca {
            tls = tls.ca_certificate(Certificate::from_pem(ca));
        }
        endpoint = endpoint
            .tls_config(tls)
            .map_err(|error| invalid_address(error.to_string()))?;
    }
    Ok(endpoint)
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
        "https" => Ok(url.to_string().trim_end_matches('/').to_owned()),
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

pub(crate) fn normalize_status(status: Status) -> RuntimeCapabilityError {
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

pub(crate) async fn consume_image_build_stream<S>(
    mut stream: S,
    maximum_bytes: usize,
    progress: Option<&tokio::sync::mpsc::Sender<citadel_execution::ProcessChunk>>,
    cancellation: &CancellationToken,
) -> Result<String, RuntimeCapabilityError>
where
    S: futures_util::Stream<Item = Result<ImageBuildResponse, Status>> + Unpin,
{
    let mut output = String::new();
    loop {
        let next = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            item = stream.next() => item,
        };
        let Some(item) = next else { break };
        let item = item.map_err(normalize_status)?;
        let error = item
            .error_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                item.error
                    .as_ref()
                    .and_then(|error| error.message.as_deref())
                    .filter(|value| !value.trim().is_empty())
            });
        if let Some(error) = error {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                error.to_owned(),
                false,
            ));
        }
        for value in [item.stream, item.status, item.progress_message]
            .into_iter()
            .flatten()
            .filter(|value| !value.trim().is_empty())
        {
            if let Some(progress) = progress {
                let retained = value.len().min(maximum_bytes.saturating_sub(output.len()));
                for bytes in value.as_bytes()[..retained].chunks(8192) {
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => return Err(cancelled_error()),
                        sent = progress.send(citadel_execution::ProcessChunk { stream: "stdout", bytes: bytes.to_vec() }) => {
                            sent.map_err(|_| RuntimeCapabilityError::new(RuntimeErrorKind::Remote, "Build output consumer closed.", false))?;
                        }
                    }
                }
            }
            if !output.is_empty() && !output.ends_with('\n') {
                append_bounded(&mut output, "\n", maximum_bytes);
            }
            append_bounded(&mut output, &value, maximum_bytes);
        }
    }
    Ok(output)
}

fn append_bounded(output: &mut String, value: &str, maximum_bytes: usize) {
    let available = maximum_bytes.saturating_sub(output.len());
    if available == 0 {
        return;
    }
    let mut end = value.len().min(available);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&value[..end]);
}

#[cfg(test)]
mod tests {
    #[test]
    fn statistics_keep_capture_time_and_accept_legacy_responses() {
        use citadel_contracts::citadel::{
            containers::v1::ContainersStatsResponse, shared_models::v1::ContainerStatMessage,
        };
        let mut wire = ContainersStatsResponse {
            containers: std::collections::HashMap::from([(
                "container".into(),
                ContainerStatMessage::default(),
            )]),
            captured_at: Some(123),
        };
        assert_eq!(super::map_container_stats(wire.clone())[0].created, 123);
        wire.captured_at = None;
        assert!(super::map_container_stats(wire)[0].created > 123);
    }

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
    fn instance_signing_key_is_private_persistent_and_initialized_atomically() {
        let root = std::env::temp_dir().join(format!("citadel-agent-key-{}", Uuid::now_v7()));
        let path = root.join("signing-key");
        let workers = (0..4)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    AgentRequestSigner::load_or_create(&path)
                        .unwrap()
                        .public_key_base64()
                })
            })
            .collect::<Vec<_>>();
        let keys = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert!(keys.iter().all(|key| key == &keys[0]));
        assert_eq!(
            AgentRequestSigner::from_file(&path)
                .unwrap()
                .public_key_base64(),
            keys[0]
        );
        assert_eq!(fs::read(&path).unwrap().len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn malformed_existing_signing_key_is_not_silently_replaced() {
        let root = std::env::temp_dir().join(format!("citadel-agent-key-{}", Uuid::now_v7()));
        fs::create_dir(&root).unwrap();
        let path = root.join("signing-key");
        fs::write(&path, b"invalid").unwrap();
        assert!(AgentRequestSigner::load_or_create(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&root).unwrap();
    }

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
            .read()
            .unwrap()
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
                    container_id: "docker-1".into(),
                    container: Some(ContainerMessage {
                        state: ContainerStateType::Exited as i32,
                        name: "nginx".into(),
                        ..Default::default()
                    }),
                },
            ),
        ))
        .unwrap();
        assert_eq!(container.resource_type, "container");
        assert_eq!(container.action, "start");
        assert_eq!(container.container_id.as_deref(), Some("docker-1"));
        assert_eq!(container.container_state.as_deref(), Some("running"));
        assert_eq!(container.container_name.as_deref(), Some("nginx"));

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
        assert_eq!(mapped.ports["80/tcp"][0]["hostIP"], "0.0.0.0");
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
            citadel_swarm_services::SwarmServiceOwnership::CitadelService
        );
    }
}

#[cfg(test)]
mod disk_mapping_tests {
    use super::*;
    #[test]
    fn platform_stats_preserve_optional_disk_measurements() {
        use citadel_contracts::citadel::{
            platforms::v1::PlatformStatsResponse, shared_models::v1::PlatformStatMessage,
        };
        let response = PlatformStatsResponse {
            stat: Some(PlatformStatMessage {
                disk_used_bytes: Some(0),
                disk_total_bytes: Some(100),
                disk_usage: Some(0.0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mapped = map_platform_stats(response);
        assert_eq!(mapped.disk().unwrap().used_bytes, 0);
        assert_eq!(
            map_platform_stats(PlatformStatsResponse::default()).disk(),
            None
        );
    }
}

#[cfg(test)]
mod platform_stats_mapping_tests {
    #[test]
    fn platform_stats_keep_storage_totals_and_outer_descriptor_counts() {
        let response = citadel_contracts::citadel::platforms::v1::PlatformStatsResponse {
            image_used_bytes: Some(2048),
            volume_used_bytes: Some(4096),
            image_count: 7,
            volume_count: 3,
            network_count: 2,
            mem_total: 8192,
            container_count: 6,
            containers_running: 3,
            containers_paused: 1,
            containers_stopped: 2,
            ..Default::default()
        };
        let sample = super::map_platform_stats(response);
        assert_eq!(
            (sample.image_used_bytes, sample.volume_used_bytes),
            (Some(2048), Some(4096))
        );
        assert_eq!(
            (
                sample.image_count,
                sample.volume_count,
                sample.network_count,
                sample.mem_total
            ),
            (7, 3, 2, 8192)
        );
        assert_eq!(
            (
                sample.container_count,
                sample.containers_running,
                sample.containers_paused,
                sample.containers_stopped
            ),
            (6, 3, 1, 2)
        );
        assert_eq!(sample.disk(), None);
    }
}
