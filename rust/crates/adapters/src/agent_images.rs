use super::*;
use citadel_contracts::citadel::images::v1::{
    GetExposedPortsRequest, GetExposedPortsResponse, InspectImageRequest, InspectImageResponse,
};
use citadel_platforms::images::{ImageContainer, ImageInspection, ImageInspectionPort, ImageLayer};

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

impl ImageInspectionPort for crate::edge::EdgeRuntime {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: GetExposedPortsResponse = crate::agent_execution::unary(
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
            let response: InspectImageResponse = crate::agent_execution::unary(
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
