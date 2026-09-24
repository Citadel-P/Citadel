use crate::connectors::docker::deployments::observed_container_state;
use crate::connectors::docker::deployments::{parse_mount, parse_port};
use std::collections::BTreeMap;

use base64::Engine;
use citadel_deployments::{
    ContainerRestartPolicy, DeploymentError, DeploymentImageInfo, DeploymentRuntime,
    PreparedDeploymentImage, ResolvedDeploymentBuild, RuntimeContainerState,
    RuntimeDeploymentCommand, RuntimeDeploymentResult, StopSignal,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::{FutureExt, StreamExt, future::BoxFuture};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::connectors::agent::client::AgentClient;
use crate::connectors::docker::DockerClient;
use crate::connectors::docker::DockerError;

const DEFAULT_DOCKER_HUB_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000100);
const DEPLOYMENT_LABEL: &str = "com.citadel.deployment-id";
const MANAGED_LABEL: &str = "com.citadel.managed";

#[derive(Clone)]
pub struct DeploymentRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
    edge: crate::connectors::edge::EdgeRegistry,
    image_cache: std::sync::Arc<crate::connectors::registries::digest_cache::ImageDigestCache>,
}

impl DeploymentRuntimeRouter {
    #[must_use]
    pub fn new(pool: PgPool, docker: DockerClient, agent: Option<AgentClient>) -> Self {
        Self {
            pool,
            docker,
            agent,
            edge: crate::connectors::edge::EdgeRegistry::default(),
            image_cache: Default::default(),
        }
    }

    pub fn with_image_cache(
        mut self,
        cache: std::sync::Arc<crate::connectors::registries::digest_cache::ImageDigestCache>,
    ) -> Self {
        self.image_cache = cache;
        self
    }

    #[must_use]
    pub fn with_edge(mut self, edge: crate::connectors::edge::EdgeRegistry) -> Self {
        self.edge = edge;
        self
    }

    async fn platform(&self, platform_id: Uuid) -> Result<PlatformTarget, DeploymentError> {
        let row = sqlx::query("SELECT connectortype,address,status FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
        let status: String = row.try_get("status").map_err(storage)?;
        if status != "Online" {
            return Err(DeploymentError::Runtime(
                "Platform not found or disconnected.".to_owned(),
            ));
        }
        Ok(PlatformTarget {
            platform_id,
            connector: crate::persistence::postgres::platforms::classification::connector_kind(
                row.try_get("connectortype").map_err(storage)?,
            )
            .map_err(storage)?,
            address: row.try_get("address").map_err(storage)?,
        })
    }

    fn agent_for(
        &self,
        target: &PlatformTarget,
    ) -> Result<crate::connectors::agent::execution::AgentExecutionClient, DeploymentError> {
        use crate::connectors::agent::execution::AgentExecutionClient;
        use crate::connectors::edge::EdgeTarget;
        if target.connector == citadel_platforms::ConnectorKind::EdgeAgent {
            return self
                .edge
                .get(&EdgeTarget::platform(target.platform_id))
                .map(AgentExecutionClient::Edge)
                .map_err(|_| {
                    DeploymentError::Runtime(
                        "The Edge Agent is disconnected or unavailable.".into(),
                    )
                });
        }
        let agent = self
            .agent
            .as_ref()
            .filter(|_| target.connector == citadel_platforms::ConnectorKind::Agent)
            .ok_or_else(|| {
                DeploymentError::Runtime("The configured Agent transport is unavailable.".into())
            })?;
        let agent = agent
            .at_address(&target.address)
            .map_err(|error| DeploymentError::Runtime(error.message))?;
        Ok(AgentExecutionClient::Direct(std::sync::Arc::new(agent)))
    }

    async fn prepare_local_image(
        &self,
        platform_id: Uuid,
        image_id: &str,
    ) -> Result<PreparedDeploymentImage, DeploymentError> {
        let id = Uuid::parse_str(image_id).map_err(|_| {
            DeploymentError::Validation("The selected local image is invalid.".to_owned())
        })?;
        let docker_image_id = sqlx::query_scalar::<_, String>(
            "SELECT dockerimageid FROM images WHERE id=$1 AND platformid=$2",
        )
        .bind(id)
        .bind(platform_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| {
            DeploymentError::Validation(
                "The selected local image is not available on this platform.".to_owned(),
            )
        })?;
        Ok(PreparedDeploymentImage {
            docker_image_id,
            digest: None,
            resolved_build: None,
        })
    }

    async fn prepare_build_image(
        &self,
        platform_id: Uuid,
        build_project_id: Uuid,
        resolved_build_run_id: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<PreparedDeploymentImage, DeploymentError> {
        let row = sqlx::query(
            r#"SELECT project.registryid,run.id,run.imagereferences,run.imagedigest
FROM buildprojects project
JOIN LATERAL (
    SELECT candidate.id,candidate.imagereferences,candidate.imagedigest
    FROM buildruns candidate
    WHERE candidate.buildprojectid=project.id
      AND candidate.status='Succeeded'
      AND ($2::uuid IS NULL OR candidate.id=$2)
    ORDER BY candidate.completedat DESC NULLS LAST,candidate.id DESC
    LIMIT 1
) run ON TRUE
WHERE project.id=$1 AND project.enabled AND project.archivedat IS NULL"#,
        )
        .bind(build_project_id)
        .bind(resolved_build_run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| {
            DeploymentError::Validation(
                "The selected Build Project has no successful Build Run to deploy.".to_owned(),
            )
        })?;
        let references: Value = row.try_get("imagereferences").map_err(storage)?;
        let reference = references
            .as_array()
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                DeploymentError::Validation(
                    "The selected Build Run has no deployable image reference.".to_owned(),
                )
            })?
            .to_owned();
        let build = ResolvedDeploymentBuild {
            image_reference: reference.clone(),
            digest: row.try_get("imagedigest").map_err(storage)?,
            build_run_id: row.try_get("id").map_err(storage)?,
        };
        let mut prepared = self
            .prepare_external_image(
                platform_id,
                row.try_get("registryid").map_err(storage)?,
                &reference,
                cancellation,
            )
            .await?;
        prepared.resolved_build = Some(build);
        Ok(prepared)
    }

