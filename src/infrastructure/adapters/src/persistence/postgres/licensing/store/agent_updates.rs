use super::*;

impl citadel_updates::AgentUpdateEntitlements for PostgresLicenseEntitlementService {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, citadel_updates::UpdateError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::AutomatedOperations)
                .await
                .map_err(|error| citadel_updates::UpdateError::Storage(error.to_string()))
        })
    }
}
