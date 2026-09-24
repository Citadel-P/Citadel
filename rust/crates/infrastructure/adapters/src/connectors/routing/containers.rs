use citadel_platforms::{
    PlatformRuntimePort, RuntimeCapabilityError, RuntimeErrorKind, containers::*,
};
use futures_util::future::BoxFuture;
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
        Box::pin(async move {
            let work = async {
                match self.resolve(target, cancellation).await? {
                    Runtime::Local(runtime) => runtime
                        .change_container_state(&target.docker_id, action)
                        .await
                        .map_err(crate::connectors::docker::runtime::normalize_docker_error),
                    Runtime::Agent(runtime) => match action {
                        ContainerAction::Delete(options) => {
                            runtime
                                .delete_container_with_options(
                                    &target.docker_id,
                                    options,
                                    cancellation,
                                )
                                .await
                        }
                        _ => {
                            runtime
                                .change_containers_state(
                                    std::slice::from_ref(&target.docker_id),
                                    agent_action(action),
                                    cancellation,
                                )
                                .await
                        }
                    },
                    Runtime::Edge(runtime) => {
                        use citadel_contracts::citadel::{
                            containers::v1::{ContainerIds, DeleteContainerRequest},
                            edge::v1::EdgeCommandKind as Kind,
                        };
                        if let ContainerAction::Delete(options) = action {
                            crate::connectors::agent::execution::unary(
                                &runtime.session,
                                Kind::ContainerDelete,
                                DeleteContainerRequest {
                                    ids: vec![target.docker_id.clone()],
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
                            crate::connectors::agent::execution::unary(
                                &runtime.session,
                                kind,
                                ContainerIds {
                                    ids: vec![target.docker_id.clone()],
                                },
                                cancellation,
                            )
                            .await
                        }
                    }
                }
            };
            tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=work=>result }
        })
    }

    fn observe<'a>(
        &'a self,
        target: &'a ContainerTarget,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let work = async {
                let observed = match self.resolve(target, cancellation).await? {
                    Runtime::Local(runtime) => runtime
                        .inspect_container(&target.docker_id)
                        .await
                        .map(|c| (c.id, c.state.status))
                        .map_err(crate::connectors::docker::runtime::normalize_docker_error),
                    Runtime::Agent(runtime) => runtime
                        .inspect_container(&target.docker_id, cancellation)
                        .await
                        .and_then(agent_state),
                    Runtime::Edge(runtime) => {
                        use citadel_contracts::citadel::{
                            containers::v1::InspectContainerRequest, edge::v1::EdgeCommandKind,
                        };
                        let inspected = crate::connectors::agent::execution::unary(
                            &runtime.session,
                            EdgeCommandKind::ContainerInspect,
                            InspectContainerRequest {
                                container_id: target.docker_id.clone(),
                            },
                            cancellation,
                        )
                        .await
                        .and_then(agent_state);
                        // The current Agent's failure envelope does not preserve Docker's 404.
                        // Prove absence with a successful read on the *same* node; a failed
                        // inspection or disconnected node alone never means "deleted".
                        if inspected.is_err()
                            && runtime
                                .list_containers(cancellation)
                                .await
                                .is_ok_and(|containers| {
                                    !containers.iter().any(|c| c.id == target.docker_id)
                                })
                        {
                            return Ok(None);
                        }
                        inspected
                    }
                };
                match observed {
                    Ok((id, state)) if id == target.docker_id && !state.is_empty() => {
                        Ok(Some(state))
                    }
                    Ok(_) => Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Container inspection returned an invalid identity or state.",
                        false,
                    )),
                    Err(error) if error.kind == RuntimeErrorKind::NotFound => Ok(None),
                    Err(error) => Err(error),
                }
            };
            tokio::select! { biased; ()=cancellation.cancelled()=>Err(cancelled()), result=work=>result }
        })
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
