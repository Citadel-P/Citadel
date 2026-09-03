use citadel_deployments::{DeploymentError, DeploymentRuntimePort};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::docker::{DockerClient, DockerError};

#[derive(Clone)]
pub struct DeploymentRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
}

impl DeploymentRuntimeRouter {
    #[must_use]
    pub fn new(pool: PgPool, docker: DockerClient, agent: Option<AgentClient>) -> Self {
        Self {
            pool,
            docker,
            agent,
        }
    }
}

impl DeploymentRuntimePort for DeploymentRuntimeRouter {
    fn delete_container<'a>(
        &'a self,
        platform_id: Uuid,
        docker_container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        async move {
            let row = sqlx::query("SELECT connectortype,address FROM platforms WHERE id=$1")
                .bind(platform_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| DeploymentError::Storage(error.to_string()))?
                .ok_or(DeploymentError::NotFound)?;
            let connector: String = row
                .try_get("connectortype")
                .map_err(|error| DeploymentError::Storage(error.to_string()))?;
            let address: String = row
                .try_get("address")
                .map_err(|error| DeploymentError::Storage(error.to_string()))?;
            if connector.eq_ignore_ascii_case("Local") {
                let result = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                    result = self.docker.delete_container(docker_container_id, true, true) => result,
                };
                return match result {
                    Ok(()) | Err(DockerError::Api { status: reqwest::StatusCode::NOT_FOUND, .. }) => Ok(()),
                    Err(error) => Err(DeploymentError::Runtime(error.to_string())),
                };
            }
            if connector.eq_ignore_ascii_case("Agent") {
                let agent = self
                    .agent
                    .as_ref()
                    .filter(|agent| agent.address().trim_end_matches('/') == address.trim_end_matches('/'))
                    .ok_or_else(|| DeploymentError::Runtime("The configured Agent transport is unavailable.".to_owned()))?;
                return match agent.delete_container(docker_container_id, cancellation).await {
                    Ok(())
                    | Err(RuntimeCapabilityError {
                        kind: RuntimeErrorKind::NotFound,
                        ..
                    }) => Ok(()),
                    Err(error) => Err(DeploymentError::Runtime(error.to_string())),
                };
            }
            Err(DeploymentError::Runtime(
                "The Edge Agent is disconnected or unavailable.".to_owned(),
            ))
        }
        .boxed()
    }
}
