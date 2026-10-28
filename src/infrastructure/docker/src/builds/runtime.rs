//! Docker build and push execution without Git, database, or resource routing dependencies.
use super::output;
use crate::client::DockerEndpoint;
use citadel_builds::{BuildArgSpec, BuildLogSink, BuildRegistryCredentials};
use citadel_execution::{OutputLimitPolicy, ProcessLimits};
use citadel_execution::{ProcessError, ProcessRequest};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum BuildRuntimeError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Command(String),
    /// Daemon terminal evidence was observed; temporary files still require cleanup.
    #[error(transparent)]
    Finished(Box<BuildRuntimeError>),
    #[error(transparent)]
    Process(ProcessError),
    /// The build/push subprocess never started. Temporary files still require cleanup.
    #[error(transparent)]
    NotStarted(Box<BuildRuntimeError>),
    #[error("Could not clean temporary Docker credentials and build files.")]
    Cleanup,
}
impl BuildRuntimeError {
    pub fn execution_finished(&self) -> bool {
        matches!(self, Self::NotStarted(_) | Self::Finished(_))
    }

    pub fn is_known_not_started(&self) -> bool {
        matches!(self, Self::NotStarted(_))
    }

    pub fn not_started(self) -> Self {
        match self {
            // Missing cleanup evidence always takes precedence over non-execution.
            Self::Cleanup | Self::NotStarted(_) | Self::Finished(_) => self,
            error => Self::NotStarted(Box::new(error)),
        }
    }

    pub fn cause(&self) -> &Self {
        match self {
            Self::NotStarted(error) | Self::Finished(error) => error.cause(),
            error => error,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotStarted(error) | Self::Finished(error) => error.code(),
            Self::Cleanup => "build.cleanup",
            Self::Validation(_) => "build.validation",
            Self::Io(_) => "build.source",
            Self::Command(_) => "build.command",
            Self::Process(ProcessError::Cancelled) => "build.cancelled",
            Self::Process(ProcessError::Timeout(_)) => "build.timeout",
            Self::Process(_) => "build.process",
        }
    }
}

pub enum DockerBuildSource<'a> {
    Directory {
        context: &'a Path,
        dockerfile: &'a Path,
    },
    /// Pass the tar directly to Docker; never unpack caller archives on the host.
    Archive { data: Vec<u8>, dockerfile: &'a str },
}

pub struct DockerBuildOptions<'a> {
    pub source: DockerBuildSource<'a>,
    pub tags: &'a [String],
    pub build_args: &'a [BuildArgSpec],
    pub target: Option<&'a str>,
    pub secrets: &'a [(String, Zeroizing<String>)],
}

pub struct DockerBuildSession {
    docker: OsString,
    endpoint: DockerEndpoint,
    config: PathBuf,
    timeout: Duration,
    maximum_log_bytes: usize,
    password: Option<Zeroizing<String>>,
    registry_auth: Option<Zeroizing<String>>,
    build_name: String,
}

impl DockerBuildSession {
    pub async fn open(
        docker: impl Into<OsString>,
        endpoint: DockerEndpoint,
        registry: Option<(&str, &BuildRegistryCredentials)>,
        timeout: Duration,
        maximum_log_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<Self, BuildRuntimeError> {
        let result = async {
            if timeout.is_zero() || maximum_log_bytes < 1024 {
                return Err(BuildRuntimeError::Validation(
                    "Invalid Docker execution limits.".into(),
                ));
            }
            endpoint
                .docker_host()
                .map_err(|error| BuildRuntimeError::Validation(error.to_string()))?;
            let config =
                std::env::temp_dir().join(format!("citadel-build-config-{}", uuid::Uuid::now_v7()));
            let mut directory = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                directory.mode(0o700);
            }
            directory
                .create(&config)
                .map_err(|error| BuildRuntimeError::Io(error.to_string()))?;
            let mut session = Self {
                docker: docker.into(),
                endpoint,
                config,
                timeout,
                maximum_log_bytes,
                password: registry.map(|(_, credentials)| credentials.password.clone()),
                registry_auth: registry.map(|(host, credentials)| encode_registry_auth(serde_json::json!({"username": credentials.username, "password": credentials.password.as_str(), "serveraddress": host}))),
                build_name: format!("citadel-{}", uuid::Uuid::now_v7()),
            };
            let authentication = async {
                if let Some((host, credentials)) = registry {
                    let request = session
                        .request(vec![
                            "login".into(),
                            normalize_registry_host(host)?.into(),
                            "--username".into(),
                            credentials.username.clone().into(),
                            "--password-stdin".into(),
                        ])?
                        .stdin(credentials.password.as_bytes().to_vec())
                        .limits(limits(Duration::from_secs(60), maximum_log_bytes));
                    let login = citadel_processes::run(request, cancellation)
                        .await
                        .map_err(BuildRuntimeError::Process)?;
                    if !login.succeeded() {
                        return Err(BuildRuntimeError::Command(redact_values(
                            &logs(&login.stdout, &login.stderr),
                            &[credentials.password.as_str()],
                        )));
                    }
                }
                Ok::<(), BuildRuntimeError>(())
            }
            .await;
            if let Err(error) = authentication {
                session.cleanup()?;
                return Err(error);
            }
            Ok(session)
        }
        .await;
        result.map_err(BuildRuntimeError::not_started)
    }

