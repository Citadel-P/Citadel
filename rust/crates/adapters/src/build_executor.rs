use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use citadel_bindings::SecretProtector;
use citadel_builds::{
    BuildClaim, BuildError, BuildExecutionResult, BuildExecutor, BuildLog,
    BuildRegistryCredentialResolver, BuildRegistryCredentials, BuildSecretResolver,
};
use citadel_execution::{OutputLimitPolicy, ProcessError, ProcessLimits, ProcessRequest, run};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::agent::{AgentBuildCommand, AgentClient};
use crate::agent_execution::AgentExecutionClient;
use crate::edge::{EdgeRegistry, EdgeTarget};
use crate::local_docker_target::LocalDockerTargetGuard;
use crate::secret_value_resolver::PostgresSecretValueResolver;

#[path = "build_output.rs"]
mod output;

#[derive(Clone)]
pub struct PostgresBuildSecretResolver {
    resolver: PostgresSecretValueResolver,
}

impl PostgresBuildSecretResolver {
    pub fn new(pool: PgPool, protector: Arc<dyn SecretProtector>) -> Result<Self, BuildError> {
        Ok(Self {
            resolver: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| BuildError::Storage(error.to_string()))?,
        })
    }
}

impl BuildSecretResolver for PostgresBuildSecretResolver {
    fn resolve(&self, secret_id: Uuid) -> BoxFuture<'_, Result<Zeroizing<String>, BuildError>> {
        Box::pin(async move {
            self.resolver
                .resolve(secret_id)
                .await
                .map_err(|error| BuildError::Validation(error.to_string()))
        })
    }
}

#[derive(Clone)]
pub struct PostgresBuildRegistryCredentialResolver {
    pool: PgPool,
}

impl PostgresBuildRegistryCredentialResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl BuildRegistryCredentialResolver for PostgresBuildRegistryCredentialResolver {
    fn resolve(
        &self,
        registry_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRegistryCredentials>, BuildError>> {
        Box::pin(async move {
            let configuration: Value = sqlx::query_scalar(
                "SELECT configuration FROM registries WHERE id=$1 AND status<>'Disabled'",
            )
            .bind(registry_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(build_storage)?
            .ok_or_else(|| BuildError::Validation("Build Registry was not found.".to_owned()))?;
            if json_bool(
                &configuration,
                &[
                    "authEnabled",
                    "AuthEnabled",
                    "ghcrAuthEnabled",
                    "GhcrAuthEnabled",
                ],
            ) == Some(false)
            {
                return Ok(None);
            }
            let username = json_string(
                &configuration,
                &[
                    "userName",
                    "UserName",
                    "username",
                    "Username",
                    "nameSpace",
                    "NameSpace",
                ],
            );
            let password = json_string(
                &configuration,
                &["password", "Password", "pat", "PAT", "token", "Token"],
            );
            match (username, password) {
                (Some(username), Some(password))
                    if !username.is_empty() && !password.is_empty() =>
                {
                    Ok(Some(BuildRegistryCredentials {
                        username: username.to_owned(),
                        password: Zeroizing::new(password.to_owned()),
                    }))
                }
                _ => Ok(None),
            }
        })
    }
}

pub struct LocalDockerBuildExecutor {
    git_cache_root: PathBuf,
    docker: OsString,
    secrets: Arc<dyn BuildSecretResolver>,
    registries: Arc<dyn BuildRegistryCredentialResolver>,
    maximum_log_bytes: usize,
    targets: LocalDockerTargetGuard,
}

pub struct PlatformBuildExecutor {
    pool: PgPool,
    local: Arc<dyn BuildExecutor>,
    agent: Option<AgentDockerBuildExecutor>,
    edge: Option<AgentDockerBuildExecutor>,
    git: Option<Arc<citadel_git::GitRepositoryExecutionService>>,
}

impl PlatformBuildExecutor {
    #[must_use]
    pub fn new(
        pool: PgPool,
        local: Arc<dyn BuildExecutor>,
        agent: Option<AgentDockerBuildExecutor>,
    ) -> Self {
        Self {
            pool,
            local,
            agent,
            edge: None,
            git: None,
        }
    }

    #[must_use]
    pub fn with_edge(mut self, edge: AgentDockerBuildExecutor) -> Self {
        self.edge = Some(edge);
        self
    }

    pub fn with_git_source(mut self, git: Arc<citadel_git::GitRepositoryExecutionService>) -> Self {
        self.git = Some(git);
        self
    }
}

impl BuildExecutor for PlatformBuildExecutor {
    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn citadel_builds::BuildLogSink,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            let mut prepared;
            let claim = if let Some(git) = &self.git {
                if let Err(error) = logs
                    .append("system", "Synchronizing Build Git source...")
                    .await
                {
                    return failed(BuildFailure::Io(error.to_string()));
                }
                let commit = match git
                    .synchronize_commit(
                        citadel_primitives::ActorId::new(claim.run.triggered_by_actor_id),
                        claim.run.git_repository_id,
                        &claim.run.branch,
                        cancellation,
                    )
                    .await
                {
                    Ok(commit) => commit,
                    Err(citadel_git::GitRepositoryExecutionError::Git(
                        citadel_git::GitError::Process(error),
                    )) => return failed(BuildFailure::Process(error)),
                    Err(error) => return failed(BuildFailure::Validation(error.to_string())),
                };
                prepared = claim.clone();
                // Webhook-pinned commits remain pinned even when the branch has
                // advanced again. Synchronization only ensures objects are local.
                if prepared.run.resolved_commit_sha.is_none() {
                    prepared.run.resolved_commit_sha = Some(commit);
                }
                &prepared
            } else {
                claim
            };
            if claim.project.builder_kind == "BuildAgentPool" {
                let Some(id) = claim.project.build_agent_pool_id else {
                    return failed(BuildFailure::Validation(
                        "Build has no selected Agent Pool.".into(),
                    ));
                };
                let spec = sqlx::query_scalar::<_, Value>("SELECT providerspec FROM buildagentpools WHERE id=$1 AND enabled AND archivedat IS NULL")
                    .bind(id).fetch_optional(&self.pool).await;
                let target = match spec {
                    Ok(Some(spec)) => match citadel_builds::pool_target(&spec) {
                        Ok(target) => target,
                        Err(error) => return failed(BuildFailure::Validation(error.to_string())),
                    },
                    Err(error) => return failed(BuildFailure::Io(error.to_string())),
                    Ok(None) => {
                        return failed(BuildFailure::Validation(
                            "The selected Build Agent Pool is unavailable.".into(),
                        ));
                    }
                };
                return match target {
                    citadel_builds::BuildPoolTarget::Edge => match &self.edge {
                        Some(edge) => edge.execute(claim, logs, cancellation).await,
                        None => failed(BuildFailure::Validation(
                            "Edge Build transport is unavailable.".into(),
                        )),
                    },
                    citadel_builds::BuildPoolTarget::Inbound(endpoint) => match &self.agent {
                        Some(agent) => match agent.for_address(&endpoint, cancellation).await {
                            Ok(agent) => agent.execute(claim, logs, cancellation).await,
                            Err(error) => failed(error),
                        },
                        None => failed(BuildFailure::Validation(
                            "Signed Agent transport is not configured.".into(),
                        )),
                    },
                };
            }
            let Some(platform_id) = claim.project.platform_id else {
                return failed(BuildFailure::Validation(
                    "Platform Build has no selected Platform.".to_owned(),
                ));
            };
            let target = sqlx::query_as::<_, (String, String, String)>(
                "SELECT connectortype,address,status FROM platforms WHERE id=$1",
            )
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await;
            let (connector, address, status) = match target {
                Ok(Some(target)) => target,
                Ok(None) => {
                    return failed(BuildFailure::Validation(
                        "Build Platform was not found.".to_owned(),
                    ));
                }
                Err(error) => return failed(BuildFailure::Io(error.to_string())),
            };
            if status != "Online" {
                return failed(BuildFailure::Validation(
                    "Build Platform is offline.".to_owned(),
                ));
            }
            match connector.as_str() {
                "Local" => self.local.execute(claim, logs, cancellation).await,
                "Agent" => match &self.agent {
                    Some(agent)
                        if agent.address().is_some_and(|configured| {
                            configured.trim_end_matches('/') == address.trim_end_matches('/')
                        }) =>
                    {
                        agent.execute(claim, logs, cancellation).await
                    }
                    _ => failed(BuildFailure::Validation(
                        "The configured Agent Build transport is unavailable.".to_owned(),
                    )),
                },
                "EdgeAgent" => match &self.edge {
                    Some(edge) => edge.execute(claim, logs, cancellation).await,
                    None => failed(BuildFailure::Validation(
                        "Edge Agent Build transport is unavailable.".into(),
                    )),
                },
                _ => failed(BuildFailure::Validation(
                    "Build Platform connector is unsupported.".to_owned(),
                )),
            }
        })
    }
}

pub struct AgentDockerBuildExecutor {
    git_cache_root: PathBuf,
    client: BuildTransport,
    secrets: Arc<dyn BuildSecretResolver>,
    registries: Arc<dyn BuildRegistryCredentialResolver>,
    maximum_log_bytes: usize,
}

