use super::bind_mount::{BIND_BACKUP_SCRIPT, BIND_PATH_PROBE, BIND_RESTORE_SCRIPT};
use super::restore_storage::RestoreVolumeOwner;
use citadel_backups::spec::{BackupExecutionLocation, BackupRepositorySpec};
use citadel_platforms::PlatformInfoPort;
use std::ffi::OsString;

use std::sync::Arc;

use std::time::Duration;

use citadel_backups::*;

use citadel_contracts::citadel::containers::v1::{
    CreateContainerRequest, ExecBinaryRequest, RestartPolicy,
};

use citadel_contracts::citadel::shared_models::v1::Mount;

use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};

use citadel_processes::run;

use futures_util::future::BoxFuture;

use serde_json::Value;

use sqlx::PgPool;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use crate::connectors::agent::client::AgentClient;
use crate::connectors::agent::client::AgentContainerAction;

use crate::connectors::agent::execution::AgentExecutionClient;

use crate::connectors::edge::EdgeRegistry;
use crate::connectors::edge::EdgeTarget;

use crate::persistence::postgres::platforms::local_target::LocalDockerTargetGuard;

#[derive(Clone)]
pub struct DockerResticBackupExecutor {
    settings: Option<super::settings::BackupExecutionSettings>,
    progress: Option<Arc<dyn Fn(BackupLog) + Send + Sync>>,
    docker: OsString,
    restic: OsString,
    image: String,
    secrets: Arc<dyn BackupSecretResolver>,
    maximum_output: usize,
    targets: LocalDockerTargetGuard,
    agent: Option<AgentClient>,
    edge: EdgeRegistry,
}

enum BackupExecutionTarget {
    Local,
    Agent(AgentExecutionClient),
}

impl DockerResticBackupExecutor {
    pub fn new(
        docker: impl Into<OsString>,
        image: impl Into<String>,
        secrets: Arc<dyn BackupSecretResolver>,
        maximum_output: usize,
        pool: PgPool,
    ) -> Self {
        Self {
            settings: None,
            progress: None,
            docker: docker.into(),
            restic: "restic".into(),
            image: image.into(),
            secrets,
            maximum_output,
            targets: LocalDockerTargetGuard::new(pool),
            agent: None,
            edge: EdgeRegistry::default(),
        }
    }

    pub fn with_settings(mut self, settings: super::settings::BackupExecutionSettings) -> Self {
        self.settings = Some(settings);
        self
    }
    fn log_line_limit(&self) -> usize {
        self.settings
            .as_ref()
            .map_or(8192, |s| s.maximum_log_line_bytes)
    }
    fn repository_timeout(&self) -> Duration {
        self.settings
            .as_ref()
            .map_or(Duration::from_secs(900), |s| s.default_timeout)
    }
    fn restore_timeout(&self) -> Duration {
        self.settings
            .as_ref()
            .map_or(Duration::from_secs(14_400), |s| s.default_timeout)
    }
    fn core_repository_path(&self, path: &str) -> Result<std::path::PathBuf, String> {
        self.settings
            .as_ref()
            .map_or_else(|| Ok(path.into()), |s| s.repository_path(path))
    }

    #[must_use]
    pub fn with_restic(mut self, restic: impl Into<OsString>) -> Self {
        self.restic = restic.into();
        self
    }

    #[must_use]
    pub fn with_agent(mut self, agent: Option<AgentClient>) -> Self {
        self.agent = agent;
        self
    }

    #[must_use]
    pub fn with_edge(mut self, edge: EdgeRegistry) -> Self {
        self.edge = edge;
        self
    }
}

impl BackupExecutor for DockerResticBackupExecutor {
    fn backup_with_progress<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancellation: &'a CancellationToken,
        progress: Arc<dyn Fn(BackupLog) + Send + Sync>,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        Box::pin(async move {
            let mut executor = self.clone();
            executor.progress = Some(progress);
            executor.backup(claim, plan, cancellation).await
        })
    }
    fn restore_with_progress<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        cancellation: &'a CancellationToken,
        progress: Arc<dyn Fn(BackupLog) + Send + Sync>,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        Box::pin(async move {
            let mut executor = self.clone();
            executor.progress = Some(progress);
            executor.restore(claim, cancellation).await
        })
    }
    fn repository<'a>(
        &'a self,
        repository: &'a BackupRepository,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
        Box::pin(async move {
            self.validate_repository_target(repository, location, platform_id)
                .await
                .map_err(BackupError::Validation)?;
            if location == "Core" {
                return self
                    .run_direct_repository_operation(repository, operation, cancellation)
                    .await
                    .map_err(BackupError::Storage);
            }
            let platform_id = platform_id.ok_or_else(|| {
                BackupError::Validation("A Platform is required for this operation.".into())
            })?;
            if let BackupExecutionTarget::Agent(agent) = self
                .execution_target(platform_id, None, cancellation)
                .await
                .map_err(BackupError::Validation)?
            {
                let arguments = match operation {
                    "Validate" | "Check" => vec!["check".into(), "--read-data-subset=1/100".into()],
                    "Initialize" => vec!["init".into()],
                    "Prune" => vec![
                        "forget".into(),
                        "--prune".into(),
                        "--keep-last".into(),
                        "14".into(),
                    ],
                    _ => {
                        return Err(BackupError::Validation(
                            "Backup Repository operation is invalid.".into(),
                        ));
                    }
                };
                return self
                    .run_agent_repository(
                        &agent,
                        platform_id,
                        repository,
                        arguments,
                        self.repository_timeout(),
                        cancellation,
                    )
                    .await
                    .map_err(BackupError::Storage);
            }
            let name = format!("citadel-repository-{}", Uuid::now_v7().simple());
            let (mut args, env) = self
                .base_args(&name, repository)
                .await
                .map_err(BackupError::Validation)?;
            args.push(self.image.clone().into());
            match operation {
                "Validate" | "Check" => {
                    args.extend(["check".into(), "--read-data-subset=1/100".into()])
                }
                "Initialize" => args.push("init".into()),
                "Prune" => args.extend([
                    "forget".into(),
                    "--prune".into(),
                    "--keep-last".into(),
                    "14".into(),
                ]),
                _ => {
                    return Err(BackupError::Validation(
                        "Backup Repository operation is invalid.".into(),
                    ));
                }
            }
            let output = self
                .execute(args, env, self.repository_timeout(), cancellation)
                .await;
            self.cleanup(&name).await;
            output
                .map(|value| output_logs(&value, self.log_line_limit(), self.maximum_output))
                .map_err(BackupError::Storage)
        })
    }
    fn backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        Box::pin(async move {
            match self.run_backup(claim, plan, cancellation).await {
                Ok((snapshot, mut logs, items, summary, mut warnings)) => {
                    let retention = self.run_retention(claim, plan, cancellation).await;
                    let status = match retention {
                        Ok(retention_logs) => {
                            logs.extend(retention_logs);
                            if warnings.is_empty() && plan.warnings.is_empty() {
                                citadel_backups::BackupRunStatus::Succeeded
                            } else {
                                citadel_backups::BackupRunStatus::SucceededWithWarnings
                            }
                        }
                        Err(error) => {
                            warnings.push(format!(
                                "Snapshot was created, but retention failed: {error}"
                            ));
                            citadel_backups::BackupRunStatus::SucceededWithWarnings
                        }
                    };
                    BackupExecutionResult {
                        status,
                        snapshot_availability:
                            citadel_backups::BackupSnapshotAvailability::Available,
                        restic_snapshot_id: snapshot,
                        parent_snapshot_id: None,
                        files_processed: summary.files_processed,
                        bytes_processed: summary.bytes_processed,
                        bytes_added: summary.bytes_added,
                        exit_code: Some(0),
                        error_code: None,
                        error_message: None,
                        warnings: plan.warnings.iter().cloned().chain(warnings).collect(),
                        logs,
                        items,
                    }
                }
                Err((error, logs, items)) => {
                    failed_backup(error, cancellation.is_cancelled(), logs, items)
                }
            }
        })
    }
    fn restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        Box::pin(async move {
            match self.run_restore(claim, cancellation).await {
                Ok(logs) => RestoreExecutionResult {
                    status: citadel_backups::BackupRestoreStatus::Succeeded,
                    exit_code: Some(0),
                    error_code: None,
                    error_message: None,
                    logs,
                },
                Err(error) => RestoreExecutionResult {
                    status: if cancellation.is_cancelled() {
                        citadel_backups::BackupRestoreStatus::Cancelled
                    } else {
                        citadel_backups::BackupRestoreStatus::Failed
                    },
                    exit_code: None,
                    error_code: Some(
                        if cancellation.is_cancelled() {
                            "Cancelled"
                        } else {
                            "ResticFailed"
                        }
                        .into(),
                    ),
                    error_message: Some(error),
                    logs: vec![],
                },
            }
        })
    }
}

