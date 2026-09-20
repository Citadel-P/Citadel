use super::*;
use citadel_swarm_services::ServiceImageDigestPort;

impl ServiceImageDigestPort for SwarmServiceRuntimeRouter {
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
                    Some(self.agent_for(platform).await?)
                };
            crate::registry_digest::inspect(
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
