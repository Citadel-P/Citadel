use citadel_platforms::{ContainerInventoryPort, PlatformInfoPort};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, containers::*};
use citadel_runtime::runtime_metrics::RuntimeWork;
use futures_util::{StreamExt, future::BoxFuture};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::connectors::agent::client::AgentClient;
use crate::connectors::agent::client::AgentContainerAction;
use crate::connectors::docker::DockerClient;
use crate::connectors::edge::EdgeRegistry;
use crate::connectors::edge::EdgeRuntime;
use crate::connectors::edge::EdgeTarget;

#[derive(Clone)]
pub struct ContainerRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
    edge: EdgeRegistry,
    local_calls: Arc<tokio::sync::Semaphore>,
}

pub(crate) enum Runtime<'a> {
    Local(&'a DockerClient),
    Agent(Arc<AgentClient>),
    Edge(EdgeRuntime),
}

impl ContainerRuntimeRouter {
    pub fn new(
        pool: PgPool,
        docker: DockerClient,
        agent: Option<AgentClient>,
        edge: EdgeRegistry,
    ) -> Self {
        Self {
            pool,
            docker,
            agent,
            edge,
            local_calls: Arc::new(tokio::sync::Semaphore::new(CONTAINER_IO_CONCURRENCY)),
        }
    }

    pub fn into_service(
        self,
        tasks: std::sync::Arc<dyn citadel_platforms::containers::ContainerTaskSpawner>,
    ) -> ContainerMutationService {
        ContainerMutationService::new(
            std::sync::Arc::new(
                crate::persistence::postgres::platforms::containers::repository::PostgresContainerRepository::new(
                    self.pool.clone(),
                ),
            ),
            std::sync::Arc::new(self),
            tasks,
        )
    }

    pub(crate) async fn resolve(
        &self,
        target: &ContainerTarget,
        cancellation: &CancellationToken,
    ) -> Result<Runtime<'_>, RuntimeCapabilityError> {
        let _resolve = RuntimeWork::ContainerRuntimeResolve.start();
        let row = sqlx::query("SELECT connectortype,address,status,platformdescriptor->>'nodeID' nodeid FROM platforms WHERE id=$1")
            .bind(target.platform_id).fetch_optional(&self.pool).await.map_err(storage)?.ok_or_else(unavailable)?;
        if row.get::<String, _>("status") != "Online" {
            return Err(unavailable());
        }
        if let Some(node) = &target.node_id {
            let stale: Option<bool> = sqlx::query_scalar(
                "SELECT isstale FROM swarmnodeprojections WHERE platformid=$1 AND dockernodeid=$2",
            )
            .bind(target.platform_id)
            .bind(node)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            if stale != Some(false) {
                return Err(unavailable());
            }
            if let Ok(session) = self
                .edge
                .get(&EdgeTarget::node(target.platform_id, node.clone()))
            {
                return Ok(Runtime::Edge(EdgeRuntime { session }));
            }
            if row.get::<Option<String>, _>("nodeid").as_ref() != Some(node) {
                return Err(unavailable());
            }
        }
        let connector: String = row.get("connectortype");
        let address: String = row.get("address");
        let runtime = match connector.as_str() {
            "Local" => Runtime::Local(&self.docker),
            "Agent" => Runtime::Agent(Arc::new(
                self.agent
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .at_address(&address)?,
            )),
            "EdgeAgent" => Runtime::Edge(EdgeRuntime {
                session: self
                    .edge
                    .get(&EdgeTarget::platform(target.platform_id))
                    .map_err(|_| unavailable())?,
            }),
            _ => return Err(unavailable()),
        };
        if let Some(node) = &target.node_id {
            let info = match &runtime {
                Runtime::Local(r) => r.get_info(cancellation).await,
                Runtime::Agent(r) => r.get_info(cancellation).await,
                Runtime::Edge(r) => r.get_info(cancellation).await,
            }?;
            if !info.swarm.is_some_and(|s| {
                s.node_id == *node && s.local_node_state.eq_ignore_ascii_case("active")
            }) {
                return Err(unavailable());
            }
        }
        Ok(runtime)
    }
}

