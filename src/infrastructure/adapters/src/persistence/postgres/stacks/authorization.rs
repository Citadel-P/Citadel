use super::*;
use crate::persistence::postgres::authorization::EFFECTIVE_RESOURCE_PERMISSION;

pub(super) async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
    requirement: PermissionRequirement,
) -> Result<(), StackError> {
    if administrator {
        return Ok(());
    }
    let (actual, specs) = sqlx::query_as::<_, (i32, i32)>(EFFECTIVE_RESOURCE_PERMISSION)
        .bind(actor.value())
        .bind(citadel_primitives::ResourceType::Stack as i32)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if !decode_permission(false, actual, specs)?.allows(requirement) {
        return Err(StackError::Forbidden);
    }
    Ok(())
}

pub(super) async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<citadel_platforms::PlatformKind, StackError> {
    let row = sqlx::query("SELECT platformdescriptor FROM platforms WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(StackError::NotFound)?;
    if !administrator {
        let allowed:bool=sqlx::query_scalar(concat!(include_str!("../enabled_actor_scope.sql"), r#"
SELECT EXISTS(
                SELECT 1 FROM actor_scope scope
                JOIN actorroles assignment ON assignment.actorid=scope.actorid
                JOIN permissions permission ON permission.roleid=assignment.roleid
                WHERE permission.resourcetype=0 AND permission.permissionlevel>=1
                UNION ALL
                SELECT 1 FROM actor_scope scope
                JOIN resourceaccesses access ON access.actorid=scope.actorid
                WHERE access.resourcetype=0 AND access.resourceid=$2 AND access.permissionlevel>=1)"#))
            .bind(actor.value()).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
        if !allowed {
            return Err(StackError::NotFound);
        }
    }
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let kind = descriptor
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or("Docker");
    crate::persistence::postgres::platforms::classification::platform_kind(kind).map_err(|_| {
        StackError::Validation("Stacks require a Docker or Docker Swarm Platform.".to_owned())
    })
}