    async fn registry(
        &self,
        registry_id: Uuid,
        image_tag: &str,
    ) -> Result<RegistryPull, DeploymentError> {
        let row =
            sqlx::query("SELECT registryhost,configuration,status FROM registries WHERE id=$1")
                .bind(registry_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or_else(|| {
                    DeploymentError::Validation("The selected Registry was not found.".to_owned())
                })?;
        let status: String = row.try_get("status").map_err(storage)?;
        if status.eq_ignore_ascii_case("Disabled") {
            return Err(DeploymentError::Validation(
                "The selected Registry is disabled.".to_owned(),
            ));
        }
        let host: String = row.try_get("registryhost").map_err(storage)?;
        let configuration: Value = row.try_get("configuration").map_err(storage)?;
        let image = qualify_image_reference(&host, image_tag)?;
        let auth = registry_auth(registry_id, &host, &configuration)?;
        Ok(RegistryPull { image, auth })
    }

    async fn prepare_external_image(
        &self,
        platform_id: Uuid,
        registry_id: Uuid,
        image_tag: &str,
        cancellation: &CancellationToken,
    ) -> Result<PreparedDeploymentImage, DeploymentError> {
        let target = self.platform(platform_id).await?;
        let pull = self.registry(registry_id, image_tag).await?;
        if target.connector == citadel_platforms::ConnectorKind::Local {
            let mut stream = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                result = self.docker.pull_image(
                    &pull.image,
                    pull.auth.as_ref().map(|auth| auth.as_str()),
                ) => result,
            }
            .map_err(runtime)?;
            while let Some(item) = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                item = stream.next() => item,
            } {
                let item = item.map_err(runtime)?;
                if let Some(message) = item
                    .error
                    .or_else(|| item.error_detail.map(|detail| detail.message))
                    .filter(|message| !message.trim().is_empty())
                {
                    return Err(DeploymentError::Runtime(message));
                }
            }
            let images = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                result = self.docker.list_images() => result,
            }
            .map_err(runtime)?;
            return find_image(
                &pull.image,
                images
                    .into_iter()
                    .map(|image| (image.id, image.repo_tags, image.repo_digests)),
            );
        }
        if matches!(
            target.connector,
            citadel_platforms::ConnectorKind::Agent | citadel_platforms::ConnectorKind::EdgeAgent
        ) {
            let agent = self.agent_for(&target)?;
            agent
                .pull_deployment_image(
                    &pull.image,
                    pull.auth.as_ref().map(|auth| auth.as_str().to_owned()),
                    cancellation,
                )
                .await
                .map_err(agent_runtime)?;
            let images = agent
                .list_images(cancellation)
                .await
                .map_err(agent_runtime)?;
            return find_image(
                &pull.image,
                images
                    .into_iter()
                    .map(|image| (image.id, image.repo_tags, image.repo_digests)),
            );
        }
        Err(edge_unavailable())
    }

    async fn apply_local(
        &self,
        command: &RuntimeDeploymentCommand,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeDeploymentResult, DeploymentError> {
        self.docker
            .apply_container_config(
                &command.name,
                &command.image_id,
                &docker_create_body(command)?,
                cancellation,
            )
            .await
    }
}

