//! Consumer-owned runtime, binding, and entitlement capabilities.
use crate::*;
use citadel_domain::LicenseCapability;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
pub trait DeploymentRuntime: Send + Sync {
    fn cached_image_digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, DeploymentError>> {
        Box::pin(async move {
            self.remote_image_digest(platform, registry, reference, cancellation)
                .await
                .map(Some)
        })
    }
    fn remote_image_digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, DeploymentError>> {
        let _ = (platform, registry, reference, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checking is unavailable.".into(),
            ))
        })
    }
    fn delete_container<'a>(
        &'a self,
        platform_id: Uuid,
        docker_container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn prepare_image<'a>(
        &'a self,
        platform_id: Uuid,
        image: &'a DeploymentImageInfo,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PreparedDeploymentImage, DeploymentError>> {
        let _ = (platform_id, image, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment image preparation is unavailable.".to_owned(),
            ))
        })
    }

    fn apply_container<'a>(
        &'a self,
        platform_id: Uuid,
        command: &'a RuntimeDeploymentCommand,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeDeploymentResult, DeploymentError>> {
        let _ = (platform_id, command, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment runtime Apply is unavailable.".to_owned(),
            ))
        })
    }

    fn observe_deployment<'a>(
        &'a self,
        platform_id: Uuid,
        deployment_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeDeploymentResult>, DeploymentError>> {
        let _ = (platform_id, deployment_id, cancellation);
        Box::pin(async { Ok(None) })
    }
}

pub trait DeploymentBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        deployment_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedDeploymentBindings, DeploymentError>>;
}

#[derive(Default)]
pub struct EmptyDeploymentBindingResolver;

impl DeploymentBindingResolverPort for EmptyDeploymentBindingResolver {
    fn resolve<'a>(
        &'a self,
        _deployment_id: Uuid,
        _referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedDeploymentBindings, DeploymentError>> {
        Box::pin(async { Ok(ResolvedDeploymentBindings::default()) })
    }
}

pub trait DeploymentEntitlementPort: Send + Sync {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, DeploymentError>>;
}
