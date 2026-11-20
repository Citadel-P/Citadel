use super::*;
use crate::persistence::postgres::authorization::EFFECTIVE_RESOURCE_PERMISSION;
pub(super) async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    resource_id: Uuid,
    requirement: PermissionRequirement,
) -> Result<(), DeploymentError> {
    if requirement.specific.is_some() {
        return ensure_specific_access(tx, actor_id, administrator, resource_id, requirement).await;
    }
    if administrator
        || has_resource_access(
            tx,
            actor_id,
            requirement.resource_type as i32,
            resource_id,
            requirement.level,
        )
        .await?
    {
        Ok(())
    } else {
        Err(DeploymentError::Forbidden)
    }
}

pub(super) async fn ensure_specific_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    resource_id: Uuid,
    requirement: PermissionRequirement,
) -> Result<(), DeploymentError> {
    if administrator {
        return Ok(());
    }
    let permission = effective_permission(
        tx,
        actor_id,
        requirement.resource_type as i32,
        Some(resource_id),
    )
    .await?;
    if permission.allows(requirement) {
        Ok(())
    } else {
        Err(DeploymentError::Forbidden)
    }
}

pub(super) async fn require_duplicate_binding_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    source_id: Uuid,
) -> Result<(), DeploymentError> {
    if administrator {
        return Ok(());
    }
    let has_bindings = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM resourcebindings WHERE scope='Deployment' AND resourceid=$1)",
    )
    .bind(source_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if !has_bindings {
        return Ok(());
    }
    let source_permission =
        effective_permission(tx, actor_id, DEPLOYMENT_RESOURCE_TYPE, Some(source_id)).await?;
    let target_permission =
        effective_permission(tx, actor_id, DEPLOYMENT_RESOURCE_TYPE, None).await?;
    if !source_permission.allows(ReadDeploymentBindings::REQUIREMENT) {
        return Err(DeploymentError::Forbidden);
    }
    if !target_permission
        .allows(citadel_deployments::permissions::WriteDeploymentBindings::REQUIREMENT)
    {
        return Err(DeploymentError::Forbidden);
    }
    Ok(())
}

pub(super) async fn effective_permission(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    resource_type: i32,
    resource_id: Option<Uuid>,
) -> Result<EffectivePermission, DeploymentError> {
    let (level, specifics) = sqlx::query_as::<_, (i32, i32)>(EFFECTIVE_RESOURCE_PERMISSION)
        .bind(actor_id.value())
        .bind(resource_type)
        .bind(resource_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    decode_permission(false, level, specifics)
}

pub(super) async fn has_resource_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    resource_type: i32,
    resource_id: Uuid,
    required: PermissionLevel,
) -> Result<bool, DeploymentError> {
    sqlx::query_scalar::<_, bool>(
        concat!(include_str!("../enabled_actor_scope.sql"), r#"
SELECT EXISTS(
               SELECT 1 FROM actor_scope scope
               JOIN actorroles assignment ON assignment.actorid=scope.actorid
               JOIN permissions permission ON permission.roleid=assignment.roleid
               WHERE permission.resourcetype=$2 AND permission.permissionlevel = ANY($4)
               UNION ALL
               SELECT 1 FROM actor_scope scope
               JOIN resourceaccesses access ON access.actorid=scope.actorid
               WHERE access.resourcetype=$2 AND access.resourceid=$3 AND access.permissionlevel = ANY($4))"#),
    )
    .bind(actor_id.value())
    .bind(resource_type)
    .bind(resource_id)
    .bind(required.accepted_database_levels())
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}