    /// Confirm private credentials and temporary build files have been removed.
    /// Call before releasing execution capacity; Drop remains a fallback on cancellation.
    pub fn cleanup(&mut self) -> Result<(), BuildRuntimeError> {
        match std::fs::remove_dir_all(&self.config) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(BuildRuntimeError::Cleanup),
        }
    }

    /// Install a registry identity token in this session's private Docker config.
    pub fn set_identity_token(
        &mut self,
        host: &str,
        token: Zeroizing<String>,
    ) -> Result<(), BuildRuntimeError> {
        use std::io::Write;
        let host = normalize_registry_host(host)?;
        let host = if host == "docker.io" {
            "https://index.docker.io/v1/"
        } else {
            host
        };
        let config = serde_json::json!({"auths": {host: {"identitytoken": token.as_str()}}});
        let bytes = Zeroizing::new(
            serde_json::to_vec(&config)
                .map_err(|error| BuildRuntimeError::Io(error.to_string()))?,
        );
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(self.config.join("config.json"))
            .map_err(|error| BuildRuntimeError::Io(error.to_string()))?;
        file.write_all(&bytes)
            .map_err(|error| BuildRuntimeError::Io(error.to_string()))?;
        self.registry_auth = Some(encode_registry_auth(
            serde_json::json!({"identitytoken": token.as_str(), "serveraddress": host}),
        ));
        self.password = Some(token);
        Ok(())
    }

    fn request(&self, args: Vec<OsString>) -> Result<ProcessRequest, BuildRuntimeError> {
        Ok(ProcessRequest::new(self.docker.clone())
            .args(args)
            .env(
                "DOCKER_HOST",
                self.endpoint
                    .docker_host()
                    .map_err(|error| BuildRuntimeError::Validation(error.to_string()))?,
            )
            .env("DOCKER_CONTEXT", "")
            .env("DOCKER_TLS_VERIFY", "")
            .env("DOCKER_CERT_PATH", "")
            .env("DOCKER_CONFIG", self.config.as_os_str())
            .env("HOME", self.config.as_os_str())
            .env("TMPDIR", self.config.as_os_str())
            .limits(limits(self.timeout, self.maximum_log_bytes)))
    }

    pub async fn build(
        &self,
        options: DockerBuildOptions<'_>,
        progress: &dyn BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<(), BuildRuntimeError> {
        let mut started = false;
        let result = async {
            let (context, dockerfile, stdin) = match options.source {
                DockerBuildSource::Directory {
                    context,
                    dockerfile,
                } => (
                    tokio::fs::canonicalize(context)
                        .await
                        .map_err(|error| BuildRuntimeError::Io(error.to_string()))?
                        .into_os_string(),
                    tokio::fs::canonicalize(dockerfile)
                        .await
                        .map_err(|error| BuildRuntimeError::Io(error.to_string()))?
                        .into_os_string(),
                    None,
                ),
                DockerBuildSource::Archive { data, dockerfile } => {
                    if data.is_empty()
                        || data.len() > 16 * 1024 * 1024
                        || dockerfile.is_empty()
                        || Path::new(dockerfile).is_absolute()
                        || dockerfile
                            .split(['/', '\\'])
                            .any(|part| matches!(part, ".." | ".git"))
                    {
                        return Err(BuildRuntimeError::Validation(
                            "Invalid transported Build context or Dockerfile path.".into(),
                        ));
                    }
                    (OsString::from("-"), OsString::from(dockerfile), Some(data))
                }
            };
            let mut args = vec![OsString::from("build"), "--file".into(), dockerfile];
            if let Some(target) = options.target {
                args.extend(["--target".into(), target.into()]);
            }
            for argument in options.build_args {
                args.extend([
                    "--build-arg".into(),
                    match &argument.value {
                        Some(value) => format!("{}={value}", argument.name),
                        None => argument.name.clone(),
                    }
                    .into(),
                ]);
            }
            for tag in options.tags {
                args.extend(["--tag".into(), tag.into()]);
            }
            let mut environment = Vec::new();
            for (index, (id, value)) in options.secrets.iter().enumerate() {
                if id.is_empty()
                    || !id
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
                {
                    return Err(BuildRuntimeError::Validation(
                        "Invalid Build secret identifier.".into(),
                    ));
                }
                let name = format!("CITADEL_BUILDKIT_SECRET_{index}");
                args.extend(["--secret".into(), format!("id={id},env={name}").into()]);
                environment.push((name, value.as_str()));
            }
            // An unguessable, reserved name lets us match authoritative BuildKit
            // history after a CLI failure without interpreting its error text.
            args.extend([
                "--build-arg".into(),
                format!("BUILDKIT_BUILD_NAME={}", self.build_name).into(),
            ]);
            args.push(context);
            let mut request = self.request(args)?.env("DOCKER_BUILDKIT", "1");
            if let Some(stdin) = stdin {
                request = request.stdin(stdin);
            }
            for (name, value) in &environment {
                request = request.env(name, *value);
            }
            let secrets: Vec<_> = environment
                .iter()
                .map(|(_, value)| *value)
                .chain(self.password.as_ref().map(|value| value.as_str()))
                .collect();
            started = true;
            let result = output::run(request, cancellation, progress, &secrets).await?;
            if !result.succeeded() {
                let error = streamed_command_failure("build", result.exit_code);
                return Err(if self.build_finished().await {
                    BuildRuntimeError::Finished(Box::new(error))
                } else {
                    error
                });
            }
            Ok(())
        }
        .await;
        result.map_err(|error| {
            if !started || matches!(error, BuildRuntimeError::Process(ProcessError::Spawn(_))) {
                error.not_started()
            } else {
                error
            }
        })
    }
    async fn build_finished(&self) -> bool {
        // Unsupported history, truncation, a missing record or a failed probe is
        // not cleanup evidence. Each probe has bounded output and wall time.
        let probe = async {
            let request = self
                .request(vec![
                    "buildx".into(),
                    "history".into(),
                    "ls".into(),
                    "--no-trunc".into(),
                    "--format".into(),
                    // Emit only this operation, so unrelated retained history
                    // cannot exhaust the bounded probe output.
                    format!(
                        "{{{{if eq .Name \"{}\"}}}}{{{{json .}}}}{{{{end}}}}",
                        self.build_name
                    )
                    .into(),
                ])
                .ok()?
                .limits(limits(Duration::from_secs(5), 64 * 1024));
            let output = citadel_processes::run(request, &CancellationToken::new())
                .await
                .ok()?;
            if !output.succeeded() || output.stdout_truncated {
                return None;
            }
            let mut matched = None;
            for (index, line) in output
                .stdout
                .split(|b| *b == b'\n')
                .filter(|line| !line.is_empty())
                .enumerate()
            {
                if index >= 128 {
                    return None;
                }
                let row: serde_json::Value = serde_json::from_slice(line).ok()?;
                if row
                    .get("Name")
                    .or_else(|| row.get("name"))
                    .and_then(|v| v.as_str())
                    == Some(&self.build_name)
                {
                    if matched.is_some() {
                        return None;
                    }
                    matched = row
                        .get("Ref")
                        .or_else(|| row.get("ref"))
                        .and_then(|v| v.as_str())
                        .map(str::to_owned);
                }
            }
            let full_reference = matched?;
            let reference = full_reference.rsplit('/').next()?;
            if reference.is_empty() || reference.starts_with('-') || reference.len() > 512 {
                return None;
            }
            let request = self
                .request(vec![
                    "buildx".into(),
                    "history".into(),
                    "inspect".into(),
                    "--format".into(),
                    "json".into(),
                    reference.into(),
                ])
                .ok()?
                .limits(limits(Duration::from_secs(5), 64 * 1024));
            let output = citadel_processes::run(request, &CancellationToken::new())
                .await
                .ok()?;
            if !output.succeeded() || output.stdout_truncated {
                return None;
            }
            let record: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
            (record["Name"].as_str() == Some(self.build_name.as_str())
                && record["CompletedAt"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
                && matches!(
                    record["Status"].as_str(),
                    Some("completed" | "failed" | "canceled")
                ))
            .then_some(())
        };
        matches!(
            tokio::time::timeout(Duration::from_secs(11), probe).await,
            Ok(Some(()))
        )
    }

    pub async fn push(
        &self,
        reference: &str,
        progress: &dyn BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<Option<String>, BuildRuntimeError> {
        use futures_util::StreamExt;
        if reference.trim().is_empty() || reference.starts_with('-') {
            return Err(
                BuildRuntimeError::Validation("Invalid image reference.".into()).not_started(),
            );
        }
        let client = crate::client::DockerClient::with_endpoint(
            self.endpoint.clone(),
            self.timeout,
            "/host",
        )
        .map_err(|error| BuildRuntimeError::Validation(error.to_string()).not_started())?;
        let execution = async {
            let mut stream = client
                .push_image(reference, self.registry_auth.as_deref().map(|v| v.as_str()))
                .await
                .map_err(|error| BuildRuntimeError::Command(error.to_string()))?;
            let mut failure = None;
            let mut digest = None;
            let mut remaining_log_bytes = self.maximum_log_bytes;
            while let Some(message) = stream.next().await {
                let message =
                    message.map_err(|error| BuildRuntimeError::Command(error.to_string()))?;
                if let Some(error) = message.error_detail.map(|v| v.message).or(message.error) {
                    failure = Some(redact_values(
                        &error,
                        &self
                            .password
                            .as_ref()
                            .map(|p| p.as_str())
                            .into_iter()
                            .collect::<Vec<_>>(),
                    ));
                }
                if let Some(value) = message
                    .aux
                    .and_then(|v| v.get("Digest").and_then(|d| d.as_str()).map(str::to_owned))
                    && value.len() == 71
                    && value.starts_with("sha256:")
                    && value[7..].bytes().all(|b| b.is_ascii_hexdigit())
                {
                    digest = Some(value);
                }
                // Docker's containerd image store sends the terminal digest in
                // the status field; the classic store can send an aux document.
                // Require the exact requested tag and a complete HTTP stream.
                if digest.is_none() {
                    digest = message
                        .status
                        .as_deref()
                        .and_then(|status| push_status_digest(reference, status));
                }
                if let Some(status) = message.status
                    && remaining_log_bytes > 0
                {
                    let mut message = redact_values(
                        &format!("{status}\n"),
                        &self
                            .password
                            .as_ref()
                            .map(|p| p.as_str())
                            .into_iter()
                            .collect::<Vec<_>>(),
                    );
                    message.truncate(message.floor_char_boundary(remaining_log_bytes));
                    remaining_log_bytes -= message.len();
                    progress.append("stdout", &message).await.map_err(|_| {
                        BuildRuntimeError::Validation("Build log persistence failed.".into())
                    })?;
                }
            }
            // A complete daemon error stream is terminal evidence. Transport
            // loss or EOF without an error/digest leaves execution uncertain.
            if let Some(mut error) = failure {
                if remaining_log_bytes > 0 {
                    error.truncate(error.floor_char_boundary(remaining_log_bytes - 1));
                    progress
                        .append("stderr", &format!("{error}\n"))
                        .await
                        .map_err(|_| {
                            BuildRuntimeError::Validation("Build log persistence failed.".into())
                        })?;
                }
                return Err(BuildRuntimeError::Finished(Box::new(
                    BuildRuntimeError::Command(
                        "Docker push failed. See the build output for details.".into(),
                    ),
                )));
            }
            digest.map(Some).ok_or_else(|| {
                BuildRuntimeError::Command("Docker push ended without terminal evidence.".into())
            })
        };
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(BuildRuntimeError::Process(ProcessError::Cancelled)),
            result = tokio::time::timeout(self.timeout, execution) => result.unwrap_or(Err(BuildRuntimeError::Process(ProcessError::Timeout(self.timeout)))),
        }
    }
}

fn push_status_digest(reference: &str, status: &str) -> Option<String> {
    let tag = reference
        .rsplit_once(':')
        .filter(|(_, tag)| !tag.contains('/'))
        .map_or("latest", |(_, tag)| tag);
    let summary = status.strip_prefix(&format!("{tag}: digest: "))?;
    let (digest, size) = summary.split_once(" size: ")?;
    (digest.len() == 71
        && digest.starts_with("sha256:")
        && digest[7..].bytes().all(|b| b.is_ascii_hexdigit())
        && size.parse::<u64>().is_ok())
    .then(|| digest.to_owned())
}

fn streamed_command_failure(operation: &str, exit_code: Option<i32>) -> BuildRuntimeError {
    // output::run already delivered the command output to the log sink. Returning
    // it again replays the entire transcript through the Agent's error frame and
    // the executor's final stderr log (and duplicates local build output too).
    let status = match exit_code {
        Some(code) => format!("with exit code {code}"),
        None => "without an exit code".into(),
    };
    BuildRuntimeError::Command(format!(
        "Docker {operation} failed {status}. See the build output for details."
    ))
}

impl Drop for DockerBuildSession {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            tracing::warn!(%error,"Failed to clean temporary Docker credentials");
        }
    }
}

