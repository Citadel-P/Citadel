use std::sync::Arc;

use citadel_backups::{BackupClaim, BackupError, BackupRunAuthorizer, RestoreClaim};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use uuid::Uuid;

#[derive(Clone)]
pub struct IdentityBackupRunAuthorizer {
    identity: Arc<IdentityService>,
}

impl IdentityBackupRunAuthorizer {
    pub fn new(identity: Arc<IdentityService>) -> Self {
        Self { identity }
    }

    async fn authorize_resource(
        &self,
        principal: &ActorPrincipal,
        resource_type: ResourceType,
        resource_id: Uuid,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> Result<(), BackupError> {
        self.identity
            .authorize_resource(principal, resource_type, resource_id, level, specific)
            .await
            .map_err(|_| {
                BackupError::Validation(
                    "The Backup run-as identity is no longer authorized for this operation."
                        .to_owned(),
                )
            })
    }

    async fn authorize_source(
        &self,
        principal: &ActorPrincipal,
        source: &citadel_backups::spec::BackupSourceSpec,
    ) -> Result<(), BackupError> {
        source.validate()?;
        let resource = source.resource();
        if let Some((resource_type, resource_id)) = resource {
            self.authorize_resource(
                principal,
                resource_type,
                resource_id,
                PermissionLevel::Read,
                None,
            )
            .await?;
        }
        Ok(())
    }
}

impl BackupRunAuthorizer for IdentityBackupRunAuthorizer {
    fn authorize_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let principal = self
                .identity
                .execution_principal(ActorId::new(claim.policy.run_as_actor_id))
                .await
                .map_err(|_| {
                    BackupError::Validation(
                        "The Backup run-as identity is unavailable or disabled.".to_owned(),
                    )
                })?;
            self.authorize_resource(
                &principal,
                ResourceType::BackupPolicy,
                claim.policy.id,
                PermissionLevel::Execute,
                None,
            )
            .await?;
            self.authorize_resource(
                &principal,
                ResourceType::BackupRepository,
                claim.repository.id,
                PermissionLevel::Execute,
                None,
            )
            .await?;
            self.authorize_source(&principal, &claim.run.source_snapshot)
                .await
        })
    }

    fn authorize_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let principal = self
                .identity
                .execution_principal(ActorId::new(claim.run.triggered_by_actor_id))
                .await
                .map_err(|_| {
                    BackupError::Validation(
                        "The Restore identity is unavailable or disabled.".to_owned(),
                    )
                })?;
            self.identity
                .require_resource::<citadel_backups::permissions::RestoreBackupPolicy>(
                    &principal,
                    claim.source.backup_policy_id,
                )
                .await
                .map_err(|_| {
                    BackupError::Validation(
                        "The Backup run-as identity is no longer authorized for this operation."
                            .to_owned(),
                    )
                })?;
            self.authorize_resource(
                &principal,
                ResourceType::Platform,
                claim.run.target_platform_id,
                PermissionLevel::Write,
                None,
            )
            .await
        })
    }
}
