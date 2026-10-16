use super::*;
use citadel_platforms::SwarmInventoryPort;
use citadel_swarm_services::ServiceImageDigestPort;

impl ServiceImageDigestPort for SwarmServiceRuntimeRouter {
    fn content_changed<'a>(
        &'a self,
        service: &'a citadel_swarm_services::SwarmService,
        remote: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, SwarmServiceError>> {
        Box::pin(async move {
            let (registry, reference) = citadel_swarm_services::checkable_image(service)?;
            let id = service.docker_service_id.as_deref().ok_or_else(|| {
                SwarmServiceError::Runtime("The applied Service is unavailable.".into())
            })?;
            let manager = self
                .runtime
                .swarm(service.platform_id, cancel)
                .await
                .map_err(update_check_error)?;
            let (nodes, tasks) = tokio::try_join!(
                manager.list_swarm_nodes(cancel),
                manager.list_swarm_tasks(cancel)
            )
            .map_err(update_check_error)?;
            let targets =
                crate::connectors::registries::workload_updates::swarm_targets(id, &nodes, &tasks)
                    .map_err(update_check_error)?;
            let adapter = crate::connectors::registries::image_updates::ImageUpdateAdapter {
                pool: self.pool.clone(),
                runtime: self.runtime.clone(),
                cache: self.image_cache.clone(),
            };
            let check = crate::connectors::registries::workload_updates::WorkloadImageCheck {
                platform: service.platform_id,
                registry,
                reference,
                current: service.applied_image_digest.as_deref().unwrap(),
                remote,
            };
            adapter
                .workload_content_changed(&check, &targets, cancel)
                .await
                .map_err(update_check_error)
        })
    }
    fn cached_digest<'a>(
        &'a self,
        _platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, SwarmServiceError>> {
        Box::pin(async move {
            if !self.image_cache.wait_ready(cancel).await {
                return Err(SwarmServiceError::Cancelled);
            }
            Ok(self
                .image_cache
                .get(registry, reference)
                .map(|entry| entry.digest))
        })
    }
    fn digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, SwarmServiceError>> {
        Box::pin(async move {
            let agent =
                if self.connector(platform).await? == citadel_platforms::ConnectorKind::Local {
                    None
                } else {
                    Some(self.agent_for(platform, cancel).await?)
                };
            crate::connectors::registries::digest::inspect(
                &self.pool,
                &self.docker,
                agent,
                registry,
                reference,
                cancel,
            )
            .await
            .map_err(|message| SwarmServiceError::Runtime(message.into()))
        })
    }
}

fn update_check_error(error: citadel_platforms::RuntimeCapabilityError) -> SwarmServiceError {
    if error.kind == citadel_platforms::RuntimeErrorKind::Cancelled {
        SwarmServiceError::Cancelled
    } else {
        SwarmServiceError::Runtime("Could not compare the Service's image content.".into())
    }
}
