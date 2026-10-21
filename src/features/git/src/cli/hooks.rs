use super::*;

impl GitCli {
    /// Run trusted resource-writer-configured shell commands. This is not a sandbox.
    /// Do not pass Git authentication or persist command text/stdout/stderr here.
    pub async fn run_hook(
        &self,
        repository: &Path,
        hook: &crate::RepoCommand,
        name: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        if hook.commands.is_empty() {
            return Ok(());
        }
        hook.validate()
            .map_err(|error| GitError::Hook(format!("{name}: {error}")))?;
        let deadline = tokio::time::Instant::now() + self.timeout;
        for (index, command) in hook.commands.iter().enumerate() {
            // Resolve again for each command: an earlier command may have changed a symlink.
            let directory = self
                .workspace
                .hook_directory(repository, &hook.path)
                .await
                .map_err(|_| {
                    GitError::Hook(format!(
                        "{name}: working directory is missing or outside the repository"
                    ))
                })?;
            let timeout = deadline.saturating_duration_since(tokio::time::Instant::now());
            if timeout.is_zero() {
                return Err(GitError::Hook(format!("{name}: timed out")));
            }
            let request = ProcessRequest::new("/bin/sh")
                .args(["-eu", "-c", command.as_str()])
                .current_dir(directory)
                .limits(ProcessLimits {
                    timeout,
                    maximum_stdout_bytes: GIT_STDOUT_LIMIT,
                    maximum_stderr_bytes: GIT_STDERR_LIMIT,
                    output_limit_policy: OutputLimitPolicy::Error,
                });
            let output = self
                .process
                .run(request, cancellation)
                .await
                .map_err(|error| {
                    let reason = match error {
                        ProcessError::Cancelled => "cancelled",
                        ProcessError::Timeout(_) => "timed out",
                        ProcessError::OutputLimit { .. } => "output limit exceeded",
                        ProcessError::Spawn(_) => "could not start shell",
                        ProcessError::Io(_) => "process I/O failed",
                    };
                    GitError::Hook(format!("{name} command {}: {reason}", index + 1))
                })?;
            if !output.succeeded() {
                let reason = output.exit_code.map_or_else(
                    || "terminated by a signal".to_owned(),
                    |code| format!("exited with status {code}"),
                );
                return Err(GitError::Hook(format!(
                    "{name} command {}: {reason}",
                    index + 1
                )));
            }
        }
        Ok(())
    }
}
