//! Docker CLI execution without Git, database, or resource routing dependencies.
use super::output;
use crate::connectors::docker::DockerEndpoint;
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
    #[error(transparent)]
    Process(ProcessError),
}
impl BuildRuntimeError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
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
    Archive { data: &'a [u8], dockerfile: &'a str },
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
        let session = Self {
            docker: docker.into(),
            endpoint,
            config,
            timeout,
            maximum_log_bytes,
            password: registry.map(|(_, credentials)| credentials.password.clone()),
        };
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
        Ok(session)
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
            .limits(limits(self.timeout, self.maximum_log_bytes)))
    }

    pub async fn build(
        &self,
        options: DockerBuildOptions<'_>,
        progress: &dyn BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<(), BuildRuntimeError> {
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
                (
                    OsString::from("-"),
                    OsString::from(dockerfile),
                    Some(data.to_vec()),
                )
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
        let result = output::run(request, cancellation, progress, &secrets).await?;
        if !result.succeeded() {
            return Err(streamed_command_failure("build", result.exit_code));
        }
        Ok(())
    }

    pub async fn push(
        &self,
        reference: &str,
        progress: &dyn BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<Option<String>, BuildRuntimeError> {
        if reference.trim().is_empty() || reference.starts_with('-') {
            return Err(BuildRuntimeError::Validation(
                "Invalid image reference.".into(),
            ));
        }
        let secrets: Vec<_> = self
            .password
            .as_ref()
            .map(|value| value.as_str())
            .into_iter()
            .collect();
        let result = output::run(
            self.request(vec!["push".into(), reference.into()])?,
            cancellation,
            progress,
            &secrets,
        )
        .await?;
        if !result.succeeded() {
            return Err(streamed_command_failure("push", result.exit_code));
        }
        Ok(find_digest(&logs(&result.stdout, &result.stderr)))
    }
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
        if let Err(error) = std::fs::remove_dir_all(&self.config) {
            tracing::warn!(%error,"Failed to clean temporary Docker credentials");
        }
    }
}

pub(super) fn limits(timeout: Duration, maximum: usize) -> ProcessLimits {
    ProcessLimits {
        timeout,
        maximum_stdout_bytes: maximum,
        maximum_stderr_bytes: maximum,
        output_limit_policy: OutputLimitPolicy::Truncate,
    }
}

pub(super) fn normalize_registry_host(value: &str) -> Result<&str, BuildRuntimeError> {
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

pub(super) fn find_digest(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|part| part.starts_with("sha256:") && part.len() == 71)
        .map(|value| value.trim_end_matches(',').to_owned())
}

pub(super) fn logs(stdout: &[u8], stderr: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !value.is_empty() {
            value.push('\n');
        }
        value.push_str(&String::from_utf8_lossy(stderr));
    }
    value
}

pub(super) fn redact(value: &str) -> String {
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

pub(super) fn redact_values(value: &str, secrets: &[&str]) -> String {
    let mut redacted = value.to_owned();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        redacted = redacted.replace(secret, "[redacted]");
    }
    redact(&redacted)
}
