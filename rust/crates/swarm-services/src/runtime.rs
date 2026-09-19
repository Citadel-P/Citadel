use crate::*;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

pub trait SwarmServiceRuntime: Send + Sync {
    fn apply<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn scale<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        replicas: i32,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn force_update<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn delete<'a>(
        &'a self,
        platform_id: Uuid,
        docker_service_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn observe<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeServiceResult>, SwarmServiceError>>;
}

#[derive(Debug)]
pub struct ResolvedSwarmServiceBinding {
    pub name: String,
    pub value: Zeroizing<String>,
    pub secret: bool,
}

#[derive(Debug, Default)]
pub struct ResolvedSwarmServiceBindings {
    pub entries: Vec<ResolvedSwarmServiceBinding>,
}

pub trait SwarmServiceBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        service_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedSwarmServiceBindings, SwarmServiceError>>;
}

#[derive(Default)]
pub struct EmptySwarmServiceBindingResolver;
impl SwarmServiceBindingResolverPort for EmptySwarmServiceBindingResolver {
    fn resolve<'a>(
        &'a self,
        _service_id: Uuid,
        _referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedSwarmServiceBindings, SwarmServiceError>> {
        Box::pin(async { Ok(ResolvedSwarmServiceBindings::default()) })
    }
}
