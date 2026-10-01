use super::runtime::Runtime;
use crate::connectors::swarm::inventory::SwarmInventoryClient;
use citadel_platforms::{RuntimeCapabilityError, RuntimeSwarmNode, swarm_mutations::*};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

impl Runtime<'_> {
    fn swarm_client(&self) -> SwarmInventoryClient<'_> {
        match self {
            Self::Local(r) => SwarmInventoryClient::Local(r),
            Self::Agent(r) => SwarmInventoryClient::Agent(r),
            Self::Edge(r) => SwarmInventoryClient::Edge(r),
        }
    }
}
impl SwarmControlPort for Runtime<'_> {
    fn inspect_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<SwarmServiceInspection, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.swarm_client()
                .inspect_service(id, cancel)
                .await
                .map(map_service)
        })
    }
    fn inspect_node<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(RuntimeSwarmNode, i32, i32), RuntimeCapabilityError>> {
        Box::pin(async move { self.swarm_client().inspect_node(id, cancel).await })
    }
    fn update_node<'a>(
        &'a self,
        id: &'a str,
        input: &'a UpdateSwarmNodeInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move { self.swarm_client().update_node(id, input, cancel).await })
    }
    fn restart_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move { self.swarm_client().restart_service(id, cancel).await })
    }
    fn delete_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move { self.swarm_client().delete_service(id, cancel).await })
    }
    fn create_material<'a>(
        &'a self,
        secret: bool,
        input: &'a CreateSwarmMaterialInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.swarm_client()
                .create_material(secret, input, cancel)
                .await
        })
    }
    fn update_labels<'a>(
        &'a self,
        secret: bool,
        id: &'a str,
        input: &'a UpdateSwarmResourceLabelsInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.swarm_client()
                .update_labels(secret, id, input, cancel)
                .await
        })
    }
    fn delete_material<'a>(
        &'a self,
        secret: bool,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.swarm_client()
                .delete_material(secret, id, cancel)
                .await
        })
    }
    fn config_data<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>> {
        Box::pin(async move { self.swarm_client().config_data(id, cancel).await })
    }
}
fn map_service(
    value: citadel_contracts::citadel::swarm::v1::SwarmServiceMessage,
) -> SwarmServiceInspection {
    SwarmServiceInspection {
        id: value.id,
        version_index: value.version_index,
        name: value.name,
        mode: value.mode,
        image: value.image,
        running_task_count: value.running_task_count,
        desired_task_count: value.desired_task_count,
        update_state: value.update_state,
        update_message: (!value.update_message.is_empty()).then_some(value.update_message),
        ports: value.ports,
        network_ids: value.network_ids,
        secret_ids: value.secret_ids,
        config_ids: value.config_ids,
        labels: value.labels.into_iter().collect(),
        created_at: value
            .created_at
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)),
        updated_at: value
            .updated_at
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)),
    }
}

impl SwarmSnapshotPort for super::runtime::PlatformRuntimeRouter {
    fn snapshot<'a>(
        &'a self,
        platform: &'a citadel_platforms::PlatformDetails,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<citadel_platforms::RuntimeInventorySnapshot, RuntimeCapabilityError>>
    {
        Box::pin(async move {
            let runtime = self.swarm(platform.id, cancel).await?;
            citadel_platforms::jobs::collect_swarm_snapshot(
                &runtime,
                &citadel_platforms::jobs::InventoryCollectionTarget {
                    platform_id: platform.id,
                    platform_type: platform.platform_type,
                },
                cancel,
            )
            .await
        })
    }
}
