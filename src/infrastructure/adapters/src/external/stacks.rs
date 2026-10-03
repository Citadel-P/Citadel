//! Shared local Compose/Swarm execution with no database or Core routing dependency.
use crate::connectors::docker::DockerEndpoint;
use citadel_stacks::{
    StackApplyEventType, StackApplySource, StackCommand, StackError, StackOrchestrationMode,
    StackProgressItem, StackReleaseStatus, StackRuntimeResult,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) const MAX_PROCESS_OUTPUT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_STACK_MESSAGES_BYTES: usize = 512 * 1024;

struct TransientDirectory(PathBuf);
impl Drop for TransientDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct TransientFile(PathBuf);
impl Drop for TransientFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub struct LocalStackApply {
    pub docker: String,
    pub endpoint: DockerEndpoint,
    pub project_name: String,
    pub orchestration: StackOrchestrationMode,
    pub destroy_before_deploy: bool,
    pub pre_deploy: Option<StackCommand>,
    pub post_deploy: Option<StackCommand>,
    pub service_names: Vec<String>,
    pub pull_images: bool,
}

impl LocalStackApply {
    pub async fn apply(
        &self,
        source: &StackApplySource,
        environment: &[String],
        registry: Option<(&str, &str)>,
        cancellation: &CancellationToken,
        progress: Option<&citadel_stacks::StackProgress>,
    ) -> Result<StackRuntimeResult, StackError> {
        let root = temporary_run_root(Uuid::now_v7());
        std::fs::create_dir_all(&root).map_err(runtime_io)?;
        let _cleanup = TransientDirectory(root.clone());
        set_private_directory(&root).await?;
        stage_source(&root, source).await?;
        self.apply_directory(
            &root,
            source,
            environment,
            registry,
            false,
            cancellation,
            progress,
        )
        .await
    }

