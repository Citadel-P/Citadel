//! Bounded local Docker commands with isolated credentials and redacted failures.
use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest, ProcessRunner};
use citadel_updates::UpdateError;
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) struct Docker<'a> {
    pub(super) process: &'a dyn ProcessRunner,
    pub(super) home: &'a Path,
}

impl Docker<'_> {
    pub(super) async fn run(
        &self,
        args: &[&str],
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, UpdateError> {
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let request = ProcessRequest::new("docker")
            .args(
                ["--host", "unix:///var/run/docker.sock"]
                    .into_iter()
                    .chain(args.iter().copied()),
            )
            .env("HOME", self.home)
            .env("DOCKER_CONFIG", self.home)
            .limits(ProcessLimits {
                timeout,
                maximum_stdout_bytes: 1024 * 1024,
                maximum_stderr_bytes: 16 * 1024,
                ..ProcessLimits::default()
            });
        let output =
            self.process
                .run(request, cancellation)
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

pub(super) fn failed() -> UpdateError {
    UpdateError::Unavailable(
        "The verified release descriptor could not be read safely from the local Docker daemon."
            .into(),
    )
}