pub fn limits(timeout: Duration, maximum: usize) -> ProcessLimits {
    ProcessLimits {
        timeout,
        maximum_stdout_bytes: maximum,
        maximum_stderr_bytes: maximum,
        output_limit_policy: OutputLimitPolicy::Truncate,
    }
}

pub fn normalize_registry_host(value: &str) -> Result<&str, BuildRuntimeError> {
    if matches!(
        value.trim_end_matches('/'),
        "docker.io" | "index.docker.io" | "registry-1.docker.io" | "https://index.docker.io/v1"
    ) {
        return Ok("docker.io");
    }
    let value = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value)
        .trim_end_matches('/');
    if value.is_empty() || value.contains(['/', '\\', '\r', '\n', ' ', '\t']) {
        Err(BuildRuntimeError::Validation(
            "Build Registry host is invalid.".to_owned(),
        ))
    } else {
        Ok(value)
    }
}

pub fn find_digest(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|part| part.starts_with("sha256:") && part.len() == 71)
        .map(|value| value.trim_end_matches(',').to_owned())
}

pub fn logs(stdout: &[u8], stderr: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !value.is_empty() {
            value.push('\n');
        }
        value.push_str(&String::from_utf8_lossy(stderr));
    }
    value
}

pub fn redact(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut redact_next = false;
    for segment in value.split_inclusive(char::is_whitespace) {
        let token_end = segment.find(char::is_whitespace).unwrap_or(segment.len());
        let (part, whitespace) = segment.split_at(token_end);
        let lower = part.to_ascii_lowercase();
        if redact_next {
            output.push_str("[redacted]");
            redact_next = false;
        } else if lower == "bearer" {
            output.push_str(part);
            redact_next = true;
        } else if lower.starts_with("password=")
            || lower.starts_with("token=")
            || lower.starts_with("secret=")
            || lower.starts_with("api_key=")
            || lower.starts_with("apikey=")
        {
            output.push_str(part.split_once('=').map_or(part, |(key, _)| key));
            output.push_str("=[redacted]");
        } else {
            output.push_str(part);
        }
        output.push_str(whitespace);
    }
    output
}

pub fn redact_values(value: &str, secrets: &[&str]) -> String {
    let mut redacted = value.to_owned();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        redacted = redacted.replace(secret, "[redacted]");
    }
    redact(&redacted)
}

fn encode_registry_auth(value: serde_json::Value) -> Zeroizing<String> {
    use base64::Engine;
    let bytes = Zeroizing::new(serde_json::to_vec(&value).expect("registry credentials serialize"));
    Zeroizing::new(base64::engine::general_purpose::URL_SAFE.encode(bytes.as_slice()))
}