    /// Execute a prepared source tree. The caller owns its lifetime, including
    /// persistent bind-mounted files that must survive a successful apply.
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_directory(
        &self,
        root: &Path,
        source: &StackApplySource,
        environment: &[String],
        registry: Option<(&str, &str)>,
        convert_compose_to_swarm: bool,
        cancellation: &CancellationToken,
        progress: Option<&citadel_stacks::StackProgress>,
    ) -> Result<StackRuntimeResult, StackError> {
        if cancellation.is_cancelled() {
            return Err(StackError::Cancelled);
        }
        let swarm = self.orchestration == StackOrchestrationMode::DockerSwarm;
        if swarm
            && (self.destroy_before_deploy
                || self.pre_deploy.is_some()
                || self.post_deploy.is_some()
                || !self.service_names.is_empty())
        {
            return Err(StackError::Validation("Swarm Stack deploy does not support service-scoped apply, destroy-before-deploy or pre/post commands.".into()));
        }
        if convert_compose_to_swarm && !swarm {
            return Err(StackError::Validation(
                "Compose conversion requires Swarm orchestration.".into(),
            ));
        }
        let environment = docker_environment(&self.endpoint, environment)?;
        let environment = environment.as_slice();
        async {
            let working_directory = resolve_source_path(root, &source.working_directory)?;
            let compose_paths = source
                .compose_paths
                .iter()
                .map(|path| resolve_source_path(root, path))
                .collect::<Result<Vec<_>, _>>()?;
            let mut env_paths = source
                .env_file_paths
                .iter()
                .map(|path| resolve_source_path(root, path))
                .collect::<Result<Vec<_>, _>>()?;
            let labels_override = source
                .labels_override_path
                .as_deref()
                .map(|path| resolve_source_path(root, path))
                .transpose()?;
            let generated_env = root.join(".citadel/environment.env");
            let _env_cleanup = TransientFile(generated_env.clone());
            let _credentials_cleanup = TransientDirectory(root.join(".citadel/docker"));
            if !environment.is_empty() {
                let mut contents = environment.join("\n");
                contents.push('\n');
                if let Some(parent) = generated_env.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(runtime_io)?;
                }
                tokio::fs::write(&generated_env, contents)
                    .await
                    .map_err(runtime_io)?;
                set_private_file(&generated_env).await?;
                env_paths.push(generated_env);
            }
            let docker_config = if let Some(registry) = registry {
                let directory = root.join(".citadel/docker");
                tokio::fs::create_dir_all(&directory)
                    .await
                    .map_err(runtime_io)?;
                set_private_directory(&directory).await?;
                let config = serde_json::to_vec(&serde_json::json!({
                    "auths": {
                        registry.0: { "auth": registry.1 }
                    }
                }))
                .map_err(runtime_io)?;
                tokio::fs::write(directory.join("config.json"), config)
                    .await
                    .map_err(runtime_io)?;
                set_private_file(&directory.join("config.json")).await?;
                Some(directory)
            } else {
                None
            };
            let mut messages = Vec::new();
            if let Some(command) = self.pre_deploy.as_ref() {
                let command_result = run_stack_commands(
                    command,
                    &working_directory,
                    environment,
                    cancellation,
                    progress,
                )
                .await?;
                let status = command_result.status;
                append_messages_bounded(&mut messages, command_result.messages);
                if status != StackReleaseStatus::Healthy {
                    return Ok(StackRuntimeResult { status, messages });
                }
            }
            if self.orchestration == StackOrchestrationMode::DockerSwarm {
                let mut restore = compose_args(
                    &compose_paths,
                    labels_override.as_deref(),
                    &env_paths,
                    &self.project_name,
                    &["up", "-d", "--pull", "missing"],
                );
                prepend_docker_config(&mut restore, docker_config.as_deref());
                if convert_compose_to_swarm {
                    let mut down = compose_args(
                        &compose_paths,
                        labels_override.as_deref(),
                        &env_paths,
                        &self.project_name,
                        &["down"],
                    );
                    prepend_docker_config(&mut down, docker_config.as_deref());
                    let stopped = run_process(
                        &self.docker,
                        &down,
                        &working_directory,
                        environment,
                        cancellation,
                        progress,
                    )
                    .await?;
                    let failed = stopped.status != StackReleaseStatus::Healthy;
                    append_messages_bounded(&mut messages, stopped.messages);
                    if failed {
                        let restored = run_process(
                            &self.docker,
                            &restore,
                            &working_directory,
                            environment,
                            cancellation,
                            progress,
                        )
                        .await?;
                        append_messages_bounded(&mut messages, restored.messages);
                        return Ok(StackRuntimeResult {
                            status: StackReleaseStatus::Failed,
                            messages,
                        });
                    }
                }
                let mut args = vec![
                    "stack".to_owned(),
                    "deploy".to_owned(),
                    "--detach=false".to_owned(),
                    "--prune".to_owned(),
                    "--resolve-image".to_owned(),
                    if self.pull_images {
                        "always".into()
                    } else {
                        "changed".into()
                    },
                ];
                for path in &compose_paths {
                    args.extend(["-c".to_owned(), path.display().to_string()]);
                }
                if let Some(path) = labels_override.as_ref() {
                    args.extend(["-c".to_owned(), path.display().to_string()]);
                }
                if registry.is_some() {
                    args.push("--with-registry-auth".to_owned());
                }
                args.push(self.project_name.clone());
                prepend_docker_config(&mut args, docker_config.as_deref());
                let result = match tokio::time::timeout(
                    std::time::Duration::from_secs(300),
                    run_process(
                        &self.docker,
                        &args,
                        &working_directory,
                        environment,
                        cancellation,
                        progress,
                    ),
                )
                .await
                {
                    Ok(result) => result?,
                    Err(_) => {
                        let item = StackProgressItem {
                            event_type: StackApplyEventType::StdErr,
                            message: Some("Swarm deployment timed out after 5 minutes.".into()),
                            exit_code: None,
                            stack_status: None,
                            severity: None,
                        };
                        if let Some(progress) = progress {
                            progress.send(item.clone()).await;
                        }
                        StackRuntimeResult {
                            status: StackReleaseStatus::Failed,
                            messages: vec![item],
                        }
                    }
                };
                append_messages_bounded(&mut messages, result.messages);
                if result.status != StackReleaseStatus::Healthy && convert_compose_to_swarm {
                    if let Some(progress) = progress {
                        progress
                            .send(StackProgressItem::system(
                                "Swarm deployment failed. Restoring the Docker Compose project...",
                            ))
                            .await;
                    }
                    let restored = run_process(
                        &self.docker,
                        &restore,
                        &working_directory,
                        environment,
                        cancellation,
                        progress,
                    )
                    .await?;
                    append_messages_bounded(&mut messages, restored.messages);
                }
                Ok(StackRuntimeResult {
                    status: result.status,
                    messages,
                })
            } else {
                if self.destroy_before_deploy && self.service_names.is_empty() {
                    let down = compose_args(
                        &compose_paths,
                        labels_override.as_deref(),
                        &env_paths,
                        &self.project_name,
                        &["down"],
                    );
                    let mut down = down;
                    prepend_docker_config(&mut down, docker_config.as_deref());
                    let down_result = run_process(
                        &self.docker,
                        &down,
                        &working_directory,
                        environment,
                        cancellation,
                        progress,
                    )
                    .await?;
                    let status = down_result.status;
                    append_messages_bounded(&mut messages, down_result.messages);
                    if status != StackReleaseStatus::Healthy {
                        return Ok(StackRuntimeResult { status, messages });
                    }
                }
                let mut up = compose_args(
                    &compose_paths,
                    labels_override.as_deref(),
                    &env_paths,
                    &self.project_name,
                    &[
                        "up",
                        "-d",
                        "--pull",
                        if self.pull_images {
                            "always"
                        } else {
                            "missing"
                        },
                    ],
                );
                if self.service_names.is_empty() {
                    up.push("--remove-orphans".into());
                } else {
                    up.push("--no-deps".into());
                    up.extend(self.service_names.iter().cloned());
                }
                prepend_docker_config(&mut up, docker_config.as_deref());
                let mut result = run_process(
                    &self.docker,
                    &up,
                    &working_directory,
                    environment,
                    cancellation,
                    progress,
                )
                .await?;
                append_messages_bounded(&mut messages, result.messages);
                if result.status == StackReleaseStatus::Healthy
                    && let Some(command) = self.post_deploy.as_ref()
                {
                    let command_result = run_stack_commands(
                        command,
                        &working_directory,
                        environment,
                        cancellation,
                        progress,
                    )
                    .await?;
                    append_messages_bounded(&mut messages, command_result.messages);
                    result.status = command_result.status;
                }
                Ok(StackRuntimeResult {
                    status: result.status,
                    messages,
                })
            }
        }
        .await
    }
}
pub(crate) fn temporary_run_root(stack_id: Uuid) -> PathBuf {
    std::env::temp_dir().join(format!(
        "citadel-stack-run-{}-{}",
        stack_id.simple(),
        Uuid::now_v7().simple()
    ))
}