impl DeploymentRuntime for DeploymentRuntimeRouter {
    fn cached_image_digest<'a>(
        &'a self,
        _platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, DeploymentError>> {
        Box::pin(async move {
            if !self.image_cache.wait_ready(cancel).await {
                return Err(DeploymentError::Cancelled);
            }
            Ok(self
                .image_cache
                .get(registry, reference)
                .map(|entry| entry.digest))
        })
    }
    fn remote_image_digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, DeploymentError>> {
        Box::pin(async move {
            let target = self.platform(platform).await?;
            let agent = if target.connector == citadel_platforms::ConnectorKind::Local {
                None
            } else {
                Some(self.agent_for(&target)?)
            };
            crate::connectors::registries::digest::inspect(
                &self.pool,
                &self.docker,
                agent,
                registry,
                reference,
                cancel,
            )
            .await
            .map_err(|message| DeploymentError::Runtime(message.into()))
        })
    }
    fn prepare_image<'a>(
        &'a self,
        platform_id: Uuid,
        image: &'a DeploymentImageInfo,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PreparedDeploymentImage, DeploymentError>> {
        async move {
            match image {
                DeploymentImageInfo::Local { image_id } => {
                    self.prepare_local_image(platform_id, image_id).await
                }
                DeploymentImageInfo::External {
                    registry_id,
                    image_tag,
                    ..
                } => {
                    self.prepare_external_image(platform_id, *registry_id, image_tag, cancellation)
                        .await
                }
                DeploymentImageInfo::Build {
                    build_project_id,
                    resolved_build_run_id,
                    ..
                } => {
                    self.prepare_build_image(
                        platform_id,
                        *build_project_id,
                        *resolved_build_run_id,
                        cancellation,
                    )
                    .await
                }
            }
        }
        .boxed()
    }

    fn apply_container<'a>(
        &'a self,
        platform_id: Uuid,
        command: &'a RuntimeDeploymentCommand,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeDeploymentResult, DeploymentError>> {
        async move {
            let target = self.platform(platform_id).await?;
            if target.connector == citadel_platforms::ConnectorKind::Local {
                return self.apply_local(command, cancellation).await;
            }
            if matches!(
                target.connector,
                citadel_platforms::ConnectorKind::Agent
                    | citadel_platforms::ConnectorKind::EdgeAgent
            ) {
                let mut normalized = command.clone();
                normalize_resource_limits(&mut normalized);
                add_ownership_labels(&mut normalized);
                return self
                    .agent_for(&target)?
                    .apply_deployment(&normalized, cancellation)
                    .await
                    .map_err(agent_runtime);
            }
            Err(edge_unavailable())
        }
        .boxed()
    }

    fn observe_deployment<'a>(
        &'a self,
        platform_id: Uuid,
        deployment_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeDeploymentResult>, DeploymentError>> {
        async move {
            let target = self.platform(platform_id).await?;
            let deployment_id = deployment_id.to_string();
            if target.connector == citadel_platforms::ConnectorKind::Local {
                let containers = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                    result = self.docker.list_containers(true) => result,
                }
                .map_err(runtime)?;
                let Some(container) = containers
                    .into_iter()
                    .filter(|container| {
                        container.labels.get(DEPLOYMENT_LABEL) == Some(&deployment_id)
                    })
                    .max_by_key(|container| {
                        (
                            container.created,
                            container.state.eq_ignore_ascii_case("running"),
                        )
                    })
                else {
                    return Ok(None);
                };
                let inspect = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                    result = self.docker.inspect_container(&container.id) => result,
                }
                .map_err(runtime)?;
                return Ok(Some(RuntimeDeploymentResult {
                    docker_container_id: container.id,
                    docker_image_id: container.image_id,
                    state: observed_container_state(&inspect.state)
                        .unwrap_or(RuntimeContainerState::Timeout),
                }));
            }
            if matches!(
                target.connector,
                citadel_platforms::ConnectorKind::Agent
                    | citadel_platforms::ConnectorKind::EdgeAgent
            ) {
                let containers = self
                    .agent_for(&target)?
                    .list_containers(cancellation)
                    .await
                    .map_err(agent_runtime)?;
                let Some(container) = containers
                    .into_iter()
                    .filter(|container| {
                        container.labels.get(DEPLOYMENT_LABEL) == Some(&deployment_id)
                    })
                    .max_by_key(|container| {
                        (
                            container.created,
                            container.state.eq_ignore_ascii_case("running"),
                        )
                    })
                else {
                    return Ok(None);
                };
                return Ok(Some(RuntimeDeploymentResult {
                    docker_container_id: container.id,
                    docker_image_id: container.image_id,
                    state: container_summary_state(&container.state, &container.status),
                }));
            }
            Err(edge_unavailable())
        }
        .boxed()
    }

    fn delete_container<'a>(
        &'a self,
        platform_id: Uuid,
        docker_container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        async move {
            let target = self.platform(platform_id).await?;
            if target.connector == citadel_platforms::ConnectorKind::Local  {
                let result = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                    result = self.docker.delete_container(docker_container_id, true, true) => result,
                };
                return match result {
                    Ok(())
                    | Err(DockerError::Api {
                        status: reqwest::StatusCode::NOT_FOUND,
                        ..
                    }) => Ok(()),
                    Err(error) => Err(runtime(error)),
                };
            }
            if matches!(target.connector, citadel_platforms::ConnectorKind::Agent | citadel_platforms::ConnectorKind::EdgeAgent) {
                return match self
                    .agent_for(&target)?
                    .delete_container(docker_container_id, cancellation)
                    .await
                {
                    Ok(())
                    | Err(RuntimeCapabilityError {
                        kind: RuntimeErrorKind::NotFound,
                        ..
                    }) => Ok(()),
                    Err(error) => Err(agent_runtime(error)),
                };
            }
            Err(edge_unavailable())
        }
        .boxed()
    }
}