enum BuildTransport {
    Direct(Arc<AgentClient>),
    Edge(EdgeRegistry),
}

impl AgentDockerBuildExecutor {
    async fn for_address(
        &self,
        endpoint: &str,
        cancellation: &CancellationToken,
    ) -> Result<Self, BuildFailure> {
        let BuildTransport::Direct(client) = &self.client else {
            return Err(BuildFailure::Validation(
                "An inbound Build Pool requires signed Agent transport.".into(),
            ));
        };
        let client = client
            .for_address(endpoint, cancellation)
            .await
            .map_err(|error| BuildFailure::Validation(error.to_string()))?;
        Ok(Self::new(
            self.git_cache_root.clone(),
            client,
            self.secrets.clone(),
            self.registries.clone(),
            self.maximum_log_bytes,
        ))
    }
    #[must_use]
    pub fn new(
        git_cache_root: PathBuf,
        client: AgentClient,
        secrets: Arc<dyn BuildSecretResolver>,
        registries: Arc<dyn BuildRegistryCredentialResolver>,
        maximum_log_bytes: usize,
    ) -> Self {
        Self {
            git_cache_root,
            client: BuildTransport::Direct(Arc::new(client)),
            secrets,
            registries,
            maximum_log_bytes: maximum_log_bytes.max(1024),
        }
    }

    pub fn new_edge(
        git_cache_root: PathBuf,
        registry: EdgeRegistry,
        secrets: Arc<dyn BuildSecretResolver>,
        registries: Arc<dyn BuildRegistryCredentialResolver>,
        maximum_log_bytes: usize,
    ) -> Self {
        Self {
            git_cache_root,
            client: BuildTransport::Edge(registry),
            secrets,
            registries,
            maximum_log_bytes: maximum_log_bytes.max(1024),
        }
    }
    fn address(&self) -> Option<&str> {
        match &self.client {
            BuildTransport::Direct(client) => Some(client.address()),
            BuildTransport::Edge(_) => None,
        }
    }

    async fn execute_inner(
        &self,
        claim: &BuildClaim,
        progress: &dyn citadel_builds::BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<BuildExecutionResult, BuildFailure> {
        let repository = tokio::fs::canonicalize(
            self.git_cache_root
                .join(claim.run.git_repository_id.to_string()),
        )
        .await
        .map_err(|error| BuildFailure::Io(format!("Git cache is unavailable: {error}")))?;
        let commit = resolve_build_commit(
            &repository,
            &claim.run.branch,
            claim.run.resolved_commit_sha.as_deref(),
            self.maximum_log_bytes,
            cancellation,
        )
        .await?;
        let references = image_references(claim, &commit)?;
        let context_archive =
            archive_build_context(&repository, &commit, &claim.run.context_path, cancellation)
                .await?;
        let dockerfile_path =
            dockerfile_in_context(&claim.run.context_path, &claim.run.dockerfile_path)?;
        let credentials = self
            .registries
            .resolve(claim.run.registry_id)
            .await
            .map_err(|error| BuildFailure::Validation(error.to_string()))?;
        let registry_auth = credentials
            .as_ref()
            .map(|credentials| encode_registry_auth(credentials, &claim.run.registry_host))
            .transpose()?;
        let mut secrets = Vec::with_capacity(claim.project.build_secrets.len());
        let mut secret_values = Vec::with_capacity(claim.project.build_secrets.len());
        for secret in &claim.project.build_secrets {
            let value = self
                .secrets
                .resolve(secret.secret_id)
                .await
                .map_err(|error| BuildFailure::Validation(error.to_string()))?;
            secrets.push((secret.id.clone(), value.to_string()));
            secret_values.push(value);
        }
        let build_args = claim
            .project
            .build_args
            .iter()
            .filter_map(|argument| {
                argument
                    .value
                    .as_ref()
                    .map(|value| (argument.name.clone(), value.clone()))
            })
            .collect::<HashMap<_, _>>();
        let client = match &self.client {
            BuildTransport::Direct(client) => AgentExecutionClient::Direct(client.clone()),
            BuildTransport::Edge(registry) => {
                let target = match claim.project.builder_kind.as_str() {
                    "Platform" => claim.project.platform_id.map(EdgeTarget::platform),
                    "BuildAgentPool" => claim
                        .project
                        .build_agent_pool_id
                        .map(EdgeTarget::build_pool),
                    _ => None,
                }
                .ok_or_else(|| {
                    BuildFailure::Validation("Build execution target is invalid.".into())
                })?;
                AgentExecutionClient::Edge(
                    registry
                        .get(&target)
                        .map_err(|error| BuildFailure::Validation(error.to_string()))?,
                )
            }
        };
        let sensitive = secret_values
            .iter()
            .map(|value| value.as_str())
            .chain(credentials.as_ref().map(|value| value.password.as_str()))
            .chain(registry_auth.as_deref())
            .collect::<Vec<_>>();
        let execution_cancellation = cancellation.child_token();
        let (sender, receiver) = tokio::sync::mpsc::channel(8);
        let execution = async {
            client
                .build_image(
                    AgentBuildCommand {
                        context_archive,
                        dockerfile_path,
                        tags: references.clone(),
                        build_args,
                        target: claim.run.target.clone(),
                        registry_auth: registry_auth.clone(),
                        registry_host: Some(
                            normalize_registry_host(&claim.run.registry_host)?.to_owned(),
                        ),
                        timeout_seconds: claim.run.timeout_seconds,
                        maximum_log_bytes: self.maximum_log_bytes,
                        secrets,
                        output: Some(sender),
                    },
                    &execution_cancellation,
                )
                .await
                .map_err(|error| {
                    if cancellation.is_cancelled() {
                        BuildFailure::Process(ProcessError::Cancelled)
                    } else {
                        BuildFailure::Command(redact_values(&error.to_string(), &sensitive))
                    }
                })
        };
        let output = output::capture(
            execution,
            receiver,
            progress,
            &sensitive,
            &execution_cancellation,
        )
        .await?;
        Ok(BuildExecutionResult {
            status: "Succeeded",
            exit_code: Some(0),
            image_digest: find_digest(&output),
            resolved_commit_sha: Some(commit),
            image_references: references,
            error_code: None,
            error_message: None,
            logs: vec![],
        })
    }
}

impl BuildExecutor for AgentDockerBuildExecutor {
    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn citadel_builds::BuildLogSink,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            self.execute_inner(claim, logs, cancellation)
                .await
                .unwrap_or_else(failed)
        })
    }
}