pub(crate) fn compose_args(
    compose: &[PathBuf],
    labels_override: Option<&Path>,
    env: &[PathBuf],
    project: &str,
    tail: &[&str],
) -> Vec<String> {
    let mut args = vec!["compose".to_owned(), "-p".to_owned(), project.to_owned()];
    for path in compose {
        args.extend(["-f".to_owned(), path.display().to_string()]);
    }
    if let Some(path) = labels_override {
        args.extend(["-f".to_owned(), path.display().to_string()]);
    }
    for path in env {
        args.extend(["--env-file".to_owned(), path.display().to_string()]);
    }
    args.extend(tail.iter().map(|value| (*value).to_owned()));
    args
}

pub(crate) fn prepend_docker_config(args: &mut Vec<String>, config: Option<&Path>) {
    if let Some(config) = config {
        args.splice(0..0, ["--config".to_owned(), config.display().to_string()]);
    }
}

pub(crate) async fn stage_source(root: &Path, source: &StackApplySource) -> Result<(), StackError> {
    for file in &source.files {
        let target = resolve_staged_path(root, &file.relative_path)?;
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(runtime_io)?;
        }
        tokio::fs::write(target, &file.content)
            .await
            .map_err(runtime_io)?;
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) async fn set_private_directory(path: &Path) -> Result<(), StackError> {
    use std::os::unix::fs::PermissionsExt as _;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .map_err(runtime_io)
}

#[cfg(not(unix))]
pub(crate) async fn set_private_directory(_path: &Path) -> Result<(), StackError> {
    Ok(())
}

#[cfg(unix)]
pub(crate) async fn set_private_file(path: &Path) -> Result<(), StackError> {
    use std::os::unix::fs::PermissionsExt as _;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(runtime_io)
}

#[cfg(not(unix))]
pub(crate) async fn set_private_file(_path: &Path) -> Result<(), StackError> {
    Ok(())
}

