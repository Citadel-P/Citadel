use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::time::Duration;

use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

#[path = "binary_exec.rs"]
mod binary_exec;

use super::generated::{
    CONFIG_LIST, CONTAINER_CREATE, CONTAINER_DELETE, CONTAINER_INSPECT, CONTAINER_LIST,
    CONTAINER_START, CONTAINER_STATS, ContainerInspect, ContainerStats, ContainerSummary,
    DockerEvent, DockerInfo, DockerNetwork, DockerVersion, DockerVolume, Endpoint, IMAGE_CREATE,
    IMAGE_LIST, ImageSummary, NETWORK_CREATE, NETWORK_DELETE, NETWORK_INSPECT, NETWORK_LIST,
    NODE_LIST, NetworkCreateRequest, NetworkCreateResponse, SECRET_LIST, SERVICE_CREATE,
    SERVICE_DELETE, SERVICE_INSPECT, SERVICE_LIST, SERVICE_UPDATE, SWARM_INSPECT, SYSTEM_EVENTS,
    SYSTEM_INFO, SYSTEM_PING, SYSTEM_VERSION, SwarmConfig, SwarmInspect, SwarmNode, SwarmSecret,
    SwarmService, SwarmTask, TASK_INSPECT, TASK_LIST, VOLUME_CREATE, VOLUME_DELETE, VOLUME_INSPECT,
    VOLUME_LIST, VolumeCreateOptions, VolumeListResponse,
};

const MINIMUM_SUPPORTED_VERSION: ApiVersion = ApiVersion::new(1, 41);
const MAXIMUM_SUPPORTED_VERSION: ApiVersion = ApiVersion::new(1, 49);
const MAX_JSON_BODY_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAM_ITEM_BYTES: usize = 1024 * 1024;

pub type DockerJsonStream<T> = Pin<Box<dyn Stream<Item = Result<T, DockerError>> + Send + 'static>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerCreateResponse {
    id: String,
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
    #[serde(rename="progressDetail",default)]
    pub progress_detail: Option<DockerImagePullProgress>,
}

