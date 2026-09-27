//! Read-only Volume access. Resolve the owning daemon once, then keep that exact
//! transport for creation, execution and cleanup (including after reconnects).
use crate::connectors::agent::client::AgentClient;
use crate::connectors::agent::client::AgentContainerAction;
use crate::connectors::agent::execution::AgentExecutionClient;
use crate::connectors::docker::DockerClient;
use crate::connectors::edge::EdgeRegistry;
use crate::connectors::routing::containers::ContainerRuntimeRouter;
use crate::connectors::routing::containers::Runtime;
use citadel_contracts::citadel::{
    containers::v1::{
        CreateContainerRequest, ExecBinaryRequest, ExecServerMessage, exec_server_message::Msg,
    },
    edge::v1::EdgeCommandKind,
    shared_models::v1::Mount,
};
use citadel_platforms::VolumeObservationPort;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, containers::ContainerTarget, volume_content::*,
};
use citadel_runtime::{DynamicTaskReservation, DynamicTasks};
use futures_util::{StreamExt, stream::BoxStream};
use serde::Deserialize;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const HELPER_BINARY: &str = "/app/Citadel.Agent.VolumeHelper";
const CORE_HELPER_BINARY: &str = "/usr/local/bin/citadel-volume-helper";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(1800);
type Frames = BoxStream<'static, Result<ExecServerMessage, tonic::Status>>;

mod cleanup_job;

pub struct VolumeContentAdapter {
    router: ContainerRuntimeRouter,
    pool: PgPool,
    helper_image: String,
    core_container: Option<String>,
    slots: Arc<Semaphore>,
    tasks: DynamicTasks,
}

pub struct VolumeDownload {
    pub filename: String,
    pub directory: bool,
    pub stream: BoxStream<'static, Result<Vec<u8>, std::io::Error>>,
}

#[derive(Clone)]
enum HelperRuntime {
    Local(DockerClient),
    Agent(AgentExecutionClient),
}

struct Helper {
    runtime: HelperRuntime,
    binary: &'static str,
    id: String,
    // Retain capacity until cleanup finishes, not merely until HTTP disconnects.
    permit: Option<OwnedSemaphorePermit>,
    cancellation: CancellationToken,
    cleanup: Option<DynamicTaskReservation>,
}

impl VolumeContentAdapter {
    pub fn new(
        pool: PgPool,
        docker: DockerClient,
        agent: Option<AgentClient>,
        edge: EdgeRegistry,
        helper_image: String,
        tasks: DynamicTasks,
    ) -> Self {
        Self {
            router: ContainerRuntimeRouter::new(pool.clone(), docker, agent, edge),
            pool,
            helper_image,
            core_container: None,
            slots: Arc::new(Semaphore::new(4)),
            tasks,
        }
    }

    /// Use the running Core's immutable image for local helpers. Remote Agents
    /// keep their own image and helper protocol on the owning daemon.
    pub fn with_core_container(mut self, container: String) -> Self {
        self.core_container = Some(container);
        self
    }