impl LocalDockerBuildExecutor {
    pub fn new(
        git_cache_root: PathBuf,
        docker: impl Into<OsString>,
        secrets: Arc<dyn BuildSecretResolver>,
        registries: Arc<dyn BuildRegistryCredentialResolver>,
        maximum_log_bytes: usize,
        pool: PgPool,
    ) -> Self {
        Self {
            git_cache_root,
            docker: docker.into(),
            secrets,
            registries,
            maximum_log_bytes: maximum_log_bytes.max(1024),
            targets: LocalDockerTargetGuard::new(pool),
        }
    }

    async fn execute_inner(
        &self,
        claim: &BuildClaim,
        progress: &dyn citadel_builds::BuildLogSink,
        cancellation: &CancellationToken,
    ) -> Result<BuildExecutionResult, BuildFailure> {
        if claim.project.builder_kind != "Platform" {
            return Err(BuildFailure::Validation(
                "External Build Agent Pool execution is not configured for this worker.".to_owned(),
            ));
        }
        let platform_id = claim.project.platform_id.ok_or_else(|| {
            BuildFailure::Validation("Platform Build has no selected Platform.".to_owned())
        })?;
        self.targets
            .require_local(platform_id)
            .await
            .map_err(BuildFailure::Validation)?;
        let repository = self
            .git_cache_root
            .join(claim.run.git_repository_id.to_string());
        let repository = tokio::fs::canonicalize(&repository)
            .await
            .map_err(|error| BuildFailure::Io(format!("Git cache is unavailable: {error}")))?;
        let revision = build_revision(&claim.run.branch, claim.run.resolved_commit_sha.as_deref())?;
        let commit_output = run(
            ProcessRequest::new("git")
                .args([
                    OsString::from("-C"),
                    repository.as_os_str().to_owned(),
                    OsString::from("rev-parse"),
                    OsString::from("--verify"),
                    OsString::from("--end-of-options"),
                    OsString::from(revision),
                ])
                .limits(limits(Duration::from_secs(30), self.maximum_log_bytes)),
            cancellation,
        )
        .await
        .map_err(BuildFailure::Process)?;
        if !commit_output.succeeded() {
            return Err(BuildFailure::Command(logs(
                &commit_output.stdout,
                &commit_output.stderr,
            )));
        }
        let commit = String::from_utf8_lossy(&commit_output.stdout)
            .trim()
            .to_ascii_lowercase();
        if commit.len() != 40 && commit.len() != 64 {
            return Err(BuildFailure::Command(
                "Git returned an invalid commit identifier.".to_owned(),
            ));
        }
        let workspace = self
            .materialize_workspace(&repository, claim.run.id, &commit, cancellation)
            .await?;
        let docker_config = std::env::temp_dir()
            .join("citadel-build-docker-config")
            .join(claim.run.id.to_string());
        let outcome = async {
            remove_directory_if_present(&docker_config).await;
            tokio::fs::create_dir_all(&docker_config)
                .await
                .map_err(|error| BuildFailure::Io(error.to_string()))?;
            let registry_credentials = self
                .registries
                .resolve(claim.run.registry_id)
                .await
                .map_err(|error| BuildFailure::Validation(error.to_string()))?;
            if let Some(credentials) = registry_credentials.as_ref() {
                let login = run(
                    ProcessRequest::new(self.docker.clone())
                        .args([
                            OsString::from("login"),
                            OsString::from(normalize_registry_host(&claim.run.registry_host)?),
                            OsString::from("--username"),
                            OsString::from(&credentials.username),
                            OsString::from("--password-stdin"),
                        ])
                        .env("DOCKER_CONFIG", docker_config.as_os_str())
                        .stdin(credentials.password.as_bytes().to_vec())
                        .limits(limits(Duration::from_secs(60), self.maximum_log_bytes)),
                    cancellation,
                )
                .await
                .map_err(BuildFailure::Process)?;
                if !login.succeeded() {
                    return Err(BuildFailure::Command(redact_values(
                        &logs(&login.stdout, &login.stderr),
                        &[credentials.password.as_str()],
                    )));
                }
            }
            let context = safe_child(&workspace, &claim.run.context_path).await?;
            let dockerfile = safe_child(&workspace, &claim.run.dockerfile_path).await?;
            let references = image_references(claim, &commit)?;
            let mut args = vec![
                OsString::from("build"),
                OsString::from("--file"),
                dockerfile.as_os_str().to_owned(),
            ];
            if let Some(target) = claim.run.target.as_deref() {
                args.extend([OsString::from("--target"), OsString::from(target)]);
            }
            for argument in &claim.project.build_args {
                args.push(OsString::from("--build-arg"));
                args.push(OsString::from(match argument.value.as_deref() {
                    Some(value) => format!("{}={value}", argument.name),
                    None => argument.name.clone(),
                }));
            }
            for reference in &references {
                args.extend([OsString::from("--tag"), OsString::from(reference)]);
            }
            let mut secret_environment = Vec::with_capacity(claim.project.build_secrets.len());
            for (index, secret) in claim.project.build_secrets.iter().enumerate() {
                let environment_name = format!("CITADEL_BUILDKIT_SECRET_{index}");
                let value = self
                    .secrets
                    .resolve(secret.secret_id)
                    .await
                    .map_err(|error| BuildFailure::Validation(error.to_string()))?;
                args.extend([
                    OsString::from("--secret"),
                    OsString::from(format!("id={},env={environment_name}", secret.id)),
                ]);
                secret_environment.push((environment_name, value));
            }
            args.push(context.as_os_str().to_owned());
            let mut request = ProcessRequest::new(self.docker.clone())
                .args(args)
                .env("DOCKER_BUILDKIT", "1")
                .env("DOCKER_CONFIG", docker_config.as_os_str())
                .limits(limits(
                    Duration::from_secs(claim.run.timeout_seconds as u64),
                    self.maximum_log_bytes,
                ));
            for (name, value) in &secret_environment {
                request = request.env(name, value.as_str());
            }
            let redaction_values = secret_environment
                .iter()
                .map(|(_, value)| value.as_str())
                .chain(
                    registry_credentials
                        .as_ref()
                        .map(|value| value.password.as_str()),
                )
                .collect::<Vec<_>>();
            let built = output::run(request, cancellation, progress, &redaction_values).await?;
            if !built.succeeded() {
                return Err(BuildFailure::Command(redact_values(
                    &logs(&built.stdout, &built.stderr),
                    &redaction_values,
                )));
            }
            let mut digest = None;
            for reference in &references {
                let pushed = output::run(
                    ProcessRequest::new(self.docker.clone())
                        .args([OsString::from("push"), OsString::from(reference)])
                        .env("DOCKER_CONFIG", docker_config.as_os_str())
                        .limits(limits(
                            Duration::from_secs(claim.run.timeout_seconds as u64),
                            self.maximum_log_bytes,
                        )),
                    cancellation,
                    progress,
                    &redaction_values,
                )
                .await?;
                let push_log = logs(&pushed.stdout, &pushed.stderr);
                if !pushed.succeeded() {
                    return Err(BuildFailure::Command(redact_values(
                        &push_log,
                        &redaction_values,
                    )));
                }
                digest = digest.or_else(|| find_digest(&push_log));
            }
            Ok(BuildExecutionResult {
                status: "Succeeded",
                exit_code: Some(0),
                image_digest: digest,
                resolved_commit_sha: Some(commit),
                image_references: references,
                error_code: None,
                error_message: None,
                logs: vec![],
            })
        }
        .await;
        remove_directory_if_present(&docker_config).await;
        remove_directory_if_present(&workspace).await;
        outcome
    }

