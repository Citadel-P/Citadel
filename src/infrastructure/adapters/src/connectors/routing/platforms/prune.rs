use crate::connectors::agent::client::AgentClient;
use crate::connectors::edge::EdgeRuntime;
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind,
    platforms::v1::{PruneRequest, PruneResponse},
};
use citadel_platforms::{RuntimeCapabilityError, prune::*};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

impl PlatformPrunePort for AgentClient {
    fn prune<'a>(
        &'a self,
        resource: PruneResource,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PrunePlatformOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(map(
                resource,
                self.prune_platform(
                    PruneRequest {
                        resource: resource.code(),
                    },
                    cancel,
                )
                .await?,
            ))
        })
    }
}
impl PlatformPrunePort for EdgeRuntime {
    fn prune<'a>(
        &'a self,
        resource: PruneResource,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PrunePlatformOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response = crate::connectors::agent::execution::unary(
                &self.session,
                EdgeCommandKind::PlatformPrune,
                PruneRequest {
                    resource: resource.code(),
                },
                cancel,
            )
            .await?;
            Ok(map(resource, response))
        })
    }
}
fn map(resource: PruneResource, v: PruneResponse) -> PrunePlatformOutcome {
    PrunePlatformOutcome {
        resource,
        space_reclaimed: v.space_reclaimed,
        volumes_deleted: v.volumes_deleted,
        networks_deleted: v.networks_deleted,
        images_deleted: v.images_deleted,
        build_cache_deleted: v.build_cache_deleted,
    }
}
