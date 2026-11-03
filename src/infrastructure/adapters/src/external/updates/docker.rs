use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest, ProcessRunner};
use citadel_updates::{UpdateError, is_sha256};
use serde_json::Value;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub(super) const SOCKET: &str = "/var/run/docker.sock";

pub(super) struct LocalDocker<'a> {
    pub(super) process: &'a dyn ProcessRunner,
    pub(super) cancellation: &'a CancellationToken,
    pub(super) home: &'a Path,
    pub(super) deadline: Instant,
}
impl LocalDocker<'_> {
    pub(super) async fn json(&self, arguments: &[&str]) -> Result<Value, UpdateError> {
        serde_json::from_slice(&self.run(arguments).await?).map_err(|_| failed())
    }
    pub(super) async fn containers(&self, filter: &str) -> Result<Vec<String>, UpdateError> {
        let bytes = self
            .run(&[
                "container",
                "ls",
                "--all",
                "--no-trunc",
                "--filter",
                filter,
                "--format",
                "{{json .ID}}",
            ])
            .await?;
        let text = std::str::from_utf8(&bytes).map_err(|_| failed())?;
        let mut ids = text
            .lines()
            .map(|line| serde_json::from_str::<String>(line).map_err(|_| failed()))
            .collect::<Result<Vec<_>, _>>()?;
        if ids.iter().any(|id| !is_sha256(id)) {
            return Err(failed());
        }
        ids.sort();
        Ok(ids)
    }
    pub(super) async fn run(&self, arguments: &[&str]) -> Result<Vec<u8>, UpdateError> {
        self.run_with_timeout(arguments, Duration::from_secs(15))
            .await
    }

    pub(super) async fn run_with_timeout(
        &self,
        arguments: &[&str],
        timeout: Duration,
    ) -> Result<Vec<u8>, UpdateError> {
        self.execute(arguments, None, timeout).await
    }

    pub(super) async fn run_with_input(
        &self,
        arguments: &[&str],
        input: Vec<u8>,
        timeout: Duration,
    ) -> Result<Vec<u8>, UpdateError> {
        self.execute(arguments, Some(input), timeout).await
    }

    async fn execute(
        &self,
        arguments: &[&str],
        input: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<Vec<u8>, UpdateError> {
        if self.cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(refuse("Installation inspection timed out."));
        }
        let mut request = ProcessRequest::new("docker")
            .args(
                ["--host", "unix:///var/run/docker.sock"]
                    .into_iter()
                    .chain(arguments.iter().copied()),
            )
            .env("HOME", self.home)
            .env("DOCKER_CONFIG", self.home)
            .limits(ProcessLimits {
                timeout: remaining.min(timeout),
                maximum_stdout_bytes: 1024 * 1024,
                maximum_stderr_bytes: 16 * 1024,
                ..ProcessLimits::default()
            });
        request.stdin = input;
        let output =
            self.process
                .run(request, self.cancellation)
                .await
                .map_err(|error| match error {
                    ProcessError::Cancelled => UpdateError::Cancelled,
                    _ => failed(),
                })?;
        if !output.succeeded() || output.stdout_truncated || output.stderr_truncated {
            return Err(failed());
        }
        Ok(output.stdout)
    }
}

pub(super) fn single(value: &Value) -> Result<&Value, UpdateError> {
    value
        .as_array()
        .filter(|v| v.len() == 1)
        .map(|v| &v[0])
        .ok_or_else(failed)
}
pub(super) fn refuse(message: &str) -> UpdateError {
    UpdateError::Unavailable(message.into())
}
pub(super) fn failed() -> UpdateError {
    refuse(
        "Cannot safely inspect the local Citadel installation; Docker output was withheld because it may contain secrets.",
    )
}
