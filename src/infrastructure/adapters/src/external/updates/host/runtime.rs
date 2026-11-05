use super::super::{
    docker::{LocalDocker, failed, single},
    files::validate_absolute,
};
use super::{HostUpdateRequest, HostUpdateResponse};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, is_sha256, official_image_digest};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct HostUpdateClient {
    process: Arc<dyn ProcessRunner>,
    state_directory: PathBuf,
}

impl HostUpdateClient {
    pub fn new(process: Arc<dyn ProcessRunner>, state_directory: PathBuf) -> Self {
        Self {
            process,
            state_directory,
        }
    }

    /// The installed image executes this bounded command. Host paths come from
    /// server configuration and the current container's Compose labels, never HTTP input.
    pub async fn exchange(
        &self,
        request: HostUpdateRequest,
        cancellation: &CancellationToken,
    ) -> Result<(HostUpdateResponse, String, String), UpdateError> {
        let hostname =
            std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
        self.exchange_from(request, cancellation, hostname.trim())
            .await
    }

    pub(super) async fn exchange_from(
        &self,
        request: HostUpdateRequest,
        cancellation: &CancellationToken,
        hostname: &str,
    ) -> Result<(HostUpdateResponse, String, String), UpdateError> {
        validate_absolute(&self.state_directory)?;
        let requested_state = match &request {
            HostUpdateRequest::Startup { state_directory }
            | HostUpdateRequest::Check { state_directory } => state_directory.as_path(),
            HostUpdateRequest::CheckDatabase(plan) => {
                std::path::Path::new(&plan.checkpoint_directory)
            }
            HostUpdateRequest::Launch(operation)
            | HostUpdateRequest::Completed(operation)
            | HostUpdateRequest::FailedJournal(operation)
            | HostUpdateRequest::RecoverSource(operation) => {
                std::path::Path::new(&operation.plan.checkpoint_directory)
            }
            HostUpdateRequest::Finalize(request) => {
                request.intent.registration.state_directory.as_path()
            }
        };
        if requested_state != self.state_directory {
            return Err(failed());
        }
        let home = tempfile::Builder::new()
            .prefix("citadel-host-update-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .map_err(|_| failed())?;
        let docker = LocalDocker {
            process: self.process.as_ref(),
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(240),
        };
        if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(failed());
        }
        let raw = docker.json(&["container", "inspect", hostname]).await?;
        let core = single(&raw)?;
        let id = core["Id"]
            .as_str()
            .filter(|id| is_sha256(id) && id.starts_with(hostname))
            .ok_or_else(failed)?;
        if core["Config"]["Hostname"] != hostname || core["State"]["Running"] != true {
            return Err(failed());
        }
        let directory = core["Config"]["Labels"]["com.docker.compose.project.working_dir"]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(failed)?;
        validate_absolute(&directory)?;
        if directory == self.state_directory || directory == std::path::Path::new("/") {
            return Err(failed());
        }
        let image_id = core["Image"]
            .as_str()
            .filter(|id| id.strip_prefix("sha256:").is_some_and(is_sha256))
            .ok_or_else(failed)?;
        let raw = docker.json(&["image", "inspect", image_id]).await?;
        let image = single(&raw)?;
        let digest = image["RepoDigests"]
            .as_array()
            .and_then(|digests| {
                digests
                    .iter()
                    .filter_map(|v| v.as_str())
                    .find(|s| official_image_digest(s))
            })
            .ok_or_else(failed)?
            .to_owned();
        let architecture = image["Architecture"]
            .as_str()
            .filter(|s| matches!(*s, "amd64" | "arm64"))
            .ok_or_else(failed)?
            .to_owned();
        if image["Os"] != "linux" {
            return Err(failed());
        }
        let nonce = Uuid::now_v7().to_string();
        let name = format!("citadel-update-host-{nonce}");
        let label = format!("com.citadel.update-host={nonce}");
        let mut args = vec![
            "container".into(),
            "create".into(),
            "--interactive".into(),
            "--name".into(),
            name.clone(),
            "--label".into(),
            label,
            "--pull=never".into(),
            "--network=none".into(),
            "--read-only".into(),
            "--restart=no".into(),
            "--no-healthcheck".into(),
            "--user=0:0".into(),
            "--env".into(),
            "TMPDIR=/tmp".into(),
            "--tmpfs=/tmp:rw,noexec,nosuid,size=64m".into(),
            "--entrypoint=/app/citadel-server".into(),
        ];
        for (path, readonly) in [
            (directory.as_path(), true),
            (self.state_directory.as_path(), false),
            (std::path::Path::new("/var/run/docker.sock"), false),
        ] {
            let path = path.to_str().ok_or_else(failed)?.replace('"', "\"\"");
            args.extend([
                "--mount".into(),
                format!(
                    "type=bind,\"source={path}\",\"destination={path}\"{}",
                    if readonly { ",readonly" } else { "" }
                ),
            ]);
        }
        args.extend([image_id.to_owned(), "update-host".into()]);
        let cleanup_token = CancellationToken::new();
        let cleanup = LocalDocker {
            process: self.process.as_ref(),
            cancellation: &cleanup_token,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(300),
        };
        // Finish bounded creation even on cancellation so cleanup owns the ID.
        let created = cleanup
            .run(&args.iter().map(String::as_str).collect::<Vec<_>>())
            .await?;
        let executor = std::str::from_utf8(&created)
            .map(str::trim)
            .map_err(|_| failed())?;
        if !is_sha256(executor) || executor == id {
            return Err(failed());
        }
        let profile = |actual: &serde_json::Value| -> Result<(), UpdateError> {
            super::super::bootstrap::validate_executor(
                actual,
                &executor[..12],
                &super::super::InstallerComposeInputs {
                    directory: directory.clone(),
                    project: String::new(),
                    compose_files: vec![],
                },
                &self.state_directory,
            )?;
            if actual["Id"] != executor
                || actual["Name"] != format!("/{name}")
                || actual["Image"] != image_id
                || actual["Config"]["Labels"]["com.citadel.update-host"] != nonce
                || actual["Path"] != "/app/citadel-server"
                || actual["Args"] != serde_json::json!(["update-host"])
                || actual["Config"]["User"] != "0:0"
                || actual["HostConfig"]["NetworkMode"] != "none"
                || actual["HostConfig"]["ReadonlyRootfs"] != true
                || actual["HostConfig"]["Privileged"] != false
                || actual["HostConfig"]["RestartPolicy"]["Name"] != "no"
                || actual["RestartCount"] != 0
            {
                return Err(failed());
            }
            Ok(())
        };
        let result = async {
            let raw = docker.json(&["container", "inspect", executor]).await?;
            profile(single(&raw)?)?;
            let bytes = docker
                .run_with_input(
                    &["container", "start", "--attach", "--interactive", executor],
                    serde_json::to_vec(&request).map_err(|_| failed())?,
                    Duration::from_secs(180),
                )
                .await?;
            let raw = docker.json(&["container", "inspect", executor]).await?;
            let actual = single(&raw)?;
            if actual["State"]["Running"] != false
                || actual["State"]["ExitCode"] != 0
                || actual["State"]["OOMKilled"] != false
                || actual["State"]["Error"] != ""
            {
                return Err(failed());
            }
            serde_json::from_slice(&bytes).map_err(|_| failed())
        }
        .await;
        let raw = cleanup.json(&["container", "inspect", executor]).await?;
        let actual = single(&raw)?;
        profile(actual)?;
        cleanup
            .run(&["container", "rm", "--force", executor])
            .await?;
        result.map(|response| (response, digest, architecture))
    }
}
