use crate::*;

pub trait BuildExecutor: Send + Sync {
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
