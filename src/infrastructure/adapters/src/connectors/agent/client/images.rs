use super::*;
use citadel_contracts::citadel::images::v1::{
    GetExposedPortsRequest, GetExposedPortsResponse, InspectImageRequest, InspectImageResponse,
};
use citadel_platforms::images::{ImageContainer, ImageInspection, ImageInspectionPort, ImageLayer};

impl AgentClient {
    pub async fn inspect_image_document(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<serde_json::Value, RuntimeCapabilityError> {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                InspectImageRequest { id: id.into() },
                "/citadel.images.v1.ImageService/Inspect",
                Some(self.operation_timeout),
            )?;
            let response = self
                .image_client()
                .inspect(request)
                .await
                .map_err(normalize_status)?
                .into_inner();
            serde_json::to_value(response).map_err(|_| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    "Invalid Image inspection.",
                    false,
                )
            })
        })
        .await
    }
}

impl citadel_platforms::images::ImageDeletionPort for AgentClient {
    fn delete_image<'a>(
        &'a self,
        id: &'a str,
        force: bool,
        no_prune: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Vec<std::collections::BTreeMap<String, String>>, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            let request = self.signer.sign(
                citadel_contracts::citadel::images::v1::DeleteImageRequest {
                    ids: vec![id.into()],
                    force,
                    noprune: no_prune,
                },
                "/citadel.images.v1.ImageService/Delete",
                Some(self.operation_timeout),
            )?;
            let mut client = self.image_client();
            let response = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.delete(request)) =>
                    result.map_err(|_| timeout_error("deleting an Image"))?.map_err(normalize_status)?.into_inner(),
            };
            Ok(response
                .items
                .into_iter()
                .map(|item| item.result.into_iter().collect())
                .collect())
        })
    }
}

impl citadel_platforms::images::ImageDeletionPort for crate::connectors::edge::EdgeRuntime {
    fn delete_image<'a>(
        &'a self,
        id: &'a str,
        force: bool,
        no_prune: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Vec<std::collections::BTreeMap<String, String>>, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            let response: citadel_contracts::citadel::images::v1::DeleteImageResponse =
                crate::connectors::agent::execution::unary(
                    &self.session,
                    citadel_contracts::citadel::edge::v1::EdgeCommandKind::ImageDelete,
                    citadel_contracts::citadel::images::v1::DeleteImageRequest {
                        ids: vec![id.into()],
                        force,
                        noprune: no_prune,
                    },
                    cancellation,
                )
                .await?;
            Ok(response
                .items
                .into_iter()
                .map(|item| item.result.into_iter().collect())
                .collect())
        })
    }
}

impl ImageInspectionPort for AgentClient {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.retry_unary(cancellation, || async {
                let request = self.signer.sign(
                    GetExposedPortsRequest { id: id.into() },
                    "/citadel.images.v1.ImageService/GetExposedPorts",
                    Some(self.operation_timeout),
                )?;
                self.image_client()
                    .get_exposed_ports(request)
                    .await
                    .map(|response| response.into_inner().ports)
                    .map_err(normalize_status)
            })
            .await
        })
    }

    fn inspect_image<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImageInspection, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.retry_unary(cancellation, || async {
                let request = self.signer.sign(
                    InspectImageRequest { id: id.into() },
                    "/citadel.images.v1.ImageService/Inspect",
                    Some(self.operation_timeout),
                )?;
                self.image_client()
                    .inspect(request)
                    .await
                    .map(|response| map_inspection(response.into_inner()))
                    .map_err(normalize_status)
            })
            .await
        })
    }
}

impl ImageInspectionPort for crate::connectors::edge::EdgeRuntime {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: GetExposedPortsResponse = crate::connectors::agent::execution::unary(
                &self.session,
                citadel_contracts::citadel::edge::v1::EdgeCommandKind::ImageExposedPorts,
                GetExposedPortsRequest { id: id.into() },
                cancellation,
            )
            .await?;
            Ok(response.ports)
        })
    }

    fn inspect_image<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImageInspection, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: InspectImageResponse = crate::connectors::agent::execution::unary(
                &self.session,
                citadel_contracts::citadel::edge::v1::EdgeCommandKind::ImageInspect,
                InspectImageRequest { id: id.into() },
                cancellation,
            )
            .await?;
            Ok(map_inspection(response))
        })
    }
}

fn map_inspection(image: InspectImageResponse) -> ImageInspection {
    let mut value = ImageInspection {
        id: image.id, size: image.size, os: image.os.unwrap_or_else(|| "unknown".into()),
        created: image.created, architecture: image.architecture.unwrap_or_else(|| "unknown".into()),
        user: image.user, working_dir: image.working_dir, entry_point: image.entry_point, stop_signal: image.stop_signal,
        env: image.env, cmd: image.cmd, repo_tags: image.repo_tags, volumes: image.volumes,
        exposed_ports: image.exposed_ports, labels: image.labels.into_iter().collect(),
        layers: image.layers.into_iter().map(|layer| ImageLayer {id:layer.id,created:layer.created,created_by:layer.created_by,size:layer.size,comment:layer.comment}).collect(),
        containers: image.containers.into_iter().map(|container| {
            let state = container.state().as_str_name().to_ascii_lowercase();
            ImageContainer {
                id:container.id,name:container.name,state,volumes:container.volumes,networks:container.networks.into_iter().collect(),
                ports: serde_json::Value::Object(container.ports.into_iter().map(|(port,bindings)| {
                    (port,serde_json::Value::Array(bindings.host_port_binding.into_iter().map(|binding| serde_json::json!({"hostIP":binding.host_ip,"hostPort":binding.host_port})).collect()))
                }).collect()),
            }
        }).collect(),
        ..Default::default()
    };
    value.volumes.sort();
    value.exposed_ports.sort();
    value.set_display_reference();
    value
}