struct PlatformTarget {
    platform_id: Uuid,
    connector: citadel_platforms::ConnectorKind,
    address: String,
}

struct RegistryPull {
    image: String,
    auth: Option<Zeroizing<String>>,
}

pub(crate) fn qualify_image_reference(host: &str, image: &str) -> Result<String, DeploymentError> {
    let image = image.trim();
    if image.is_empty() || image.len() > 2048 {
        return Err(DeploymentError::Validation(
            "The external image reference is invalid.".to_owned(),
        ));
    }
    let docker_hub = host.eq_ignore_ascii_case("hub.docker.com")
        || host.eq_ignore_ascii_case("docker.io")
        || host.eq_ignore_ascii_case("registry-1.docker.io");
    let first = image.split('/').next().unwrap_or_default();
    let already_qualified = image.contains('/')
        && (first.contains('.') || first.contains(':') || first.eq_ignore_ascii_case("localhost"));
    let mut qualified = if docker_hub || already_qualified {
        image.to_owned()
    } else {
        format!(
            "{}/{}",
            host.trim_end_matches('/'),
            image.trim_start_matches('/')
        )
    };
    let last = qualified.rsplit('/').next().unwrap_or_default();
    if !qualified.contains('@') && !last.contains(':') {
        qualified.push_str(":latest");
    }
    Ok(qualified)
}