pub(crate) fn resolve_staged_path(root: &Path, relative: &str) -> Result<PathBuf, StackError> {
    use std::path::Component;

    let path = Path::new(relative);
    if relative.trim().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(StackError::Validation(format!(
            "Stack source path '{relative}' must be relative and cannot escape its source root."
        )));
    }
    Ok(root.join(path))
}

fn docker_environment(
    endpoint: &DockerEndpoint,
    environment: &[String],
) -> Result<Vec<String>, StackError> {
    let mut environment = environment.to_vec();
    environment.retain(|entry| {
        !entry.starts_with("DOCKER_HOST=")
            && !entry.starts_with("DOCKER_CONTEXT=")
            && !entry.starts_with("DOCKER_TLS_VERIFY=")
            && !entry.starts_with("DOCKER_CERT_PATH=")
    });
    environment.push(format!(
        "DOCKER_HOST={}",
        endpoint.docker_host().map_err(runtime_io)?
    ));
    environment.push("DOCKER_CONTEXT=".into());
    environment.push("DOCKER_TLS_VERIFY=".into());
    environment.push("DOCKER_CERT_PATH=".into());
    Ok(environment)
}

pub(crate) async fn run_docker(
    endpoint: &DockerEndpoint,
    args: &[String],
    directory: &Path,
    environment: &[String],
    cancellation: &CancellationToken,
    progress: Option<&citadel_stacks::StackProgress>,
) -> Result<StackRuntimeResult, StackError> {
    run_process(
        "docker",
        args,
        directory,
        &docker_environment(endpoint, environment)?,
        cancellation,
        progress,
    )
    .await
}

pub(crate) async fn run_stack_commands(
    command: &StackCommand,
    root: &Path,
    environment: &[String],
    cancellation: &CancellationToken,
    progress: Option<&citadel_stacks::StackProgress>,
) -> Result<StackRuntimeResult, StackError> {
    let directory = resolve_staged_path(root, &command.path)?;
    if !tokio::fs::try_exists(&directory)
        .await
        .map_err(runtime_io)?
    {
        let item = StackProgressItem {
            event_type: StackApplyEventType::StdErr,
            message: Some(format!("Command path '{}' does not exist.", command.path)),
            exit_code: None,
            stack_status: None,
            severity: None,
        };
        if let Some(progress) = progress {
            progress.send(item.clone()).await;
        }
        return Ok(StackRuntimeResult {
            status: StackReleaseStatus::Failed,
            messages: vec![item],
        });
    }

    let mut messages = Vec::new();
    for value in command
        .commands
        .iter()
        .filter(|value| !value.trim().is_empty())
    {
        let (program, arguments) = shell_invocation(value);
        let result = run_process(
            program,
            &arguments,
            &directory,
            environment,
            cancellation,
            progress,
        )
        .await?;
        append_messages_bounded(&mut messages, result.messages);
        if result.status != StackReleaseStatus::Healthy {
            return Ok(StackRuntimeResult {
                status: result.status,
                messages,
            });
        }
    }
    Ok(StackRuntimeResult {
        status: StackReleaseStatus::Healthy,
        messages,
    })
}

pub(crate) fn append_messages_bounded(
    target: &mut Vec<StackProgressItem>,
    source: Vec<StackProgressItem>,
) {
    let mut remaining = MAX_STACK_MESSAGES_BYTES.saturating_sub(
        target
            .iter()
            .filter_map(|item| item.message.as_ref())
            .map(String::len)
            .sum::<usize>(),
    );
    for mut item in source {
        if let Some(message) = item.message.as_mut() {
            if remaining == 0 {
                continue;
            }
            if message.len() > remaining {
                message.truncate(message.floor_char_boundary(remaining));
            }
            remaining = remaining.saturating_sub(message.len());
        }
        target.push(item);
    }
}

pub(crate) fn shell_invocation(command: &str) -> (&'static str, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd.exe",
            vec![
                "/D".to_owned(),
                "/S".to_owned(),
                "/C".to_owned(),
                command.to_owned(),
            ],
        )
    } else {
        ("/bin/sh", vec!["-c".to_owned(), command.to_owned()])
    }
}

