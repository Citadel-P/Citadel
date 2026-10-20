use crate::client::DockerClient;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, prune::*};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
impl PlatformPrunePort for DockerClient {
    fn prune<'a>(
        &'a self,
        resource: PruneResource,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PrunePlatformOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            let kinds: &[PruneResource] = if resource == PruneResource::All {
                &[
                    PruneResource::Volume,
                    PruneResource::Network,
                    PruneResource::Image,
                    PruneResource::Build,
                ]
            } else {
                std::slice::from_ref(&resource)
            };
            let mut result = PrunePlatformOutcome::empty(resource);
            for kind in kinds {
                let value = tokio::select! {
                    biased;
                    () = cancel.cancelled() => return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled,"Prune cancelled. Completed deletions are not rolled back.",false)),
                    value = self.prune_resources(*kind) => value.map_err(|e| RuntimeCapabilityError::new(RuntimeErrorKind::Remote,format!("Prune failed; earlier deletions may have completed: {e}"),false))?,
                };
                result.space_reclaimed = result.space_reclaimed.saturating_add(
                    value
                        .get("SpaceReclaimed")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                );
                let strings = |field| {
                    value
                        .get(field)
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                };
                result.volumes_deleted.extend(strings("VolumesDeleted"));
                result.networks_deleted.extend(strings("NetworksDeleted"));
                result.build_cache_deleted.extend(strings("CachesDeleted"));
                for item in value
                    .get("ImagesDeleted")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    for key in ["Deleted", "Untagged"] {
                        if let Some(id) = item
                            .get(key)
                            .and_then(serde_json::Value::as_str)
                            .filter(|v| !v.is_empty())
                        {
                            result.images_deleted.push(id.into());
                        }
                    }
                }
            }
            Ok(result)
        })
    }
}
