use crate::AuthorizedReadError;
use crate::*;
use citadel_primitives::ActorId;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PlatformReadService {
    store: Arc<dyn PlatformReader>,
    inventory_reads: Arc<std::sync::Mutex<BTreeMap<Uuid, std::sync::Weak<tokio::sync::Mutex<()>>>>>,
}

impl PlatformReadService {
    pub async fn container_identities(
        &self,
        platform: Uuid,
    ) -> Result<Vec<ContainerIdentity>, AuthorizedReadError> {
        self.store.container_identities(platform).await
    }
    pub async fn platform_telemetry(
        &self,
        platform: Uuid,
    ) -> Result<Option<PlatformTelemetryContext>, AuthorizedReadError> {
        self.store.platform_telemetry(platform).await
    }

    #[must_use]
    pub fn new(store: Arc<dyn PlatformReader>) -> Self {
        Self {
            store,
            inventory_reads: Default::default(),
        }
    }

    /// Coalesce concurrent lazy initialization on one Platform without serializing
    /// different Platforms or holding database locks while querying the daemon.
    pub async fn inventory_guard(&self, platform: Uuid) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self
                .inventory_reads
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            locks.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = locks.get(&platform).and_then(std::sync::Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                locks.insert(platform, Arc::downgrade(&lock));
                lock
            }
        };
        lock.lock_owned().await
    }

    pub async fn list_authorized(
        &self,
        actor_id: ActorId,
        is_administrator: bool,
        tag_ids: &[Uuid],
    ) -> Result<Vec<PlatformDetails>, AuthorizedReadError> {
        self.store
            .list_authorized(actor_id, is_administrator, tag_ids)
            .await
    }

    pub async fn get_platform(
        &self,
        id: Uuid,
    ) -> Result<Option<PlatformDetails>, AuthorizedReadError> {
        self.store.get_platform(id).await
    }

    pub async fn permissions_for_platforms(
        &self,
        actor_id: ActorId,
        platform_ids: &[Uuid],
    ) -> Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError> {
        self.store
            .permissions_for_platforms(actor_id, platform_ids)
            .await
    }

    pub async fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<ContainerDetails>, AuthorizedReadError> {
        self.store.list_containers(platform_id).await
    }

    pub async fn get_container(
        &self,
        id: Uuid,
    ) -> Result<Option<ContainerDetails>, AuthorizedReadError> {
        self.store.get_container(id).await
    }

    pub async fn containers_by_runtime_id(
        &self,
        platform_id: Uuid,
        docker_id: &str,
    ) -> Result<Vec<ContainerDetails>, AuthorizedReadError> {
        self.store
            .containers_by_runtime_id(platform_id, docker_id)
            .await
    }

    pub async fn containers_by_runtime_ids(
        &self,
        platform_id: Uuid,
        docker_ids: &[String],
    ) -> Result<Vec<ContainerDetails>, AuthorizedReadError> {
        self.store
            .containers_by_runtime_ids(platform_id, docker_ids)
            .await
    }

    pub async fn list_stack_containers(
        &self,
        stack_id: Uuid,
    ) -> Result<Vec<ContainerDetails>, AuthorizedReadError> {
        self.store.list_stack_containers(stack_id).await
    }

    pub async fn get_container_by_reference(
        &self,
        reference: &str,
    ) -> Result<Option<ContainerDetails>, AuthorizedReadError> {
        let Some(id) = self.store.resolve_container_reference(reference).await? else {
            return Ok(None);
        };
        self.store.get_container(id).await
    }

    pub async fn list_images(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<ImageDetails>, AuthorizedReadError> {
        self.store.list_images(platform_id).await
    }

    pub async fn list_node_volumes(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<NodeResourceProjection<crate::RuntimeVolumeSummary>>, AuthorizedReadError> {
        self.store.list_node_volumes(platform_id).await
    }

    pub async fn list_node_networks(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<NodeResourceProjection<crate::RuntimeNetworkSummary>>, AuthorizedReadError>
    {
        self.store.list_node_networks(platform_id).await
    }

    pub async fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmNodeSummary>, AuthorizedReadError> {
        self.store.list_swarm_nodes(platform_id).await
    }

    pub async fn swarm_summary(
        &self,
        platform: Uuid,
    ) -> Result<crate::swarm_overview::SwarmSummary, AuthorizedReadError> {
        self.store.swarm_summary(platform).await
    }

    pub async fn get_swarm_node(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmNodeSummary>, AuthorizedReadError> {
        self.store.get_swarm_node(platform_id, id).await
    }

    pub async fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmServiceSummary>, AuthorizedReadError> {
        self.store.list_swarm_services(platform_id).await
    }

    pub async fn get_swarm_service(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmServiceSummary>, AuthorizedReadError> {
        self.store.get_swarm_service(platform_id, id).await
    }

    pub async fn list_swarm_tasks(
        &self,
        platform_id: Uuid,
        service_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SwarmTaskSummary>, AuthorizedReadError> {
        self.store
            .list_swarm_tasks(platform_id, service_id, limit)
            .await
    }

    pub async fn get_swarm_task(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmTaskSummary>, AuthorizedReadError> {
        self.store.get_swarm_task(platform_id, id).await
    }

    pub async fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmConfigSummary>, AuthorizedReadError> {
        self.store.list_swarm_configs(platform_id).await
    }

    pub async fn get_swarm_config(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmConfigSummary>, AuthorizedReadError> {
        self.store.get_swarm_config(platform_id, id).await
    }

    pub async fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmNetworkSummary>, AuthorizedReadError> {
        self.store.list_swarm_networks(platform_id).await
    }

    pub async fn get_swarm_network(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmNetworkSummary>, AuthorizedReadError> {
        self.store.get_swarm_network(platform_id, id).await
    }

    pub async fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmSecretSummary>, AuthorizedReadError> {
        self.store.list_swarm_secrets(platform_id).await
    }

    pub async fn get_swarm_secret(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmSecretSummary>, AuthorizedReadError> {
        self.store.get_swarm_secret(platform_id, id).await
    }
}
