use crate::*;

pub trait BuildExecutor: Send + Sync {
    /// Preparation remains charged until its last owned input buffer is consumed.
    /// Local executors conservatively retain it through their execution.
    fn execute_with_preparation<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn BuildLogSink,
        cancellation: &'a CancellationToken,
        preparation: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            let _preparation = preparation;
            self.execute(claim, logs, cancellation).await
        })
    }

    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn BuildLogSink,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult>;
}

pub trait BuildSecretResolver: Send + Sync {
    fn resolve(
        &self,
        secret_id: Uuid,
        build_id: Uuid,
    ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BuildError>>;
}

pub struct BuildRegistryCredentials {
    pub username: String,
    pub password: zeroize::Zeroizing<String>,
}

pub trait BuildRegistryCredentialResolver: Send + Sync {
    fn resolve(
        &self,
        registry_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRegistryCredentials>, BuildError>>;
}

pub trait BuildEntitlements: Send + Sync {
    fn enabled(
        &self,
        capability: citadel_licensing::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, BuildError>>;
}