    async fn materialize_workspace(
        &self,
        repository: &Path,
        run_id: uuid::Uuid,
        commit: &str,
        cancellation: &CancellationToken,
    ) -> Result<PathBuf, BuildFailure> {
        let root = std::env::temp_dir().join("citadel-build-workspaces");
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|error| BuildFailure::Io(error.to_string()))?;
        let workspace = root.join(run_id.to_string());
        remove_directory_if_present(&workspace).await;
        let cloned = run(
            ProcessRequest::new("git")
                .args([
                    OsString::from("clone"),
                    OsString::from("--shared"),
                    OsString::from("--no-checkout"),
                    OsString::from("--"),
                    repository.as_os_str().to_owned(),
                    workspace.as_os_str().to_owned(),
                ])
                .limits(limits(Duration::from_secs(120), self.maximum_log_bytes)),
            cancellation,
        )
        .await;
        let cloned = match cloned {
            Ok(output) => output,
            Err(error) => {
                remove_directory_if_present(&workspace).await;
                return Err(BuildFailure::Process(error));
            }
        };
        if !cloned.succeeded() {
            remove_directory_if_present(&workspace).await;
            return Err(BuildFailure::Command(logs(&cloned.stdout, &cloned.stderr)));
        }
        let checked_out = run(
            ProcessRequest::new("git")
                .args([
                    OsString::from("-C"),
                    workspace.as_os_str().to_owned(),
                    OsString::from("checkout"),
                    OsString::from("--detach"),
                    OsString::from("--force"),
                    OsString::from(commit),
                ])
                .limits(limits(Duration::from_secs(120), self.maximum_log_bytes)),
            cancellation,
        )
        .await;
        let checked_out = match checked_out {
            Ok(output) => output,
            Err(error) => {
                remove_directory_if_present(&workspace).await;
                return Err(BuildFailure::Process(error));
            }
        };
        if !checked_out.succeeded() {
            remove_directory_if_present(&workspace).await;
            return Err(BuildFailure::Command(logs(
                &checked_out.stdout,
                &checked_out.stderr,
            )));
        }
        Ok(workspace)
    }
}