pub(crate) async fn run_process(
    program: &str,
    args: &[String],
    directory: &Path,
    environment: &[String],
    cancellation: &CancellationToken,
    progress: Option<&citadel_stacks::StackProgress>,
) -> Result<StackRuntimeResult, StackError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for entry in environment {
        if let Some((name, value)) = entry.split_once('=') {
            command.env(name, value);
        }
    }
    collect_process(command, cancellation, progress).await
}

pub(crate) async fn collect_process(
    mut command: Command,
    cancellation: &CancellationToken,
    progress: Option<&citadel_stacks::StackProgress>,
) -> Result<StackRuntimeResult, StackError> {
    if cancellation.is_cancelled() {
        return Err(StackError::Cancelled);
    }
    command.kill_on_drop(true);
    let mut child = command.spawn().map_err(runtime_io)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| StackError::Runtime("Process stdout was not captured.".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| StackError::Runtime("Process stderr was not captured.".into()))?;
    // Poll both pipes and process completion together. Dropping this future also
    // drops the readers and kills the child; there are no detached reader tasks.
    let (stdout, stderr, status) = tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(StackError::Cancelled),
        result = async {
            tokio::try_join!(
                read_process_output(stdout, StackApplyEventType::StdOut, progress),
                read_process_output(stderr, StackApplyEventType::StdErr, progress),
                async { child.wait().await.map_err(runtime_io) },
            )
        } => result?,
    };
    let mut messages = stdout;
    append_messages_bounded(&mut messages, stderr);
    let completed = StackProgressItem {
        event_type: StackApplyEventType::CommandCompleted,
        message: None,
        exit_code: status.code(),
        stack_status: None,
        severity: None,
    };
    if let Some(progress) = progress {
        progress.send(completed.clone()).await;
    }
    messages.push(completed);
    Ok(StackRuntimeResult {
        status: if status.success() {
            StackReleaseStatus::Healthy
        } else {
            StackReleaseStatus::Failed
        },
        messages,
    })
}

pub(crate) async fn read_process_output(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    event_type: StackApplyEventType,
    progress: Option<&citadel_stacks::StackProgress>,
) -> Result<Vec<StackProgressItem>, StackError> {
    let mut messages = Vec::new();
    let mut retained = 0usize;
    let mut pending = Vec::new();
    let mut oversized = false;
    let mut chunk = [0u8; 8192];
    loop {
        let count = reader.read(&mut chunk).await.map_err(runtime_io)?;
        for &byte in &chunk[..count] {
            if matches!(byte, b'\n' | b'\r') {
                if !oversized {
                    emit_process_line(&pending, event_type, progress, &mut messages, &mut retained)
                        .await;
                }
                pending.clear();
                oversized = false;
            } else if pending.len() < MAX_PROCESS_OUTPUT_BYTES && !oversized {
                pending.push(byte);
            } else {
                // Discard an oversized line as a whole, never expose a partial
                // secret or let an unterminated output line grow without bound.
                pending.clear();
                oversized = true;
            }
        }
        if count == 0 {
            if !oversized {
                emit_process_line(&pending, event_type, progress, &mut messages, &mut retained)
                    .await;
            }
            break;
        }
    }
    Ok(messages)
}

pub(crate) async fn emit_process_line(
    bytes: &[u8],
    event_type: StackApplyEventType,
    progress: Option<&citadel_stacks::StackProgress>,
    messages: &mut Vec<StackProgressItem>,
    retained: &mut usize,
) {
    let line = String::from_utf8_lossy(bytes);
    if line.trim().is_empty() {
        return;
    }
    let item = StackProgressItem {
        event_type,
        message: Some(line.into_owned()),
        exit_code: None,
        stack_status: None,
        severity: None,
    };
    if let Some(progress) = progress {
        progress.send(item.clone()).await;
    }
    let cost = bytes.len() + 64;
    if retained.saturating_add(cost) <= MAX_PROCESS_OUTPUT_BYTES {
        *retained += cost;
        messages.push(item);
    }
}

fn runtime_io(error: impl std::fmt::Display) -> StackError {
    StackError::Runtime(error.to_string())
}

fn resolve_source_path(root: &Path, path: &str) -> Result<PathBuf, StackError> {
    if Path::new(path).is_absolute() {
        Ok(PathBuf::from(path))
    } else {
        resolve_staged_path(root, path)
    }
}
