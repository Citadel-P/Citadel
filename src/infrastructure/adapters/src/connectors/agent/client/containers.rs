use super::*;
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest, shared_models::v1::InspectContainerResponse,
};

impl AgentClient {
    pub async fn container_processes(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<
        citadel_contracts::citadel::containers::v1::ContainerProcessesResponse,
        RuntimeCapabilityError,
    > {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                InspectContainerRequest {
                    container_id: id.into(),
                },
                "/citadel.containers.v1.ContainerService/Processes",
                Some(self.operation_timeout),
            )?;
            self.container_client()
                .processes(request)
                .await
                .map(|response| response.into_inner())
                .map_err(normalize_status)
        })
        .await
    }

    pub async fn inspect_container(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<InspectContainerResponse, RuntimeCapabilityError> {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                InspectContainerRequest {
                    container_id: id.into(),
                },
                "/citadel.containers.v1.ContainerService/Inspect",
                Some(self.operation_timeout),
            )?;
            self.container_client()
                .inspect(request)
                .await
                .map(|response| response.into_inner())
                .map_err(normalize_status)
        })
        .await
    }
}
