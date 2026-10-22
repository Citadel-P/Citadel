use crate::connectors::agent::client::AgentClient;
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest, shared_models::v1::InspectContainerResponse,
};
use citadel_platforms::{RuntimeCapabilityError, containers::ContainerInspectionPort};
use futures_util::future::BoxFuture;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::connectors::edge::EdgeRuntime;
use citadel_docker::containers::inspection::*;

impl ContainerInspectionPort for AgentClient {
    fn processes<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<citadel_platforms::containers::ContainerProcesses, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            let value = self.container_processes(id, cancel).await?;
            Ok(map_processes(value))
        })
    }

    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Value, RuntimeCapabilityError>> {
        Box::pin(async move { map_agent(self.inspect_container(id, cancel).await?, id) })
    }
}
impl ContainerInspectionPort for EdgeRuntime {
    fn processes<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<citadel_platforms::containers::ContainerProcesses, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            let value = crate::connectors::agent::execution::unary::<
                _,
                citadel_contracts::citadel::containers::v1::ContainerProcessesResponse,
            >(
                &self.session,
                citadel_contracts::citadel::edge::v1::EdgeCommandKind::ContainerProcesses,
                InspectContainerRequest {
                    container_id: id.into(),
                },
                cancel,
            )
            .await?;
            Ok(map_processes(value))
        })
    }

    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Value, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response = crate::connectors::agent::execution::unary(
                &self.session,
                citadel_contracts::citadel::edge::v1::EdgeCommandKind::ContainerInspect,
                InspectContainerRequest {
                    container_id: id.into(),
                },
                cancel,
            )
            .await?;
            map_agent(response, id)
        })
    }
}
fn map_agent(
    response: InspectContainerResponse,
    id: &str,
) -> Result<Value, RuntimeCapabilityError> {
    let document = serde_json::to_value(response)
        .map_err(|_| invalid("Invalid Agent inspection response."))?;
    map_inspection(document, id)
}

fn map_processes(
    value: citadel_contracts::citadel::containers::v1::ContainerProcessesResponse,
) -> citadel_platforms::containers::ContainerProcesses {
    // Bound Agent responses again before they reach the shared WebSocket.
    let cells = |values: Vec<String>| {
        values
            .into_iter()
            .take(5)
            .map(|value| value.chars().take(128).collect())
            .collect()
    };
    citadel_platforms::containers::ContainerProcesses {
        running: true,
        titles: cells(value.titles),
        truncated: value.truncated || value.processes.len() > 1024,
        processes: value
            .processes
            .into_iter()
            .take(1024)
            .map(|row| cells(row.values))
            .collect(),
    }
}
