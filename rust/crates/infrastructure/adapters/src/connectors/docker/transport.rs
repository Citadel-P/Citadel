use super::DockerEndpoint;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::time::Duration;

use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tokio::sync::{Mutex, RwLock};

mod binary_exec;
pub use binary_exec::{DockerExecError, DockerExecEvent, DockerExecStream};

use super::projection::{ContainerStats, DockerEvent, DockerVersion};

const MINIMUM_SUPPORTED_VERSION: ApiVersion = ApiVersion::new(1, 41);
const MAXIMUM_SUPPORTED_VERSION: ApiVersion = ApiVersion::new(1, 49);
const MAX_JSON_BODY_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAM_ITEM_BYTES: usize = 1024 * 1024;

pub type DockerJsonStream<T> = Pin<Box<dyn Stream<Item = Result<T, DockerError>> + Send + 'static>>;

#[derive(Default)]
pub struct DockerImagePullOptions<'a> {
    pub from_image: Option<&'a str>,
    pub from_source: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub changes: &'a [String],
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerImagePullMessage {
    #[serde(default)]
    pub stream: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub progress: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(rename = "errorDetail", default)]
    pub error_detail: Option<DockerImagePullError>,
    #[serde(rename = "progressDetail", default)]
    pub progress_detail: Option<DockerImagePullProgress>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DockerImagePullProgress {
    pub units: Option<String>,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub start: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DockerImagePullError {
    pub code: Option<i64>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    major: u16,
    minor: u16,
}

impl ApiVersion {
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for ApiVersion {
    type Err = DockerError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (major, minor) = value
            .split_once('.')
            .ok_or_else(|| DockerError::InvalidVersion(value.to_owned()))?;
        Ok(Self {
            major: major
                .parse()
                .map_err(|_| DockerError::InvalidVersion(value.to_owned()))?,
            minor: minor
                .parse()
                .map_err(|_| DockerError::InvalidVersion(value.to_owned()))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Docker Unix-socket transport is only available on Unix targets")]
    UnsupportedPlatform,
    #[error("Invalid Docker endpoint: {0}")]
    InvalidEndpoint(&'static str),
    #[error("Docker transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Docker protocol IO failed: {0}")]
    ProtocolIo(std::io::Error),
    #[error("Docker returned HTTP {status}: {message}")]
    Api { status: StatusCode, message: String },
    #[error("Docker response exceeded the {limit}-byte bound")]
    ResponseTooLarge { limit: usize },
    #[error("Docker stream item exceeded the {limit}-byte bound")]
    StreamItemTooLarge { limit: usize },
    #[error("Docker returned invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid Docker API version '{0}'")]
    InvalidVersion(String),
    #[error("Docker resource identifier must not be empty")]
    InvalidIdentifier,
    #[error(
        "Docker Engine API range {daemon_minimum}-{daemon_maximum} is incompatible with Citadel's supported range {supported_minimum}-{supported_maximum}"
    )]
    IncompatibleVersion {
        daemon_minimum: String,
        daemon_maximum: String,
        supported_minimum: ApiVersion,
        supported_maximum: ApiVersion,
    },
    #[error("Docker ping returned '{0}' instead of OK")]
    InvalidPing(String),
}

type NegotiatedConfiguration = (
    ApiVersion,
    std::sync::Arc<citadel_docker_api::apis::configuration::Configuration>,
);

#[derive(Clone)]
pub struct DockerClient {
    pub(crate) host_disk:
        std::sync::Arc<crate::filesystem::host::disk_usage::HostDiskUsageProvider>,
    pub(super) client: Client,
    endpoint: DockerEndpoint,
    pub(super) base_url: String,
    pub(super) request_timeout: Duration,
    pub(super) version: std::sync::Arc<RwLock<Option<ApiVersion>>>,
    negotiation: std::sync::Arc<Mutex<()>>,
    version_document: std::sync::Arc<Mutex<Option<DockerVersion>>>,
    generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub(super) daemon_id: std::sync::Arc<Mutex<Option<String>>>,
    pub(super) configuration: std::sync::Arc<RwLock<Option<NegotiatedConfiguration>>>,
    pub(super) storage_usage: std::sync::Arc<Mutex<super::storage_usage::UsageCache>>,
}

impl DockerClient {
    pub fn new(
        socket_path: impl Into<PathBuf>,
        request_timeout: Duration,
    ) -> Result<Self, DockerError> {
        Self::with_host_root(socket_path, request_timeout, "/host")
    }

    pub fn with_host_root(
        socket_path: impl Into<PathBuf>,
        request_timeout: Duration,
        host_root: impl Into<PathBuf>,
    ) -> Result<Self, DockerError> {
        Self::with_endpoint(
            DockerEndpoint::Unix(socket_path.into()),
            request_timeout,
            host_root,
        )
    }

    pub fn with_endpoint(
        endpoint: DockerEndpoint,
        request_timeout: Duration,
        host_root: impl Into<PathBuf>,
    ) -> Result<Self, DockerError> {
        endpoint.validate()?;
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(request_timeout);
        if let DockerEndpoint::Unix(path) = &endpoint {
            #[cfg(unix)]
            {
                builder = builder.unix_socket(path.clone());
            }
            #[cfg(not(unix))]
            {
                let _ = path;
                return Err(DockerError::UnsupportedPlatform);
            }
        }
        let client = builder.build()?;
        let base_url = endpoint.http_origin();
        Ok(Self {
            host_disk: std::sync::Arc::new(
                crate::filesystem::host::disk_usage::HostDiskUsageProvider::new(host_root.into()),
            ),
            client,
            endpoint,
            base_url,
            request_timeout,
            version: std::sync::Arc::new(RwLock::new(None)),
            negotiation: std::sync::Arc::new(Mutex::new(())),
            version_document: Default::default(),
            generation: Default::default(),
            daemon_id: Default::default(),
            storage_usage: Default::default(),
            configuration: Default::default(),
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> &DockerEndpoint {
        &self.endpoint
    }

    pub(crate) async fn open_logs(
        &self,
        id: &str,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::logs::RuntimeLogStream, DockerError> {
        self.open_resource_logs(
            citadel_platforms::logs::LogResource::Container(id),
            true,
            100,
            cancel,
        )
        .await
    }

    pub async fn open_resource_logs(
        &self,
        resource: citadel_platforms::logs::LogResource<'_>,
        follow: bool,
        tail: u16,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::logs::RuntimeLogStream, DockerError> {
        use citadel_platforms::logs::LogResource;
        let (id, kind, inspection, tty_path) = match resource {
            LogResource::Container(id) => (
                id,
                "containers",
                self.inspect_container_document(id).await?,
                "/Config/Tty",
            ),
            LogResource::Service(id) => (
                id,
                "services",
                serde_json::to_value(self.inspect_swarm_service(id).await?)?,
                "/Spec/TaskTemplate/ContainerSpec/TTY",
            ),
        };
        let tty = inspection
            .pointer(tty_path)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        self.log_stream(id, kind, tty, follow, tail, cancel).await
    }

    pub async fn task_logs(
        &self,
        id: &str,
        tail: u16,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::logs::RuntimeLogStream, DockerError> {
        validate_identifier(id)?;
        let task = self
            .raw_document(&format!("/tasks/{}", urlencoding::encode(id)))
            .await?;
        let tty = task
            .pointer("/Spec/ContainerSpec/TTY")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        self.log_stream(id, "tasks", tty, false, tail, cancel).await
    }

    async fn log_stream(
        &self,
        id: &str,
        kind: &str,
        tty: bool,
        follow: bool,
        tail: u16,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::logs::RuntimeLogStream, DockerError> {
        validate_identifier(id)?;
        let version = self.negotiated_version().await?;
        let path = format!("/{kind}/{}/logs", urlencoding::encode(id));
        let details = kind != "containers";
        let response=self.client.get(format!("{}/v{version}{path}?stdout=true&stderr=true&timestamps=true&follow={follow}&tail={tail}&details={details}", self.base_url)).send().await?;
        if !response.status().is_success() {
            return Err(DockerError::Api {
                status: response.status(),
                message: String::from_utf8_lossy(&bounded_body(response, 64 * 1024).await?)
                    .into_owned(),
            });
        }
        Ok(crate::connectors::containers::logs::decode_frames(
            Box::pin(
                response
                    .bytes_stream()
                    .map(|r| r.map_err(DockerError::from)),
            ),
            tty,
            cancel.clone(),
        ))
    }

    pub(crate) async fn open_terminal(
        &self,
        id: &str,
        shell: citadel_platforms::terminal::TerminalShell,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::terminal::TerminalSession, DockerError> {
        self.execute_terminal(id, &[shell.command().to_owned()], cancel)
            .await
    }

    pub async fn execute_terminal(
        &self,
        id: &str,
        command: &[String],
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::terminal::TerminalSession, DockerError> {
        validate_identifier(id)?;
        let created = self.create_exec(id, serde_json::json!({"AttachStdin":true,"AttachStdout":true,"AttachStderr":true,"Tty":true,"Cmd":command})).await?;
        validate_identifier(&created)?;
        let version = self.negotiated_version().await?;
        let start = format!("/exec/{}/start", urlencoding::encode(&created));
        let response = self
            .client
            .post(format!("{}/v{version}{start}", self.base_url))
            .header("Connection", "Upgrade")
            .header("Upgrade", "tcp")
            .json(&serde_json::json!({"Detach":false,"Tty":true}))
            .send()
            .await?;
        if response.status() != StatusCode::SWITCHING_PROTOCOLS {
            return Err(DockerError::Api {
                status: response.status(),
                message: "Docker did not upgrade the terminal connection.".into(),
            });
        }
        let socket = response.upgrade().await?;
        Ok(crate::connectors::containers::terminal::local_session(
            socket,
            self.clone(),
            created,
            self.request_timeout,
            cancel.clone(),
        ))
    }

    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub async fn ping(&self) -> Result<(), DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerPing.start();
        let response = self.send_unversioned("/_ping").await?;
        let body = bounded_body(response, 64).await?;
        let ping = String::from_utf8_lossy(&body).trim().to_owned();
        if ping != "OK" {
            return Err(DockerError::InvalidPing(ping));
        }
        Ok(())
    }

    pub async fn version(&self) -> Result<DockerVersion, DockerError> {
        let mut cached = self.version_document.lock().await;
        if let Some(version) = &*cached {
            return Ok(version.clone());
        }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerVersion.start();
        let response = self.send_unversioned("/version").await?;
        let body = bounded_body(response, MAX_JSON_BODY_BYTES).await?;
        let version: DockerVersion = serde_json::from_slice(&body)?;
        *cached = Some(version.clone());
        Ok(version)
    }

    pub fn daemon_generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Reconnect/daemon replacement invalidates negotiated and static metadata together.
    pub async fn invalidate_daemon(&self) {
        self.invalidate_version().await;
        *self.storage_usage.lock().await = Default::default();
    }

    pub(super) async fn invalidate_version(&self) {
        *self.version_document.lock().await = None;
        *self.version.write().await = None;
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }

    pub async fn negotiated_version(&self) -> Result<ApiVersion, DockerError> {
        if let Some(version) = *self.version.read().await {
            return Ok(version);
        }

        let _guard = self.negotiation.lock().await;
        if let Some(version) = *self.version.read().await {
            return Ok(version);
        }

        let daemon = self.version().await?;
        let maximum = ApiVersion::from_str(&daemon.api_version)?;
        let minimum = ApiVersion::from_str(&daemon.min_api_version)?;
        if maximum < MINIMUM_SUPPORTED_VERSION || minimum > MAXIMUM_SUPPORTED_VERSION {
            return Err(DockerError::IncompatibleVersion {
                daemon_minimum: minimum.to_string(),
                daemon_maximum: maximum.to_string(),
                supported_minimum: MINIMUM_SUPPORTED_VERSION,
                supported_maximum: MAXIMUM_SUPPORTED_VERSION,
            });
        }
        let selected = std::cmp::min(maximum, MAXIMUM_SUPPORTED_VERSION);
        if selected < minimum {
            return Err(DockerError::IncompatibleVersion {
                daemon_minimum: minimum.to_string(),
                daemon_maximum: maximum.to_string(),
                supported_minimum: MINIMUM_SUPPORTED_VERSION,
                supported_maximum: MAXIMUM_SUPPORTED_VERSION,
            });
        }
        *self.version.write().await = Some(selected);
        Ok(selected)
    }

    pub async fn pull_image(
        &self,
        image: &str,
        registry_auth: Option<&str>,
    ) -> Result<DockerJsonStream<DockerImagePullMessage>, DockerError> {
        if image.trim().is_empty() || image.len() > 2048 {
            return Err(DockerError::InvalidIdentifier);
        }
        self.pull_image_with_options(
            DockerImagePullOptions {
                from_image: Some(image),
                ..Default::default()
            },
            registry_auth,
        )
        .await
    }

    pub async fn pull_image_with_options(
        &self,
        options: DockerImagePullOptions<'_>,
        registry_auth: Option<&str>,
    ) -> Result<DockerJsonStream<DockerImagePullMessage>, DockerError> {
        if options
            .from_image
            .is_none_or(|value| value.trim().is_empty())
            && options
                .from_source
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(DockerError::InvalidIdentifier);
        }
        let version = self.negotiated_version().await?;
        let query = {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in [
                ("fromImage", options.from_image),
                ("fromSrc", options.from_source),
                ("repo", options.repository),
                ("tag", options.tag),
            ] {
                if let Some(value) = value {
                    query.append_pair(key, value);
                }
            }
            for change in options.changes {
                query.append_pair("changes", change);
            }
            query.finish()
        };
        let url = format!("{}/v{version}/images/create?{query}", self.base_url);
        let mut request = self.client.post(url);
        if let Some(auth) = registry_auth {
            request = request.header("X-Registry-Auth", auth);
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = bounded_body(response, 64 * 1024).await?;
            return Err(DockerError::Api {
                status,
                message: String::from_utf8_lossy(&body).into_owned(),
            });
        }
        Ok(json_lines(response, MAX_STREAM_ITEM_BYTES))
    }

    pub async fn events(
        &self,
        since: Option<i64>,
        filters: Option<&HashMap<String, Vec<String>>>,
    ) -> Result<DockerJsonStream<DockerEvent>, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerEvents.start();
        let mut parts = Vec::with_capacity(2);
        if let Some(since) = since {
            parts.push(format!("since={since}"));
        }
        if let Some(filters) = filters {
            let serialized = serde_json::to_string(filters)?;
            let encoded = urlencoding::encode(&serialized);
            parts.push(format!("filters={encoded}"));
        }
        let query = (!parts.is_empty()).then(|| parts.join("&"));
        let response = self.send_stream("/events", query.as_deref()).await?;
        Ok(json_lines(response, MAX_STREAM_ITEM_BYTES))
    }

    pub async fn container_stats(
        &self,
        id: &str,
    ) -> Result<DockerJsonStream<ContainerStats>, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerStats.start();
        validate_identifier(id)?;
        let path = format!("/containers/{}/stats", urlencoding::encode(id));
        let response = self.send_stream(&path, Some("stream=true")).await?;
        Ok(json_lines(response, MAX_STREAM_ITEM_BYTES))
    }

    // Raw inspect documents are an intentional UI boundary: retain every daemon field.
    pub async fn inspect_container_document(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, DockerError> {
        validate_identifier(id)?;
        self.raw_document(&format!("/containers/{}/json", urlencoding::encode(id)))
            .await
    }
    pub async fn inspect_image_document(&self, id: &str) -> Result<serde_json::Value, DockerError> {
        validate_identifier(id)?;
        self.raw_document(&format!("/images/{}/json", urlencoding::encode(id)))
            .await
    }
    async fn raw_document(&self, path: &str) -> Result<serde_json::Value, DockerError> {
        let version = self.negotiated_version().await?;
        let response = self
            .client
            .get(format!("{}/v{version}{path}", self.base_url))
            .timeout(self.request_timeout)
            .send()
            .await?;
        let response = self.checked_response(response).await?;
        Ok(serde_json::from_slice(
            &bounded_body(response, MAX_JSON_BODY_BYTES).await?,
        )?)
    }
    async fn send_stream(&self, path: &str, query: Option<&str>) -> Result<Response, DockerError> {
        let version = self.negotiated_version().await?;
        let query = query.map(|q| format!("?{q}")).unwrap_or_default();
        let response = self
            .client
            .get(format!("{}/v{version}{path}{query}", self.base_url))
            .send()
            .await?;
        self.checked_response(response).await
    }
    async fn checked_response(&self, response: Response) -> Result<Response, DockerError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        if status == StatusCode::BAD_REQUEST {
            self.invalidate_version().await;
        }
        let body = bounded_body(response, 64 * 1024).await?;
        Err(DockerError::Api {
            status,
            message: String::from_utf8_lossy(&body).into_owned(),
        })
    }

    async fn send_unversioned(&self, path: &str) -> Result<Response, DockerError> {
        let response = self
            .client
            .get(format!("{}{path}", self.base_url))
            .timeout(self.request_timeout)
            .send()
            .await?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let body = bounded_body(response, 64 * 1024).await?;
        Err(DockerError::Api {
            status,
            message: String::from_utf8_lossy(&body).into_owned(),
        })
    }
}

pub(super) fn validate_identifier(id: &str) -> Result<(), DockerError> {
    if id.trim().is_empty() {
        return Err(DockerError::InvalidIdentifier);
    }
    Ok(())
}

async fn bounded_body(mut response: Response, limit: usize) -> Result<Vec<u8>, DockerError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(DockerError::ResponseTooLarge { limit });
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(DockerError::ResponseTooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn json_lines<T>(response: Response, limit: usize) -> DockerJsonStream<T>
where
    T: DeserializeOwned + Send + 'static,
{
    let output = try_stream! {
        let mut chunks = response.bytes_stream();
        let mut line = Vec::with_capacity(8 * 1024);
        while let Some(chunk) = chunks.next().await {
            let chunk: Bytes = chunk?;
            let mut start = 0;
            for (index, byte) in chunk.iter().enumerate() {
                if *byte != b'\n' {
                    continue;
                }
                append_bounded(&mut line, &chunk[start..index], limit)?;
                if !line.iter().all(u8::is_ascii_whitespace) {
                    yield serde_json::from_slice(&line)?;
                }
                line.clear();
                start = index + 1;
            }
            append_bounded(&mut line, &chunk[start..], limit)?;
        }
        if !line.iter().all(u8::is_ascii_whitespace) {
            yield serde_json::from_slice(&line)?;
        }
    };
    Box::pin(output)
}

fn append_bounded(target: &mut Vec<u8>, source: &[u8], limit: usize) -> Result<(), DockerError> {
    if target.len().saturating_add(source.len()) > limit {
        return Err(DockerError::StreamItemTooLarge { limit });
    }
    target.extend_from_slice(source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiates_supported_versions_without_lexical_comparison() {
        assert!(ApiVersion::from_str("1.49").unwrap() > ApiVersion::from_str("1.9").unwrap());
        assert_eq!(ApiVersion::from_str("1.41").unwrap().to_string(), "1.41");
    }

    #[test]
    fn rejects_empty_resource_identifiers_before_transport() {
        assert!(matches!(
            validate_identifier("  "),
            Err(DockerError::InvalidIdentifier)
        ));
    }
}