impl ContainerMutationRuntime for ContainerRuntimeRouter {
    fn mutate<'a>(
        &'a self,
        target: &'a ContainerTarget,
        action: ContainerAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        self.mutate_batch(std::slice::from_ref(target), action, cancellation)
    }
    fn mutate_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        action: ContainerAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let _mutation = RuntimeWork::ContainerMutation.start();
            RuntimeWork::ContainerMutation.units(targets.len() as u64);
            let mut errors = Vec::new();
            for targets in groups(targets).values() {
                let result = tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=self.resolve(targets[0], cancellation)=>result };
                let runtime = match result {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        errors.extend(targets.iter().map(|t| (t.id, copy_error(&error))));
                        continue;
                    }
                };
                if let Runtime::Local(runtime) = runtime {
                    // No tasks are spawned; the shared permit also bounds concurrent
                    // operations on this Local daemon, including verification.
                    let mut pending = futures_util::stream::iter(targets.iter().map(|t| (*t).clone()).collect::<Vec<_>>()).map(|target| async move {
                        let work = async {
                            let _permit = self.local_calls.acquire().await.map_err(|_| cancelled())?;
                            runtime.change_container_state(&target.docker_id, action).await.map_err(crate::connectors::docker::runtime::normalize_docker_error)
                        };
                        let result = tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=work=>result };
                        (target.id, result)
                    }).buffer_unordered(CONTAINER_IO_CONCURRENCY);
                    while let Some((id, result)) = pending.next().await {
                        if let Err(error) = result {
                            errors.push((id, error));
                        }
                    }
                } else {
                    let ids: Vec<_> = targets.iter().map(|t| t.docker_id.clone()).collect();
                    let result = mutate_remote(&runtime, ids, action, cancellation).await;
                    // Protocols return a batch acknowledgement, not per-ID outcomes.
                    // A failed acknowledgement is ambiguous for every member; verify all.
                    if let Err(error) = result {
                        errors.extend(targets.iter().map(|t| (t.id, copy_error(&error))));
                    }
                }
            }
            container_batch_result(errors)
        })
    }
    fn observe<'a>(
        &'a self,
        target: &'a ContainerTarget,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut results = self
                .observe_batch(std::slice::from_ref(target), cancellation)
                .await;
            if let Some((_, error)) = results.errors.pop() {
                Err(error)
            } else {
                Ok(results.observed.pop().expect("one target observed").state)
            }
        })
    }
    fn observe_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, ContainerObservations> {
        Box::pin(async move {
            let mut results = ContainerObservations::default();
            for targets in groups(targets).values() {
                let resolved = tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=self.resolve(targets[0], cancellation)=>result };
                let runtime = match resolved {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        for target in targets {
                            results.push(target, Err(copy_error(&error)));
                        }
                        continue;
                    }
                };
                // Match the sequential completion verification budget. Mutations
                // retain their own bounded concurrency; each outcome is still inspected.
                let raw = futures_util::stream::iter(targets.iter().map(|t| (*t).clone()).collect::<Vec<_>>()).map(|target| {
                    let runtime = &runtime;
                    async move {
                        let _verification = RuntimeWork::ContainerVerification.start();
                        let work = async {
                            let _permit = if matches!(runtime, Runtime::Local(_)) { Some(self.local_calls.acquire().await.map_err(|_| cancelled())?) } else { None };
                            inspect(runtime, &target.docker_id, cancellation).await
                        };
                        let result = tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=work=>result };
                        (target, result)
                    }
                }).buffer_unordered(1).collect::<Vec<_>>().await;
                // Legacy Edge errors lose Docker's 404. A successful list on the
                // exact same session proves absence, once per group, only on failure.
                let present = if raw.iter().any(|(_, result)| result.is_err()) {
                    if let Runtime::Edge(runtime) = &runtime {
                        runtime
                            .list_containers(cancellation)
                            .await
                            .ok()
                            .map(|values| {
                                values
                                    .into_iter()
                                    .map(|v| v.id)
                                    .collect::<std::collections::BTreeSet<_>>()
                            })
                    } else {
                        None
                    }
                } else {
                    None
                };
                for (target, observed) in raw {
                    let state = match observed {
                        Ok((id, state)) if id == target.docker_id && !state.is_empty() => {
                            Ok(Some(state))
                        }
                        Ok(_) => Err(RuntimeCapabilityError::new(
                            RuntimeErrorKind::Remote,
                            "Container inspection returned an invalid identity or state.",
                            false,
                        )),
                        Err(_)
                            if present
                                .as_ref()
                                .is_some_and(|ids| !ids.contains(&target.docker_id)) =>
                        {
                            Ok(None)
                        }
                        Err(error) if error.kind == RuntimeErrorKind::NotFound => Ok(None),
                        Err(error) => Err(error),
                    };
                    results.push(&target, state);
                }
            }
            results
        })
    }
}
fn groups(
    targets: &[ContainerTarget],
) -> std::collections::BTreeMap<(uuid::Uuid, Option<&str>), Vec<&ContainerTarget>> {
    let mut groups = std::collections::BTreeMap::new();
    for target in targets {
        groups
            .entry((target.platform_id, target.node_id.as_deref()))
            .or_insert_with(Vec::new)
            .push(target);
    }
    groups
}
fn copy_error(error: &RuntimeCapabilityError) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(error.kind, error.message.clone(), error.retryable)
}
async fn inspect(
    runtime: &Runtime<'_>,
    id: &str,
    cancellation: &CancellationToken,
) -> Result<(String, String), RuntimeCapabilityError> {
    match runtime {
        Runtime::Local(runtime) => runtime
            .inspect_container(id)
            .await
            .map(|c| (c.id, c.state.status))
            .map_err(crate::connectors::docker::runtime::normalize_docker_error),
        Runtime::Agent(runtime) => runtime
            .inspect_container(id, cancellation)
            .await
            .and_then(agent_state),
        Runtime::Edge(runtime) => {
            use citadel_contracts::citadel::{
                containers::v1::InspectContainerRequest, edge::v1::EdgeCommandKind,
            };
            crate::connectors::agent::execution::unary(
                &runtime.session,
                EdgeCommandKind::ContainerInspect,
                InspectContainerRequest {
                    container_id: id.into(),
                },
                cancellation,
            )
            .await
            .and_then(agent_state)
        }
    }
}
async fn mutate_remote(
    runtime: &Runtime<'_>,
    ids: Vec<String>,
    action: ContainerAction,
    cancellation: &CancellationToken,
) -> Result<(), RuntimeCapabilityError> {
    match runtime {
        Runtime::Local(_) => unreachable!(),
        Runtime::Agent(runtime) => match action {
            ContainerAction::Delete(options) => {
                runtime
                    .delete_containers_with_options(&ids, options, cancellation)
                    .await
            }
            _ => {
                runtime
                    .change_containers_state(&ids, agent_action(action), cancellation)
                    .await
            }
        },
        Runtime::Edge(runtime) => {
            use citadel_contracts::citadel::{
                containers::v1::{ContainerIds, DeleteContainerRequest},
                edge::v1::EdgeCommandKind as Kind,
            };
            if let ContainerAction::Delete(options) = action {
                crate::connectors::agent::execution::unary::<_, ()>(
                    &runtime.session,
                    Kind::ContainerDelete,
                    DeleteContainerRequest {
                        ids,
                        v: Some(options.v),
                        force: Some(options.force),
                        link: Some(options.link),
                    },
                    cancellation,
                )
                .await
            } else {
                let kind = match action {
                    ContainerAction::Start => Kind::ContainerStart,
                    ContainerAction::Stop => Kind::ContainerStop,
                    ContainerAction::Restart => Kind::ContainerRestart,
                    ContainerAction::Pause => Kind::ContainerPause,
                    ContainerAction::Unpause => Kind::ContainerUnpause,
                    ContainerAction::Delete(_) => unreachable!(),
                };
                crate::connectors::agent::execution::unary::<_, ()>(
                    &runtime.session,
                    kind,
                    ContainerIds { ids },
                    cancellation,
                )
                .await
            }
        }
    }
}

fn agent_action(action: ContainerAction) -> AgentContainerAction {
    match action {
        ContainerAction::Start => AgentContainerAction::Start,
        ContainerAction::Stop => AgentContainerAction::Stop,
        ContainerAction::Restart => AgentContainerAction::Restart,
        ContainerAction::Pause => AgentContainerAction::Pause,
        ContainerAction::Unpause => AgentContainerAction::Unpause,
        ContainerAction::Delete(_) => unreachable!(),
    }
}

fn agent_state(
    response: citadel_contracts::citadel::shared_models::v1::InspectContainerResponse,
) -> Result<(String, String), RuntimeCapabilityError> {
    use citadel_contracts::citadel::shared_models::v1::ContainerStateType;
    let state = response.state.ok_or_else(|| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Remote,
            "Container inspection did not include its state.",
            false,
        )
    })?;
    let state = ContainerStateType::try_from(state.status).map_err(|_| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Remote,
            "Container inspection returned an invalid state.",
            false,
        )
    })?;
    Ok((response.id, state.as_str_name().to_owned()))
}
fn unavailable() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "The owning Container runtime is disconnected or unavailable.",
        false,
    )
}
fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Container operation cancelled.",
        false,
    )
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}