    async fn open(
        &self,
        platform: Uuid,
        volume: &str,
        node: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Helper, RuntimeCapabilityError> {
        if volume.is_empty()
            || volume.len() > 255
            || !volume
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(error(
                RuntimeErrorKind::InvalidRequest,
                "Invalid Volume name.",
            ));
        }
        let permit = self.slots.clone().try_acquire_owned().map_err(|_| {
            error(
                RuntimeErrorKind::ResourceExhausted,
                "Volume browsing is busy.",
            )
        })?;
        let cleanup = self.tasks.reserve().ok_or_else(|| {
            error(
                RuntimeErrorKind::ResourceExhausted,
                "Volume browsing is shutting down.",
            )
        })?;
        let target = ContainerTarget {
            id: Uuid::nil(),
            platform_id: platform,
            docker_id: String::new(),
            node_id: node.map(str::to_owned),
        };
        let runtime = self.router.resolve(&target, cancellation).await?;
        // Docker create would otherwise silently create a new empty named Volume.
        let runtime = match runtime {
            Runtime::Local(r) => {
                r.inspect_volume(volume)
                    .await
                    .map_err(crate::connectors::docker::runtime::normalize_docker_error)?;
                HelperRuntime::Local(r.clone())
            }
            Runtime::Agent(r) => {
                r.inspect_volume(volume, cancellation).await?;
                HelperRuntime::Agent(AgentExecutionClient::Direct(r))
            }
            Runtime::Edge(r) => {
                r.inspect_volume(volume, cancellation).await?;
                HelperRuntime::Agent(AgentExecutionClient::Edge(r.session))
            }
        };
        // The helper must be available on the owning Agent's daemon. Core's
        // configured image can differ from the image that Agent actually runs.
        let agent_image = match &runtime {
            HelperRuntime::Agent(agent) => Some(
                agent
                    .runtime_image(cancellation)
                    .await
                    .map_err(|error| helper_startup_error(error.message))?,
            ),
            HelperRuntime::Local(_) => None,
        };
        let (image, binary) = if let (HelperRuntime::Local(docker), Some(container)) =
            (&runtime, &self.core_container)
        {
            let document = docker
                .inspect_container_document(container)
                .await
                .map_err(|error| helper_startup_error(error.to_string()))?;
            (core_image(&document)?, CORE_HELPER_BINARY)
        } else if let Some(image) = agent_image.filter(|image| !image.trim().is_empty()) {
            (image, HELPER_BINARY)
        } else {
            let image = if node.is_some() {
                sqlx::query_as::<_, (String, String)>("SELECT agentimagereference,agentimagedigest FROM swarmnodeagentinstallations WHERE platformid=$1 AND desiredstate='Installed'")
                .bind(platform).fetch_optional(&self.pool).await.map_err(|_| error(RuntimeErrorKind::Remote,"Could not resolve the Node helper image."))?
                .map(|(reference, digest)| citadel_platforms::node_agents::setup::pin_image_reference(&reference, &digest))
                .transpose()?
                .unwrap_or_else(|| self.helper_image.clone())
            } else {
                self.helper_image.clone()
            };
            (image, HELPER_BINARY)
        };
        let name = format!("citadel-volume-helper-{}", Uuid::now_v7().simple());
        let request = helper_request(platform, volume, &name, &image, binary);
        // Own cleanup before sending Create, including an ambiguous response. The
        // name is unique to this operation, and no mutation is retried.
        let mut helper = Helper {
            runtime,
            binary,
            id: name,
            permit: Some(permit),
            cancellation: cancellation.clone(),
            cleanup: Some(cleanup),
        };
        helper.id = match &helper.runtime {
            HelperRuntime::Local(r) => r
                .create_container(&request.name, &local_request(&request))
                .await
                .map_err(|error| helper_startup_error(error.to_string()))?,
            HelperRuntime::Agent(r) => r
                .create_container(request, cancellation)
                .await
                .map_err(|error| helper_startup_error(error.message))?,
        };
        match &helper.runtime {
            HelperRuntime::Local(r) => r
                .start_container(&helper.id)
                .await
                .map_err(|error| helper_startup_error(error.to_string()))?,
            HelperRuntime::Agent(r) => r
                .change_containers_state(
                    std::slice::from_ref(&helper.id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| helper_startup_error(error.message))?,
        }
        Ok(helper)
    }

    pub async fn list(
        &self,
        platform: Uuid,
        volume: &str,
        path: &str,
        node: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<VolumeDirectory, RuntimeCapabilityError> {
        normalize_path(Some(path))?;
        let work = async {
            let helper = self.open(platform, volume, node, cancellation).await?;
            let frames = helper.exec("list", path).await?;
            let response: Listing = metadata(frames, cancellation).await?;
            helper.close().await;
            check_helper_error(response.error_code.as_deref())?;
            if response.path != path || response.entries.len() > DIRECTORY_ENTRY_LIMIT {
                return Err(error(
                    RuntimeErrorKind::Remote,
                    "Invalid Volume directory response.",
                ));
            }
            for entry in &response.entries {
                if entry.name.is_empty()
                    || entry.name.contains('/')
                    || entry.name.contains('\\')
                    || normalize_path(Some(&entry.path)).is_err()
                    || entry.path != format!("{}/{}", path.trim_end_matches('/'), entry.name)
                {
                    return Err(error(
                        RuntimeErrorKind::Remote,
                        "Invalid Volume entry path.",
                    ));
                }
            }
            let mut entries = response.entries;
            entries.sort_by_cached_key(|entry| {
                (
                    entry.entry_type != VolumeEntryType::Directory,
                    entry.name.to_lowercase(),
                )
            });
            Ok(VolumeDirectory {
                platform_id: platform,
                volume_name: volume.into(),
                path: path.into(),
                entries,
                is_truncated: response.is_truncated,
            })
        };
        tokio::select! { biased; ()=cancellation.cancelled()=>Err(canceled()), result=tokio::time::timeout(Duration::from_secs(30), work)=>result.map_err(|_| canceled())? }
    }

    pub async fn download(
        &self,
        platform: Uuid,
        volume: &str,
        path: &str,
        node: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<VolumeDownload, RuntimeCapabilityError> {
        normalize_path(Some(path))?;
        if path == "/" {
            return Err(error(
                RuntimeErrorKind::InvalidRequest,
                "Select a file or directory to download, not the Volume root.",
            ));
        }
        let prepare = async {
            let helper = self.open(platform, volume, node, cancellation).await?;
            let inspected: Inspection =
                metadata(helper.exec("inspect", path).await?, cancellation).await?;
            check_helper_error(inspected.error_code.as_deref())?;
            if !inspected.exists {
                return Err(error(RuntimeErrorKind::NotFound, "Volume path not found."));
            }
            let directory = match inspected.entry_type {
                VolumeEntryType::Directory => true,
                VolumeEntryType::File => false,
                _ => {
                    return Err(error(
                        RuntimeErrorKind::InvalidRequest,
                        "Only regular files and directories can be downloaded; symlinks cannot be followed.",
                    ));
                }
            };
            let name = path.rsplit('/').next().expect("validated non-root path");
            let filename = if directory {
                format!("{name}.tar")
            } else {
                name.to_owned()
            };
            let frames = helper
                .exec(
                    if directory {
                        "stream-directory"
                    } else {
                        "stream-file"
                    },
                    path,
                )
                .await?;
            Ok::<_, RuntimeCapabilityError>((helper, frames, filename, directory))
        };
        let (helper, mut frames, filename, directory) = tokio::select! {
            biased; ()=cancellation.cancelled()=>return Err(canceled()),
            result=tokio::time::timeout(Duration::from_secs(30),prepare)=>result.map_err(|_| canceled())??
        };
        let deadline = tokio::time::Instant::now() + DOWNLOAD_TIMEOUT;
        let cancellation = cancellation.clone();
        let stream = Box::pin(async_stream::try_stream! {
            let helper = helper;
            let mut exit = None;
            let mut error_bytes = 0usize;
            loop {
                let frame = tokio::select! {
                    biased;
                    ()=cancellation.cancelled()=>Err(std::io::Error::other("Volume download canceled.")),
                    result=tokio::time::timeout_at(deadline,frames.next())=>result.map_err(|_| std::io::Error::other("Volume download timed out."))
                }?;
                let Some(frame) = frame else { break; };
                match frame.map_err(|_| std::io::Error::other("Volume download interrupted."))?.msg {
                    Some(Msg::Output(output)) => {
                        if exit.is_some() || output.data.len() > DIRECTORY_PAYLOAD_LIMIT { Err(std::io::Error::other("Invalid Volume download frame."))?; }
                        match output.stream {
                            0 => yield output.data,
                            1 => { error_bytes = error_bytes.saturating_add(output.data.len()); if error_bytes > 16 * 1024 { Err(std::io::Error::other("Volume helper failed."))?; } },
                            _ => Err(std::io::Error::other("Invalid Volume download stream."))?,
                        }
                    },
                    Some(Msg::Exit(value)) if exit.is_none() => exit = Some(value.exit_code),
                    _ => Err(std::io::Error::other("Volume helper failed."))?,
                }
            }
            if exit != Some(0) { Err(std::io::Error::other("Volume download did not complete successfully."))?; }
            helper.close().await;
        });
        Ok(VolumeDownload {
            filename,
            directory,
            stream,
        })
    }
}

impl Helper {
    async fn exec(&self, operation: &str, path: &str) -> Result<Frames, RuntimeCapabilityError> {
        let mut cmd: Vec<String> = [
            self.binary,
            "volume-helper",
            operation,
            "--root",
            "/data",
            "--path",
            path,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        if operation == "list" {
            cmd.extend(
                ["--max-entries", "1000", "--max-payload-bytes", "1048576"].map(str::to_owned),
            );
        }
        let request = ExecBinaryRequest {
            container_id: self.id.clone(),
            cmd,
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: false,
            env: Default::default(),
        };
        match &self.runtime {
            HelperRuntime::Local(r) => {
                r.exec_binary_stream(&self.id, &request.cmd, &self.cancellation)
                    .await
            }
            HelperRuntime::Agent(AgentExecutionClient::Direct(r)) => {
                r.exec_binary_stream(request, &self.cancellation).await
            }
            HelperRuntime::Agent(AgentExecutionClient::Edge(r)) => {
                crate::connectors::agent::execution::stream(
                    r,
                    EdgeCommandKind::ContainerExecBinary,
                    request,
                    &self.cancellation,
                )
            }
        }
    }

    async fn close(mut self) {
        if let Some(permit) = self.permit.take() {
            let runtime = self.runtime.clone();
            let id = self.id.clone();
            let (completed, completion) = tokio::sync::oneshot::channel();
            self.cleanup
                .take()
                .expect("helper owns its cleanup registration")
                .spawn(async move {
                    cleanup(runtime, id, permit).await;
                    let _ = completed.send(());
                });
            // Cancellation while awaiting close must not cancel cleanup itself.
            let _ = completion.await;
        }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        if let Some(permit) = self.permit.take() {
            let runtime = self.runtime.clone();
            let id = self.id.clone();
            self.cleanup
                .take()
                .expect("helper owns its cleanup registration")
                .spawn(cleanup(runtime, id, permit));
        }
    }
}

async fn cleanup(runtime: HelperRuntime, id: String, _permit: OwnedSemaphorePermit) {
    let cancellation = CancellationToken::new();
    let guard = cancellation.clone().drop_guard();
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        match runtime {
            HelperRuntime::Local(r) => r
                .delete_container_with_options(&id, false, true, false)
                .await
                .map_err(crate::connectors::docker::runtime::normalize_docker_error),
            HelperRuntime::Agent(AgentExecutionClient::Direct(r)) => {
                r.delete_container_with_options(
                    &id,
                    citadel_platforms::containers::DeleteContainerOptions {
                        force: true,
                        ..Default::default()
                    },
                    &cancellation,
                )
                .await
            }
            HelperRuntime::Agent(AgentExecutionClient::Edge(r)) => {
                crate::connectors::agent::execution::unary(
                    &r,
                    EdgeCommandKind::ContainerDelete,
                    citadel_contracts::citadel::containers::v1::DeleteContainerRequest {
                        ids: vec![id],
                        v: Some(false),
                        force: Some(true),
                        link: Some(false),
                    },
                    &cancellation,
                )
                .await
            }
        }
    })
    .await;
    if !matches!(result, Ok(Ok(()))) {
        tracing::warn!("Volume helper cleanup could not be confirmed on its owning node.");
    }
    drop(guard);
}

fn helper_startup_error(message: String) -> RuntimeCapabilityError {
    error(
        RuntimeErrorKind::Unavailable,
        &format!("Volume browser could not start: {message}"),
    )
}

fn core_image(document: &serde_json::Value) -> Result<String, RuntimeCapabilityError> {
    if document
        .pointer("/Config/Labels/com.citadel.system-role")
        .and_then(serde_json::Value::as_str)
        != Some("core")
    {
        return Err(helper_startup_error(
            "The running Core container could not be identified.".into(),
        ));
    }
    document
        .get("Image")
        .and_then(serde_json::Value::as_str)
        .filter(|image| {
            image.starts_with("sha256:")
                && image.len() == 71
                && image[7..].bytes().all(|b| b.is_ascii_hexdigit())
        })
        .map(str::to_owned)
        .ok_or_else(|| helper_startup_error("The running Core image could not be resolved.".into()))
}

fn helper_request(
    platform: Uuid,
    volume: &str,
    name: &str,
    image: &str,
    binary: &str,
) -> CreateContainerRequest {
    CreateContainerRequest {
        image_id: image.into(),
        name: name.into(),
        user: Some("0".into()),
        auto_remove: Some(true),
        memory_limit: Some(128 * 1024 * 1024),
        memory_swap: Some(128 * 1024 * 1024),
        pids_limit: Some(64),
        privileged: Some(false),
        readonly_rootfs: Some(true),
        network_mode: Some("none".into()),
        labels: [
            ("citadel.volume-browser", "true".into()),
            ("com.citadel.system", "true".into()),
            ("com.citadel.system-role", "volume-helper".into()),
            ("com.citadel.platform-id", platform.to_string()),
            (
                "com.citadel.helper-expires-at",
                (chrono::Utc::now().timestamp() + 1920).to_string(),
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect(),
        entry_point: vec![binary.into()],
        command: vec!["volume-helper".into(), "idle".into()],
        mounts: vec![Mount {
            target: Some("/data".into()),
            source: Some(volume.into()),
            r#type: Some("volume".into()),
            read_only: Some(true),
            ..Default::default()
        }],
        cap_add: vec!["DAC_READ_SEARCH".into()],
        cap_drop: vec!["ALL".into()],
        security_opt: vec!["no-new-privileges".into()],
        ..Default::default()
    }
}

fn local_request(request: &CreateContainerRequest) -> serde_json::Value {
    serde_json::json!({"Image":request.image_id,"User":"0","Labels":request.labels,"Entrypoint":request.entry_point,"Cmd":request.command,
        "Healthcheck":{"Test":["NONE"]},
        "HostConfig":{"AutoRemove":true,"ReadonlyRootfs":true,"NetworkMode":"none","Privileged":false,"Memory":request.memory_limit,"MemorySwap":request.memory_swap,"PidsLimit":request.pids_limit,
        "CapAdd":request.cap_add,"CapDrop":request.cap_drop,"SecurityOpt":request.security_opt,
        "Mounts":[{"Type":"volume","Source":request.mounts[0].source,"Target":"/data","ReadOnly":true}]}})
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Listing {
    path: String,
    entries: Vec<VolumeFileEntry>,
    is_truncated: bool,
    error_code: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspection {
    exists: bool,
    #[serde(rename = "Type")]
    entry_type: VolumeEntryType,
    error_code: Option<String>,
}

async fn metadata<T: serde::de::DeserializeOwned>(
    mut frames: Frames,
    cancellation: &CancellationToken,
) -> Result<T, RuntimeCapabilityError> {
    let mut output = Vec::new();
    let mut total = 0usize;
    let mut exit = None;
    loop {
        let frame = tokio::select! { biased; ()=cancellation.cancelled()=>return Err(canceled()), frame=frames.next()=>frame };
        let Some(frame) = frame else {
            break;
        };
        match frame
            .map_err(|_| error(RuntimeErrorKind::Remote, "Volume helper stream failed."))?
            .msg
        {
            Some(Msg::Output(value)) if exit.is_none() => {
                total = total.saturating_add(value.data.len());
                if total > DIRECTORY_PAYLOAD_LIMIT {
                    return Err(error(
                        RuntimeErrorKind::ResourceExhausted,
                        "Volume directory response is too large.",
                    ));
                }
                if value.stream == 0 {
                    output.extend_from_slice(&value.data);
                } else if value.stream != 1 {
                    return Err(error(
                        RuntimeErrorKind::Remote,
                        "Invalid Volume helper stream.",
                    ));
                }
            }
            Some(Msg::Exit(value)) if exit.is_none() => exit = Some(value.exit_code),
            _ => return Err(error(RuntimeErrorKind::Remote, "Volume helper failed.")),
        }
    }
    if exit != Some(0) {
        return Err(error(
            RuntimeErrorKind::Remote,
            "Volume helper did not complete successfully.",
        ));
    }
    serde_json::from_slice(&output)
        .map_err(|_| error(RuntimeErrorKind::Remote, "Invalid Volume helper response."))
}

fn check_helper_error(code: Option<&str>) -> Result<(), RuntimeCapabilityError> {
    match code {
        None | Some("") => Ok(()),
        Some("VolumePathNotFound") => {
            Err(error(RuntimeErrorKind::NotFound, "Volume path not found."))
        }
        Some("VolumeDirectoryListingTooLarge") => Err(error(
            RuntimeErrorKind::ResourceExhausted,
            "Volume directory response is too large.",
        )),
        Some("VolumePathIsSymlink" | "VolumePathIsNotDirectory" | "VolumePathInvalid") => {
            Err(error(
                RuntimeErrorKind::InvalidRequest,
                "Volume path is invalid or is a symlink.",
            ))
        }
        _ => Err(error(
            RuntimeErrorKind::Remote,
            "Could not read the Volume path.",
        )),
    }
}
fn canceled() -> RuntimeCapabilityError {
    error(
        RuntimeErrorKind::Timeout,
        "Volume content operation canceled or timed out.",
    )
}
fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}

#[cfg(test)]
mod tests;