pub(crate) fn registry_auth(
    registry_id: Uuid,
    host: &str,
    configuration: &Value,
) -> Result<Option<Zeroizing<String>>, DeploymentError> {
    let kind = configuration
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let field = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| configuration.get(*name).and_then(Value::as_str))
    };
    let enabled = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| configuration.get(*name).and_then(Value::as_bool))
    };
    let credentials = match kind {
        "DockerHub" => field(&["UserName", "userName", "username"]).zip(field(&["PAT", "pat"])),
        "GitHub" => {
            if enabled(&["GhcrAuthEnabled", "ghcrAuthEnabled"]) == Some(true) {
                field(&["NameSpace", "nameSpace"]).zip(field(&["PAT", "pat"]))
            } else {
                None
            }
        }
        "Custom" => {
            if enabled(&["AuthEnabled", "authEnabled"]) == Some(true) {
                field(&["UserName", "userName", "username"]).zip(field(&["Password", "password"]))
            } else {
                None
            }
        }
        "" if registry_id == DEFAULT_DOCKER_HUB_ID => None,
        _ => {
            return Err(DeploymentError::Validation(
                "This Registry type is not available for Deployment Apply in the Rust server yet."
                    .to_owned(),
            ));
        }
    };
    let Some((username, password)) = credentials else {
        return Ok(None);
    };
    let json = serde_json::to_vec(&json!({
        "username": username,
        "password": password,
        "serveraddress": host,
    }))
    .map_err(|error| DeploymentError::Storage(error.to_string()))?;
    Ok(Some(Zeroizing::new(
        base64::engine::general_purpose::STANDARD.encode(json),
    )))
}

fn find_image<I>(requested: &str, images: I) -> Result<PreparedDeploymentImage, DeploymentError>
where
    I: IntoIterator<Item = (String, Vec<String>, Vec<String>)>,
{
    let images = images.into_iter().collect::<Vec<_>>();
    let repository = tag_repository(requested);
    for exact_only in [true, false] {
        for (id, tags, digests) in &images {
            let exact = tags.iter().any(|tag| tag == requested)
                || digests.iter().any(|digest| digest == requested);
            let same_repository = tags.iter().any(|tag| tag_repository(tag) == repository)
                || digests
                    .iter()
                    .any(|digest| tag_repository(digest) == repository);
            if (exact_only && !exact) || (!exact_only && !same_repository) {
                continue;
            }
            let digest = digests
                .iter()
                .find(|digest| tag_repository(digest) == repository);
            return Ok(PreparedDeploymentImage {
                docker_image_id: id.clone(),
                digest: digest.cloned(),
                resolved_build: None,
            });
        }
    }
    Err(DeploymentError::Runtime(format!(
        "Docker pulled '{requested}' but did not report the image afterwards."
    )))
}

fn tag_repository(reference: &str) -> &str {
    let without_digest = reference.split('@').next().unwrap_or(reference);
    let last_slash = without_digest.rfind('/');
    match without_digest.rfind(':') {
        Some(colon) if last_slash.is_none_or(|slash| colon > slash) => &without_digest[..colon],
        _ => without_digest,
    }
}