impl BuildExecutor for LocalDockerBuildExecutor {
    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn citadel_builds::BuildLogSink,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            self.execute_inner(claim, logs, cancellation)
                .await
                .unwrap_or_else(failed)
        })
    }
}

fn failed(error: BuildFailure) -> BuildExecutionResult {
    let message = redact(&error.to_string());
    BuildExecutionResult {
        status: match error {
            BuildFailure::Process(ProcessError::Cancelled) => "Cancelled",
            BuildFailure::Process(ProcessError::Timeout(_)) => "TimedOut",
            _ => "Failed",
        },
        exit_code: None,
        image_digest: None,
        resolved_commit_sha: None,
        image_references: vec![],
        error_code: Some(error.code().to_owned()),
        error_message: Some(message.clone()),
        logs: vec![BuildLog {
            stream: "stderr".to_owned(),
            message,
        }],
    }
}

async fn resolve_build_commit(
    repository: &Path,
    branch: &str,
    pinned: Option<&str>,
    maximum_log_bytes: usize,
    cancellation: &CancellationToken,
) -> Result<String, BuildFailure> {
    let revision = build_revision(branch, pinned)?;
    let output = run(
        ProcessRequest::new("git")
            .args([
                OsString::from("-C"),
                repository.as_os_str().to_owned(),
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from("--end-of-options"),
                OsString::from(revision),
            ])
            .limits(limits(Duration::from_secs(30), maximum_log_bytes)),
        cancellation,
    )
    .await
    .map_err(BuildFailure::Process)?;
    if !output.succeeded() {
        return Err(BuildFailure::Command(logs(&output.stdout, &output.stderr)));
    }
    let commit = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_ascii_lowercase();
    if (commit.len() != 40 && commit.len() != 64)
        || !commit.bytes().all(|value| value.is_ascii_hexdigit())
    {
        return Err(BuildFailure::Command(
            "Git returned an invalid commit identifier.".to_owned(),
        ));
    }
    Ok(commit)
}

