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

use sqlx::{PgPool, Row};

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
                            if warnings.is_empty() {
                                "Succeeded"
                            } else {
                                "SucceededWithWarnings"
                            }
                        }
                        Err(error) => {
                            warnings.push(format!(
                                "Snapshot was created, but retention failed: {error}"
                            ));
                            "SucceededWithWarnings"
                        }
                    };
                    BackupExecutionResult {
                        status,
                        snapshot_availability: "Available",
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
                    status: "Succeeded",
                    exit_code: Some(0),
                    error_code: None,
                    error_message: None,
                    logs,
                },
                Err(error) => RestoreExecutionResult {
                    status: if cancellation.is_cancelled() {
                        "Cancelled"
                    } else {
                        "Failed"
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
                        status: "Succeeded",
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
                            "Cancelled"
                        } else {
                            "Failed"
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
        let name = format!(
            "citadel-backup-{}-{}",
            claim.run.id.simple(),
            item.id.simple()
        );
        let (mut args, env) = self
            .base_args(&name, &claim.repository)
            .await
            .map_err(|error| (error, vec![]))?;
        args.extend([
            "--volume".into(),
            format!("{volume}:/data:ro").into(),
            self.image.clone().into(),
            "backup".into(),
            "--json".into(),
            "--tag".into(),
            policy_tag(claim.policy.id).into(),
            "--tag".into(),
            format!("backup-run:{}", claim.run.id).into(),
            "--tag".into(),
            format!("backup-item:{}", item.id).into(),
            "--tag".into(),
            format!("volume:{volume}").into(),
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
        let container_id = self
            .create_agent_helper(
                agent,
                item.platform_id,
                Some((&item.volume_name, true)),
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
            let command = vec![
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
                format!("volume:{}", item.volume_name),
                "/source".to_owned(),
            ];
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
        let subtree = snapshot_subtree(&metadata?.stdout, snapshot)?;
        let created = !existing;
        if created {
            self.execute(
                vec!["volume".into(), "create".into(), volume.into()],
                vec![],
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        }
        let (mut args, env) = self.base_args(&name, &claim.repository).await?;
        args.extend([
            "--volume".into(),
            format!("{volume}:/restore/data").into(),
            self.image.clone().into(),
            "restore".into(),
            subtree.into(),
            "--target".into(),
            "/restore/data".into(),
            "--delete".into(),
        ]);
        let output = self
            .execute(args, env, self.restore_timeout(), cancellation)
            .await;
        self.cleanup(&name).await;
        if output.is_err() && created {
            let _ = self
                .execute(
                    vec![
                        "volume".into(),
                        "rm".into(),
                        "--force".into(),
                        volume.into(),
                    ],
                    vec![],
                    Duration::from_secs(30),
                    &CancellationToken::new(),
                )
                .await;
        }
        output.map(|output| output_logs(&output, self.log_line_limit(), self.maximum_output))
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
        let existing = agent
            .volume_exists(&claim.run.target_volume_name, cancellation)
            .await
            .map_err(|error| error.to_string())?;
        if existing && !claim.run.overwrite_existing {
            return Err("Target Volume already exists.".to_owned());
        }
        let timeout = self.restore_timeout();
        let container_id = self
            .create_agent_helper(
                agent,
                claim.run.target_platform_id,
                Some((&claim.run.target_volume_name, false)),
                timeout,
                cancellation,
            )
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
            let environment = self.agent_repository_environment(&claim.repository).await?;
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
            let subtree = snapshot_subtree(&metadata.stdout, snapshot)?;
            let output = self
                .exec_agent(
                    agent,
                    ExecBinaryRequest {
                        container_id: container_id.clone(),
                        cmd: vec![
                            "restic".to_owned(),
                            "restore".to_owned(),
                            subtree,
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
        let row = sqlx::query("SELECT connectortype,address,status FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(self.targets.pool())
            .await
            .map_err(|error| format!("Platform lookup failed: {error}"))?
            .ok_or_else(|| "The selected Platform was not found.".to_owned())?;
        if row.try_get::<String, _>("status").map_err(storage_text)? != "Online" {
            return Err("The selected Platform is disconnected or unavailable.".to_owned());
        }
        let connector: String = row.try_get("connectortype").map_err(storage_text)?;
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
                use citadel_platforms::PlatformRuntimePort;
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
        let address: String = row.try_get("address").map_err(storage_text)?;
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
        volume: Option<(&str, bool)>,
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
        if volume.is_some_and(|(_, read_only)| !read_only) {
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
                    readonly_rootfs: Some(false),
                    mounts: volume.into_iter().map(|(name, read_only)| Mount {
                        target: Some(if read_only { "/source" } else { "/target" }.to_owned()),
                        source: Some(name.to_owned()),
                        r#type: Some("volume".to_owned()),
                        read_only: Some(read_only),
                        consistency: None,
                        bind_options: None,
                        volume_options: None,
                    }).collect(),
                    cap_add: capabilities,
                    cap_drop: vec!["ALL".to_owned()],
                    security_opt: vec!["no-new-privileges".to_owned()],
                    network_mode: None,
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
        if repository.repository_type != "S3Compatible" {
            return Err(
                "Agent backup execution currently requires an S3-compatible Backup Repository."
                    .to_owned(),
            );
        }
        let endpoint = required(&repository.spec, "endpoint")?
            .trim()
            .trim_end_matches('/');
        let bucket = required(&repository.spec, "bucket")?;
        let prefix = repository
            .spec
            .get("prefix")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim_matches('/');
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
        for (field, key) in [
            ("accessKeySecretId", "AWS_ACCESS_KEY_ID"),
            ("secretKeySecretId", "AWS_SECRET_ACCESS_KEY"),
        ] {
            environment.insert(
                key.to_owned(),
                self.secrets
                    .resolve(uuid_field(&repository.spec, field)?)
                    .await
                    .map_err(|error| error.to_string())?
                    .to_string(),
            );
        }
        if let Some(id) = repository
            .spec
            .get("sessionTokenSecretId")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
        {
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
        match repo.repository_type.as_str() {
            "FileSystem" => {
                let path = required(&repo.spec, "path")?;
                let path = if required(&repo.spec, "location")? == "Core" {
                    self.core_repository_path(path)?.display().to_string()
                } else {
                    path.to_owned()
                };
                args.extend(["--volume".into(), format!("{path}:/repository").into()]);
                env.push(("RESTIC_REPOSITORY".into(), "/repository".into()));
            }
            "S3Compatible" => {
                let endpoint = required(&repo.spec, "endpoint")?
                    .trim()
                    .trim_end_matches('/');
                let bucket = required(&repo.spec, "bucket")?;
                let prefix = repo
                    .spec
                    .get("prefix")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim_matches('/');
                let repository = if prefix.is_empty() {
                    format!("s3:{endpoint}/{bucket}")
                } else {
                    format!("s3:{endpoint}/{bucket}/{prefix}")
                };
                env.push(("RESTIC_REPOSITORY".into(), repository.into()));
                for (field, key) in [
                    ("accessKeySecretId", "AWS_ACCESS_KEY_ID"),
                    ("secretKeySecretId", "AWS_SECRET_ACCESS_KEY"),
                ] {
                    let secret = self
                        .secrets
                        .resolve(uuid_field(&repo.spec, field)?)
                        .await
                        .map_err(|e| e.to_string())?;
                    args.extend(["--env".into(), key.into()]);
                    env.push((key.into(), OsString::from(secret.as_str())));
                }
                if let Some(id) = repo
                    .spec
                    .get("sessionTokenSecretId")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok())
                {
                    let secret = self.secrets.resolve(id).await.map_err(|e| e.to_string())?;
                    args.extend(["--env".into(), "AWS_SESSION_TOKEN".into()]);
                    env.push(("AWS_SESSION_TOKEN".into(), OsString::from(secret.as_str())));
                }
            }
            other => return Err(format!("Backup Repository type '{other}' is unsupported.")),
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
        let location = match repository.repository_type.as_str() {
            "FileSystem" => {
                if required(&repository.spec, "location")? != "Core" {
                    return Err(
                        "Citadel system backup requires a Core or S3-compatible repository.".into(),
                    );
                }
                self.core_repository_path(required(&repository.spec, "path")?)?
                    .into_os_string()
            }
            "S3Compatible" => {
                let endpoint = required(&repository.spec, "endpoint")?
                    .trim()
                    .trim_end_matches('/');
                let bucket = required(&repository.spec, "bucket")?;
                let prefix = repository
                    .spec
                    .get("prefix")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim_matches('/');
                for (field, key) in [
                    ("accessKeySecretId", "AWS_ACCESS_KEY_ID"),
                    ("secretKeySecretId", "AWS_SECRET_ACCESS_KEY"),
                ] {
                    let value = self
                        .secrets
                        .resolve(uuid_field(&repository.spec, field)?)
                        .await
                        .map_err(|error| error.to_string())?;
                    environment.push((key.into(), OsString::from(value.as_str())));
                }
                if let Some(id) = repository
                    .spec
                    .get("sessionTokenSecretId")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok())
                {
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
            kind => return Err(format!("Backup Repository type '{kind}' is unsupported.")),
        };
        Ok((location, environment))
    }

    async fn validate_repository_target(
        &self,
        repository: &BackupRepository,
        location: &str,
        platform_id: Option<Uuid>,
    ) -> Result<(), String> {
        if repository.repository_type == "FileSystem"
            && required(&repository.spec, "location")? == "Core"
        {
            self.core_repository_path(required(&repository.spec, "path")?)?;
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
        if repository.repository_type == "FileSystem"
            && required(&repository.spec, "location")? == "Platform"
        {
            let repository_platform = uuid_field(&repository.spec, "platformId")?;
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
fn snapshot_subtree(metadata: &[u8], expected_id: &str) -> Result<String, String> {
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
    Ok(format!("{id}:{path}"))
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
        status: if cancelled { "Cancelled" } else { "Failed" },
        snapshot_availability: "NotCreated",
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

fn storage_text(error: impl std::fmt::Display) -> String {
    format!("Platform data could not be read: {error}")
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

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("Backup field '{key}' is missing."))
}

fn uuid_field(value: &Value, key: &str) -> Result<Uuid, String> {
    required(value, key).and_then(|value| {
        Uuid::parse_str(value).map_err(|_| format!("Backup field '{key}' is invalid."))
    })
}

fn policy_tag(id: Uuid) -> String {
    format!("citadel-policy-{id}")
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
            spec: serde_json::json!({"location":"Core","path":"../outside"}),
            password_secret_id: Uuid::now_v7(),
            status: "Unknown".into(),
            control_state: "Idle".into(),
            current_run_id: None,
            control_started_at: None,
            last_pruned_at: None,
            last_checked_at: None,
            created_by_actor_id: Uuid::now_v7(),
            created_at: now,
            updated_at: now,
            archived_at: None,
            row_version: 1,
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
                snapshot_subtree(&metadata, &id).unwrap(),
                format!("{id}:{root}")
            );
            assert_eq!(
                snapshot_subtree(&metadata, &id[..8]).unwrap(),
                format!("{id}:{root}")
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
            assert!(snapshot_subtree(&serde_json::to_vec(&metadata).unwrap(), &id).is_err());
        }
        assert!(snapshot_subtree(b"truncated JSON", &id).is_err());
    }

    #[test]
    fn redaction_preserves_lines_and_removes_credentials() {
        assert_eq!(redact("ok\npassword=abc\ndone"), "ok\n[redacted]\ndone");
    }
}
