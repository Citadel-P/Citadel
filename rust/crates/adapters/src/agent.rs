use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_stream::stream;
use base64::Engine;
use citadel_contracts::citadel::containers::v1::{
    ListContainersRequest, container_service_client::ContainerServiceClient,
};
use citadel_contracts::citadel::platforms::v1::{
    PlatformStatsRequest, platform_service_client::PlatformServiceClient,
};
use citadel_contracts::citadel::shared_models::v1::{ContainerMessage, PlatformInfoResponse};
use citadel_platforms::{
    PlatformRuntimePort, RuntimeCapabilityError, RuntimeErrorKind, RuntimeStatsStream,
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
const STREAM_PLATFORM_STATS_METHOD: &str =
    "/citadel.platforms.v1.PlatformService/StreamPlatformStats";

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
        let endpoint = Endpoint::from_shared(address)
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
        })
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

fn map_platform_info(value: PlatformInfoResponse) -> RuntimePlatformInfo {
    let stats = value.platform_stat.unwrap_or_default();
    RuntimePlatformInfo {
        daemon_id: value.id,
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
    RuntimeContainerSummary {
        id: value.id,
        name: value.name.trim_start_matches('/').to_owned(),
        image: value.image,
        state,
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
    use ed25519_dalek::{Signature, Verifier};

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
}