fn build_revision(branch: &str, pinned: Option<&str>) -> Result<String, BuildFailure> {
    match pinned {
        Some(commit)
            if matches!(commit.len(), 40 | 64)
                && commit.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(format!("{commit}^{{commit}}"))
        }
        Some(_) => Err(BuildFailure::Validation(
            "Pinned Build source must be a full commit ID.".into(),
        )),
        None => Ok(format!("refs/remotes/origin/{branch}^{{commit}}")),
    }
}

async fn archive_build_context(
    repository: &Path,
    commit: &str,
    context_path: &str,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, BuildFailure> {
    let tree = if matches!(context_path, "." | "./") {
        commit.to_owned()
    } else {
        format!("{commit}:{}", context_path.trim_matches('/'))
    };
    let output = run(
        ProcessRequest::new("git")
            .args([
                OsString::from("-C"),
                repository.as_os_str().to_owned(),
                OsString::from("archive"),
                OsString::from("--format=tar"),
                OsString::from(tree),
            ])
            .limits(ProcessLimits {
                timeout: Duration::from_secs(120),
                maximum_stdout_bytes: 12 * 1024 * 1024,
                maximum_stderr_bytes: 64 * 1024,
                output_limit_policy: OutputLimitPolicy::Error,
            }),
        cancellation,
    )
    .await
    .map_err(BuildFailure::Process)?;
    if !output.succeeded() {
        return Err(BuildFailure::Command(logs(&output.stdout, &output.stderr)));
    }
    if output.stdout.is_empty() {
        return Err(BuildFailure::Validation(
            "Build context is empty.".to_owned(),
        ));
    }
    Ok(output.stdout)
}

fn dockerfile_in_context(
    context_path: &str,
    dockerfile_path: &str,
) -> Result<String, BuildFailure> {
    let context = context_path.trim_matches('/').trim_start_matches("./");
    let dockerfile = dockerfile_path.trim_matches('/').trim_start_matches("./");
    let relative = if context.is_empty() || context == "." {
        dockerfile
    } else {
        dockerfile
            .strip_prefix(context)
            .and_then(|value| value.strip_prefix('/'))
            .ok_or_else(|| {
                BuildFailure::Validation(
                    "Dockerfile must be inside the transported Build context.".to_owned(),
                )
            })?
    };
    if relative.is_empty()
        || relative
            .split('/')
            .any(|part| matches!(part, ".." | ".git"))
    {
        return Err(BuildFailure::Validation(
            "Dockerfile path is invalid.".to_owned(),
        ));
    }
    Ok(relative.to_owned())
}

fn encode_registry_auth(
    credentials: &BuildRegistryCredentials,
    registry_host: &str,
) -> Result<String, BuildFailure> {
    let payload = serde_json::to_vec(&serde_json::json!({
        "username": credentials.username,
        "password": credentials.password.as_str(),
        "serveraddress": normalize_registry_host(registry_host)?,
    }))
    .map_err(|error| BuildFailure::Validation(error.to_string()))?;
    Ok(STANDARD.encode(payload))
}

fn limits(timeout: Duration, maximum: usize) -> ProcessLimits {
    ProcessLimits {
        timeout,
        maximum_stdout_bytes: maximum,
        maximum_stderr_bytes: maximum,
        output_limit_policy: OutputLimitPolicy::Truncate,
    }
}
async fn safe_child(root: &Path, relative: &str) -> Result<PathBuf, BuildFailure> {
    if relative
        .split(['/', '\\'])
        .any(|part| matches!(part, ".." | ".git"))
    {
        return Err(BuildFailure::Validation(
            "Build path cannot traverse or access Git metadata.".to_owned(),
        ));
    }
    let path = tokio::fs::canonicalize(root.join(relative))
        .await
        .map_err(|error| BuildFailure::Io(error.to_string()))?;
    if !path.starts_with(root) {
        return Err(BuildFailure::Validation(
            "Build path escapes the synchronized repository.".to_owned(),
        ));
    }
    Ok(path)
}
async fn remove_directory_if_present(path: &Path) {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "Could not remove Build workspace")
        }
    }
}
fn image_references(claim: &BuildClaim, commit: &str) -> Result<Vec<String>, BuildFailure> {
    let short = &commit[..12.min(commit.len())];
    let branch = claim.run.branch.replace(['/', '\\'], "-");
    let mut references = Vec::with_capacity(claim.project.tag_templates.len());
    for template in &claim.project.tag_templates {
        let tag = template
            .replace("{branch}", &branch)
            .replace("{shortSha}", short)
            .replace("{sha}", commit);
        if tag.is_empty() || tag.contains(char::is_whitespace) {
            return Err(BuildFailure::Validation(
                "Build tag template produced an invalid tag.".to_owned(),
            ));
        }
        let host = normalize_registry_host(&claim.run.registry_host)?;
        references.push(format!("{host}/{}:{tag}", claim.run.image_repository));
    }
    Ok(references)
}
fn normalize_registry_host(value: &str) -> Result<&str, BuildFailure> {
    let value = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value)
        .trim_end_matches('/');
    if value.is_empty() || value.contains(['/', '\\', '\r', '\n', ' ', '\t']) {
        Err(BuildFailure::Validation(
            "Build Registry host is invalid.".to_owned(),
        ))
    } else {
        Ok(value)
    }
}
fn json_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
}
fn json_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}
fn find_digest(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|part| part.starts_with("sha256:") && part.len() == 71)
        .map(|value| value.trim_end_matches(',').to_owned())
}
fn logs(stdout: &[u8], stderr: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !value.is_empty() {
            value.push('\n');
        }
        value.push_str(&String::from_utf8_lossy(stderr));
    }
    value
}
fn redact(value: &str) -> String {
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

fn redact_values(value: &str, secrets: &[&str]) -> String {
    let mut redacted = value.to_owned();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        redacted = redacted.replace(secret, "[redacted]");
    }
    redact(&redacted)
}

