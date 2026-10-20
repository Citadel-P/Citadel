//! Registry authorization and transport adapters for image metadata checks.
use super::digest_cache::ImageDigestCache;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, image_updates::*};
use citadel_primitives::ActorId;
use citadel_registries::RegistryRepository;
use futures_util::future::BoxFuture;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct ImageUpdateAdapter {
    pub pool: sqlx::PgPool,
    pub runtime: crate::connectors::routing::platforms::runtime::PlatformRuntimeRouter,
    pub cache: Arc<ImageDigestCache>,
}
pub type MetadataKey = (Uuid, String, String, ImagePlatform);
#[derive(Default)]
pub struct MetadataAttempt {
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<Result<RegistryImageObservation, (RuntimeErrorKind, String)>>,
}
impl MetadataAttempt {
    fn reusable(&self, reference: &str, requested: chrono::DateTime<chrono::Utc>) -> bool {
        self.completed_at.is_some_and(|at| {
            at >= requested
                || (reference.contains('@')
                    && requested - at < chrono::Duration::days(1)
                    && self.result.as_ref().is_some_and(Result::is_ok))
        })
    }
}
impl ImageUpdateAdapter {
    pub(crate) async fn load_registry(
        &self,
        id: Uuid,
    ) -> Result<ImageUpdateRegistry, RuntimeCapabilityError> {
        let config =
            crate::persistence::postgres::registries::pull_configuration::load(&self.pool, id)
                .await
                .map_err(|_| failure("Registry configuration is unavailable."))?
                .filter(|config| config.status != "Disabled")
                .ok_or_else(|| failure("Registry configuration is unavailable."))?;
        Ok(ImageUpdateRegistry {
            id,
            revision: revision(&config.host, &config.configuration),
            host: config.host,
        })
    }
}
impl ImageUpdateResolver for ImageUpdateAdapter {
    fn registries(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<ImageUpdateRegistry>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let repository =
                crate::persistence::postgres::registries::PostgresRegistryRepository::new(
                    self.pool.clone(),
                );
            let registries = repository
                .list_registries(actor, administrator)
                .await
                .map_err(|_| failure("Registry access could not be verified."))?;
            Ok(registries
                .into_iter()
                .filter(|r| r.status != citadel_registries::RegistryStatus::Disabled)
                .map(|r| ImageUpdateRegistry {
                    id: r.id,
                    host: r.registry_host.clone(),
                    revision: revision(&r.registry_host, &r.configuration),
                })
                .collect())
        })
    }
    fn observe<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        registry: &'a ImageUpdateRegistry,
        reference: &'a str,
        target: &'a ImagePlatform,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RegistryImageObservation, RuntimeCapabilityError>> {
        Box::pin(async move {
            let requested = chrono::Utc::now();
            let key = (
                registry.id,
                reference.to_owned(),
                registry.revision.clone(),
                target.clone(),
            );
            let slot = self.cache.metadata_slot(key)?;
            let mut entry = tokio::select! {()=cancel.cancelled()=>return Err(failure("Image check cancelled.")),entry=slot.lock()=>entry};
            // Pinned documents are immutable. Share successful platform-specific
            // observations across workload checks without reusing mutable tags.
            if entry.reusable(reference, requested)
                && let Some(result) = &entry.result
            {
                return result
                    .clone()
                    .map_err(|(kind, message)| RuntimeCapabilityError::new(kind, message, false));
            }
            let auth = if registry.id.is_nil() {
                None
            } else {
                let configuration =
                    crate::persistence::postgres::registries::pull_configuration::load(
                        &self.pool,
                        registry.id,
                    )
                    .await
                    .map_err(|_| failure("Registry configuration is unavailable."))?
                    .ok_or_else(|| failure("Registry configuration is unavailable."))?;
                if configuration.status == "Disabled"
                    || revision(&configuration.host, &configuration.configuration)
                        != registry.revision
                {
                    return Err(failure("Registry configuration changed. Check again."));
                }
                crate::connectors::routing::deployments::registry_auth(
                    registry.id,
                    &configuration.host,
                    &configuration.configuration,
                )
                .map_err(|_| failure("Registry credentials are unavailable."))?
            };
            let result=async {
                use crate::connectors::routing::platforms::runtime::Runtime;
                use citadel_contracts::citadel::images::v1::{DistributionInspectRequest,DistributionInspectResponse,OciPlatformResponse};
                let request=DistributionInspectRequest {image_name:reference.into(),auth:auth.as_deref().cloned(),target_platform:Some(OciPlatformResponse {os:target.os.clone(),architecture:target.architecture.clone(),variant:target.variant.clone(),..Default::default()})};
                let response:DistributionInspectResponse=match self.runtime.resolve(platform,node,true,cancel).await? {
                    Runtime::Local(_)=>return citadel_docker::image_metadata::inspect(reference,auth.as_deref().map(String::as_str),target,cancel).await,
                    Runtime::Agent(agent)=>agent.node_agent_distribution(request,cancel).await.map_err(metadata_transport_error)?,
                    Runtime::Edge(edge)=>crate::connectors::agent::execution::unary(&edge.session,citadel_contracts::citadel::edge::v1::EdgeCommandKind::ImageDistributionInspect,request,cancel).await.map_err(metadata_transport_error)?,
                };
                if !valid_digest(&response.config_digest) || !valid_digest(&response.manifest_digest) {return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable,"The Agent did not provide comparable manifest metadata. Update the Agent and check again.",false));}
                if let Some((_, expected)) = reference.split_once('@')
                    && response.descriptor.as_ref().is_none_or(|descriptor| descriptor.digest != expected) {
                    return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable,"The Agent did not confirm the requested immutable image.",false));
                }
                Ok(RegistryImageObservation {config_digest:response.config_digest,manifest_digest:response.manifest_digest,index_digest:response.descriptor.map(|d|d.digest).unwrap_or_default(),observed_at:chrono::Utc::now()})
            }.await;
            if let Ok(observed) = &result {
                self.cache.set(
                    registry.id,
                    reference,
                    observed.index_digest.clone(),
                    observed.observed_at,
                );
            }
            entry.completed_at = Some(chrono::Utc::now());
            entry.result = Some(
                result
                    .as_ref()
                    .cloned()
                    .map_err(|e| (e.kind, e.message.clone())),
            );
            result
        })
    }
}
fn valid_digest(digest: &str) -> bool {
    digest
        .strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn revision(host: &str, configuration: &serde_json::Value) -> String {
    Sha256::digest(format!("{host}\n{configuration}"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}

// Preserve the error category across Direct and Edge without returning transport text.
fn metadata_transport_error(error: RuntimeCapabilityError) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        error.kind,
        "Agent registry metadata check failed. Verify connectivity and Registry access.",
        error.retryable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_successful_immutable_observations_outlive_overlapping_checks() {
        let now = chrono::Utc::now();
        let mut entry = MetadataAttempt {
            completed_at: Some(now - chrono::Duration::seconds(1)),
            result: Some(Ok(RegistryImageObservation {
                index_digest: format!("sha256:{}", "a".repeat(64)),
                manifest_digest: format!("sha256:{}", "b".repeat(64)),
                config_digest: format!("sha256:{}", "c".repeat(64)),
                observed_at: now,
            })),
        };
        let pinned = format!("app@sha256:{}", "a".repeat(64));
        assert!(entry.reusable(&pinned, now));
        assert!(!entry.reusable("app:latest", now));
        assert!(!entry.reusable(&pinned, now + chrono::Duration::days(1)));
        entry.result = Some(Err((RuntimeErrorKind::Unavailable, "unavailable".into())));
        assert!(!entry.reusable(&pinned, now));
        assert!(entry.reusable("app:latest", now - chrono::Duration::seconds(2)));
    }
    #[test]
    fn agent_metadata_failures_keep_their_category_but_drop_transport_text() {
        for kind in [
            RuntimeErrorKind::Authentication,
            RuntimeErrorKind::NotFound,
            RuntimeErrorKind::ResourceExhausted,
            RuntimeErrorKind::Unavailable,
            RuntimeErrorKind::Cancelled,
        ] {
            let error = metadata_transport_error(RuntimeCapabilityError::new(
                kind,
                "private-transport-secret",
                false,
            ));
            assert_eq!(error.kind, kind);
            assert!(!error.message.contains("secret"));
        }
    }
}