impl DockerResticBackupExecutor {
    async fn run_direct_repository_operation(
        &self,
        repository: &BackupRepository,
        operation: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        let (location, environment) = self.direct_repository_environment(repository).await?;
        let mut arguments = vec!["-r".into(), location];
        match operation {
            "Validate" | "Check" => {
                arguments.extend(["check".into(), "--read-data-subset=1/100".into()]);
            }
            "Initialize" => arguments.push("init".into()),
            "Prune" => arguments.extend([
                "forget".into(),
                "--prune".into(),
                "--keep-last".into(),
                "14".into(),
            ]),
            _ => return Err("Backup Repository operation is invalid.".into()),
        }
        let mut request = ProcessRequest::new(self.restic.clone())
            .args(arguments)
            .limits(ProcessLimits {
                timeout: self.repository_timeout(),
                maximum_stdout_bytes: self.maximum_output,
                maximum_stderr_bytes: self.maximum_output,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        for (key, value) in environment {
            request = request.env(key, value);
        }
        let output = self
            .run_observed(request, cancellation)
            .await
            .map_err(|error| error.to_string())?;
        if output.succeeded() {
            Ok(output_logs(
                &output,
                self.log_line_limit(),
                self.maximum_output,
            ))
        } else {
            Err(redact(&String::from_utf8_lossy(&output.stderr)))
        }
    }

    async fn run_backup(
        &self,
        claim: &BackupClaim,
        plan: &BackupSourcePlan,
        cancellation: &CancellationToken,
    ) -> Result<
        (
            Option<String>,
            Vec<BackupLog>,
            Vec<BackupRunItemResult>,
            ResticSummary,
            Vec<String>,
        ),
        (String, Vec<BackupLog>, Vec<BackupRunItemResult>),
    > {
        if let Some(directory) = plan.local_directory.as_deref() {
            let result = self.run_system_backup(claim, directory, cancellation).await;
            let cleanup = tokio::fs::remove_dir_all(directory).await;
            return match result {
                Ok((snapshot, logs, summary)) => {
                    let warnings = cleanup
                        .err()
                        .filter(|error| error.kind() != std::io::ErrorKind::NotFound)
                        .map(|error| {
                            format!(
                                "Snapshot was created, but recovery staging cleanup failed: {error}"
                            )
                        })
                        .into_iter()
                        .collect();
                    Ok((Some(snapshot), logs, vec![], summary, warnings))
                }
                Err((mut error, logs)) => {
                    if let Err(cleanup) = cleanup
                        && cleanup.kind() != std::io::ErrorKind::NotFound
                    {
                        error.push_str(&format!(
                            "; recovery staging cleanup also failed: {cleanup}"
                        ));
                    }
                    Err((error, logs, vec![]))
                }
            };
        }
        let mut logs = Vec::new();
        let mut results = Vec::with_capacity(plan.items.len());
        for item in &plan.items {
            let target = match self
                .execution_target(
                    item.platform_id,
                    item.docker_node_id.as_deref(),
                    cancellation,
                )
                .await
            {
                Ok(target) => target,
                Err(error) => return Err((error, logs, results)),
            };
            let outcome = match target {
                BackupExecutionTarget::Local => {
                    self.run_backup_item(claim, item, cancellation).await
                }
                BackupExecutionTarget::Agent(agent) => {
                    self.run_agent_backup_item(&agent, claim, item, cancellation)
                        .await
                }
            };
            match outcome {
                Ok((snapshot, item_logs, summary)) => {
                    logs.extend(item_logs);
                    results.push(BackupRunItemResult {
                        id: item.id,
                        status: citadel_backups::BackupRunItemStatus::Succeeded,
                        restic_snapshot_id: Some(snapshot),
                        parent_snapshot_id: summary.parent_snapshot_id,
                        files_processed: summary.files_processed,
                        bytes_processed: summary.bytes_processed,
                        bytes_added: summary.bytes_added,
                        exit_code: Some(0),
                        error_code: None,
                        error_message: None,
                    });
                }
                Err((error, item_logs)) => {
                    logs.extend(item_logs);
                    results.push(BackupRunItemResult {
                        id: item.id,
                        status: if cancellation.is_cancelled() {
                            citadel_backups::BackupRunItemStatus::Cancelled
                        } else {
                            citadel_backups::BackupRunItemStatus::Failed
                        },
                        restic_snapshot_id: None,
                        parent_snapshot_id: None,
                        files_processed: None,
                        bytes_processed: None,
                        bytes_added: None,
                        exit_code: None,
                        error_code: Some(
                            if cancellation.is_cancelled() {
                                "Cancelled"
                            } else {
                                "ResticFailed"
                            }
                            .into(),
                        ),
                        error_message: Some(error.clone()),
                    });
                    return Err((error, logs, results));
                }
            }
        }
        let single_snapshot = (results.len() == 1)
            .then(|| results[0].restic_snapshot_id.clone())
            .flatten();
        let summary = ResticSummary {
            parent_snapshot_id: (results.len() == 1)
                .then(|| results[0].parent_snapshot_id.clone())
                .flatten(),
            files_processed: sum_item_metric(&results, |item| item.files_processed),
            bytes_processed: sum_item_metric(&results, |item| item.bytes_processed),
            bytes_added: sum_item_metric(&results, |item| item.bytes_added),
        };
        Ok((single_snapshot, logs, results, summary, vec![]))
    }

    async fn run_system_backup(
        &self,
        claim: &BackupClaim,
        directory: &std::path::Path,
        cancellation: &CancellationToken,
    ) -> Result<(String, Vec<BackupLog>, ResticSummary), (String, Vec<BackupLog>)> {
        let (repository, environment) = self
            .direct_repository_environment(&claim.repository)
            .await
            .map_err(|error| (error, vec![]))?;
        let args = vec![
            "-r".into(),
            repository,
            "backup".into(),
            "--json".into(),
            "--tag".into(),
            policy_tag(claim.policy.id).into(),
            "--tag".into(),
            format!("backup-run:{}", claim.run.id).into(),
            "--tag".into(),
            "citadel-system".into(),
            directory.as_os_str().to_owned(),
        ];
        let mut request =
            ProcessRequest::new(self.restic.clone())
                .args(args)
                .limits(ProcessLimits {
                    timeout: Duration::from_secs(
                        u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400),
                    ),
                    maximum_stdout_bytes: self.maximum_output,
                    maximum_stderr_bytes: self.maximum_output,
                    output_limit_policy: OutputLimitPolicy::Truncate,
                });
        for (key, value) in environment {
            request = request.env(key, value);
        }
        let output = match self.run_observed(request, cancellation).await {
            Ok(output) if output.succeeded() => output,
            Ok(output) => {
                let logs = output_logs(&output, self.log_line_limit(), self.maximum_output);
                return Err((redact(&String::from_utf8_lossy(&output.stderr)), logs));
            }
            Err(error) => return Err((error.to_string(), vec![])),
        };
        let logs = output_logs(&output, self.log_line_limit(), self.maximum_output);
        let summary = parse_restic_summary(&output.stdout).ok_or_else(|| {
            (
                "Restic completed without returning a summary.".to_owned(),
                logs.clone(),
            )
        })?;
        let snapshot = summary.0.ok_or_else(|| {
            (
                "Restic completed without returning a snapshot ID.".to_owned(),
                logs.clone(),
            )
        })?;
        Ok((snapshot, logs, summary.1))
    }

    async fn run_backup_item(
        &self,
        claim: &BackupClaim,
        item: &BackupSourceItem,
        cancellation: &CancellationToken,
    ) -> Result<(String, Vec<BackupLog>, ResticSummary), (String, Vec<BackupLog>)> {
        let volume = &item.volume_name;
        if let Some(path) = &item.host_path {
            citadel_backups::spec::validate_bind_path(path)
                .map_err(|error| (error.to_string(), vec![]))?;
            validate_bind_repository(&claim.repository, path).map_err(|error| (error, vec![]))?;
            self.validate_local_bind_path(path, cancellation)
                .await
                .map_err(|error| (error, vec![]))?;
        } else {
            let inspect = self
                .execute(
                    vec!["volume".into(), "inspect".into(), volume.into()],
                    vec![],
                    Duration::from_secs(30),
                    cancellation,
                )
                .await;
            if let Err(error) = inspect {
                return Err((
                    format!("Docker Volume '{volume}' is unavailable: {error}"),
                    vec![],
                ));
            }
        }
        let name = format!(
            "citadel-backup-{}-{}",
            claim.run.id.simple(),
            item.id.simple()
        );
        let (mut args, env) = self
            .base_args(&name, &claim.repository)
            .await
            .map_err(|error| (error, vec![]))?;
        if let Some(path) = &item.host_path {
            args.extend([
                "--mount".into(),
                self.bind_mount_argument(path, "/data", true, cancellation)
                    .await
                    .map_err(|error| (error, vec![]))?
                    .into(),
                "--entrypoint".into(),
                "/bin/sh".into(),
                self.image.clone().into(),
                "-c".into(),
                BIND_BACKUP_SCRIPT.into(),
                "sh".into(),
            ]);
        } else {
            args.extend([
                "--volume".into(),
                format!("{volume}:/data:ro").into(),
                self.image.clone().into(),
            ]);
        }
        args.extend([
            "backup".into(),
            "--json".into(),
            "--tag".into(),
            policy_tag(claim.policy.id).into(),
            "--tag".into(),
            format!("backup-run:{}", claim.run.id).into(),
            "--tag".into(),
            format!("backup-item:{}", item.id).into(),
            "--tag".into(),
            item.host_path
                .as_ref()
                .map_or_else(|| format!("volume:{volume}"), |path| format!("bind:{path}"))
                .into(),
            "/data".into(),
        ]);
        let output = self
            .execute(
                args,
                env,
                Duration::from_secs(u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400)),
                cancellation,
            )
            .await;
        self.cleanup(&name).await;
        let output = match output {
            Ok(output) => output,
            Err(error) => return Err((error, vec![])),
        };
        let logs = output_logs(&output, self.log_line_limit(), self.maximum_output);
        let summary = parse_restic_summary(&output.stdout).ok_or_else(|| {
            (
                "Restic completed without returning a summary.".to_owned(),
                logs.clone(),
            )
        })?;
        let snapshot = summary.0.ok_or_else(|| {
            (
                "Restic completed without returning a snapshot ID.".to_owned(),
                logs.clone(),
            )
        })?;
        Ok((snapshot, logs, summary.1))
    }

