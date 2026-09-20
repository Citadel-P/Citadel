use crate::*;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub trait StackRuntime: Send + Sync {
    fn apply<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a crate::StackApplySource,
        environment: &'a [String],
        cancellation: &'a CancellationToken,
        progress: Option<&'a StackProgress>,
    ) -> BoxFuture<'a, Result<StackRuntimeResult, StackError>>;
    fn observe<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<StackRuntimeResult>, StackError>>;
    fn delete<'a>(
        &'a self,
        claim: &'a StackDeletionClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn change_state<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        action: StackAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, StackError>>;
    fn runtime_snapshot<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeSnapshot, StackError>>;
    fn reconcile<'a>(
        &'a self,
        platform_id: Uuid,
        drifts: &'a [StackDrift],
        policy: &'a crate::StackDriftPolicy,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<crate::StackReconciliationAction>, StackError>>;
    fn import_claim<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        import_kind: Option<crate::StackImportKind>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackImportClaim, StackError>>;
}

pub trait StackSourceMaterializerPort: Send + Sync {
    fn materialize<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<crate::StackApplySource, StackError>>;
}

pub trait StackBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        stack_id: Uuid,
        names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedStackBindings, StackError>>;
}

pub trait StackBuildImageResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        bindings: &'a [crate::StackBuildImageBinding],
    ) -> BoxFuture<'a, Result<Vec<ResolvedStackBuildImageBinding>, StackError>>;
}