#[derive(Debug,Clone,Default,Deserialize)]
pub struct DockerImagePullProgress {
    pub units: Option<String>, pub current: Option<i64>, pub total: Option<i64>, pub start: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DockerImagePullError {
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
    #[error("Docker transport failed: {0}")]
    Transport(#[from] reqwest::Error),
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
    #[error("unsupported generated Docker HTTP method '{0}'")]
    InvalidMethod(String),
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

#[derive(Clone)]
pub struct DockerClient {
    pub(crate) host_disk: std::sync::Arc<crate::host_disk_usage::HostDiskUsageProvider>,
    client: Client,
    socket_path: PathBuf,
    request_timeout: Duration,
    version: std::sync::Arc<RwLock<Option<ApiVersion>>>,
    negotiation: std::sync::Arc<Mutex<()>>,
    pub(super) storage_usage: std::sync::Arc<Mutex<super::storage_usage::UsageCache>>,
}

impl DockerClient {
    pub fn new(
        socket_path: impl Into<PathBuf>,
        request_timeout: Duration,
    ) -> Result<Self, DockerError> {
        let socket_path = socket_path.into();
        #[cfg(unix)]
        let client = Client::builder().unix_socket(socket_path.clone()).build()?;
        #[cfg(not(unix))]
        let client = Client::builder().build()?;

        Ok(Self {
            host_disk: std::sync::Arc::new(crate::host_disk_usage::HostDiskUsageProvider::from_env()),
            client,
            socket_path,
            request_timeout,
            version: std::sync::Arc::new(RwLock::new(None)),
            negotiation: std::sync::Arc::new(Mutex::new(())),
            storage_usage: Default::default(),
        })
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
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

    pub(crate) async fn open_resource_logs(
        &self,
        resource: citadel_platforms::logs::LogResource<'_>,
        follow: bool,
        tail: u16,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<citadel_platforms::logs::RuntimeLogStream, DockerError> {
        use citadel_platforms::logs::LogResource;
        let (id, endpoint, inspect, tty_path, details) = match resource {
            LogResource::Container(id) => (
                id,
                &super::generated::CONTAINER_LOGS,
                &CONTAINER_INSPECT,
                "/Config/Tty",
                false,
            ),
            LogResource::Service(id) => (
                id,
                &super::generated::SERVICE_LOGS,
                &super::generated::SERVICE_INSPECT,
                "/Spec/TaskTemplate/ContainerSpec/TTY",
                true,
            ),
        };
        validate_identifier(id)?;
        let version = self.negotiated_version().await?;
        let path = inspect.path.replace("{id}", &urlencoding::encode(id));
        let inspection: serde_json::Value = self.get_json(inspect, &path, None).await?;
        let tty = inspection
            .pointer(tty_path)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let path = endpoint.path.replace("{id}", &urlencoding::encode(id));
        let response=self.client.get(format!("http://localhost/v{version}{path}?stdout=true&stderr=true&timestamps=true&follow={follow}&tail={tail}&details={details}")).send().await?;
        if !response.status().is_success() {
            return Err(DockerError::Api {
                status: response.status(),
                message: String::from_utf8_lossy(&bounded_body(response, 64 * 1024).await?)
                    .into_owned(),
            });
        }
        Ok(crate::container_logs::decode_frames(
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
        use super::generated::{CONTAINER_EXEC, EXEC_RESIZE, EXEC_START};
        validate_identifier(id)?;
        let path = CONTAINER_EXEC
            .path
            .replace("{id}", &urlencoding::encode(id));
        let created: ContainerCreateResponse = self.request_json(&CONTAINER_EXEC, &path, None,
            Some(&serde_json::json!({"AttachStdin":true,"AttachStdout":true,"AttachStderr":true,"Tty":true,"Cmd":[shell.command()]}))).await?;
        validate_identifier(&created.id)?;
        let version = self.negotiated_version().await?;
        let start = EXEC_START
            .path
            .replace("{id}", &urlencoding::encode(&created.id));
        let response = self
            .client
            .post(format!("http://localhost/v{version}{start}"))
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
        let resize = EXEC_RESIZE
            .path
            .replace("{id}", &urlencoding::encode(&created.id));
        Ok(crate::container_terminal::local_session(
            socket,
            self.client.clone(),
            format!("http://localhost/v{version}{resize}"),
            self.request_timeout,
            cancel.clone(),
        ))
    }

    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub async fn ping(&self) -> Result<(), DockerError> {
        self.ensure_supported()?;
        let response = self.send(&SYSTEM_PING, SYSTEM_PING.path, None).await?;
        let body = bounded_body(response, 64).await?;
        let ping = String::from_utf8_lossy(&body).trim().to_owned();
        if ping != "OK" {
            return Err(DockerError::InvalidPing(ping));
        }
        Ok(())
    }

    pub async fn version(&self) -> Result<DockerVersion, DockerError> {
        let response = self.send_unversioned(SYSTEM_VERSION.path).await?;
        let body = bounded_body(response, MAX_JSON_BODY_BYTES).await?;
        Ok(serde_json::from_slice(&body)?)
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

    pub async fn info(&self) -> Result<DockerInfo, DockerError> {
        self.get_json(&SYSTEM_INFO, SYSTEM_INFO.path, None).await
    }

    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DockerError> {
        let query = if all { "all=true" } else { "all=false" };
        self.get_json(&CONTAINER_LIST, CONTAINER_LIST.path, Some(query))
            .await
    }

    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspect, DockerError> {
        validate_identifier(id)?;
        let path = CONTAINER_INSPECT
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.get_json(&CONTAINER_INSPECT, &path, None).await
    }

    pub async fn delete_container(
        &self,
        id: &str,
        remove_volumes: bool,
        force: bool,
    ) -> Result<(), DockerError> {
        self.delete_container_with_options(id, remove_volumes, force, false)
            .await
    }

    pub async fn delete_container_with_options(
        &self,
        id: &str,
        remove_volumes: bool,
        force: bool,
        link: bool,
    ) -> Result<(), DockerError> {
        validate_identifier(id)?;
        let path = CONTAINER_DELETE
            .path
            .replace("{id}", &urlencoding::encode(id));
        let query = format!("v={remove_volumes}&force={force}&link={link}");
        self.send_request::<()>(&CONTAINER_DELETE, &path, Some(&query), None)
            .await?;
        Ok(())
    }

    pub async fn create_container<T: Serialize + ?Sized>(
        &self,
        name: &str,
        body: &T,
    ) -> Result<String, DockerError> {
        validate_identifier(name)?;
        let query = format!("name={}", urlencoding::encode(name));
        let response: ContainerCreateResponse = self
            .request_json(
                &CONTAINER_CREATE,
                CONTAINER_CREATE.path,
                Some(&query),
                Some(body),
            )
            .await?;
        Ok(response.id)
    }

    pub async fn start_container(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        let path = CONTAINER_START
            .path
            .replace("{id}", &urlencoding::encode(id));
        match self
            .send_request::<()>(&CONTAINER_START, &path, None, None)
            .await
        {
            Ok(_)
            | Err(DockerError::Api {
                status: StatusCode::NOT_MODIFIED,
                ..
            }) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub async fn change_container_state(
        &self,
        id: &str,
        action: citadel_platforms::containers::ContainerAction,
    ) -> Result<(), DockerError> {
        use super::generated::{
            CONTAINER_PAUSE, CONTAINER_RESTART, CONTAINER_STOP, CONTAINER_UNPAUSE,
        };
        use citadel_platforms::containers::ContainerAction::*;
        let endpoint = match action {
            Start => return self.start_container(id).await,
            Stop => &CONTAINER_STOP,
            Restart => &CONTAINER_RESTART,
            Pause => &CONTAINER_PAUSE,
            Unpause => &CONTAINER_UNPAUSE,
            Delete(options) => {
                return self
                    .delete_container_with_options(id, options.v, options.force, options.link)
                    .await;
            }
        };
        validate_identifier(id)?;
        let path = endpoint.path.replace("{id}", &urlencoding::encode(id));
        let query = matches!(action, Stop | Restart).then_some("t=10");
        match self.send_request::<()>(endpoint, &path, query, None).await {
            Ok(_)
            | Err(DockerError::Api {
                status: StatusCode::NOT_MODIFIED,
                ..
            }) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub async fn pull_image(
        &self,
        image: &str,
        registry_auth: Option<&str>,
    ) -> Result<DockerJsonStream<DockerImagePullMessage>, DockerError> {
        if image.trim().is_empty() || image.len() > 2048 {
            return Err(DockerError::InvalidIdentifier);
        }
        self.ensure_supported()?;
        let version = self.negotiated_version().await?;
        let query = format!("fromImage={}", urlencoding::encode(image));
        let url = format!("http://localhost/v{version}{}?{query}", IMAGE_CREATE.path);
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

    pub async fn prune_resources(&self, resource: citadel_platforms::prune::PruneResource) -> Result<serde_json::Value, DockerError> {
        use super::generated::{VOLUME_PRUNE,NETWORK_PRUNE,IMAGE_PRUNE,BUILD_PRUNE};
        use citadel_platforms::prune::PruneResource;
        let (endpoint,query) = match resource {
            PruneResource::Volume => (&VOLUME_PRUNE,"filters=%7B%7D"),
            PruneResource::Network => (&NETWORK_PRUNE,"filters=%7B%7D"),
            PruneResource::Image => (&IMAGE_PRUNE,"filters=%7B%22dangling%22%3A%5B%22false%22%5D%7D"),
            PruneResource::Build => (&BUILD_PRUNE,"all=true&filters=%7B%7D"),
            PruneResource::All => return Err(DockerError::InvalidIdentifier),
        };
        self.request_json::<(),serde_json::Value>(endpoint,endpoint.path,Some(query),None).await
    }

    pub async fn inspect_swarm(&self) -> Result<SwarmInspect, DockerError> {
        self.get_json(&SWARM_INSPECT, SWARM_INSPECT.path, None)
            .await
    }

    pub async fn list_images(&self) -> Result<Vec<ImageSummary>, DockerError> {
        self.get_json(&IMAGE_LIST, IMAGE_LIST.path, Some("all=true"))
            .await
    }

    pub async fn inspect_image(
        &self,
        id: &str,
    ) -> Result<super::generated::ImageInspect, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::IMAGE_INSPECT;
        self.get_json(
            endpoint,
            &endpoint.path.replace("{name}", &urlencoding::encode(id)),
            None,
        )
        .await
    }

    pub async fn inspect_image_document(&self, id: &str) -> Result<serde_json::Value, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::IMAGE_INSPECT;
        self.get_json(
            endpoint,
            &endpoint.path.replace("{name}", &urlencoding::encode(id)),
            None,
        )
        .await
    }

    pub async fn inspect_container_document(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, DockerError> {
        validate_identifier(id)?;
        self.get_json(
            &CONTAINER_INSPECT,
            &CONTAINER_INSPECT
                .path
                .replace("{id}", &urlencoding::encode(id)),
            None,
        )
        .await
    }

    pub async fn image_history(
        &self,
        id: &str,
    ) -> Result<Vec<super::generated::ImageHistoryItem>, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::IMAGE_HISTORY;
        self.get_json(
            endpoint,
            &endpoint.path.replace("{name}", &urlencoding::encode(id)),
            None,
        )
        .await
    }

    pub async fn delete_image(
        &self,
        id: &str,
        force: bool,
        no_prune: bool,
    ) -> Result<Vec<std::collections::BTreeMap<String, String>>, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::IMAGE_DELETE;
        self.request_json::<(), _>(
            endpoint,
            &endpoint.path.replace("{name}", &urlencoding::encode(id)),
            Some(&format!("force={force}&noprune={no_prune}")),
            None,
        )
        .await
    }

    pub async fn image_containers(
        &self,
        id: &str,
    ) -> Result<Vec<super::generated::ImageUsageContainer>, DockerError> {
        validate_identifier(id)?;
        let filters = serde_json::json!({"ancestor": [id]}).to_string();
        self.get_json(
            &CONTAINER_LIST,
            CONTAINER_LIST.path,
            Some(&format!(
                "all=true&filters={}",
                urlencoding::encode(&filters)
            )),
        )
        .await
    }

    pub async fn list_volumes(&self) -> Result<Vec<DockerVolume>, DockerError> {
        let mut volumes = self
            .get_json::<VolumeListResponse>(&VOLUME_LIST, VOLUME_LIST.path, None)
            .await?
            .volumes;
        // Docker normally includes UsageData only in /system/df, not /volumes.
        // Fetch just volume usage once, and join by name rather than response order.
        if volumes.iter().any(|volume| volume.usage_data.is_none()) {
            let mut usage = self.volume_usage().await?;
            for volume in &mut volumes {
                if volume.usage_data.is_none() {
                    volume.usage_data = usage.remove(&volume.name);
                }
            }
        }
        Ok(volumes)
    }

    async fn volume_usage(&self) -> Result<HashMap<String, serde_json::Value>, DockerError> {
        use super::generated::SYSTEM_DATA_USAGE;
        // The relevant subset of SystemDataUsage has the same Volumes envelope.
        let usage: VolumeListResponse = self
            .get_json(
                &SYSTEM_DATA_USAGE,
                SYSTEM_DATA_USAGE.path,
                Some("type=volume"),
            )
            .await?;
        Ok(usage
            .volumes
            .into_iter()
            .filter_map(|volume| volume.usage_data.map(|usage| (volume.name, usage)))
            .collect())
    }

    pub async fn inspect_volume(&self, name: &str) -> Result<DockerVolume, DockerError> {
        validate_identifier(name)?;
        let path = VOLUME_INSPECT
            .path
            .replace("{name}", &urlencoding::encode(name));
        let mut volume: DockerVolume = self.get_json(&VOLUME_INSPECT, &path, None).await?;
        if volume.usage_data.is_none() {
            volume.usage_data = self.volume_usage().await?.remove(&volume.name);
        }
        Ok(volume)
    }

    pub async fn create_volume(
        &self,
        request: &VolumeCreateOptions,
    ) -> Result<DockerVolume, DockerError> {
        self.request_json(&VOLUME_CREATE, VOLUME_CREATE.path, None, Some(request))
            .await
    }

    pub async fn delete_volume(&self, name: &str, force: bool) -> Result<(), DockerError> {
        validate_identifier(name)?;
        let path = VOLUME_DELETE
            .path
            .replace("{name}", &urlencoding::encode(name));
        self.send_request::<()>(
            &VOLUME_DELETE,
            &path,
            Some(if force { "force=true" } else { "force=false" }),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn list_networks(&self) -> Result<Vec<DockerNetwork>, DockerError> {
        self.get_json(&NETWORK_LIST, NETWORK_LIST.path, None).await
    }

    pub async fn inspect_network(&self, id: &str) -> Result<DockerNetwork, DockerError> {
        validate_identifier(id)?;
        let path = NETWORK_INSPECT
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.get_json(&NETWORK_INSPECT, &path, None).await
    }

    pub async fn create_network(
        &self,
        request: &NetworkCreateRequest,
    ) -> Result<NetworkCreateResponse, DockerError> {
        self.request_json(&NETWORK_CREATE, NETWORK_CREATE.path, None, Some(request))
            .await
    }

    pub async fn delete_network(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        let path = NETWORK_DELETE
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.send_request::<()>(&NETWORK_DELETE, &path, None, None)
            .await?;
        Ok(())
    }

    pub async fn list_swarm_nodes(&self) -> Result<Vec<SwarmNode>, DockerError> {
        self.get_json(&NODE_LIST, NODE_LIST.path, None).await
    }

    pub async fn inspect_swarm_node(&self, id: &str) -> Result<SwarmNode, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::NODE_INSPECT;
        self.get_json(endpoint, &endpoint.path.replace("{id}", &urlencoding::encode(id)), None).await
    }

    pub async fn update_swarm_node(&self, id: &str, version: i64, spec: &serde_json::Value) -> Result<(), DockerError> {
        self.update_swarm_spec(&super::generated::NODE_UPDATE, id, version, spec).await
    }

    pub async fn update_swarm_material(&self, secret: bool, id: &str, version: i64, spec: &serde_json::Value) -> Result<(), DockerError> {
        let endpoint = if secret { &super::generated::SECRET_UPDATE } else { &super::generated::CONFIG_UPDATE };
        self.update_swarm_spec(endpoint, id, version, spec).await
    }

    async fn update_swarm_spec(&self, endpoint: &Endpoint, id: &str, version: i64, spec: &serde_json::Value) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.send_request(endpoint, &endpoint.path.replace("{id}", &urlencoding::encode(id)), Some(&format!("version={version}")), Some(spec)).await?;
        Ok(())
    }

    pub async fn list_swarm_service_tasks(&self, id: &str) -> Result<Vec<SwarmTask>, DockerError> {
        validate_identifier(id)?;
        let filters = serde_json::json!({"service":[id],"desired-state":["running"]}).to_string();
        self.get_json(&TASK_LIST, TASK_LIST.path, Some(&format!("filters={}", urlencoding::encode(&filters)))).await
    }
    pub async fn list_swarm_node_tasks(&self, id: &str) -> Result<Vec<SwarmTask>, DockerError> {
        validate_identifier(id)?;
        let filters = serde_json::json!({"node":[id],"desired-state":["running"]}).to_string();
        self.get_json(&TASK_LIST, TASK_LIST.path, Some(&format!("filters={}", urlencoding::encode(&filters)))).await
    }

    pub async fn list_swarm_services(&self) -> Result<Vec<SwarmService>, DockerError> {
        self.get_json(&SERVICE_LIST, SERVICE_LIST.path, Some("status=true"))
            .await
    }

    pub async fn inspect_swarm_service(&self, id: &str) -> Result<SwarmService, DockerError> {
        validate_identifier(id)?;
        let path = SERVICE_INSPECT
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.get_json(&SERVICE_INSPECT, &path, None).await
    }

    pub async fn inspect_swarm_task(&self, id: &str) -> Result<SwarmTask, DockerError> {
        validate_identifier(id)?;
        let path = TASK_INSPECT.path.replace("{id}", &urlencoding::encode(id));
        self.get_json(&TASK_INSPECT, &path, None).await
    }

    pub async fn create_swarm_service(
        &self,
        spec: &serde_json::Value,
    ) -> Result<serde_json::Value, DockerError> {
        self.request_json(&SERVICE_CREATE, SERVICE_CREATE.path, None, Some(spec))
            .await
    }

    pub async fn update_swarm_service(
        &self,
        id: &str,
        version: i64,
        spec: &serde_json::Value,
    ) -> Result<serde_json::Value, DockerError> {
        validate_identifier(id)?;
        let path = SERVICE_UPDATE
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.request_json(
            &SERVICE_UPDATE,
            &path,
            Some(&format!("version={version}")),
            Some(spec),
        )
        .await
    }

    pub async fn delete_swarm_service(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        let path = SERVICE_DELETE
            .path
            .replace("{id}", &urlencoding::encode(id));
        self.send_request::<()>(&SERVICE_DELETE, &path, None, None)
            .await?;
        Ok(())
    }

    pub async fn list_swarm_tasks(&self) -> Result<Vec<SwarmTask>, DockerError> {
        let filters = urlencoding::encode(r#"{"desired-state":["running"]}"#);
        let query = format!("filters={filters}");
        self.get_json(&TASK_LIST, TASK_LIST.path, Some(&query))
            .await
    }

    pub async fn list_swarm_secrets(&self) -> Result<Vec<SwarmSecret>, DockerError> {
        self.get_json(&SECRET_LIST, SECRET_LIST.path, None).await
    }

    pub async fn inspect_swarm_secret(&self, id: &str) -> Result<SwarmSecret, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::SECRET_INSPECT;
        self.get_json(
            endpoint,
            &endpoint.path.replace("{id}", &urlencoding::encode(id)),
            None,
        )
        .await
    }
    pub async fn distribution_inspect(
        &self,
        image: &str,
    ) -> Result<serde_json::Value, DockerError> {
        self.distribution_inspect_authenticated(image, None).await
    }

    pub async fn distribution_inspect_authenticated(
        &self,
        image: &str,
        registry_auth: Option<&str>,
    ) -> Result<serde_json::Value, DockerError> {
        validate_identifier(image)?;
        self.ensure_supported()?;
        let endpoint = &super::generated::DISTRIBUTION_INSPECT;
        let path = endpoint.path.replace("{name}", &urlencoding::encode(image));
        let version = self.negotiated_version().await?;
        let mut request = self
            .client
            .get(format!("http://localhost/v{version}{path}"))
            .timeout(self.request_timeout);
        if let Some(auth) = registry_auth {
            request = request.header("X-Registry-Auth", auth);
        }
        let response = request.send().await?;
        let status = response.status();
        let body = bounded_body(response, MAX_JSON_BODY_BYTES).await?;
        if !status.is_success() {
            return Err(DockerError::Api {
                status,
                message: "Registry inspection failed.".into(),
            });
        }
        Ok(serde_json::from_slice(&body)?)
    }
    pub async fn create_swarm_material(
        &self,
        secret: bool,
        spec: &serde_json::Value,
    ) -> Result<serde_json::Value, DockerError> {
        let endpoint = if secret {
            &super::generated::SECRET_CREATE
        } else {
            &super::generated::CONFIG_CREATE
        };
        self.request_json(endpoint, endpoint.path, None, Some(spec))
            .await
    }
    pub async fn inspect_swarm_config(&self, id: &str) -> Result<SwarmConfig, DockerError> {
        validate_identifier(id)?;
        let endpoint = &super::generated::CONFIG_INSPECT;
        self.get_json(
            endpoint,
            &endpoint.path.replace("{id}", &urlencoding::encode(id)),
            None,
        )
        .await
    }
    pub async fn delete_swarm_secret(&self, id: &str) -> Result<(), DockerError> {
        self.delete_swarm_resource(&super::generated::SECRET_DELETE, id)
            .await
    }
    pub async fn delete_swarm_config(&self, id: &str) -> Result<(), DockerError> {
        self.delete_swarm_resource(&super::generated::CONFIG_DELETE, id)
            .await
    }
    async fn delete_swarm_resource(
        &self,
        endpoint: &Endpoint,
        id: &str,
    ) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.send_request::<()>(
            endpoint,
            &endpoint.path.replace("{id}", &urlencoding::encode(id)),
            None,
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn list_swarm_configs(&self) -> Result<Vec<SwarmConfig>, DockerError> {
        self.get_json(&CONFIG_LIST, CONFIG_LIST.path, None).await
    }

    pub async fn events(
        &self,
        since: Option<i64>,
        filters: Option<&HashMap<String, Vec<String>>>,
    ) -> Result<DockerJsonStream<DockerEvent>, DockerError> {
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
        let response = self
            .send(&SYSTEM_EVENTS, SYSTEM_EVENTS.path, query.as_deref())
            .await?;
        Ok(json_lines(response, MAX_STREAM_ITEM_BYTES))
    }

    pub async fn container_stats(
        &self,
        id: &str,
    ) -> Result<DockerJsonStream<ContainerStats>, DockerError> {
        validate_identifier(id)?;
        let path = CONTAINER_STATS
            .path
            .replace("{id}", &urlencoding::encode(id));
        let response = self
            .send(&CONTAINER_STATS, &path, Some("stream=true"))
            .await?;
        Ok(json_lines(response, MAX_STREAM_ITEM_BYTES))
    }

    pub async fn container_stats_once(&self, id: &str) -> Result<ContainerStats, DockerError> {
        validate_identifier(id)?;
        let path = CONTAINER_STATS
            .path
            .replace("{id}", &urlencoding::encode(id));
        let endpoint = Endpoint {
            streaming: false,
            ..CONTAINER_STATS
        };
        self.get_json(&endpoint, &path, Some("stream=false&one-shot=true"))
            .await
    }

    pub(super) async fn get_json<T: DeserializeOwned>(
        &self,
        endpoint: &Endpoint,
        path: &str,
        query: Option<&str>,
    ) -> Result<T, DockerError> {
        let response = self.send(endpoint, path, query).await?;
        let body = bounded_body(response, MAX_JSON_BODY_BYTES).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    async fn request_json<TRequest, TResponse>(
        &self,
        endpoint: &Endpoint,
        path: &str,
        query: Option<&str>,
        body: Option<&TRequest>,
    ) -> Result<TResponse, DockerError>
    where
        TRequest: Serialize + ?Sized,
        TResponse: DeserializeOwned,
    {
        let response = self.send_request(endpoint, path, query, body).await?;
        let body = bounded_body(response, MAX_JSON_BODY_BYTES).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    async fn send(
        &self,
        endpoint: &Endpoint,
        path: &str,
        query: Option<&str>,
    ) -> Result<Response, DockerError> {
        self.send_request::<()>(endpoint, path, query, None).await
    }

    async fn send_request<T: Serialize + ?Sized>(
        &self,
        endpoint: &Endpoint,
        path: &str,
        query: Option<&str>,
        body: Option<&T>,
    ) -> Result<Response, DockerError> {
        self.ensure_supported()?;
        let version_prefix = if endpoint.versioned {
            format!("/v{}", self.negotiated_version().await?)
        } else {
            String::new()
        };
        let query = query.map(|value| format!("?{value}")).unwrap_or_default();
        let url = format!("http://localhost{version_prefix}{path}{query}");
        let request = match endpoint.method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "DELETE" => self.client.delete(url),
            method => return Err(DockerError::InvalidMethod(method.to_owned())),
        };
        let request = if let Some(body) = body {
            request.json(body)
        } else {
            request
        };
        let request = if endpoint.streaming {
            request
        } else {
            request.timeout(self.request_timeout)
        };
        let response = request.send().await?;
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        if status == StatusCode::BAD_REQUEST && endpoint.versioned {
            *self.version.write().await = None;
        }
        let body = bounded_body(response, 64 * 1024).await?;
        Err(DockerError::Api {
            status,
            message: String::from_utf8_lossy(&body).into_owned(),
        })
    }

    async fn send_unversioned(&self, path: &str) -> Result<Response, DockerError> {
        self.ensure_supported()?;
        let response = self
            .client
            .get(format!("http://localhost{path}"))
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

    fn ensure_supported(&self) -> Result<(), DockerError> {
        #[cfg(unix)]
        {
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(DockerError::UnsupportedPlatform)
        }
    }
}

fn validate_identifier(id: &str) -> Result<(), DockerError> {
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
    fn generated_subset_matches_citadel_version_range() {
        assert_eq!(
            super::super::generated::SCHEMA_API_VERSION,
            MAXIMUM_SUPPORTED_VERSION.to_string()
        );
        assert_eq!(SYSTEM_EVENTS.operation_id, "SystemEvents");
    }

    #[test]
    fn rejects_empty_resource_identifiers_before_transport() {
        assert!(matches!(
            validate_identifier("  "),
            Err(DockerError::InvalidIdentifier)
        ));
    }

    #[test]
    fn generated_models_accept_null_optional_collections_emitted_by_docker() {
        let info: DockerInfo = serde_json::from_value(serde_json::json!({
            "Swarm": { "RemoteManagers": null }
        }))
        .unwrap();
        assert!(info.swarm.unwrap().remote_managers.is_empty());

        let volumes: VolumeListResponse = serde_json::from_value(serde_json::json!({
            "Volumes": [{
                "Name": "data",
                "Status": null,
                "Labels": null,
                "Options": null
            }],
            "Warnings": null
        }))
        .unwrap();
        assert!(volumes.warnings.is_empty());
        let volume = &volumes.volumes[0];
        assert!(volume.status.is_empty());
        assert!(volume.labels.is_empty());
        assert!(volume.options.is_empty());

        let network: DockerNetwork = serde_json::from_value(serde_json::json!({
            "Id": "network-1",
            "Containers": null,
            "Peers": null
        }))
        .unwrap();
        assert!(network.containers.is_empty());
        assert!(network.peers.is_empty());
    }
}