    async fn run_agent_backup_item(
        &self,
        agent: &AgentExecutionClient,
        claim: &BackupClaim,
        item: &BackupSourceItem,
        cancellation: &CancellationToken,
    ) -> Result<(String, Vec<BackupLog>, ResticSummary), (String, Vec<BackupLog>)> {
        let timeout =
            Duration::from_secs(u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400));
        if let Some(path) = &item.host_path {
            self.validate_agent_bind_path(agent, item.platform_id, path, cancellation)
                .await
                .map_err(|error| (error, vec![]))?;
        }
        let container_id = self
            .create_agent_helper(
                agent,
                item.platform_id,
                Some((
                    item.host_path.as_deref().unwrap_or(&item.volume_name),
                    true,
                    item.host_path.is_some(),
                    "/source",
                )),
                timeout,
                cancellation,
            )
            .await
            .map_err(|error| (error, Vec::new()))?;
        let result: Result<_, (String, Vec<BackupLog>)> = async {
            agent
                .change_containers_state(
                    std::slice::from_ref(&container_id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| (error.to_string(), Vec::new()))?;
            let environment = self
                .agent_repository_environment(&claim.repository)
                .await
                .map_err(|error| (error, Vec::new()))?;
            let mut command = vec![
                "restic".to_owned(),
                "backup".to_owned(),
                "--json".to_owned(),
                "--tag".to_owned(),
                policy_tag(claim.policy.id),
                "--tag".to_owned(),
                format!("backup-run:{}", claim.run.id),
                "--tag".to_owned(),
                format!("backup-item:{}", item.id),
                "--tag".to_owned(),
                item.host_path.as_ref().map_or_else(
                    || format!("volume:{}", item.volume_name),
                    |path| format!("bind:{path}"),
                ),
                "/source".to_owned(),
            ];
            if let Some(path) = &item.host_path {
                citadel_backups::spec::validate_bind_path(path)
                    .map_err(|error| (error.to_string(), Vec::new()))?;
                command.remove(0);
                command.splice(
                    0..0,
                    [
                        "/bin/sh".into(),
                        "-c".into(),
                        BIND_BACKUP_SCRIPT.replace("/data", "/source"),
                        "sh".into(),
                    ],
                );
            }
            let output = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: container_id.clone(),
                        cmd: command,
                        env: environment,
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        tty: false,
                    },
                    timeout,
                    self.maximum_output,
                    cancellation,
                )
                .await
                .map_err(|error| (error.to_string(), Vec::new()))?;
            let logs = agent_output_logs(
                &output.stdout,
                &output.stderr,
                self.log_line_limit(),
                self.maximum_output,
            );
            if output.exit_code != 0 {
                return Err((redact(&String::from_utf8_lossy(&output.stderr)), logs));
            }
            let summary = parse_restic_summary(&output.stdout).ok_or_else(|| {
                (
                    "Restic completed without returning a summary.".to_owned(),
                    logs.clone(),
                )
            })?;
            let snapshot = summary.0.clone().ok_or_else(|| {
                (
                    "Restic completed without returning a snapshot ID.".to_owned(),
                    logs.clone(),
                )
            })?;
            Ok((snapshot, logs, summary.1))
        }
        .await;
        let _ = agent
            .delete_container(&container_id, &CancellationToken::new())
            .await;
        result
    }
    async fn run_agent_repository(
        &self,
        agent: &AgentExecutionClient,
        platform_id: Uuid,
        repository: &BackupRepository,
        arguments: Vec<String>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        // Repository checks and retention need no source-volume mount, but must
        // run on the selected Agent, not accidentally on Core's Docker socket.
        let environment = self.agent_repository_environment(repository).await?;
        let container_id = self
            .create_agent_helper(agent, platform_id, None, timeout, cancellation)
            .await?;
        let result = async {
            agent
                .change_containers_state(
                    std::slice::from_ref(&container_id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            let mut command = vec!["restic".to_owned()];
            command.extend(arguments);
            let output = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: container_id.clone(),
                        cmd: command,
                        env: environment,
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        tty: false,
                    },
                    timeout,
                    self.maximum_output,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            if output.exit_code != 0 {
                return Err(redact(&String::from_utf8_lossy(&output.stderr)));
            }
            Ok(agent_output_logs(
                &output.stdout,
                &output.stderr,
                self.log_line_limit(),
                self.maximum_output,
            ))
        }
        .await;
        let _ = agent
            .delete_container(&container_id, &CancellationToken::new())
            .await;
        result
    }

    async fn run_retention(
        &self,
        claim: &BackupClaim,
        plan: &BackupSourcePlan,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        if plan.local_directory.is_some() {
            return self.run_direct_retention(claim, cancellation).await;
        }
        let item = plan
            .items
            .first()
            .ok_or_else(|| "Backup plan has no source items.".to_owned())?;
        if let BackupExecutionTarget::Agent(agent) = self
            .execution_target(
                item.platform_id,
                item.docker_node_id.as_deref(),
                cancellation,
            )
            .await?
        {
            return self
                .run_agent_repository(
                    &agent,
                    item.platform_id,
                    &claim.repository,
                    vec![
                        "forget".into(),
                        "--json".into(),
                        "--tag".into(),
                        policy_tag(claim.policy.id),
                        "--keep-last".into(),
                        claim.policy.keep_last_successful.to_string(),
                        "--prune".into(),
                    ],
                    Duration::from_secs(
                        u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400),
                    ),
                    cancellation,
                )
                .await;
        }
        let name = format!("citadel-retention-{}", claim.run.id.simple());
        let (mut args, env) = self.base_args(&name, &claim.repository).await?;
        args.extend([
            self.image.clone().into(),
            "forget".into(),
            "--json".into(),
            "--tag".into(),
            policy_tag(claim.policy.id).into(),
            "--keep-last".into(),
            claim.policy.keep_last_successful.to_string().into(),
            "--prune".into(),
        ]);
        let output = self
            .execute(
                args,
                env,
                Duration::from_secs(u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400)),
                cancellation,
            )
            .await;
        self.cleanup(&name).await;
        output.map(|output| output_logs(&output, self.log_line_limit(), self.maximum_output))
    }

    async fn run_direct_retention(
        &self,
        claim: &BackupClaim,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        let (repository, environment) = self
            .direct_repository_environment(&claim.repository)
            .await?;
        let arguments = vec![
            "-r".into(),
            repository,
            "forget".into(),
            "--json".into(),
            "--tag".into(),
            policy_tag(claim.policy.id).into(),
            "--keep-last".into(),
            claim.policy.keep_last_successful.to_string().into(),
            "--prune".into(),
        ];
        let mut request = ProcessRequest::new(self.restic.clone())
            .args(arguments)
            .limits(ProcessLimits {
                timeout: Duration::from_secs(
                    u64::try_from(claim.policy.timeout_seconds).unwrap_or(14_400),
                ),
                maximum_stdout_bytes: self.maximum_output,
                maximum_stderr_bytes: self.maximum_output,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        for (key, value) in environment {
            request = request.env(key, value);
        }
        let output = self
            .run_observed(request, cancellation)
            .await
            .map_err(|error| error.to_string())?;
        if output.succeeded() {
            Ok(output_logs(
                &output,
                self.log_line_limit(),
                self.maximum_output,
            ))
        } else {
            Err(redact(&String::from_utf8_lossy(&output.stderr)))
        }
    }
    async fn run_restore(
        &self,
        claim: &RestoreClaim,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        let target = self
            .execution_target(
                claim.run.target_platform_id,
                claim.run.target_docker_node_id.as_deref(),
                cancellation,
            )
            .await?;
        if let Some(path) = &claim.run.target_host_path {
            citadel_backups::spec::validate_bind_path(path).map_err(|error| error.to_string())?;
            validate_bind_repository(&claim.repository, path)?;
            return self
                .run_bind_restore(target, claim, path, cancellation)
                .await;
        }
        if let BackupExecutionTarget::Agent(agent) = target {
            return self.run_agent_restore(&agent, claim, cancellation).await;
        }
        let snapshot = claim
            .source_item
            .as_ref()
            .and_then(|item| item.restic_snapshot_id.as_deref())
            .or(claim.source.restic_snapshot_id.as_deref())
            .ok_or_else(|| "Backup Run has no Restic snapshot ID.".to_owned())?;
        let name = format!("citadel-restore-{}", claim.run.id.simple());
        let volume = &claim.run.target_volume_name;
        // A failed inspect is not proof that a volume is absent (the daemon
        // may be unavailable). Only a successful, complete listing may do that.
        let listed = self
            .execute(
                vec![
                    "volume".into(),
                    "ls".into(),
                    "--filter".into(),
                    format!("name={volume}").into(),
                    "--format".into(),
                    "{{.Name}}".into(),
                ],
                vec![],
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        let existing = listed_volume_exists(&listed, volume)?;
        if cancellation.is_cancelled() {
            return Err("Restore was cancelled.".into());
        }
        if existing && !claim.run.overwrite_existing {
            return Err("Target Volume already exists.".into());
        }
        let metadata_name = format!("citadel-restore-metadata-{}", claim.run.id.simple());
        let (mut metadata_args, metadata_env) =
            self.base_args(&metadata_name, &claim.repository).await?;
        metadata_args.extend([
            self.image.clone().into(),
            "snapshots".into(),
            "--json".into(),
            snapshot.into(),
        ]);
        let metadata = self
            .execute(
                metadata_args,
                metadata_env,
                Duration::from_secs(60),
                cancellation,
            )
            .await;
        self.cleanup(&metadata_name).await;
        let (snapshot, root) = snapshot_root(&metadata?.stdout, snapshot)?;
        let owner = RestoreVolumeOwner::new(claim.run.target_platform_id, claim.run.id);
        if !existing {
            let request = owner.request(volume);
            let mut args = vec![
                "volume".into(),
                "create".into(),
                "--driver".into(),
                "local".into(),
            ];
            for (key, value) in request.labels {
                args.extend(["--label".into(), format!("{key}={value}").into()]);
            }
            args.push(volume.into());
            self.execute(args, vec![], Duration::from_secs(30), cancellation)
                .await?;
            // A concurrent creator may have won the name. Docker returns that
            // existing volume without applying our ownership labels.
            self.verify_local_restore_owner(&owner, volume, cancellation)
                .await?;
        }
        let (mut args, env) = self.base_args(&name, &claim.repository).await?;
        // A created (even stopped) helper holds a reference to the volume.
        // Recheck ownership after attaching it, before any restore can start.
        args[0] = "create".into();
        // Each attempt has its own identity, so recovery cannot delete an older
        // helper or an unrelated container that won the same name.
        let helper_attempt = Uuid::now_v7();
        args.extend([
            "--label".into(),
            format!("citadel.backup.restoreHelperId={helper_attempt}").into(),
        ]);
        args.extend([
            "--volume".into(),
            format!("{volume}:/restore{root}").into(),
            self.image.clone().into(),
            "restore".into(),
            snapshot.into(),
            "--target".into(),
            "/restore".into(),
            "--delete".into(),
        ]);
        let created = self
            .execute(args, env, Duration::from_secs(60), cancellation)
            .await
            .and_then(|output| restore_helper_identity(&output));
        let container_id = match created {
            Ok(id) => id,
            Err(mut error) => {
                // Docker may have committed the create before the CLI lost its
                // response. Recover with a fresh token even during cancellation.
                if self
                    .cleanup_ambiguous_restore_helper(&name, helper_attempt)
                    .await
                    .is_err()
                {
                    error.push_str(" Restore helper cleanup could not be verified; inspect the destination before retrying.");
                }
                return Err(error);
            }
        };
        let output = async {
            if !existing {
                self.verify_local_restore_owner(&owner, volume, cancellation)
                    .await?;
            }
            let usage = self.execute(
                vec![
                    "ps".into(), "--all".into(), "--quiet".into(),
                    "--no-trunc".into(), "--filter".into(), format!("volume={volume}").into(),
                ],
                vec![], Duration::from_secs(30), cancellation,
            ).await?;
            let ids = String::from_utf8_lossy(&usage.stdout);
            if usage.stdout_truncated
                || ids.lines().any(|id| id.trim() != container_id)
                || !ids.lines().any(|id| id.trim() == container_id)
            {
                return Err("Target Volume usage could not be verified or it is in use by another container.".into());
            }
            self.execute(
                vec![
                    "start".into(),
                    "--attach".into(),
                    container_id.clone().into(),
                ],
                vec![],
                self.restore_timeout(),
                cancellation,
            )
            .await
        }
        .await;
        self.cleanup(&container_id).await;
        // Preserve partial data for inspection. Never remove a volume based on
        // a pre-create existence check or a remembered name after cancellation.
        output.map(|output| output_logs(&output, self.log_line_limit(), self.maximum_output))
    }

    async fn verify_local_restore_owner(
        &self,
        owner: &RestoreVolumeOwner,
        volume: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        let output = self
            .execute(
                vec![
                    "volume".into(),
                    "inspect".into(),
                    "--format".into(),
                    owner.local_probe(volume).into(),
                    volume.into(),
                ],
                vec![],
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        if output.stdout_truncated || String::from_utf8_lossy(&output.stdout).trim() != "true" {
            return Err(RestoreVolumeOwner::conflict());
        }
        Ok(())
    }

    async fn cleanup_ambiguous_restore_helper(
        &self,
        name: &str,
        attempt: Uuid,
    ) -> Result<(), String> {
        let cancellation = CancellationToken::new();
        // Only emit an immutable ID, never the inspect document containing the
        // helper's repository credentials. A name alone is not ownership proof.
        let probe = format!(
            "{{{{if and (eq (index .Config.Labels \"citadel.backup.restoreHelperId\") \"{attempt}\") (eq (index .Config.Labels \"com.citadel.system-role\") \"backup-helper\")}}}}{{{{.Id}}}}{{{{end}}}}"
        );
        let output = self
            .execute(
                vec![
                    "container".into(),
                    "inspect".into(),
                    "--format".into(),
                    probe.into(),
                    name.into(),
                ],
                vec![],
                Duration::from_secs(30),
                &cancellation,
            )
            .await?;
        let id = restore_helper_identity(&output)?;
        self.execute(
            vec!["rm".into(), "--force".into(), id.into()],
            vec![],
            Duration::from_secs(30),
            &cancellation,
        )
        .await?;
        Ok(())
    }

    // A separate credential-free probe checks the daemon host, not Core's
    // filesystem. This also avoids Desktop silently creating missing bind paths.
    async fn validate_local_bind_path(
        &self,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        citadel_backups::spec::validate_bind_path(path).map_err(|error| error.to_string())?;
        let name = format!("citadel-bind-probe-{}", Uuid::now_v7().simple());
        let output = self
            .execute(
                vec![
                    "run".into(),
                    "--rm".into(),
                    "--name".into(),
                    name.clone().into(),
                    "--label".into(),
                    "com.citadel.system=true".into(),
                    "--label".into(),
                    "com.citadel.system-role=backup-helper".into(),
                    "--mount".into(),
                    "type=bind,source=/,target=/source,readonly".into(),
                    "--network".into(),
                    "none".into(),
                    "--read-only".into(),
                    "--cap-drop".into(),
                    "ALL".into(),
                    "--cap-add".into(),
                    "DAC_READ_SEARCH".into(),
                    "--security-opt".into(),
                    "no-new-privileges".into(),
                    "--entrypoint".into(),
                    "/bin/sh".into(),
                    self.image.clone().into(),
                    "-c".into(),
                    BIND_PATH_PROBE.into(),
                    "sh".into(),
                    path.into(),
                ],
                vec![],
                Duration::from_secs(30),
                cancellation,
            )
            .await;
        self.cleanup(&name).await;
        output.map(|_| ())
    }

    async fn validate_agent_bind_path(
        &self,
        agent: &AgentExecutionClient,
        platform: Uuid,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        citadel_backups::spec::validate_bind_path(path).map_err(|error| error.to_string())?;
        let id = self
            .create_agent_helper(
                agent,
                platform,
                Some(("/", true, true, "/source")),
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        let result = async {
            agent
                .change_containers_state(
                    std::slice::from_ref(&id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            let output = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: id.clone(),
                        cmd: vec![
                            "/bin/sh".into(),
                            "-c".into(),
                            BIND_PATH_PROBE.into(),
                            "sh".into(),
                            path.into(),
                        ],
                        env: Default::default(),
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        tty: false,
                    },
                    Duration::from_secs(30),
                    self.maximum_output,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            if output.exit_code == 0 {
                Ok(())
            } else {
                Err(redact(&String::from_utf8_lossy(&output.stderr)))
            }
        }
        .await;
        let _ = agent.delete_container(&id, &CancellationToken::new()).await;
        result
    }

    async fn bind_mount_argument(
        &self,
        path: &str,
        target: &str,
        read_only: bool,
        cancellation: &CancellationToken,
    ) -> Result<String, String> {
        let output = self
            .execute(
                vec![
                    "version".into(),
                    "--format".into(),
                    "{{.Client.Version}}".into(),
                ],
                vec![],
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        let version = String::from_utf8_lossy(&output.stdout);
        let major = version
            .trim()
            .split('.')
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or("Could not determine Docker CLI version for nonrecursive bind mounts.")?;
        let recursive = if major >= 25 {
            "bind-recursive=disabled"
        } else {
            "bind-nonrecursive=true"
        };
        Ok(format!(
            "type=bind,source={path},target={target},{recursive}{}",
            if read_only { ",readonly" } else { "" }
        ))
    }

    async fn run_bind_restore(
        &self,
        target: BackupExecutionTarget,
        claim: &RestoreClaim,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        if !claim.source_item.as_ref().map_or_else(
            || {
                matches!(
                    claim.source.source_snapshot,
                    citadel_backups::spec::BackupSourceSpec::BindMount { .. }
                )
            },
            |item| item.host_path.is_some(),
        ) {
            return Err("A host-path restore requires a bind-mount snapshot.".into());
        }
        let snapshot = claim
            .source_item
            .as_ref()
            .and_then(|item| item.restic_snapshot_id.as_deref())
            .or(claim.source.restic_snapshot_id.as_deref())
            .ok_or("Backup has no snapshot ID.")?;
        let overwrite = if claim.run.overwrite_existing {
            "true"
        } else {
            "false"
        };
        match target {
            BackupExecutionTarget::Local => {
                self.validate_local_bind_path(path, cancellation).await?;
                let name = format!("citadel-restore-{}", claim.run.id.simple());
                let (mut args, env) = self.base_args(&name, &claim.repository).await?;
                args.extend([
                    "--mount".into(),
                    self.bind_mount_argument(path, "/target", false, cancellation)
                        .await?
                        .into(),
                    "--entrypoint".into(),
                    "/bin/sh".into(),
                    self.image.clone().into(),
                    "-c".into(),
                    BIND_RESTORE_SCRIPT.into(),
                    "sh".into(),
                    snapshot.into(),
                    overwrite.into(),
                ]);
                let output = self
                    .execute(args, env, self.restore_timeout(), cancellation)
                    .await;
                self.cleanup(&name).await;
                output
                    .map(|output| output_logs(&output, self.log_line_limit(), self.maximum_output))
            }
            BackupExecutionTarget::Agent(agent) => {
                self.validate_agent_bind_path(
                    &agent,
                    claim.run.target_platform_id,
                    path,
                    cancellation,
                )
                .await?;
                let timeout = self.restore_timeout();
                let id = self
                    .create_agent_helper(
                        &agent,
                        claim.run.target_platform_id,
                        Some((path, false, true, "/target")),
                        timeout,
                        cancellation,
                    )
                    .await?;
                let result = async {
                    agent
                        .change_containers_state(
                            std::slice::from_ref(&id),
                            AgentContainerAction::Start,
                            cancellation,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    let env = self.agent_repository_environment(&claim.repository).await?;
                    let output = self
                        .exec_agent(
                            &agent,
                            ExecBinaryRequest {
                                container_id: id.clone(),
                                cmd: vec![
                                    "/bin/sh".into(),
                                    "-c".into(),
                                    BIND_RESTORE_SCRIPT.into(),
                                    "sh".into(),
                                    snapshot.into(),
                                    overwrite.into(),
                                ],
                                env,
                                attach_stdout: Some(true),
                                attach_stderr: Some(true),
                                tty: false,
                            },
                            timeout,
                            self.maximum_output,
                            cancellation,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if output.exit_code != 0 {
                        return Err(redact(&String::from_utf8_lossy(&output.stderr)));
                    }
                    Ok(agent_output_logs(
                        &output.stdout,
                        &output.stderr,
                        self.log_line_limit(),
                        self.maximum_output,
                    ))
                }
                .await;
                let _ = agent.delete_container(&id, &CancellationToken::new()).await;
                result
            }
        }
    }

    async fn agent_snapshot_root(
        &self,
        agent: &AgentExecutionClient,
        platform: Uuid,
        snapshot: &str,
        environment: &std::collections::HashMap<String, String>,
        cancellation: &CancellationToken,
    ) -> Result<(String, String), String> {
        let container_id = self
            .create_agent_helper(agent, platform, None, Duration::from_secs(60), cancellation)
            .await?;
        let result = async {
            agent
                .change_containers_state(
                    std::slice::from_ref(&container_id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            let metadata = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: container_id.clone(),
                        cmd: vec![
                            "restic".into(),
                            "snapshots".into(),
                            "--json".into(),
                            snapshot.into(),
                        ],
                        env: environment.clone(),
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        tty: false,
                    },
                    Duration::from_secs(60),
                    self.maximum_output,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            if metadata.exit_code != 0 {
                return Err(redact(&String::from_utf8_lossy(&metadata.stderr)));
            }
            snapshot_root(&metadata.stdout, snapshot)
        }
        .await;
        let _ = agent
            .delete_container(&container_id, &CancellationToken::new())
            .await;
        result
    }

    async fn run_agent_restore(
        &self,
        agent: &AgentExecutionClient,
        claim: &RestoreClaim,
        cancellation: &CancellationToken,
    ) -> Result<Vec<BackupLog>, String> {
        let snapshot = claim
            .source_item
            .as_ref()
            .and_then(|item| item.restic_snapshot_id.as_deref())
            .or(claim.source.restic_snapshot_id.as_deref())
            .ok_or_else(|| "Backup Run has no Restic snapshot ID.".to_owned())?;
        let environment = self.agent_repository_environment(&claim.repository).await?;
        let (snapshot, root) = self
            .agent_snapshot_root(
                agent,
                claim.run.target_platform_id,
                snapshot,
                &environment,
                cancellation,
            )
            .await?;
        let mount_target = format!("/target{root}");
        let existing = agent
            .volume_exists(&claim.run.target_volume_name, cancellation)
            .await
            .map_err(|error| error.to_string())?;
        if existing && !claim.run.overwrite_existing {
            return Err("Target Volume already exists.".to_owned());
        }
        let owner = RestoreVolumeOwner::new(claim.run.target_platform_id, claim.run.id);
        if !existing {
            let volume = agent
                .create_volume(&owner.request(&claim.run.target_volume_name), cancellation)
                .await
                .map_err(|error| error.to_string())?;
            owner.verify(&claim.run.target_volume_name, &volume)?;
        }
        let timeout = self.restore_timeout();
        let container_id = self
            .create_agent_helper(
                agent,
                claim.run.target_platform_id,
                Some((&claim.run.target_volume_name, false, false, &mount_target)),
                timeout,
                cancellation,
            )
            .await?;
        let result = async {
            if !existing {
                let volume = agent
                    .inspect_volume(&claim.run.target_volume_name, cancellation)
                    .await
                    .map_err(|error| error.to_string())?;
                owner.verify(&claim.run.target_volume_name, &volume)?;
            }

            let containers = agent
                .list_containers(cancellation)
                .await
                .map_err(|error| error.to_string())?;
            super::restore_storage::ensure_unused(
                &claim.run.target_volume_name,
                &container_id,
                &containers,
            )?;

            agent
                .change_containers_state(
                    std::slice::from_ref(&container_id),
                    AgentContainerAction::Start,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            let output = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: container_id.clone(),
                        cmd: vec![
                            "restic".to_owned(),
                            "restore".to_owned(),
                            snapshot,
                            "--target".to_owned(),
                            "/target".to_owned(),
                            "--delete".to_owned(),
                        ],
                        env: environment,
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        tty: false,
                    },
                    timeout,
                    self.maximum_output,
                    cancellation,
                )
                .await
                .map_err(|error| error.to_string())?;
            if output.exit_code == 0 {
                Ok(agent_output_logs(
                    &output.stdout,
                    &output.stderr,
                    self.log_line_limit(),
                    self.maximum_output,
                ))
            } else {
                Err(redact(&String::from_utf8_lossy(&output.stderr)))
            }
        }
        .await;
        let _ = agent
            .delete_container(&container_id, &CancellationToken::new())
            .await;
        result
    }

    async fn execution_target(
        &self,
        platform_id: Uuid,
        node_id: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<BackupExecutionTarget, String> {
        let (connector, address, status) =
            crate::persistence::postgres::platforms::connection::execution_target(
                self.targets.pool(),
                platform_id,
            )
            .await
            .map_err(|error| format!("Platform lookup failed: {error}"))?
            .ok_or_else(|| "The selected Platform was not found.".to_owned())?;
        if status != "Online" {
            return Err("The selected Platform is disconnected or unavailable.".to_owned());
        }
        if let Some(node_id) = node_id {
            let target = EdgeTarget::node(platform_id, node_id.to_owned());
            if let Ok(session) = self.edge.get(&target) {
                return Ok(BackupExecutionTarget::Agent(AgentExecutionClient::Edge(
                    session,
                )));
            }
        }
        if connector == "EdgeAgent" {
            let session = self
                .edge
                .get(&EdgeTarget::platform(platform_id))
                .map_err(|error| error.to_string())?;
            if let Some(expected) = node_id {
                let info = crate::connectors::edge::EdgeRuntime {
                    session: session.clone(),
                }
                .get_info(cancellation)
                .await
                .map_err(|error| error.to_string())?;
                if info.swarm.as_ref().map(|swarm| swarm.node_id.as_str()) != Some(expected) {
                    return Err(
                        "The connected Edge Agent is not on the required Swarm Node.".into(),
                    );
                }
            }
            return Ok(BackupExecutionTarget::Agent(AgentExecutionClient::Edge(
                session,
            )));
        }
        if connector == "Local" {
            if let Some(expected) = node_id {
                let actual = self
                    .execute(
                        vec!["info".into(), "--format".into(), "{{.Swarm.NodeID}}".into()],
                        vec![],
                        Duration::from_secs(30),
                        cancellation,
                    )
                    .await?;
                if !actual.succeeded() || String::from_utf8_lossy(&actual.stdout).trim() != expected
                {
                    return Err(
                        "The selected Swarm Node Agent is disconnected or unavailable.".into(),
                    );
                }
            }
            return Ok(BackupExecutionTarget::Local);
        }
        if connector != "Agent" {
            return Err(format!(
                "Platform connector '{connector}' does not support backup execution."
            ));
        }
        let agent = self
            .agent
            .as_ref()
            .ok_or_else(|| "The configured Agent backup transport is unavailable.".to_owned())?
            .at_address(&address)
            .map_err(|error| error.message)?;
        if let Some(expected_node) = node_id {
            let info = agent
                .handshake(cancellation)
                .await
                .map_err(|error| error.to_string())?;
            let actual = info
                .swarm
                .as_ref()
                .map(|swarm| swarm.node_id.as_str())
                .filter(|node| !node.is_empty());
            if actual != Some(expected_node) {
                return Err(format!(
                    "The configured Agent is not connected to required Swarm Node '{expected_node}'."
                ));
            }
        }
        Ok(BackupExecutionTarget::Agent(AgentExecutionClient::Direct(
            Arc::new(agent.clone()),
        )))
    }

    async fn create_agent_helper(
        &self,
        agent: &AgentExecutionClient,
        platform_id: Uuid,
        volume: Option<(&str, bool, bool, &str)>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<String, String> {
        // Packaged Agents include Restic. Use the image already installed on
        // this daemon instead of requiring Core's helper image on every node.
        let runtime_image = agent
            .runtime_image(cancellation)
            .await
            .map_err(|error| format!("Could not resolve the Agent backup helper image: {error}"))?;
        let image = if runtime_image.trim().is_empty() {
            &self.image
        } else {
            &runtime_image
        };
        let lifetime = timeout.as_secs().saturating_add(300).clamp(300, 86_700);
        let mut capabilities = vec!["DAC_READ_SEARCH".to_owned(), "FOWNER".to_owned()];
        if volume.is_some_and(|(_, read_only, _, _)| !read_only) {
            // Restic restores the snapshot's numeric file and symlink owners.
            capabilities.push("CHOWN".to_owned());
        }
        agent
            .create_container(
                CreateContainerRequest {
                    platform_address: String::new(),
                    image_id: image.clone(),
                    name: format!("citadel-backup-helper-{}", Uuid::now_v7().simple()),
                    working_dir: Some("/tmp".to_owned()),
                    user: Some("0".to_owned()),
                    memory_limit: Some(512 * 1024 * 1024),
                    cpu_quota: None,
                    memory_reservation: None,
                    auto_remove: Some(true),
                    restart_policy: RestartPolicy::Empty as i32,
                    labels: [
                        ("citadel.backup-helper".to_owned(), "true".to_owned()),
                        ("citadel.platform-id".to_owned(), platform_id.to_string()),
                        ("com.citadel.system".to_owned(), "true".to_owned()),
                        ("com.citadel.system-role".to_owned(), "backup-helper".to_owned()),
                        ("com.citadel.platform-id".to_owned(), platform_id.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    env_vars: Vec::new(),
                    ports: Vec::new(),
                    volumes: Vec::new(),
                    networks: Default::default(),
                    entry_point: vec!["/bin/sh".to_owned()],
                    command: vec![
                        "-c".to_owned(),
                        format!("trap 'exit 0' TERM INT; sleep {lifetime}"),
                    ],
                    memory_swap: Some(512 * 1024 * 1024),
                    pids_limit: Some(128),
                    privileged: Some(false),
                    readonly_rootfs: Some(volume.is_some_and(|(name, _, bind, _)| bind && name == "/")),
                    mounts: volume.into_iter().map(|(name, read_only, bind, target)| Mount {
                        target: Some(target.to_owned()),
                        source: Some(name.to_owned()),
                        r#type: Some(if bind { "bind" } else { "volume" }.to_owned()),
                        read_only: Some(read_only),
                        consistency: None,
                        bind_options: bind.then_some(citadel_contracts::citadel::shared_models::v1::BindOptions {
                            propagation: Some("rprivate".into()), non_recursive: Some(name != "/"), create_mountpoint: Some(false), ..Default::default()
                        }),
                        volume_options: None,
                    }).collect(),
                    cap_add: capabilities,
                    cap_drop: vec!["ALL".to_owned()],
                    security_opt: vec!["no-new-privileges".to_owned()],
                    network_mode: volume.filter(|(name, _, bind, _)| *bind && *name == "/").map(|_| "none".into()),
                },
                cancellation,
            )
            .await
            .map_err(|error| {
                format!(
                    "Backup helper image '{}' is unavailable on the target Agent or the helper could not be created: {error}",
                    image
                )
            })
    }

    async fn agent_repository_environment(
        &self,
        repository: &BackupRepository,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let BackupRepositorySpec::S3Compatible {
            endpoint,
            bucket,
            prefix,
            access_key_secret_id,
            secret_key_secret_id,
            session_token_secret_id,
            ..
        } = &repository.spec
        else {
            return Err(
                "Agent backup execution currently requires an S3-compatible Backup Repository."
                    .to_owned(),
            );
        };
        let endpoint = endpoint.trim().trim_end_matches('/');
        let prefix = prefix.as_deref().unwrap_or("").trim_matches('/');
        let location = if prefix.is_empty() {
            format!("s3:{endpoint}/{bucket}")
        } else {
            format!("s3:{endpoint}/{bucket}/{prefix}")
        };
        let mut environment = std::collections::HashMap::from([
            (
                "RESTIC_PASSWORD".to_owned(),
                self.secrets
                    .resolve(repository.password_secret_id)
                    .await
                    .map_err(|error| error.to_string())?
                    .to_string(),
            ),
            ("RESTIC_REPOSITORY".to_owned(), location),
        ]);
        for (id, key) in [
            (*access_key_secret_id, "AWS_ACCESS_KEY_ID"),
            (*secret_key_secret_id, "AWS_SECRET_ACCESS_KEY"),
        ] {
            environment.insert(
                key.to_owned(),
                self.secrets
                    .resolve(id)
                    .await
                    .map_err(|error| error.to_string())?
                    .to_string(),
            );
        }
        if let Some(id) = *session_token_secret_id {
            environment.insert(
                "AWS_SESSION_TOKEN".to_owned(),
                self.secrets
                    .resolve(id)
                    .await
                    .map_err(|error| error.to_string())?
                    .to_string(),
            );
        }
        Ok(environment)
    }
    async fn base_args(
        &self,
        name: &str,
        repo: &BackupRepository,
    ) -> Result<(Vec<OsString>, Vec<(OsString, OsString)>), String> {
        let password = self
            .secrets
            .resolve(repo.password_secret_id)
            .await
            .map_err(|e| e.to_string())?;
        let mut args = vec![
            "run".into(),
            "--rm".into(),
            "--name".into(),
            name.into(),
            "--label".into(),
            "com.citadel.system=true".into(),
            "--label".into(),
            "com.citadel.system-role=backup-helper".into(),
            "--env".into(),
            "RESTIC_PASSWORD".into(),
            "--env".into(),
            "RESTIC_REPOSITORY".into(),
        ];
        let mut env = vec![("RESTIC_PASSWORD".into(), OsString::from(password.as_str()))];
        match &repo.spec {
            BackupRepositorySpec::FileSystem { location, path, .. } => {
                let path = if *location == BackupExecutionLocation::Core {
                    self.core_repository_path(path)?.display().to_string()
                } else {
                    path.to_owned()
                };
                args.extend(["--volume".into(), format!("{path}:/repository").into()]);
                env.push(("RESTIC_REPOSITORY".into(), "/repository".into()));
            }
            BackupRepositorySpec::S3Compatible {
                endpoint,
                bucket,
                prefix,
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                ..
            } => {
                let endpoint = endpoint.trim().trim_end_matches('/');
                let prefix = prefix.as_deref().unwrap_or("").trim_matches('/');
                let repository = if prefix.is_empty() {
                    format!("s3:{endpoint}/{bucket}")
                } else {
                    format!("s3:{endpoint}/{bucket}/{prefix}")
                };
                env.push(("RESTIC_REPOSITORY".into(), repository.into()));
                for (id, key) in [
                    (*access_key_secret_id, "AWS_ACCESS_KEY_ID"),
                    (*secret_key_secret_id, "AWS_SECRET_ACCESS_KEY"),
                ] {
                    let secret = self.secrets.resolve(id).await.map_err(|e| e.to_string())?;
                    args.extend(["--env".into(), key.into()]);
                    env.push((key.into(), OsString::from(secret.as_str())));
                }
                if let Some(id) = *session_token_secret_id {
                    let secret = self.secrets.resolve(id).await.map_err(|e| e.to_string())?;
                    args.extend(["--env".into(), "AWS_SESSION_TOKEN".into()]);
                    env.push(("AWS_SESSION_TOKEN".into(), OsString::from(secret.as_str())));
                }
            }
        }
        Ok((args, env))
    }

    async fn direct_repository_environment(
        &self,
        repository: &BackupRepository,
    ) -> Result<(OsString, Vec<(OsString, OsString)>), String> {
        let password = self
            .secrets
            .resolve(repository.password_secret_id)
            .await
            .map_err(|error| error.to_string())?;
        let mut environment = vec![("RESTIC_PASSWORD".into(), OsString::from(password.as_str()))];
        let location = match &repository.spec {
            BackupRepositorySpec::FileSystem { location, path, .. } => {
                if *location != BackupExecutionLocation::Core {
                    return Err(
                        "Citadel system backup requires a Core or S3-compatible repository.".into(),
                    );
                }
                self.core_repository_path(path)?.into_os_string()
            }
            BackupRepositorySpec::S3Compatible {
                endpoint,
                bucket,
                prefix,
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                ..
            } => {
                let endpoint = endpoint.trim().trim_end_matches('/');
                let prefix = prefix.as_deref().unwrap_or("").trim_matches('/');
                for (id, key) in [
                    (*access_key_secret_id, "AWS_ACCESS_KEY_ID"),
                    (*secret_key_secret_id, "AWS_SECRET_ACCESS_KEY"),
                ] {
                    let value = self
                        .secrets
                        .resolve(id)
                        .await
                        .map_err(|error| error.to_string())?;
                    environment.push((key.into(), OsString::from(value.as_str())));
                }
                if let Some(id) = *session_token_secret_id {
                    let value = self
                        .secrets
                        .resolve(id)
                        .await
                        .map_err(|error| error.to_string())?;
                    environment.push(("AWS_SESSION_TOKEN".into(), OsString::from(value.as_str())));
                }
                if prefix.is_empty() {
                    format!("s3:{endpoint}/{bucket}").into()
                } else {
                    format!("s3:{endpoint}/{bucket}/{prefix}").into()
                }
            }
        };
        Ok((location, environment))
    }

    async fn validate_repository_target(
        &self,
        repository: &BackupRepository,
        location: &str,
        platform_id: Option<Uuid>,
    ) -> Result<(), String> {
        if let BackupRepositorySpec::FileSystem {
            location: BackupExecutionLocation::Core,
            path,
            ..
        } = &repository.spec
        {
            self.core_repository_path(path)?;
        }
        match location {
            "Core" if platform_id.is_none() => {}
            "Platform" => {
                let platform_id = platform_id
                    .ok_or_else(|| "A Platform is required for this operation.".to_owned())?;
                if repository.repository_type != "S3Compatible" {
                    self.targets.require_local(platform_id).await?;
                }
            }
            _ => return Err("Backup Repository operation location is invalid.".to_owned()),
        }
        if let BackupRepositorySpec::FileSystem {
            location: BackupExecutionLocation::Platform,
            platform_id: repository_platform,
            ..
        } = &repository.spec
        {
            let repository_platform =
                repository_platform.ok_or_else(|| "Repository Platform is required.".to_owned())?;
            self.targets.require_local(repository_platform).await?;
            if location != "Platform" || platform_id != Some(repository_platform) {
                return Err(
                    "Filesystem Backup Repository operations must run on their configured Platform."
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
    async fn execute(
        &self,
        args: Vec<OsString>,
        env: Vec<(OsString, OsString)>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<citadel_execution::ProcessOutput, String> {
        let mut request =
            ProcessRequest::new(self.docker.clone())
                .args(args)
                .limits(ProcessLimits {
                    timeout,
                    maximum_stdout_bytes: self.maximum_output,
                    maximum_stderr_bytes: self.maximum_output,
                    output_limit_policy: OutputLimitPolicy::Truncate,
                });
        for (key, value) in env {
            request = request.env(key, value);
        }
        let output = self
            .run_observed(request, cancellation)
            .await
            .map_err(|e| e.to_string())?;
        if output.succeeded() {
            Ok(output)
        } else {
            Err(redact(&String::from_utf8_lossy(&output.stderr)))
        }
    }
    async fn run_observed(
        &self,
        mut request: ProcessRequest,
        cancellation: &CancellationToken,
    ) -> Result<citadel_execution::ProcessOutput, citadel_execution::ProcessError> {
        if let Some(settings) = &self.settings {
            tokio::fs::create_dir_all(&settings.working_directory)
                .await
                .map_err(citadel_execution::ProcessError::Io)?;
            if request.current_directory.is_none() {
                request.current_directory = Some(settings.working_directory.clone());
            }
        }
        let Some(progress) = &self.progress else {
            return run(request, cancellation).await;
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(32);
        let (result, ()) = tokio::join!(
            run(request.output(sender), cancellation),
            observe_output(
                receiver,
                progress,
                self.log_line_limit(),
                self.maximum_output
            )
        );
        result
    }
    async fn exec_agent(
        &self,
        agent: &AgentExecutionClient,
        request: ExecBinaryRequest,
        timeout: Duration,
        maximum: usize,
        cancellation: &CancellationToken,
    ) -> Result<
        crate::connectors::agent::client::AgentBinaryExecOutput,
        citadel_platforms::RuntimeCapabilityError,
    > {
        let Some(progress) = &self.progress else {
            return agent
                .exec_binary(request, timeout, maximum, cancellation)
                .await;
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(32);
        let (result, ()) = tokio::join!(
            agent.exec_binary_observed(request, timeout, maximum, cancellation, Some(sender)),
            observe_output(
                receiver,
                progress,
                self.log_line_limit(),
                self.maximum_output
            )
        );
        result
    }
    async fn cleanup(&self, name: &str) {
        let _ = run(
            ProcessRequest::new(self.docker.clone())
                .args(["rm", "--force", name])
                .limits(ProcessLimits {
                    timeout: Duration::from_secs(30),
                    maximum_stdout_bytes: 4096,
                    maximum_stderr_bytes: 4096,
                    output_limit_policy: OutputLimitPolicy::Truncate,
                }),
            &CancellationToken::new(),
        )
        .await;
    }
}

fn restore_helper_identity(output: &citadel_execution::ProcessOutput) -> Result<String, String> {
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.stdout_truncated || id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Restore helper creation returned no valid container identity.".into());
    }
    Ok(id)
}

// Frame stdout/stderr independently before redaction: a split credential keyword
// must never bypass the same redaction used by persisted logs. Oversized lines
// are discarded rather than retained without a bound or partially disclosed.
async fn observe_output(
    mut receiver: tokio::sync::mpsc::Receiver<citadel_execution::ProcessChunk>,
    progress: &Arc<dyn Fn(BackupLog) + Send + Sync>,
    line_limit: usize,
    maximum_log_bytes: usize,
) {
    let mut budget = maximum_log_bytes;
    let mut emit = |log: BackupLog| {
        let cost = log.message.len() + log.stream.len();
        if let Some(remaining) = budget.checked_sub(cost) {
            budget = remaining;
            progress(log);
        }
    };
    let mut lines = [(Vec::new(), false), (Vec::new(), false)];
    while let Some(chunk) = receiver.recv().await {
        let index = usize::from(chunk.stream == "stderr");
        let (line, discard) = &mut lines[index];
        for byte in chunk.bytes {
            if byte == b'\n' {
                let message = if *discard {
                    "[oversized output line omitted]".into()
                } else {
                    redact(&String::from_utf8_lossy(line))
                };
                emit(BackupLog {
                    stream: chunk.stream.into(),
                    message,
                });
                line.clear();
                *discard = false;
            } else if !*discard {
                if line.len() < line_limit {
                    line.push(byte);
                } else {
                    line.clear();
                    *discard = true;
                }
            }
        }
    }
    for (index, (line, discard)) in lines.into_iter().enumerate() {
        if discard || !line.is_empty() {
            emit(BackupLog {
                stream: if index == 1 { "stderr" } else { "stdout" }.into(),
                message: if discard {
                    "[oversized output line omitted]".into()
                } else {
                    redact(&String::from_utf8_lossy(&line))
                },
            });
        }
    }
}

fn listed_volume_exists(
    output: &citadel_execution::ProcessOutput,
    name: &str,
) -> Result<bool, String> {
    if !output.succeeded() || output.stdout_truncated || output.stderr_truncated {
        return Err("Could not determine whether the target Volume exists.".into());
    }
    let names = std::str::from_utf8(&output.stdout)
        .map_err(|_| "Docker returned an invalid Volume listing.".to_owned())?;
    Ok(names.lines().any(|value| value == name))
}

/// Local and remote historical backups used different helper mount paths.
/// Read the snapshot's actual path, rather than guessing from the restore Node.
fn snapshot_root(metadata: &[u8], expected_id: &str) -> Result<(String, String), String> {
    let invalid = || "Backup snapshot has an invalid or unsupported Volume root.".to_owned();
    let snapshots: Vec<Value> = serde_json::from_slice(metadata).map_err(|_| invalid())?;
    let [snapshot] = snapshots.as_slice() else {
        return Err(invalid());
    };
    let id = snapshot
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if !(8..=64).contains(&expected_id.len())
        || !expected_id.bytes().all(|b| b.is_ascii_hexdigit())
        || id.len() != 64
        || !id.bytes().all(|b| b.is_ascii_hexdigit())
        || !id.starts_with(expected_id)
    {
        return Err(invalid());
    }
    let paths = snapshot
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    let [path] = paths.as_slice() else {
        return Err(invalid());
    };
    let path = path
        .as_str()
        .filter(|path| matches!(*path, "/source" | "/data"))
        .ok_or_else(invalid)?;
    Ok((id.to_owned(), path.to_owned()))
}

fn sum_item_metric(
    items: &[BackupRunItemResult],
    select: impl Fn(&BackupRunItemResult) -> Option<i64>,
) -> Option<i64> {
    let mut found = false;
    let sum = items
        .iter()
        .filter_map(select)
        .inspect(|_| found = true)
        .sum();
    found.then_some(sum)
}

#[derive(Debug, Default)]
struct ResticSummary {
    parent_snapshot_id: Option<String>,
    files_processed: Option<i64>,
    bytes_processed: Option<i64>,
    bytes_added: Option<i64>,
}

impl ResticSummary {
    fn from_json(value: &Value) -> Self {
        Self {
            parent_snapshot_id: value
                .get("parent_snapshot_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            files_processed: value.get("total_files_processed").and_then(Value::as_i64),
            bytes_processed: value.get("total_bytes_processed").and_then(Value::as_i64),
            bytes_added: value.get("data_added").and_then(Value::as_i64),
        }
    }
}

fn parse_restic_summary(bytes: &[u8]) -> Option<(Option<String>, ResticSummary)> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("message_type").and_then(Value::as_str) == Some("summary"))
        .map(|value| {
            let snapshot = value
                .get("snapshot_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let summary = ResticSummary::from_json(&value);
            (snapshot, summary)
        })
}

fn failed_backup(
    error: String,
    cancelled: bool,
    logs: Vec<BackupLog>,
    items: Vec<BackupRunItemResult>,
) -> BackupExecutionResult {
    BackupExecutionResult {
        status: if cancelled {
            citadel_backups::BackupRunStatus::Cancelled
        } else {
            citadel_backups::BackupRunStatus::Failed
        },
        snapshot_availability: citadel_backups::BackupSnapshotAvailability::NotCreated,
        restic_snapshot_id: None,
        parent_snapshot_id: None,
        files_processed: None,
        bytes_processed: None,
        bytes_added: None,
        exit_code: None,
        error_code: Some(
            if cancelled {
                "Cancelled"
            } else {
                "ResticFailed"
            }
            .into(),
        ),
        error_message: Some(error),
        warnings: vec![],
        logs,
        items,
    }
}

fn output_logs(
    output: &citadel_execution::ProcessOutput,
    line_limit: usize,
    budget: usize,
) -> Vec<BackupLog> {
    agent_output_logs(&output.stdout, &output.stderr, line_limit, budget)
}
fn agent_output_logs(
    stdout: &[u8],
    stderr: &[u8],
    line_limit: usize,
    mut budget: usize,
) -> Vec<BackupLog> {
    let mut logs = Vec::new();
    for (stream, data) in [("stdout", stdout), ("stderr", stderr)] {
        for line in String::from_utf8_lossy(data).lines().take(10_000) {
            let message = if line.len() > line_limit {
                "[oversized output line omitted]".into()
            } else {
                redact(line)
            };
            let cost = message.len() + stream.len();
            if cost > budget {
                return logs;
            }
            budget -= cost;
            logs.push(BackupLog {
                stream: stream.into(),
                message,
            });
        }
    }
    logs
}

fn redact(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            if line.to_ascii_lowercase().contains("password")
                || line.contains("AWS_SECRET_ACCESS_KEY")
            {
                "[redacted]".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn policy_tag(id: Uuid) -> String {
    format!("citadel-policy-{id}")
}

fn validate_bind_repository(repository: &BackupRepository, path: &str) -> Result<(), String> {
    if let BackupRepositorySpec::FileSystem {
        path: repository_path,
        ..
    } = &repository.spec
    {
        let source = std::path::Path::new(path);
        let repository = std::path::Path::new(repository_path);
        if repository.is_absolute()
            && (source.starts_with(repository) || repository.starts_with(source))
        {
            return Err("Bind path and filesystem backup repository must not overlap.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn core_repository_paths_are_validated_before_secrets_or_process_execution() {
        struct NoSecrets;
        impl BackupSecretResolver for NoSecrets {
            fn resolve(
                &self,
                _: Uuid,
            ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>> {
                panic!("Rejected paths must not resolve secrets or run Restic")
            }
        }
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap();
        let executor = DockerResticBackupExecutor::new(
            "must-not-run",
            "unused",
            Arc::new(NoSecrets),
            4096,
            pool,
        )
        .with_settings(super::super::settings::BackupExecutionSettings {
            working_directory: std::env::temp_dir(),
            allowed_core_paths: vec![std::env::temp_dir().join("allowed-backup-root")],
            default_timeout: Duration::from_secs(120),
            maximum_log_line_bytes: 8192,
        });
        let now = chrono::Utc::now();
        let repository = BackupRepository {
            id: Uuid::now_v7(),
            name: "test".into(),
            normalized_name: "test".into(),
            description: None,
            repository_type: "FileSystem".into(),
            spec: serde_json::from_value(
                serde_json::json!({"$type":"FileSystem","location":"Core","path":"../outside"}),
            )
            .unwrap(),
            password_secret_id: Uuid::now_v7(),
            status: citadel_backups::BackupRepositoryStatus::Unknown,
            control_state: citadel_primitives::ResourceControlState::Idle,
            current_run_id: None,
            control_started_at: None,
            last_pruned_at: None,
            last_checked_at: None,

            updated_at: now,
            archived_at: None,
            row_version: 1,

            audit: citadel_primitives::AuditMetadata {
                created_at: now,
                created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
            },
        };
        let error = executor
            .repository(
                &repository,
                "Initialize",
                "Core",
                None,
                &CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, BackupError::Validation(_)));
    }

    #[tokio::test]
    async fn progress_frames_split_credentials_and_bounds_oversized_lines() {
        let output = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = output.clone();
        let sink: Arc<dyn Fn(BackupLog) + Send + Sync> =
            Arc::new(move |log| capture.lock().unwrap().push(log));
        let (sender, receiver) = tokio::sync::mpsc::channel(4);
        sender
            .send(citadel_execution::ProcessChunk {
                stream: "stderr",
                bytes: b"pass".to_vec(),
            })
            .await
            .unwrap();
        sender
            .send(citadel_execution::ProcessChunk {
                stream: "stderr",
                bytes: b"word=never-expose\n".to_vec(),
            })
            .await
            .unwrap();
        sender
            .send(citadel_execution::ProcessChunk {
                stream: "stdout",
                bytes: [vec![b'x'; 9000], b"\ncomplete\n".to_vec()].concat(),
            })
            .await
            .unwrap();
        drop(sender);
        observe_output(receiver, &sink, 8192, 1024 * 1024).await;
        let output = output.lock().unwrap();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0].message, "[redacted]");
        assert_eq!(output[1].message, "[oversized output line omitted]");
        assert_eq!(output[2].message, "complete");
    }
    #[test]
    fn configured_log_limits_apply_across_both_output_streams() {
        let logs = agent_output_logs(b"1234567890\nokay\n", b"last\n", 8, 50);
        assert_eq!(logs[0].message, "[oversized output line omitted]");
        assert_eq!(logs[1].message, "okay");
        assert_eq!(logs.len(), 2);
        assert!(
            logs.iter()
                .map(|log| log.message.len() + log.stream.len())
                .sum::<usize>()
                <= 50
        );
    }

    #[test]
    fn restores_the_recorded_volume_root_not_the_helper_directory() {
        let id = "a".repeat(64);
        for root in ["/source", "/data"] {
            let metadata =
                serde_json::to_vec(&serde_json::json!([{"id": id, "paths": [root]}])).unwrap();
            assert_eq!(
                snapshot_root(&metadata, &id).unwrap(),
                (id.clone(), root.to_owned())
            );
            assert_eq!(
                snapshot_root(&metadata, &id[..8]).unwrap(),
                (id.clone(), root.to_owned())
            );
        }
    }

    #[test]
    fn restore_requires_a_successful_complete_volume_listing() {
        let mut output = citadel_execution::ProcessOutput {
            exit_code: Some(0),
            stdout: b"target-other\ntarget\n".to_vec(),
            stderr: vec![],
            stdout_truncated: false,
            stderr_truncated: false,
        };
        assert_eq!(listed_volume_exists(&output, "target"), Ok(true));
        assert_eq!(listed_volume_exists(&output, "absent"), Ok(false));
        output.stdout_truncated = true;
        assert!(listed_volume_exists(&output, "absent").is_err());
        output.stdout_truncated = false;
        output.exit_code = Some(1);
        assert!(listed_volume_exists(&output, "absent").is_err());
        output.exit_code = Some(0);
        output.stdout = vec![0xff];
        assert!(listed_volume_exists(&output, "absent").is_err());
        output.stdout.clear();
        assert_eq!(listed_volume_exists(&output, "absent"), Ok(false));
    }

    #[test]
    fn restore_rejects_ambiguous_unknown_and_mismatched_snapshots() {
        let id = "a".repeat(64);
        for metadata in [
            serde_json::json!([]),
            serde_json::json!([{"id": id, "paths": ["/"]}]),
            serde_json::json!([{"id": id, "paths": ["/source", "/data"]}]),
            serde_json::json!([{"id": "b".repeat(64), "paths": ["/data"]}]),
            serde_json::json!([{"id": id, "paths": ["/data"]}, {"id": id, "paths": ["/data"]}]),
        ] {
            assert!(snapshot_root(&serde_json::to_vec(&metadata).unwrap(), &id).is_err());
        }
        assert!(snapshot_root(b"truncated JSON", &id).is_err());
    }

    #[test]
    fn redaction_preserves_lines_and_removes_credentials() {
        assert_eq!(redact("ok\npassword=abc\ndone"), "ok\n[redacted]\ndone");
    }
}