#[derive(Debug, thiserror::Error)]
enum BuildFailure {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Command(String),
    #[error(transparent)]
    Process(ProcessError),
}

fn build_storage(error: impl std::fmt::Display) -> BuildError {
    BuildError::Storage(error.to_string())
}
impl BuildFailure {
    fn code(&self) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digest_parser_is_strict() {
        assert_eq!(find_digest("digest: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa size: 1").as_deref(),Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(find_digest("sha256:short").is_none());
    }
    #[test]
    fn redaction_hides_inline_credentials() {
        assert_eq!(
            redact("token=abc password=def ok"),
            "token=[redacted] password=[redacted] ok"
        );
    }
    #[test]
    fn registry_host_is_normalized_without_accepting_paths() {
        assert_eq!(
            normalize_registry_host("https://registry.example.test/").unwrap(),
            "registry.example.test"
        );
        assert!(normalize_registry_host("registry.example.test/path").is_err());
    }
    #[test]
    fn resolved_build_secrets_are_removed_from_multiline_output() {
        assert_eq!(
            redact_values("step one\nsecret-value\nstep two", &["secret-value"]),
            "step one\n[redacted]\nstep two"
        );
    }

    #[test]
    fn pinned_build_revision_does_not_follow_a_later_branch_update() {
        let commit = "a".repeat(40);
        assert_eq!(
            build_revision("main", Some(&commit)).unwrap(),
            format!("{commit}^{{commit}}")
        );
        assert_eq!(
            build_revision("main", None).unwrap(),
            "refs/remotes/origin/main^{commit}"
        );
        for invalid in ["main", "--all", "HEAD", "aaaa", "../config"] {
            assert!(build_revision("main", Some(invalid)).is_err());
        }
    }

    #[test]
    fn agent_dockerfile_is_relative_to_the_transported_context() {
        assert_eq!(
            dockerfile_in_context("apps/api", "apps/api/docker/Dockerfile").unwrap(),
            "docker/Dockerfile"
        );
        assert!(dockerfile_in_context("apps/api", "apps/web/Dockerfile").is_err());
        assert!(dockerfile_in_context(".", "../Dockerfile").is_err());
        assert!(dockerfile_in_context(".", ".git/config").is_err());
    }

    #[test]
    fn agent_registry_auth_is_docker_compatible_and_rejects_host_paths() {
        let encoded = encode_registry_auth(
            &BuildRegistryCredentials {
                username: "builder".to_owned(),
                password: Zeroizing::new("secret".to_owned()),
            },
            "https://registry.example.test/",
        )
        .unwrap();
        let decoded: Value = serde_json::from_slice(&STANDARD.decode(encoded).unwrap()).unwrap();
        assert_eq!(decoded["username"], "builder");
        assert_eq!(decoded["password"], "secret");
        assert_eq!(decoded["serveraddress"], "registry.example.test");
        assert!(
            encode_registry_auth(
                &BuildRegistryCredentials {
                    username: "builder".to_owned(),
                    password: Zeroizing::new("secret".to_owned()),
                },
                "registry.example.test/path",
            )
            .is_err()
        );
    }
}