fn docker_create_body(command: &RuntimeDeploymentCommand) -> Result<Value, DeploymentError> {
    let mut command = command.clone();
    normalize_resource_limits(&mut command);
    add_ownership_labels(&mut command);
    let mut exposed_ports = serde_json::Map::new();
    let mut port_bindings = serde_json::Map::new();
    for value in command.spec.ports.as_deref().unwrap_or_default() {
        let (key, host_port) = parse_port(value)?;
        exposed_ports.insert(key.clone(), json!({}));
        if let Some(host_port) = host_port {
            port_bindings.insert(key, json!([{ "HostPort": host_port.to_string() }]));
        }
    }
    let mounts = command
        .spec
        .volumes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|value| parse_mount(value))
        .collect::<Result<Vec<_>, _>>()?;
    let endpoints = command
        .spec
        .networks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|network| (network.to_owned(), json!({})))
        .collect::<serde_json::Map<_, _>>();
    let resource = command.spec.resource_spec.as_ref();
    let memory = resource
        .and_then(|value| value.memory_limit)
        .map(|value| value as i64)
        .unwrap_or_default();
    let nano_cpus = resource
        .and_then(|value| value.nano_cpus)
        .map(|value| value as i64)
        .unwrap_or_default();
    let lifecycle = command.spec.life_cycle_spec.as_ref();
    let restart = lifecycle.map_or(ContainerRestartPolicy::No, |value| value.restart_policy);
    Ok(json!({
        "Image": command.image_id,
        "Cmd": command.spec.command,
        "Env": command.environment_variables,
        "Labels": command.spec.labels,
        "StopSignal": lifecycle.and_then(|value| value.stop_signal).map(stop_signal),
        "StopTimeout": lifecycle.and_then(|value| value.stop_timeout),
        "ExposedPorts": exposed_ports,
        "HostConfig": {
            "PortBindings": port_bindings,
            "Mounts": mounts,
            "Memory": memory,
            "NanoCpus": nano_cpus,
            "RestartPolicy": { "Name": restart_policy(restart) }
        },
        "NetworkingConfig": { "EndpointsConfig": endpoints }
    }))
}

fn normalize_resource_limits(command: &mut RuntimeDeploymentCommand) {
    if let Some(resource) = command.spec.resource_spec.as_mut() {
        resource.nano_cpus = resource
            .nano_cpus
            .filter(|value| *value > 0.0)
            .map(|value| value * 1_000_000_000.0);
        resource.memory_limit = resource
            .memory_limit
            .filter(|value| *value > 0.0)
            .map(|value| value * 1024.0 * 1024.0);
    }
}

fn add_ownership_labels(command: &mut RuntimeDeploymentCommand) {
    let labels = command.spec.labels.get_or_insert_with(BTreeMap::new);
    labels.insert(MANAGED_LABEL.to_owned(), "true".to_owned());
    labels.insert(
        DEPLOYMENT_LABEL.to_owned(),
        command.deployment_id.to_string(),
    );
}

const fn stop_signal(signal: StopSignal) -> &'static str {
    match signal {
        StopSignal::SIGTERM => "SIGTERM",
        StopSignal::SIGKILL => "SIGKILL",
        StopSignal::SIGINT => "SIGINT",
        StopSignal::SIGQUIT => "SIGQUIT",
    }
}

const fn restart_policy(policy: ContainerRestartPolicy) -> &'static str {
    match policy {
        ContainerRestartPolicy::No => "no",
        ContainerRestartPolicy::Always => "always",
        ContainerRestartPolicy::OnFailure => "on-failure",
        ContainerRestartPolicy::UnlessStopped => "unless-stopped",
    }
}

fn container_summary_state(state: &str, status: &str) -> RuntimeContainerState {
    if state.eq_ignore_ascii_case("running") && !status.to_ascii_lowercase().contains("unhealthy") {
        RuntimeContainerState::Running
    } else {
        RuntimeContainerState::Exited
    }
}

fn runtime(error: DockerError) -> DeploymentError {
    DeploymentError::Runtime(error.to_string())
}

fn agent_runtime(error: RuntimeCapabilityError) -> DeploymentError {
    if error.kind == RuntimeErrorKind::Cancelled {
        DeploymentError::Cancelled
    } else {
        DeploymentError::Runtime(error.to_string())
    }
}

fn edge_unavailable() -> DeploymentError {
    DeploymentError::Runtime("The Edge Agent is disconnected or unavailable.".to_owned())
}

