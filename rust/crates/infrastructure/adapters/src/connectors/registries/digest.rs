//! Shared distribution inspection; neither pull nor credentials appear in errors.
use crate::connectors::agent::execution::AgentExecutionClient;
use crate::connectors::docker::DockerClient;
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind,
    images::v1::{DistributionInspectRequest, DistributionInspectResponse},
};
use serde_json::Value;
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) const CHECK_FAILED: &str =
    "Registry update check failed. Verify registry connectivity and credentials.";

pub(crate) async fn inspect(
    pool: &PgPool,
    docker: &DockerClient,
    agent: Option<AgentExecutionClient>,
    registry: Uuid,
    reference: &str,
    cancel: &CancellationToken,
) -> Result<String, &'static str> {
    let row = sqlx::query(
        "SELECT registryhost,configuration FROM registries WHERE id=$1 AND status<>'Disabled'",
    )
    .bind(registry)
    .fetch_optional(pool)
    .await
    .map_err(|_| CHECK_FAILED)?
    .ok_or("The configured Registry does not exist or is disabled.")?;
    let host: String = row.try_get("registryhost").map_err(|_| CHECK_FAILED)?;
    let configuration: Value = row.try_get("configuration").map_err(|_| CHECK_FAILED)?;
    let image = crate::connectors::routing::deployments::qualify_image_reference(&host, reference)
        .map_err(|_| CHECK_FAILED)?;
    let auth =
        crate::connectors::routing::deployments::registry_auth(registry, &host, &configuration)
            .map_err(|_| CHECK_FAILED)?;
    let digest = match agent {
        None => {
            let response = docker
                .distribution_inspect_authenticated(&image, auth.as_deref().map(String::as_str))
                .await
                .map_err(|_| CHECK_FAILED)?;
            response
                .get("Descriptor")
                .and_then(|v| v.get("digest"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        }
        Some(agent) => {
            let request = DistributionInspectRequest {
                image_name: image,
                auth: auth.as_deref().cloned(),
            };
            let response: DistributionInspectResponse = match agent {
                AgentExecutionClient::Direct(agent) => agent
                    .node_agent_distribution(request, cancel)
                    .await
                    .map_err(|_| CHECK_FAILED)?,
                AgentExecutionClient::Edge(session) => crate::connectors::agent::execution::unary(
                    &session,
                    EdgeCommandKind::ImageDistributionInspect,
                    request,
                    cancel,
                )
                .await
                .map_err(|_| CHECK_FAILED)?,
            };
            response.descriptor.map(|v| v.digest).unwrap_or_default()
        }
    };
    if !digest
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(CHECK_FAILED);
    }
    Ok(digest)
}
