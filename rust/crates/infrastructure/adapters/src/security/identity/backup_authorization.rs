use std::sync::Arc;

use citadel_backups::{BackupClaim, BackupError, BackupRunAuthorizer, RestoreClaim};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde_json::Value;
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
        source: &Value,
    ) -> Result<(), BackupError> {
        let resource = match source.get("$type").and_then(Value::as_str) {
            Some("DockerVolume") => {
                json_uuid(source, "platformId").map(|id| (ResourceType::Platform, id))
            }
            Some("Stack") => json_uuid(source, "stackId").map(|id| (ResourceType::Stack, id)),
            Some("Deployment") => {
                json_uuid(source, "deploymentId").map(|id| (ResourceType::Deployment, id))
            }
            Some("SwarmService") => {
                json_uuid(source, "swarmServiceId").map(|id| (ResourceType::SwarmService, id))
            }
            Some("CitadelSystem") => None,
            _ => {
                return Err(BackupError::Validation(
                    "The Backup source is invalid.".to_owned(),
                ));
            }
        };
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
            self.authorize_resource(
                &principal,
                ResourceType::BackupPolicy,
                claim.source.backup_policy_id,
                PermissionLevel::Execute,
                Some(SpecificPermission::Restore),
            )
            .await?;
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

fn json_uuid(value: &Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}