fn storage(error: sqlx::Error) -> DeploymentError {
    DeploymentError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::docker::deployments::observed_container_state;
    use citadel_deployments::{DeploymentImageInfo, DeploymentSpec, UpdateBehavior};

    fn command() -> RuntimeDeploymentCommand {
        RuntimeDeploymentCommand {
            deployment_id: Uuid::from_u128(0x1234),
            name: "web".to_owned(),
            image_id: "sha256:image".to_owned(),
            spec: DeploymentSpec {
                image: DeploymentImageInfo::Local {
                    image_id: Uuid::from_u128(0x99).to_string(),
                },
                update_behavior: UpdateBehavior::Disabled,
                life_cycle_spec: None,
                resource_spec: None,
                labels: None,
                ports: Some(vec!["8080:80/tcp".to_owned(), "53/udp".to_owned()]),
                volumes: Some(vec!["data:/data:ro".to_owned()]),
                networks: Some(vec!["frontend".to_owned()]),
                command: Some(vec!["nginx".to_owned()]),
                environment_variables: Some(vec!["IGNORED=source".to_owned()]),
            },
            environment_variables: vec!["ACTIVE=resolved".to_owned()],
        }
    }

    #[test]
    fn create_body_maps_runtime_values_and_ownership_without_source_environment() {
        let body = docker_create_body(&command()).unwrap();

        assert_eq!(body["Image"], "sha256:image");
        assert_eq!(body["Env"][0], "ACTIVE=resolved");
        assert!(body.to_string().find("IGNORED=source").is_none());
        assert_eq!(body["ExposedPorts"]["80/tcp"], json!({}));
        assert_eq!(
            body["HostConfig"]["PortBindings"]["80/tcp"][0]["HostPort"],
            "8080"
        );
        assert_eq!(body["HostConfig"]["Mounts"][0]["Type"], "volume");
        assert_eq!(body["HostConfig"]["Mounts"][0]["ReadOnly"], true);
        assert_eq!(body["HostConfig"]["Memory"], 0);
        assert_eq!(body["HostConfig"]["NanoCpus"], 0);
        assert_eq!(
            body["Labels"][DEPLOYMENT_LABEL],
            Uuid::from_u128(0x1234).to_string()
        );
        assert_eq!(body["Labels"][MANAGED_LABEL], "true");
    }

    #[test]
    fn image_reference_qualification_preserves_ports_and_adds_default_tag() {
        assert_eq!(
            qualify_image_reference("hub.docker.com", "nginx").unwrap(),
            "nginx:latest"
        );
        assert_eq!(
            qualify_image_reference("registry.example:5000", "team/web").unwrap(),
            "registry.example:5000/team/web:latest"
        );
        assert_eq!(
            tag_repository("registry.example:5000/team/web:1.2"),
            "registry.example:5000/team/web"
        );
        let selected = find_image(
            "team/web:latest",
            [
                (
                    "sha256:old".to_owned(),
                    vec!["team/web:old".to_owned()],
                    vec!["team/web@sha256:old".to_owned()],
                ),
                (
                    "sha256:current".to_owned(),
                    vec!["team/web:latest".to_owned()],
                    vec!["team/web@sha256:current".to_owned()],
                ),
            ],
        )
        .unwrap();
        assert_eq!(selected.docker_image_id, "sha256:current");
    }

    #[test]
    fn malformed_port_and_mount_are_rejected_before_docker_is_called() {
        assert!(parse_port("abc:80").is_err());
        assert!(parse_port("80/http").is_err());
        assert!(parse_mount("data:relative").is_err());
        assert!(parse_mount("a:/data:ro:extra").is_err());
    }

    #[test]
    fn container_health_takes_precedence_over_the_running_flag() {
        let mut state = crate::connectors::docker::projection::ContainerState {
            running: true,
            health: Some(crate::connectors::docker::projection::ContainerHealth {
                status: "starting".to_owned(),
            }),
            ..Default::default()
        };

        assert_eq!(observed_container_state(&state), None);
        state.health.as_mut().unwrap().status = "unhealthy".to_owned();
        assert_eq!(
            observed_container_state(&state),
            Some(RuntimeContainerState::Exited)
        );
        state.health.as_mut().unwrap().status = "healthy".to_owned();
        assert_eq!(
            observed_container_state(&state),
            Some(RuntimeContainerState::Running)
        );
        assert_eq!(
            container_summary_state("running", "Up 30 seconds (unhealthy)"),
            RuntimeContainerState::Exited
        );
    }
}
