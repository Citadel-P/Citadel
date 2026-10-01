//! Scoped inspection selections and inventory preconditions shared by transports.
use super::PlatformReadService;
use crate::{AuthorizedReadError, ContainerDetails, ImageIdentity, PlatformKind};
use uuid::Uuid;

impl PlatformReadService {
    pub async fn platform_kind(&self, id: Uuid) -> Result<PlatformKind, AuthorizedReadError> {
        self.store
            .platform_kind(id)
            .await?
            .ok_or(AuthorizedReadError::NotFound)
    }
    pub async fn has_swarm_nodes(&self, id: Uuid) -> Result<bool, AuthorizedReadError> {
        self.store.has_swarm_nodes(id).await
    }
    pub async fn deployment_container(
        &self,
        id: Uuid,
    ) -> Result<ContainerDetails, AuthorizedReadError> {
        let ids = self.store.deployment_container_ids(id).await?;
        let container_id = unique_deployment_container(&ids)?;
        let container = self
            .store
            .get_container(container_id)
            .await?
            .ok_or(AuthorizedReadError::NotFound)?;
        // A concurrent reparenting must not turn this into an unrelated inspection.
        if container.deployment_id != Some(id) {
            return Err(AuthorizedReadError::NotFound);
        }
        Ok(container)
    }
    pub async fn stack_container(
        &self,
        stack: Uuid,
        reference: &str,
    ) -> Result<ContainerDetails, AuthorizedReadError> {
        let id = self
            .store
            .stack_container_id(stack, reference)
            .await?
            .ok_or(AuthorizedReadError::NotFound)?;
        let container = self
            .store
            .get_container(id)
            .await?
            .ok_or(AuthorizedReadError::NotFound)?;
        if container.stack_id != Some(stack) {
            return Err(AuthorizedReadError::NotFound);
        }
        Ok(container)
    }
    pub async fn image_identity(
        &self,
        platform: Uuid,
        image: Uuid,
    ) -> Result<ImageIdentity, AuthorizedReadError> {
        self.store
            .image_identity(platform, image)
            .await?
            .ok_or(AuthorizedReadError::NotFound)
    }
    pub async fn image_registry_id(
        &self,
        platform: Uuid,
        docker_image: &str,
    ) -> Result<Option<Uuid>, AuthorizedReadError> {
        self.store.image_registry_id(platform, docker_image).await
    }
    pub async fn validate_swarm_network_deletion(
        &self,
        platform: Uuid,
        network: &str,
        name: &str,
    ) -> Result<(), AuthorizedReadError> {
        let projection = self.store.get_swarm_network(platform, network).await?;
        let current = projection.filter(|p| !p.is_stale).ok_or_else(|| {
            AuthorizedReadError::Conflict(format!(
                "Swarm network '{name}' has no current inventory observation and cannot be deleted."
            ))
        })?;
        if !current.service_names.is_empty() {
            return Err(AuthorizedReadError::Conflict(format!(
                "Network '{name}' is used by one or more Services and cannot be deleted."
            )));
        }
        Ok(())
    }
}

fn unique_deployment_container(ids: &[Uuid]) -> Result<Uuid, AuthorizedReadError> {
    match ids {
        [id] => Ok(*id),
        [] => Err(AuthorizedReadError::NotFound),
        _ => Err(AuthorizedReadError::Conflict(
            "Deployment container identity is ambiguous. Refresh inventory before inspecting."
                .into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deployment_inspection_rejects_missing_or_ambiguous_identity() {
        let id = Uuid::now_v7();
        assert_eq!(unique_deployment_container(&[id]).unwrap(), id);
        assert!(matches!(
            unique_deployment_container(&[]),
            Err(AuthorizedReadError::NotFound)
        ));
        assert!(matches!(
            unique_deployment_container(&[id, Uuid::now_v7()]),
            Err(AuthorizedReadError::Conflict(_))
        ));
    }
}
